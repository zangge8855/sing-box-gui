use crate::config::{get_app_dir, get_profile_path};
use crate::state::GuiConfig;
use futures::StreamExt;
use std::fs::{self, File};
#[cfg(target_os = "windows")]
use std::io;
use std::io::{BufReader, Read};
#[cfg(target_os = "windows")]
use std::io::{Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

static CURRENT_PROCESS: Mutex<Option<Child>> = Mutex::new(None);
/// Set when the user (or app shutdown) intentionally stops the core.
static INTENTIONAL_STOP: AtomicBool = AtomicBool::new(false);
/// Message captured when the core dies without an intentional stop.
static LAST_UNEXPECTED_EXIT: Mutex<Option<String>> = Mutex::new(None);
/// Single source of truth for the sing-box core version this GUI downloads.
pub const CORE_VERSION: &str = "1.13.14";
/// Cached running flag, updated by `is_core_running_locked` so that the UI Tick
/// can poll without contending on `CURRENT_PROCESS` (which `start_core` holds
/// for up to `STARTUP_GRACE_MS` while waiting for the child to survive).
static CORE_RUNNING_CACHED: AtomicBool = AtomicBool::new(false);
static CORE_DOWNLOAD_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

/// How long to wait after spawn before treating the core as "started".
/// Catches immediate FATAL exits (bad config, rule-set init, port bind, …).
pub const STARTUP_GRACE_MS: u64 = 3000;
/// Max time `stop_core` waits for the child to exit gracefully before forcing.
/// Lets sing-box flush its cache file and tear down TUN/sysproxy state.
fn get_process_lock() -> std::sync::MutexGuard<'static, Option<Child>> {
    CURRENT_PROCESS.lock().unwrap_or_else(|e| e.into_inner())
}

/// Build a user-visible error from core exit status + captured log lines.
/// Prefers FATAL/ERROR/FAILED lines; falls back to the last few lines.
pub fn format_core_early_exit_error(exit_code: Option<i32>, lines: &[String]) -> String {
    let interesting: Vec<&str> = lines
        .iter()
        .map(|s| s.as_str())
        .filter(|l| {
            let u = l.to_ascii_uppercase();
            u.contains("FATAL") || u.contains("ERROR") || u.contains("FAILED")
        })
        .collect();
    let body = if !interesting.is_empty() {
        interesting.join("\n")
    } else if !lines.is_empty() {
        let start = lines.len().saturating_sub(8);
        lines[start..].join("\n")
    } else {
        "no core output captured".to_string()
    };
    match exit_code {
        Some(c) => format!("Core exited during startup (code {c}):\n{body}"),
        None => format!("Core exited during startup:\n{body}"),
    }
}

fn format_unexpected_exit(exit_code: Option<i32>) -> String {
    match exit_code {
        Some(c) => format!(
            "Core process exited unexpectedly (code {c}). Check logs for FATAL/ERROR details."
        ),
        None => "Core process exited unexpectedly. Check logs for FATAL/ERROR details.".to_string(),
    }
}

fn decode_log_line(bytes: &[u8]) -> String {
    // Try UTF-8 first
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    // Fallback to GBK on Windows, or default lossy UTF-8
    #[cfg(target_os = "windows")]
    {
        let (cow, _, had_errors) = encoding_rs::GBK.decode(bytes);
        if !had_errors {
            return cow.into_owned();
        }
    }
    String::from_utf8_lossy(bytes).into_owned()
}

fn spawn_log_forwarder<R: Read + Send + 'static>(
    pipe: R,
    sender: Sender<String>,
    early_buf: Arc<Mutex<Vec<String>>>,
) {
    thread::spawn(move || {
        let mut reader = BufReader::new(pipe);
        const MAX_CORE_LOG_LINE_BYTES: usize = 64 * 1024;
        let emit = |buf: &mut Vec<u8>, truncated: bool| {
            while buf.ends_with(b"\r") {
                buf.pop();
            }
            let mut line_str = decode_log_line(buf);
            if truncated {
                line_str.push_str(" … [truncated]");
            }
            if let Ok(mut buf_guard) = early_buf.lock() {
                // Ring buffer capped at 500 lines. Once full we drop the
                // oldest entries so the final lines on a FATAL early-exit
                // (whichever are closer to the actual error) stay visible
                // — no silent loss mid-grace. Drop in chunks to amortize.
                if buf_guard.len() >= 500 {
                    let drop = buf_guard.len().saturating_sub(400) + 50;
                    buf_guard.drain(..drop);
                }
                buf_guard.push(line_str.clone());
            }
            let _ = sender.try_send(line_str);
        };

        let mut pending = Vec::with_capacity(4096);
        let mut chunk = [0u8; 8192];
        let mut truncated = false;
        loop {
            let n = match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };
            let mut start = 0usize;
            for (index, byte) in chunk[..n].iter().enumerate() {
                if *byte != b'\n' {
                    continue;
                }
                if !truncated {
                    let slice = &chunk[start..index];
                    let available = MAX_CORE_LOG_LINE_BYTES.saturating_sub(pending.len());
                    pending.extend_from_slice(&slice[..slice.len().min(available)]);
                    truncated = slice.len() > available;
                }
                emit(&mut pending, truncated);
                pending.clear();
                truncated = false;
                start = index + 1;
            }
            if start < n && !truncated {
                let slice = &chunk[start..n];
                let available = MAX_CORE_LOG_LINE_BYTES.saturating_sub(pending.len());
                pending.extend_from_slice(&slice[..slice.len().min(available)]);
                truncated = slice.len() > available;
            }
        }
        if !pending.is_empty() || truncated {
            emit(&mut pending, truncated);
        }
    });
}

