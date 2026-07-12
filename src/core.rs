use std::fs::{self, File};
use futures::StreamExt;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;
use crate::config::{get_app_dir, get_profile_path};
use crate::state::GuiConfig;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

static CURRENT_PROCESS: Mutex<Option<Child>> = Mutex::new(None);
/// Set when the user (or app shutdown) intentionally stops the core.
static INTENTIONAL_STOP: AtomicBool = AtomicBool::new(false);
/// Message captured when the core dies without an intentional stop.
static LAST_UNEXPECTED_EXIT: Mutex<Option<String>> = Mutex::new(None);
/// Cached running flag, updated by `is_core_running_locked` so that the UI Tick
/// can poll without contending on `CURRENT_PROCESS` (which `start_core` holds
/// for up to `STARTUP_GRACE_MS` while waiting for the child to survive).
static CORE_RUNNING_CACHED: AtomicBool = AtomicBool::new(false);

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
        None => {
            "Core process exited unexpectedly. Check logs for FATAL/ERROR details.".to_string()
        }
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
    sender: UnboundedSender<String>,
    early_buf: Arc<Mutex<Vec<String>>>,
) {
    thread::spawn(move || {
        let mut reader = BufReader::new(pipe);
        let mut buf = Vec::new();
        while let Ok(n) = reader.read_until(b'\n', &mut buf) {
            if n == 0 {
                break; // EOF
            }
            // Trim trailing newline characters (\r and \n)
            while buf.ends_with(b"\n") || buf.ends_with(b"\r") {
                buf.pop();
            }
            let line_str = decode_log_line(&buf);
            buf.clear();
            
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
            let _ = sender.send(line_str);
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

/// Take and clear any unexpected-exit message recorded by `is_core_running`.
pub fn take_unexpected_core_exit() -> Option<String> {
    LAST_UNEXPECTED_EXIT
        .lock()
        .ok()
        .and_then(|mut g| g.take())
}

pub fn get_core_filename() -> &'static str {
    #[cfg(target_os = "windows")]
    { "sing-box.exe" }
    #[cfg(not(target_os = "windows"))]
    { "sing-box" }
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

/// Download official sing-box into the managed bin folder.
/// When `force` is true, replace an existing binary (reinstall / upgrade pin).
pub async fn download_core(
    progress_sender: UnboundedSender<String>,
    force: bool,
) -> Result<(), String> {
    let app_dir = get_app_dir();
    let bin_dir = app_dir.join("bin");
    fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("Failed to create bin directory: {}", e))?;
    let dest_path = bin_dir.join(get_core_filename());
    
    if dest_path.exists() && !force {
        return Ok(());
    }
    if force && dest_path.exists() {
        let _ = progress_sender.send("Removing existing core for reinstall...".to_string());
        fs::remove_file(&dest_path)
            .map_err(|e| format!("Failed to remove existing core: {}", e))?;
    }
    
    let _ = progress_sender.send("Downloading sing-box core...".to_string());
    
    let version = "1.13.14";
    
    #[cfg(target_os = "windows")]
    let (url, archive_name) = {
        #[cfg(target_arch = "aarch64")]
        let arch = "windows-arm64";
        #[cfg(not(target_arch = "aarch64"))]
        let arch = "windows-amd64";
        (
            format!("https://github.com/SagerNet/sing-box/releases/download/v{}/sing-box-{}-{}.zip", version, version, arch),
            "temp_core.zip"
        )
    };
    
    #[cfg(target_os = "macos")]
    let (url, archive_name, arch) = {
        #[cfg(target_arch = "aarch64")]
        let arch = "darwin-arm64";
        #[cfg(not(target_arch = "aarch64"))]
        let arch = "darwin-amd64";
        (
            format!("https://github.com/SagerNet/sing-box/releases/download/v{}/sing-box-{}-{}.tar.gz", version, version, arch),
            "temp_core.tar.gz",
            arch
        )
    };
    
    #[cfg(target_os = "linux")]
    let (url, archive_name, arch) = {
        #[cfg(target_arch = "aarch64")]
        let arch = "linux-arm64";
        #[cfg(not(target_arch = "aarch64"))]
        let arch = "linux-amd64";
        (
            format!("https://github.com/SagerNet/sing-box/releases/download/v{}/sing-box-{}-{}.tar.gz", version, version, arch),
            "temp_core.tar.gz",
            arch
        )
    };
    
    let temp_archive_path = app_dir.join(archive_name);
    
    let res = async {
        let response = reqwest::get(&url).await
            .map_err(|e| format!("Failed to download core: {}", e))?;
            
        if !response.status().is_success() {
            return Err(format!("Server returned error: {}", response.status()));
        }
        
        let mut file = File::create(&temp_archive_path)
            .map_err(|e| format!("Failed to create temp archive file: {}", e))?;
            
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Failed to read chunk: {}", e))?;
            file.write_all(&chunk).map_err(|e| format!("Failed to write chunk: {}", e))?;
        }
            
        let _ = progress_sender.send("Extracting core...".to_string());
        
        #[cfg(target_os = "windows")]
        {
            let zip_file = File::open(&temp_archive_path)
                .map_err(|e| format!("Failed to open temp zip: {}", e))?;
                
            let mut archive = zip::ZipArchive::new(zip_file)
                .map_err(|e| format!("Invalid zip archive: {}", e))?;
                
            let mut extracted = false;
            let core_name = get_core_filename();
            for i in 0..archive.len() {
                let mut file = archive.by_index(i)
                    .map_err(|e| format!("Failed to read zip index: {}", e))?;
                    
                let name = file.name().to_string();
                if name.ends_with(core_name) {
                    let mut outfile = File::create(&dest_path)
                        .map_err(|e| format!("Failed to create target: {}", e))?;
                    io::copy(&mut file, &mut outfile)
                        .map_err(|e| format!("Failed to extract: {}", e))?;
                    extracted = true;
                    break;
                }
            }
            if extracted {
                Ok(())
            } else {
                Err(format!("Could not find {} inside downloaded zip package", core_name))
            }
        }
        
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            // Extract .tar.gz natively (no dependency on the system `tar` binary,
            // works on Alpine / minimal images) and copy the binary into place.
            let tar_gz = File::open(&temp_archive_path)
                .map_err(|e| format!("Failed to open temp archive: {}", e))?;
            let gz = flate2::read::GzDecoder::new(tar_gz);
            let mut archive = tar::Archive::new(gz);

            let extracted_dir_name = format!("sing-box-{}-{}", version, arch);
            let extracted_dir = app_dir.join(&extracted_dir_name);
            // Extract everything under the temp app dir so entry paths are safe.
            archive
                .unpack(&app_dir)
                .map_err(|e| format!("Failed to extract tar.gz: {}", e))?;

            let src_binary = extracted_dir.join("sing-box");
            if src_binary.exists() {
                fs::copy(&src_binary, &dest_path)
                    .map_err(|e| format!("Failed to copy sing-box binary: {}", e))?;
                let _ = fs::remove_dir_all(&extracted_dir);

                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&dest_path, fs::Permissions::from_mode(0o755));
                Ok(())
            } else {
                Err(format!(
                    "Could not find sing-box inside extracted tar folder {}",
                    extracted_dir_name
                ))
            }
        }
    }.await;

    let _ = fs::remove_file(&temp_archive_path);

    match res {
        Ok(_) => {
            let _ = progress_sender.send("Core installed successfully!".to_string());
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
    fs::write(&run_config_path, &run_config_content)
        .map_err(|e| format!("Failed to save final run_config: {}", e))?;
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
    cmd.args([
        "check",
        "-c",
        &run_config_path.to_string_lossy(),
    ]);
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

pub fn start_core(
    gui_config: &GuiConfig,
    log_sender: UnboundedSender<String>,
) -> Result<(), String> {
    let mut lock = get_process_lock();
    if lock.is_some() {
        // Already tracking a child — only treat as running if it is still alive.
        if is_core_running_locked(&mut lock) {
            return Ok(());
        }
    }

    let core_path = get_core_path(gui_config);
    if !core_path.exists() {
        return Err("sing-box core not found! Please download it or specify correct path in Settings.".to_string());
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
    if let Ok(mut g) = LAST_UNEXPECTED_EXIT.lock() {
        *g = None;
    }
    CORE_RUNNING_CACHED.store(true, Ordering::SeqCst);
    *lock = Some(child);
    Ok(())
}

pub fn stop_core() {
    INTENTIONAL_STOP.store(true, Ordering::SeqCst);
    CORE_RUNNING_CACHED.store(false, Ordering::SeqCst);
    let mut lock = get_process_lock();
    if let Some(mut child) = lock.take() {
        // Ask the OS to terminate, then give it a bounded window to flush
        // its cache file, tear down the TUN interface and unwind cleanly
        // before we resort to a forceful kill.
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn is_core_running_locked(lock: &mut Option<Child>) -> bool {
    if let Some(ref mut child) = *lock {
        match child.try_wait() {
            Ok(None) => {
                CORE_RUNNING_CACHED.store(true, Ordering::SeqCst);
                true
            }
            Ok(Some(status)) => {
                *lock = None;
                let intentional = INTENTIONAL_STOP.swap(false, Ordering::SeqCst);
                if !intentional {
                    let msg = format_unexpected_exit(status.code());
                    if let Ok(mut g) = LAST_UNEXPECTED_EXIT.lock() {
                        *g = Some(msg);
                    }
                }
                CORE_RUNNING_CACHED.store(false, Ordering::SeqCst);
                false
            }
            Err(_) => {
                *lock = None;
                CORE_RUNNING_CACHED.store(false, Ordering::SeqCst);
                false
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
        assert!(!msg.contains("DEBUG noise"), "should not dump non-error noise: {msg}");
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
        // Drive the same grace/poll helper start_core uses, with a process that
        // dies immediately. We use a minimal, instantly-exiting command instead
        // of `echo ... 1>&2 & exit 7` because that form races the stderr pipe
        // forwarder on CI runners and may observe Ok within grace.
        //
        // `format_core_early_exit_error` always includes `code <n>` for the
        // status, so the assertions below cover the FATAL-text path indirectly.
        #[cfg(target_os = "windows")]
        let mut child = Command::new("cmd")
            .args(["/C", "exit /B 7"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn dying child");

        #[cfg(not(target_os = "windows"))]
        let mut child = Command::new("sh")
            .args(["-c", "exit 7"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn dying child");

        let early_buf = Arc::new(Mutex::new(Vec::<String>::new()));
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        // Re-use production forwarder path (sender is a throwaway channel).
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        spawn_log_forwarder(stdout, tx.clone(), Arc::clone(&early_buf));
        spawn_log_forwarder(stderr, tx, Arc::clone(&early_buf));

        let err = wait_core_startup_grace(&mut child, &early_buf, 5000)
            .expect_err("child should exit during grace");
        // Status code propagated to the error message regardless of log capture.
        assert!(
            err.contains("403") || err.contains("FATAL") || err.contains("code 7"),
            "unexpected error text: {err}"
        );
        assert!(err.contains("code 7") || err.contains("startup"), "msg={err}");

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
}