/// Poll `child` until it exits or `grace_ms` elapses.
/// On early exit, returns Err with formatted core output from `early_buf`.
pub fn wait_core_startup_grace(
    child: &mut Child,
    early_buf: &Mutex<Vec<String>>,
    grace_ms: u64,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_millis(grace_ms);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Let pipe reader threads finish draining after process exit.
                thread::sleep(Duration::from_millis(150));
                let lines = early_buf.lock().map(|g| g.clone()).unwrap_or_default();
                return Err(format_core_early_exit_error(status.code(), &lines));
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                return Err(format!("Failed to poll core process: {e}"));
            }
        }
    }
}

fn get_last_unexpected_exit_lock() -> std::sync::MutexGuard<'static, Option<String>> {
    LAST_UNEXPECTED_EXIT
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Take and clear any unexpected-exit message recorded by `is_core_running`.
pub fn take_unexpected_core_exit() -> Option<String> {
    get_last_unexpected_exit_lock().take()
}

pub fn cleanup_stale_temp_binaries() {
    let bin_dir = get_app_dir().join("bin");
    if let Ok(entries) = std::fs::read_dir(&bin_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && (name.starts_with(".sing-box.") || name.starts_with(".sing-box-"))
            {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn bind_child_to_job_object(child: &std::process::Child) -> Result<(), String> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::JobObjects::*;

    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job == 0 {
            return Err("Failed to create JobObject".to_string());
        }
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let set_res = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        if set_res == 0 {
            return Err("Failed to set JobObject info".to_string());
        }
        let assign_res = AssignProcessToJobObject(job, child.as_raw_handle() as HANDLE);
        if assign_res == 0 {
            return Err("Failed to assign process to JobObject".to_string());
        }
    }
    Ok(())
}

pub fn get_core_filename() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "sing-box.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "sing-box"
    }
}

pub fn get_core_path(gui_config: &GuiConfig) -> PathBuf {
    if let Some(ref path) = gui_config.core_path {
        let p = PathBuf::from(path);
        if p.exists() {
            return p;
        }
    }
    get_app_dir().join("bin").join(get_core_filename())
}

pub fn is_core_installed(gui_config: &GuiConfig) -> bool {
    get_core_path(gui_config).exists()
}

const MAX_CORE_ARCHIVE_BYTES: usize = 256 * 1024 * 1024;
const MAX_CORE_BINARY_BYTES: u64 = 128 * 1024 * 1024;

fn unique_sibling_path(dest_path: &Path, label: &str) -> PathBuf {
    let file_name = dest_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("sing-box");
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    dest_path.with_file_name(format!(
        ".{file_name}.{label}-{}-{nonce}",
        std::process::id()
    ))
}

#[cfg(test)]
fn staged_core_path(dest_path: &Path) -> PathBuf {
    let file_name = dest_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("sing-box");
    dest_path.with_file_name(format!("{file_name}.new"))
}

fn install_staged_core(staged_path: &Path, dest_path: &Path) -> Result<(), String> {
    let backup_path = unique_sibling_path(dest_path, "backup");
    if dest_path.exists() {
        fs::rename(dest_path, &backup_path)
            .map_err(|e| format!("Failed to stage the existing core for replacement: {e}"))?;
    }
    if let Err(error) = fs::rename(staged_path, dest_path) {
        if backup_path.exists()
            && let Err(restore_error) = fs::rename(&backup_path, dest_path)
        {
            return Err(format!(
                "Failed to install the downloaded core: {error}; rollback also failed: {restore_error}"
            ));
        }
        return Err(format!("Failed to install the downloaded core: {error}"));
    }
    let _ = fs::remove_file(backup_path);
    Ok(())
}

fn normalize_sha256_digest(digest: &str) -> Result<String, String> {
    let hex = digest
        .strip_prefix("sha256:")
        .ok_or_else(|| "Release asset is missing a SHA-256 digest".to_string())?;
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Release asset contains an invalid SHA-256 digest".to_string());
    }
    Ok(hex.to_ascii_lowercase())
}

fn is_official_core_asset_url(url: &str, version: &str, asset_name: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    if parsed.scheme() != "https" || parsed.host_str() != Some("github.com") {
        return false;
    }
    let mut segments = match parsed.path_segments() {
        Some(segments) => segments,
        None => return false,
    };
    let expected_tag = format!("v{version}");
    matches!(segments.next(), Some("SagerNet"))
        && matches!(segments.next(), Some("sing-box"))
        && matches!(segments.next(), Some("releases"))
        && matches!(segments.next(), Some("download"))
        && segments.next().is_some_and(|tag| tag == expected_tag)
        && segments.next().is_some_and(|name| name == asset_name)
        && segments.next().is_none()
}

fn is_safe_core_entry(path: &Path, expected_basename: &str) -> bool {
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return false;
    }
    path.file_name().and_then(|name| name.to_str()) == Some(expected_basename)
}

pub(crate) fn validate_binary_magic(path: &Path) -> Result<(), String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to inspect binary: {e}"))?;
    let mut magic = [0u8; 4];
    let read = file
        .read(&mut magic)
        .map_err(|e| format!("Failed to read binary header: {e}"))?;

    #[cfg(target_os = "windows")]
    let valid = if read >= 2 && &magic[..2] == b"MZ" {
        let mut offset_bytes = [0u8; 4];
        file.seek(SeekFrom::Start(0x3c)).is_ok() && file.read_exact(&mut offset_bytes).is_ok() && {
            let pe_offset = u32::from_le_bytes(offset_bytes) as u64;
            let mut signature = [0u8; 4];
            pe_offset <= 16 * 1024 * 1024
                && file.seek(SeekFrom::Start(pe_offset)).is_ok()
                && file.read_exact(&mut signature).is_ok()
                && signature == *b"PE\0\0"
        }
    } else {
        false
    };
    #[cfg(target_os = "linux")]
    let valid = read == 4 && magic == [0x7f, b'E', b'L', b'F'];
    #[cfg(target_os = "macos")]
    let valid = read == 4
        && matches!(
            magic,
            [0xfe, 0xed, 0xfa, 0xce]
                | [0xce, 0xfa, 0xed, 0xfe]
                | [0xfe, 0xed, 0xfa, 0xcf]
                | [0xcf, 0xfa, 0xed, 0xfe]
                | [0xca, 0xfe, 0xba, 0xbe]
                | [0xbe, 0xba, 0xfe, 0xca]
                | [0xca, 0xfe, 0xba, 0xbf]
                | [0xbf, 0xba, 0xfe, 0xca]
        );
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    let valid = read >= 2;

    valid
        .then_some(())
        .ok_or_else(|| "Downloaded file does not match this platform's binary format".to_string())
}

#[derive(serde::Deserialize)]
struct CoreReleaseAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    digest: Option<String>,
    size: u64,
}

#[derive(serde::Deserialize)]
struct CoreRelease {
    #[serde(default)]
    assets: Vec<CoreReleaseAsset>,
}

/// Download official sing-box into the managed bin folder.
/// When `force` is true, replace an existing binary (reinstall / upgrade pin).
pub async fn download_core(progress_sender: Sender<String>, force: bool) -> Result<(), String> {
    let download_lock = CORE_DOWNLOAD_LOCK.get_or_init(|| tokio::sync::Mutex::new(()));
    let _guard = download_lock
        .try_lock()
        .map_err(|_| "A core download is already in progress".to_string())?;
    if is_core_running() {
        return Err("Stop the core before reinstalling or replacing it".to_string());
    }

    let app_dir = get_app_dir();
    let bin_dir = app_dir.join("bin");
    fs::create_dir_all(&bin_dir).map_err(|e| format!("Failed to create bin directory: {}", e))?;
    let dest_path = bin_dir.join(get_core_filename());

    if dest_path.exists() && !force {
        return Ok(());
    }

    let _ = progress_sender.try_send("Downloading sing-box core...".to_string());

    let version = CORE_VERSION;

    #[cfg(target_os = "windows")]
    let asset_name = {
        #[cfg(target_arch = "aarch64")]
        let arch = "windows-arm64";
        #[cfg(not(target_arch = "aarch64"))]
        let arch = "windows-amd64";
        format!("sing-box-{version}-{arch}.zip")
    };

    #[cfg(target_os = "macos")]
    let asset_name = {
        #[cfg(target_arch = "aarch64")]
        let arch = "darwin-arm64";
        #[cfg(not(target_arch = "aarch64"))]
        let arch = "darwin-amd64";
        format!("sing-box-{version}-{arch}.tar.gz")
    };

    #[cfg(target_os = "linux")]
    let asset_name = {
        #[cfg(target_arch = "aarch64")]
        let arch = "linux-arm64";
        #[cfg(not(target_arch = "aarch64"))]
        let arch = "linux-amd64";
        format!("sing-box-{version}-{arch}.tar.gz")
    };

    let temp_archive_path = unique_sibling_path(&dest_path, "archive");
    let staged_path = unique_sibling_path(&dest_path, "staged");

    let res = async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| format!("Failed to build core download client: {e}"))?;
        let metadata_response = client
            .get(format!(
                "https://api.github.com/repos/SagerNet/sing-box/releases/tags/v{version}"
            ))
            .header("User-Agent", "sing-box-gui")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch core release metadata: {e}"))?;
        if !metadata_response.status().is_success() {
            return Err(format!(
                "Core release metadata returned status {}",
                metadata_response.status()
            ));
        }
        let release: CoreRelease = metadata_response
            .json()
            .await
            .map_err(|e| format!("Failed to parse core release metadata: {e}"))?;
        let asset = release
            .assets
            .into_iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("Official core asset not found: {asset_name}"))?;
        if !is_official_core_asset_url(&asset.browser_download_url, version, &asset_name) {
            return Err("Core release asset URL is not an official sing-box download".to_string());
        }
        if asset.size == 0 || asset.size > MAX_CORE_ARCHIVE_BYTES as u64 {
            return Err("Core release asset has an invalid size".to_string());
        }
        let expected_digest = normalize_sha256_digest(
            asset
                .digest
                .as_deref()
                .ok_or_else(|| "Core release asset has no SHA-256 digest".to_string())?,
        )?;
        let response = client
            .get(&asset.browser_download_url)
            .header("User-Agent", "sing-box-gui")
            .send()
            .await
            .map_err(|e| format!("Failed to download core: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Server returned error: {}", response.status()));
        }

        if response
            .content_length()
            .is_some_and(|size| size != asset.size)
        {
            return Err("Core download size does not match release metadata".to_string());
        }

        use sha2::{Digest, Sha256};
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_archive_path)
            .await
            .map_err(|e| format!("Failed to create temp archive file: {}", e))?;

        let mut stream = response.bytes_stream();
        let mut downloaded = 0usize;
        let mut hasher = Sha256::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Failed to read chunk: {}", e))?;
            downloaded = downloaded.saturating_add(chunk.len());
            if downloaded > MAX_CORE_ARCHIVE_BYTES {
                return Err("Downloaded core archive exceeds the 256 MiB safety limit".to_string());
            }
            hasher.update(&chunk);
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to write chunk: {}", e))?;
        }
        file.flush()
            .await
            .map_err(|e| format!("Failed to flush core archive: {e}"))?;
        drop(file);

        let _ = progress_sender.try_send("Verifying core...".to_string());
        if downloaded as u64 != asset.size {
            return Err(format!(
                "Core download size mismatch: expected {}, received {}",
                asset.size, downloaded
            ));
        }
        let actual_digest = format!("{:x}", hasher.finalize());
        if actual_digest != expected_digest {
            return Err("Core archive SHA-256 verification failed".to_string());
        }

        let _ = progress_sender.try_send("Extracting core...".to_string());

        #[cfg(target_os = "windows")]
        {
            let archive_path = temp_archive_path.clone();
            let output_path = staged_path.clone();
            tokio::task::spawn_blocking(move || {
                let zip_file = File::open(&archive_path)
                    .map_err(|e| format!("Failed to open temp zip: {e}"))?;
                let mut archive = zip::ZipArchive::new(zip_file)
                    .map_err(|e| format!("Invalid zip archive: {e}"))?;

                let mut matches = 0usize;
                for index in 0..archive.len() {
                    let mut entry = archive
                        .by_index(index)
                        .map_err(|e| format!("Failed to read zip entry: {e}"))?;
                    let Some(path) = entry.enclosed_name() else {
                        continue;
                    };
                    let is_symlink = entry
                        .unix_mode()
                        .is_some_and(|mode| mode & 0o170000 == 0o120000);
                    if !entry.is_file()
                        || is_symlink
                        || !is_safe_core_entry(&path, get_core_filename())
                    {
                        continue;
                    }
                    matches += 1;
                    if matches > 1 {
                        return Err(
                            "Core archive contains ambiguous duplicate binaries".to_string()
                        );
                    }
                    if entry.size() == 0 || entry.size() > MAX_CORE_BINARY_BYTES {
                        return Err("Downloaded core binary has an invalid size".to_string());
                    }
                    let mut output = fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&output_path)
                        .map_err(|e| format!("Failed to create staged core: {e}"))?;
                    io::copy(&mut entry, &mut output)
                        .map_err(|e| format!("Failed to extract core: {e}"))?;
                    output
                        .sync_all()
                        .map_err(|e| format!("Failed to flush staged core: {e}"))?;
                }
                (matches == 1).then_some(()).ok_or_else(|| {
                    format!(
                        "Could not find {} inside downloaded zip package",
                        get_core_filename()
                    )
                })
            })
            .await
            .map_err(|e| format!("Core extraction task failed: {e}"))??;
        }

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let archive_path = temp_archive_path.clone();
            let output_path = staged_path.clone();
            tokio::task::spawn_blocking(move || {
                let tar_gz = File::open(&archive_path)
                    .map_err(|e| format!("Failed to open temp archive: {e}"))?;
                let gz = flate2::read::GzDecoder::new(tar_gz);
                let mut archive = tar::Archive::new(gz);

                let mut matches = 0usize;
                for entry in archive
                    .entries()
                    .map_err(|e| format!("Failed to inspect tar.gz: {e}"))?
                {
                    let mut entry = entry.map_err(|e| format!("Failed to read tar entry: {e}"))?;
                    let path = entry
                        .path()
                        .map_err(|e| format!("Invalid tar entry path: {e}"))?;
                    let is_core = entry.header().entry_type().is_file()
                        && is_safe_core_entry(&path, "sing-box");
                    if !is_core {
                        continue;
                    }
                    matches += 1;
                    if matches > 1 {
                        return Err(
                            "Core archive contains ambiguous duplicate binaries".to_string()
                        );
                    }
                    let size = entry.header().size().unwrap_or(0);
                    if size == 0 || size > MAX_CORE_BINARY_BYTES {
                        return Err("Downloaded core binary has an invalid size".to_string());
                    }
                    let mut output = fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&output_path)
                        .map_err(|e| format!("Failed to create staged core: {e}"))?;
                    std::io::copy(&mut entry, &mut output)
                        .map_err(|e| format!("Failed to extract core: {e}"))?;
                    output
                        .sync_all()
                        .map_err(|e| format!("Failed to flush staged core: {e}"))?;

                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        fs::set_permissions(&output_path, fs::Permissions::from_mode(0o755))
                            .map_err(|e| format!("Failed to mark staged core executable: {e}"))?;
                    }
                }
                (matches == 1).then_some(()).ok_or_else(|| {
                    "Could not find sing-box inside downloaded tar package".to_string()
                })
            })
            .await
            .map_err(|e| format!("Core extraction task failed: {e}"))??;
        }

        let staged_size = fs::metadata(&staged_path)
            .map_err(|e| format!("Failed to verify staged core: {e}"))?
            .len();
        if staged_size == 0 || staged_size > MAX_CORE_BINARY_BYTES {
            return Err("Downloaded core binary has an invalid size".to_string());
        }
        validate_binary_magic(&staged_path)?;

        let _ = progress_sender.try_send("Installing core...".to_string());
        install_staged_core(&staged_path, &dest_path)?;

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            if let Ok(metadata) = fs::metadata(&dest_path) {
                use std::os::unix::fs::PermissionsExt;
                let mut permissions = metadata.permissions();
                permissions.set_mode(0o755);
                let _ = fs::set_permissions(&dest_path, permissions);
            }
        }

        Ok(())
    }
    .await;

    let _ = fs::remove_file(&temp_archive_path);
    let _ = fs::remove_file(&staged_path);

    match res {
        Ok(_) => {
            let _ = progress_sender.try_send("Core installed successfully!".to_string());
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Build and write `run_config.json` from the active profile + GUI settings.
pub fn prepare_run_config(gui_config: &GuiConfig) -> Result<PathBuf, String> {
    let active_id = gui_config
        .active_profile_id
        .as_ref()
        .ok_or_else(|| "No active profile selected".to_string())?;

    let config_path = get_profile_path(active_id);
    if !config_path.exists() {
        return Err("Active profile config file not found!".to_string());
    }

    let profile_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read profile file: {}", e))?;

    let trimmed = profile_content.trim();
    let mut final_config = if trimmed.starts_with('{') || trimmed.starts_with('[') {
        crate::config::merge_native_json_profile(&profile_content, gui_config)?
    } else {
        crate::config::convert_clash_to_singbox(&profile_content, gui_config)?
    };

    // Fix known startup footguns (broken rule-set proxies, relative cache path).
    crate::config::mitigate_run_config(&mut final_config);

    let run_config_path = get_app_dir().join("run_config.json");
    let run_config_content = serde_json::to_string_pretty(&final_config)
        .map_err(|e| format!("Failed to serialize final config: {}", e))?;
    crate::config::atomic_write(&run_config_path, run_config_content.as_bytes())?;
    Ok(run_config_path)
}

/// Run `sing-box check -c run_config.json` after generating config.
/// Also used internally by `start_core`; public for Settings “validate” hooks.
#[allow(dead_code)]
pub fn check_core_config(gui_config: &GuiConfig) -> Result<(), String> {
    let core_path = get_core_path(gui_config);
    if !core_path.exists() {
        return Err(
            "sing-box core not found! Please download it or specify correct path in Settings."
                .to_string(),
        );
    }
    let run_config_path = prepare_run_config(gui_config)?;
    let mut cmd = Command::new(&core_path);
    cmd.args(["check", "-c", &run_config_path.to_string_lossy()]);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run sing-box check: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let msg = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else if !stdout.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            "sing-box check failed".to_string()
        };
        Err(msg)
    }
}

pub fn start_core(gui_config: &GuiConfig, log_sender: Sender<String>) -> Result<(), String> {
    // Reset the intentional-stop flag up front so a failed restart does not
    // leave a stale flag lingering when no child is being tracked.
    INTENTIONAL_STOP.store(false, Ordering::SeqCst);
    {
        let mut lock = get_process_lock();
        if lock.is_some() {
            // Already tracking a child — only treat as running if it is still alive.
            if is_core_running_locked(&mut lock) {
                return Ok(());
            }
        }
    }

    let core_path = get_core_path(gui_config);
    if !core_path.exists() {
        return Err(
            "sing-box core not found! Please download it or specify correct path in Settings."
                .to_string(),
        );
    }

    // Validate generated config before spawning the process.
    // Spawn/check failures are hard errors — never skip validation silently.
    let run_config_path = prepare_run_config(gui_config)?;
    {
        let mut check_cmd = Command::new(&core_path);
        check_cmd.args(["check", "-c", &run_config_path.to_string_lossy()]);
        check_cmd.stdout(Stdio::piped());
        check_cmd.stderr(Stdio::piped());
        #[cfg(target_os = "windows")]
        check_cmd.creation_flags(0x08000000);
        let output = check_cmd
            .output()
            .map_err(|e| format!("Failed to run config check: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let msg = if !stderr.trim().is_empty() {
                stderr.trim().to_string()
            } else if !stdout.trim().is_empty() {
                stdout.trim().to_string()
            } else {
                "Configuration check failed".to_string()
            };
            return Err(format!("Config check failed: {}", msg));
        }
    }

    let app_dir = get_app_dir();
    let mut cmd = Command::new(&core_path);
    cmd.args(["run", "-c", &run_config_path.to_string_lossy()]);
    cmd.current_dir(&app_dir);
    cmd.env("ENABLE_DEPRECATED_LEGACY_DNS_SERVERS", "true");
    cmd.env("ENABLE_DEPRECATED_MISSING_DOMAIN_RESOLVER", "true");
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start process: {}", e))?;

    #[cfg(target_os = "windows")]
    let _ = bind_child_to_job_object(&child);

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture stderr".to_string())?;

    let early_buf = Arc::new(Mutex::new(Vec::<String>::new()));
    spawn_log_forwarder(stdout, log_sender.clone(), Arc::clone(&early_buf));
    spawn_log_forwarder(stderr, log_sender, Arc::clone(&early_buf));

    // Do not advertise success until the process survives the grace window.
    // On early exit, surface FATAL/ERROR text instead of a silent UI flip.
    if let Err(e) = wait_core_startup_grace(&mut child, &early_buf, STARTUP_GRACE_MS) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(e);
    }

    INTENTIONAL_STOP.store(false, Ordering::SeqCst);
    {
        let mut g = get_last_unexpected_exit_lock();
        *g = None;
    }
    CORE_RUNNING_CACHED.store(true, Ordering::SeqCst);
    let mut lock = get_process_lock();
    *lock = Some(child);
    Ok(())
}

pub fn stop_core() {
    INTENTIONAL_STOP.store(true, Ordering::SeqCst);
    CORE_RUNNING_CACHED.store(false, Ordering::SeqCst);
    let mut lock = get_process_lock();
    if let Some(child) = lock.as_mut() {
        match child.try_wait() {
            Ok(Some(_)) => {
                lock.take();
                return;
            }
            Ok(None) => {}
            Err(error) => {
                let mut g = get_last_unexpected_exit_lock();
                *g = Some(format!("Failed to inspect core before stopping: {error}"));
            }
        }
        #[cfg(unix)]
        {
            let pid = child.id() as libc::pid_t;
            unsafe {
                let result = libc::kill(pid, libc::SIGTERM);
                if result != 0 {
                    let error = std::io::Error::last_os_error();
                    // ESRCH means the process exited between try_wait and
                    // kill; it is a successful stop from the user's point of
                    // view. Other errors are retained for diagnostics.
                    if error.raw_os_error() != Some(libc::ESRCH) {
                        let mut g = get_last_unexpected_exit_lock();
                        *g = Some(format!("Failed to terminate core: {error}"));
                    }
                }
            }
            let start = Instant::now();
            let mut exited = false;
            while start.elapsed() < Duration::from_secs(2) {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        exited = true;
                        break;
                    }
                    Ok(None) => {
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(error) => {
                        let mut g = get_last_unexpected_exit_lock();
                        *g = Some(format!("Failed while waiting for core: {error}"));
                        break;
                    }
                }
            }
            if !exited {
                let _ = child.kill();
            }
        }
        #[cfg(not(unix))]
        {
            let _ = child.kill();
        }
        let _ = child.wait();
        lock.take();
    }
}

static CONSECUTIVE_TRY_WAIT_ERRORS: AtomicUsize = AtomicUsize::new(0);

fn is_core_running_locked(lock: &mut Option<Child>) -> bool {
    if let Some(ref mut child) = *lock {
        match child.try_wait() {
            Ok(None) => {
                CONSECUTIVE_TRY_WAIT_ERRORS.store(0, Ordering::SeqCst);
                CORE_RUNNING_CACHED.store(true, Ordering::SeqCst);
                true
            }
            Ok(Some(status)) => {
                CONSECUTIVE_TRY_WAIT_ERRORS.store(0, Ordering::SeqCst);
                *lock = None;
                let intentional = INTENTIONAL_STOP.swap(false, Ordering::SeqCst);
                if !intentional {
                    let msg = format_unexpected_exit(status.code());
                    let mut g = get_last_unexpected_exit_lock();
                    *g = Some(msg);
                }
                CORE_RUNNING_CACHED.store(false, Ordering::SeqCst);
                false
            }
            Err(error) => {
                let count = CONSECUTIVE_TRY_WAIT_ERRORS.fetch_add(1, Ordering::SeqCst) + 1;
                let mut g = get_last_unexpected_exit_lock();
                *g = Some(format!("Failed to inspect core process: {error}"));
                if count >= 3 {
                    *lock = None;
                    CORE_RUNNING_CACHED.store(false, Ordering::SeqCst);
                    CONSECUTIVE_TRY_WAIT_ERRORS.store(0, Ordering::SeqCst);
                    false
                } else {
                    CORE_RUNNING_CACHED.store(true, Ordering::SeqCst);
                    true
                }
            }
        }
    } else {
        CORE_RUNNING_CACHED.store(false, Ordering::SeqCst);
        false
    }
}

pub fn is_core_running() -> bool {
    let mut lock = get_process_lock();
    is_core_running_locked(&mut lock)
}

/// Lock-free fast path for UI Tick / background pollers.
/// Returns the cached liveness from the last authoritative check; refresh it by
/// calling [`is_core_running`] (which acquires the process lock, blocking up to
/// `STARTUP_GRACE_MS` while `start_core` is in flight). Use this in per-second
/// subscriptions to avoid stalling the GUI thread.
pub fn is_core_running_fast() -> bool {
    CORE_RUNNING_CACHED.load(Ordering::SeqCst)
}

/// Run `sing-box version` and return the first line of stdout.
pub fn get_core_version(gui_config: &GuiConfig) -> Result<String, String> {
    let path = get_core_path(gui_config);
    if !path.exists() {
        return Err("Core not installed".to_string());
    }
    let mut cmd = Command::new(&path);
    cmd.arg("version");
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run core version: {}", e))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let first = text.lines().next().unwrap_or("unknown").trim().to_string();
    if first.is_empty() {
        Err("Empty version output".to_string())
    } else {
        Ok(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    #[test]
    fn is_core_running_fast_reads_cached_flag_without_process() {
        // Fast path must not require a live child — only the atomic cache.
        CORE_RUNNING_CACHED.store(false, Ordering::SeqCst);
        assert!(!is_core_running_fast());
        CORE_RUNNING_CACHED.store(true, Ordering::SeqCst);
        assert!(is_core_running_fast());
        CORE_RUNNING_CACHED.store(false, Ordering::SeqCst);
        assert!(!is_core_running_fast());
    }

    #[test]
    fn staged_core_replacement_preserves_a_working_destination_until_install() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join(get_core_filename());
        let staged = staged_core_path(&dest);
        fs::write(&dest, b"old core").expect("seed old core");
        fs::write(&staged, b"new core").expect("seed staged core");

        install_staged_core(&staged, &dest).expect("install staged core");

        assert_eq!(fs::read(&dest).expect("read installed core"), b"new core");
        assert!(!staged.exists());
        let backup = dest.with_file_name(format!(
            "{}.bak",
            dest.file_name().and_then(|name| name.to_str()).unwrap()
        ));
        assert!(!backup.exists());
    }

    #[test]
    fn digest_parser_fails_closed() {
        assert!(normalize_sha256_digest("sha256:abcd").is_err());
        assert!(normalize_sha256_digest(&format!("sha256:{}", "a".repeat(64))).is_ok());
        assert!(normalize_sha256_digest(&"a".repeat(64)).is_err());
    }

    #[test]
    fn core_asset_url_must_match_official_release_path() {
        let name = "sing-box-1.13.14-windows-amd64.zip";
        assert!(is_official_core_asset_url(
            "https://github.com/SagerNet/sing-box/releases/download/v1.13.14/sing-box-1.13.14-windows-amd64.zip",
            "1.13.14",
            name
        ));
        assert!(!is_official_core_asset_url(
            "https://example.com/SagerNet/sing-box/releases/download/v1.13.14/sing-box-1.13.14-windows-amd64.zip",
            "1.13.14",
            name
        ));
        assert!(!is_official_core_asset_url(
            "https://github.com/SagerNet/sing-box/releases/download/v1.13.13/sing-box-1.13.14-windows-amd64.zip",
            "1.13.14",
            name
        ));
    }

    #[test]
    fn archive_entry_filter_rejects_traversal_and_accepts_exact_basename() {
        assert!(is_safe_core_entry(
            Path::new("sing-box-1.0/sing-box.exe"),
            "sing-box.exe"
        ));
        assert!(!is_safe_core_entry(
            Path::new("../sing-box.exe"),
            "sing-box.exe"
        ));
        assert!(!is_safe_core_entry(
            Path::new("sing-box.exe/other"),
            "sing-box.exe"
        ));
    }

    #[test]
    fn binary_magic_rejects_plain_text() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("candidate");
        fs::write(&path, b"not an executable").unwrap();
        assert!(validate_binary_magic(&path).is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn binary_magic_accepts_pe_signature() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("candidate.exe");
        let mut bytes = vec![0u8; 0x84];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&(0x80u32).to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        fs::write(&path, bytes).unwrap();
        assert!(validate_binary_magic(&path).is_ok());
    }

    #[test]
    fn format_early_exit_prefers_fatal_lines() {
        let lines = vec![
            "TRACE[0000] initialize cache-file...".to_string(),
            "FATAL[0001] start service: initialize rule-set: telegram-ip: unexpected status: 403 Forbidden".to_string(),
            "DEBUG noise".to_string(),
        ];
        let msg = format_core_early_exit_error(Some(1), &lines);
        assert!(msg.contains("code 1"), "msg={msg}");
        assert!(msg.contains("403 Forbidden"), "msg={msg}");
        assert!(msg.contains("FATAL"), "msg={msg}");
        assert!(
            !msg.contains("DEBUG noise"),
            "should not dump non-error noise: {msg}"
        );
    }

    #[test]
    fn format_early_exit_falls_back_when_no_fatal() {
        let lines = vec!["hello".to_string(), "world".to_string()];
        let msg = format_core_early_exit_error(None, &lines);
        assert!(msg.contains("hello"));
        assert!(msg.contains("world"));
    }

    #[test]
    fn wait_grace_reports_early_exit_from_real_child() {
        // Drive the same grace/poll helper start_core uses with a real process
        // that exits promptly. The test harness itself is portable and avoids
        // platform-shell quoting and process-lifecycle differences.
        // Re-run this test harness in list-only mode. It is an actual child
        // process that exits promptly on every supported platform, avoiding
        // shell-specific quoting and lifecycle behavior on Windows runners.
        let mut child = Command::new(std::env::current_exe().expect("test executable path"))
            .arg("--list")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn dying child");

        let early_buf = Arc::new(Mutex::new(Vec::<String>::new()));
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        // Re-use production forwarder path (sender is a throwaway channel).
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(128);
        spawn_log_forwarder(stdout, tx.clone(), Arc::clone(&early_buf));
        spawn_log_forwarder(stderr, tx, Arc::clone(&early_buf));

        let err = wait_core_startup_grace(&mut child, &early_buf, 5000)
            .expect_err("child should exit during grace");
        // Status code is propagated regardless of how much output was captured.
        assert!(err.contains("code 0"), "unexpected error text: {err}");
        assert!(err.contains("startup"), "msg={err}");

        // Drain channel so the forwarder threads can exit cleanly.
        while rx.try_recv().is_ok() {}
    }

    #[test]
    fn wait_grace_ok_when_process_stays_up() {
        #[cfg(target_os = "windows")]
        let mut child = Command::new("cmd")
            .args(["/C", "ping -n 5 127.0.0.1 >nul"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn long child");

        #[cfg(not(target_os = "windows"))]
        let mut child = Command::new("sh")
            .args(["-c", "sleep 5"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn long child");

        let early_buf = Mutex::new(Vec::new());
        let result = wait_core_startup_grace(&mut child, &early_buf, 400);
        let _ = child.kill();
        let _ = child.wait();
        assert!(result.is_ok(), "expected Ok, got {result:?}");
    }

    #[test]
    fn test_decode_log_line() {
        let utf8_bytes = "hello world 国外".as_bytes();
        let decoded = decode_log_line(utf8_bytes);
        assert_eq!(decoded, "hello world 国外");

        #[cfg(target_os = "windows")]
        {
            let gbk_bytes = b"hello \xb9\xfa\xcd\xe2"; // "hello 国外" in GBK
            let decoded_gbk = decode_log_line(gbk_bytes);
            assert_eq!(decoded_gbk, "hello 国外");
        }
    }

    #[test]
    fn log_forwarder_truncates_oversized_unterminated_lines_and_recovers() {
        let mut input = vec![b'x'; 70 * 1024];
        input.extend_from_slice(b"\nok\n");
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(8);
        let early = Arc::new(Mutex::new(Vec::new()));
        spawn_log_forwarder(std::io::Cursor::new(input), tx, early);

        let first = rx.blocking_recv().expect("truncated line");
        let second = rx.blocking_recv().expect("following line");
        assert!(first.ends_with("[truncated]"));
        assert!(first.len() < 66 * 1024);
        assert_eq!(second, "ok");
    }
}
