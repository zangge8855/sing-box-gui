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
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

static CURRENT_PROCESS: Mutex<Option<Child>> = Mutex::new(None);
/// Active startup PID (while `start_core` is inside `STARTUP_GRACE_MS`), allowing
/// `stop_core()` to abort a startup without waiting for the full grace window.
#[derive(Debug, Clone)]
struct StartupHandle {
    pid: u32,
    core_name: String,
    #[cfg(unix)]
    process_group: bool,
}
static STARTUP_HANDLE: Mutex<Option<StartupHandle>> = Mutex::new(None);
/// Set when the user (or app shutdown) intentionally stops the core.
static INTENTIONAL_STOP: AtomicBool = AtomicBool::new(false);
/// Number of `stop_core` calls waiting to serialize with `LIFECYCLE_LOCK`.
/// Prevents a queued `start_core` from clearing `INTENTIONAL_STOP` ahead of a pending stop.
static STOP_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
/// Message captured when the core dies without an intentional stop.
static LAST_UNEXPECTED_EXIT: Mutex<Option<String>> = Mutex::new(None);
/// Single source of truth for the sing-box core version this GUI downloads.
pub const CORE_VERSION: &str = "1.13.14";
/// Cached running flag, updated by `is_core_running_locked` so that the UI Tick
/// can poll without contending on `CURRENT_PROCESS` (which `start_core` holds
/// for up to `STARTUP_GRACE_MS` while waiting for the child to survive).
static CORE_RUNNING_CACHED: AtomicBool = AtomicBool::new(false);
static CORE_DOWNLOAD_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
static CORE_DOWNLOAD_PROGRESS: OnceLock<tokio::sync::watch::Sender<CoreDownloadStage>> =
    OnceLock::new();

/// High-level progress state for [`download_core_with_progress`] and
/// [`subscribe_core_download_progress`].
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CoreDownloadStage {
    #[default]
    Idle,
    Starting,
    FetchingMetadata,
    Downloading {
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
    },
    Verifying,
    Extracting,
    Installing,
    Completed,
    Failed(String),
}

fn download_progress_tx() -> &'static tokio::sync::watch::Sender<CoreDownloadStage> {
    CORE_DOWNLOAD_PROGRESS.get_or_init(|| {
        let (tx, _) = tokio::sync::watch::channel(CoreDownloadStage::Idle);
        tx
    })
}

/// Subscribe to structured core download progress updates without scraping log lines.
#[allow(dead_code)]
pub fn subscribe_core_download_progress() -> tokio::sync::watch::Receiver<CoreDownloadStage> {
    download_progress_tx().subscribe()
}

/// Return the most recent core download stage snapshot.
#[allow(dead_code)]
pub fn current_core_download_progress() -> CoreDownloadStage {
    download_progress_tx().borrow().clone()
}

/// How long to wait after spawn before treating the core as "started".
/// Catches immediate FATAL exits (bad config, rule-set init, port bind, …).
pub const STARTUP_GRACE_MS: u64 = 3000;
/// Max time `stop_core` waits for the child to exit gracefully before forcing.
/// Lets sing-box flush its cache file and tear down TUN/sysproxy state.
pub const GRACEFUL_STOP_TIMEOUT_MS: u64 = 2000;

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
        let start = interesting.len().saturating_sub(20);
        interesting[start..].join("\n")
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
        Some(c) => format!("Exit code {c}. Check the logs for FATAL/ERROR details."),
        None => "Check the logs for FATAL/ERROR details.".to_string(),
    }
}

/// sing-box colours its level tags unless `log.disable_color` is set, even
/// when stdout is a pipe, so native profiles would show `\x1b[36mINFO\x1b[0m`
/// on the Logs page. ESC never occurs inside UTF-8 or GBK multibyte
/// sequences, so this is safe to run before decoding.
fn strip_ansi(bytes: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    if !bytes.contains(&0x1b) {
        return std::borrow::Cow::Borrowed(bytes);
    }
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != 0x1b {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        i += 1;
        if bytes.get(i) == Some(&b'[') {
            // CSI: parameter/intermediate bytes up to a final byte 0x40..=0x7E.
            i += 1;
            while i < bytes.len() && !(0x40..=0x7e).contains(&bytes[i]) {
                i += 1;
            }
        }
        // Skip the CSI final byte, or the second byte of a two-byte escape.
        i += 1;
    }
    std::borrow::Cow::Owned(out)
}

fn decode_log_line(bytes: &[u8]) -> String {
    let bytes = &*strip_ansi(bytes);
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

fn trim_incomplete_utf8_tail(buf: &mut Vec<u8>) {
    match std::str::from_utf8(buf) {
        Ok(_) => return,
        Err(err)
            if err.error_len().is_none()
                && err.valid_up_to() > 0
                && buf.len().saturating_sub(err.valid_up_to()) <= 3 =>
        {
            buf.truncate(err.valid_up_to());
            return;
        }
        Err(_) => {}
    }

    // Earlier invalid bytes can mask an incomplete UTF-8 character at the 64 KiB
    // truncation boundary; avoid chopping valid Windows GBK trailing bytes.
    #[cfg(target_os = "windows")]
    if !encoding_rs::GBK.decode(buf).2 {
        return;
    }

    for back in 1..=3.min(buf.len()) {
        let idx = buf.len() - back;
        let b = buf[idx];
        if b < 0x80 {
            break;
        }
        if (0xC2..=0xF4).contains(&b) {
            let expected_len = if b < 0xE0 {
                2
            } else if b < 0xF0 {
                3
            } else {
                4
            };
            if back < expected_len && buf[idx + 1..].iter().all(|&c| (0x80..=0xBF).contains(&c)) {
                buf.truncate(idx);
            }
            break;
        }
    }
}

#[cfg(test)]
fn spawn_log_forwarder<R: Read + Send + 'static>(
    pipe: R,
    sender: Sender<String>,
    early_buf: Arc<Mutex<Vec<String>>>,
) {
    spawn_log_forwarder_with_flag(pipe, sender, early_buf, Arc::new(AtomicBool::new(true)));
}

fn spawn_log_forwarder_with_flag<R: Read + Send + 'static>(
    pipe: R,
    sender: Sender<String>,
    early_buf: Arc<Mutex<Vec<String>>>,
    capture_early: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let mut reader = BufReader::new(pipe);
        const MAX_CORE_LOG_LINE_BYTES: usize = 64 * 1024;
        let emit = |buf: &mut Vec<u8>, truncated: bool| {
            while buf.ends_with(b"\r") {
                buf.pop();
            }
            if truncated {
                trim_incomplete_utf8_tail(buf);
            }
            let mut line_str = decode_log_line(buf);
            if truncated {
                line_str.push_str(" … [truncated]");
            }
            if capture_early.load(Ordering::Acquire) {
                let mut buf_guard = early_buf.lock().unwrap_or_else(|e| e.into_inner());
                if capture_early.load(Ordering::Acquire) {
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
#[allow(dead_code)]
pub fn wait_core_startup_grace(
    child: &mut Child,
    early_buf: &Mutex<Vec<String>>,
    grace_ms: u64,
) -> Result<(), String> {
    let never_cancelled = AtomicBool::new(false);
    wait_core_startup_grace_interruptible(child, early_buf, grace_ms, &never_cancelled)
}

fn wait_core_startup_grace_interruptible(
    child: &mut Child,
    early_buf: &Mutex<Vec<String>>,
    grace_ms: u64,
    cancelled: &AtomicBool,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_millis(grace_ms);
    loop {
        if cancelled.load(Ordering::SeqCst) {
            return Err("Core startup cancelled".to_string());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if cancelled.load(Ordering::SeqCst) {
                    return Err("Core startup cancelled".to_string());
                }
                // Let pipe reader threads finish draining after process exit.
                thread::sleep(Duration::from_millis(150));
                let lines = early_buf.lock().unwrap_or_else(|e| e.into_inner()).clone();
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
static CURRENT_JOB_OBJECT: Mutex<Option<windows_sys::Win32::Foundation::HANDLE>> = Mutex::new(None);

#[cfg(target_os = "windows")]
fn close_current_job_object() {
    let mut job_lock = CURRENT_JOB_OBJECT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(job) = job_lock.take() {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(job);
        }
    }
}

#[cfg(target_os = "windows")]
fn bind_child_to_job_object(child: &std::process::Child) -> Result<(), String> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
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
            CloseHandle(job);
            return Err("Failed to set JobObject info".to_string());
        }
        let assign_res = AssignProcessToJobObject(job, child.as_raw_handle() as HANDLE);
        if assign_res == 0 {
            CloseHandle(job);
            return Err("Failed to assign process to JobObject".to_string());
        }

        let mut lock = CURRENT_JOB_OBJECT.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(old_job) = lock.take() {
            CloseHandle(old_job);
        }
        *lock = Some(job);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
mod proc_ctl {
    use std::ffi::c_void;
    use std::path::Path;

    const PROCESS_TERMINATE: u32 = 0x0001;
    const PROCESS_SYNCHRONIZE: u32 = 0x0010_0000;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;
    const WAIT_TIMEOUT: u32 = 258;

    unsafe extern "system" {
        fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> *mut c_void;
        fn TerminateProcess(process: *mut c_void, exit_code: u32) -> i32;
        fn WaitForSingleObject(handle: *mut c_void, milliseconds: u32) -> u32;
        fn GetExitCodeProcess(process: *mut c_void, exit_code: *mut u32) -> i32;
        fn QueryFullProcessImageNameW(
            process: *mut c_void,
            flags: u32,
            exe_name: *mut u16,
            size: *mut u32,
        ) -> i32;
        fn CloseHandle(object: *mut c_void) -> i32;
    }

    struct OwnedHandle(*mut c_void);
    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
    }

    fn open_process(pid: u32, expected_name: Option<&str>, access: u32) -> Option<OwnedHandle> {
        if pid == 0 {
            return None;
        }
        let handle = unsafe {
            OpenProcess(
                access | PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                pid,
            )
        };
        if handle.is_null() {
            return None;
        }
        let owned = OwnedHandle(handle);
        let mut exit_code: u32 = 0;
        if unsafe { GetExitCodeProcess(owned.0, &mut exit_code) } == 0
            || exit_code != STILL_ACTIVE
            || unsafe { WaitForSingleObject(owned.0, 0) } != WAIT_TIMEOUT
        {
            return None;
        }
        if let Some(expected) = expected_name {
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            if unsafe { QueryFullProcessImageNameW(owned.0, 0, buf.as_mut_ptr(), &mut len) } == 0 {
                return None;
            }
            let image_path = String::from_utf16_lossy(&buf[..len as usize]);
            let matches_name = Path::new(&image_path)
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case(expected));
            if !matches_name {
                return None;
            }
        }
        Some(owned)
    }

    pub fn is_pid_alive(pid: u32, expected_name: &str) -> bool {
        open_process(pid, Some(expected_name), 0).is_some()
    }

    pub fn terminate_pid(
        pid: u32,
        expected_name: Option<&str>,
        _process_group: bool,
        timeout_ms: u32,
    ) -> Result<bool, String> {
        let Some(handle) =
            open_process(pid, expected_name, PROCESS_TERMINATE | PROCESS_SYNCHRONIZE)
        else {
            return Ok(false);
        };
        if unsafe { TerminateProcess(handle.0, 1) } == 0 {
            let mut exit_code: u32 = 0;
            if unsafe { GetExitCodeProcess(handle.0, &mut exit_code) } != 0
                && exit_code != STILL_ACTIVE
            {
                return Ok(true);
            }
            return Err(format!(
                "Failed to terminate core process {pid}: {}",
                std::io::Error::last_os_error()
            ));
        }
        unsafe {
            WaitForSingleObject(handle.0, timeout_ms);
        }
        Ok(true)
    }
}

#[cfg(unix)]
mod proc_ctl {
    use std::path::Path;
    use std::time::{Duration, Instant};

    #[cfg(target_os = "linux")]
    fn is_zombie(pid: u32) -> bool {
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            return false;
        };
        let Some(after_comm) = stat.rfind(')') else {
            return false;
        };
        stat[after_comm + 1..].trim_start().starts_with('Z')
    }

    pub fn is_pid_alive(pid: u32, expected_name: &str) -> bool {
        let Ok(pid_t) = libc::pid_t::try_from(pid) else {
            return false;
        };
        if pid_t <= 0 {
            return false;
        }
        let probe = unsafe { libc::kill(pid_t, 0) };
        if probe != 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::EPERM) {
            return false;
        }

        #[cfg(target_os = "linux")]
        {
            if is_zombie(pid) {
                return false;
            }
            if let Ok(exe) = std::fs::read_link(format!("/proc/{pid}/exe")) {
                let name = exe
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .trim_end_matches(" (deleted)");
                return name == expected_name;
            }
            if let Ok(cmdline) = std::fs::read(format!("/proc/{pid}/cmdline"))
                && let Some(arg0) = cmdline.split(|&b| b == 0).next()
            {
                let arg0 = String::from_utf8_lossy(arg0);
                return Path::new(arg0.as_ref())
                    .file_name()
                    .and_then(|n| n.to_str())
                    == Some(expected_name);
            }
            false
        }

        #[cfg(not(target_os = "linux"))]
        {
            let Ok(out) = std::process::Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "state=,comm="])
                .output()
            else {
                return false;
            };
            if !out.status.success() {
                return false;
            }
            let text = String::from_utf8_lossy(&out.stdout);
            let trimmed = text.trim();
            let Some((state, comm)) = trimmed.split_once(char::is_whitespace) else {
                return false;
            };
            if state.starts_with('Z') {
                return false;
            }
            Path::new(comm.trim()).file_name().and_then(|n| n.to_str()) == Some(expected_name)
        }
    }

    fn send_signal(
        pid: libc::pid_t,
        process_group: bool,
        signal: libc::c_int,
    ) -> Result<(), std::io::Error> {
        if pid <= 0 {
            return Err(std::io::Error::from_raw_os_error(libc::EINVAL));
        }
        let rc = unsafe {
            if process_group {
                let pg_rc = libc::kill(-pid, signal);
                if pg_rc != 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
                {
                    libc::kill(pid, signal)
                } else {
                    pg_rc
                }
            } else {
                libc::kill(pid, signal)
            }
        };
        if rc == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    pub fn terminate_pid(
        pid: u32,
        expected_name: Option<&str>,
        process_group: bool,
        timeout_ms: u32,
    ) -> Result<bool, String> {
        let Ok(pid_t) = libc::pid_t::try_from(pid) else {
            return Ok(false);
        };
        if pid_t <= 0 {
            return Ok(false);
        }
        if let Some(name) = expected_name
            && !is_pid_alive(pid, name)
        {
            return Ok(false);
        }
        if let Err(err) = send_signal(pid_t, process_group, libc::SIGTERM) {
            if err.raw_os_error() == Some(libc::ESRCH) {
                return Ok(false);
            }
            return Err(format!("Failed to signal core process {pid}: {err}"));
        }

        if timeout_ms > 0 {
            let check_alive = || match expected_name {
                Some(name) => is_pid_alive(pid, name),
                None => {
                    let rc = unsafe { libc::kill(pid_t, 0) };
                    rc == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
                }
            };
            let deadline = Instant::now() + Duration::from_millis(u64::from(timeout_ms));
            while Instant::now() < deadline {
                if !check_alive() {
                    return Ok(true);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            let _ = send_signal(pid_t, process_group, libc::SIGKILL);
            let kill_deadline = Instant::now() + Duration::from_millis(250);
            while Instant::now() < kill_deadline && check_alive() {
                std::thread::sleep(Duration::from_millis(25));
            }
        }
        Ok(true)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct CorePidRecord {
    pid: u32,
    core_name: String,
    #[serde(default)]
    process_group: bool,
}

fn core_pid_file_path() -> PathBuf {
    get_app_dir().join("core.pid")
}

fn read_pid_record_from(path: &Path) -> Option<CorePidRecord> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_pid_record_to(path: &Path, record: &CorePidRecord) -> Result<(), String> {
    let bytes = serde_json::to_vec(record)
        .map_err(|e| format!("Failed to serialize core PID record: {e}"))?;
    crate::config::atomic_write(path, &bytes)
}

fn remove_pid_record_at(path: &Path) {
    let _ = fs::remove_file(path);
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        let _ = fs::remove_file(path.with_file_name(format!("{name}.bak")));
    }
}

fn record_running_core_pid(pid: u32, core_path: &Path, process_group: bool) {
    let core_name = core_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_else(|| get_core_filename())
        .to_string();
    let record = CorePidRecord {
        pid,
        core_name,
        process_group,
    };
    let _ = write_pid_record_to(&core_pid_file_path(), &record);
}

/// Return the live sing-box PID currently tracked in-process or via `core.pid`
/// (from a `close_core_on_exit == false` session), cleaning up stale PID files.
pub fn get_tracked_core_pid() -> Option<u32> {
    {
        let mut lock = get_process_lock();
        if let Some(ref mut child) = *lock
            && matches!(child.try_wait(), Ok(None))
        {
            return Some(child.id());
        }
    }
    let pid_path = core_pid_file_path();
    let record = read_pid_record_from(&pid_path)?;
    if proc_ctl::is_pid_alive(record.pid, &record.core_name) {
        Some(record.pid)
    } else {
        remove_pid_record_at(&pid_path);
        None
    }
}

/// Return `true` if a detached core from an earlier session (`close_core_on_exit == false`)
/// is still running outside the current child handle.
#[allow(dead_code)]
pub fn is_detached_core_running(_gui_config: &GuiConfig) -> bool {
    if is_core_running() {
        return false;
    }
    let pid_path = core_pid_file_path();
    let Some(record) = read_pid_record_from(&pid_path) else {
        return false;
    };
    if proc_ctl::is_pid_alive(record.pid, &record.core_name) {
        true
    } else {
        remove_pid_record_at(&pid_path);
        false
    }
}

/// Terminate an orphaned/detached core recorded in `core.pid` (for example after
/// restarting the GUI with `close_core_on_exit == false`). Returns `Ok(true)` if
/// a detached process was stopped.
#[allow(dead_code)]
pub fn stop_detached_core(_gui_config: &GuiConfig) -> Result<bool, String> {
    stop_detached_core_internal()
}

fn stop_detached_core_internal() -> Result<bool, String> {
    let pid_path = core_pid_file_path();
    let Some(record) = read_pid_record_from(&pid_path) else {
        return Ok(false);
    };
    let res = proc_ctl::terminate_pid(
        record.pid,
        Some(&record.core_name),
        record.process_group,
        GRACEFUL_STOP_TIMEOUT_MS as u32,
    );
    if res.is_ok() {
        remove_pid_record_at(&pid_path);
    }
    res
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

/// Return the GUI-managed download destination inside `<app_dir>/bin`.
pub fn managed_core_path() -> PathBuf {
    get_app_dir().join("bin").join(get_core_filename())
}

pub fn get_core_path(gui_config: &GuiConfig) -> PathBuf {
    if let Some(ref path) = gui_config.core_path {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    managed_core_path()
}

pub fn is_core_installed(gui_config: &GuiConfig) -> bool {
    get_core_path(gui_config).is_file()
}

const MAX_CORE_ARCHIVE_BYTES: usize = 256 * 1024 * 1024;
const MAX_CORE_BINARY_BYTES: u64 = 128 * 1024 * 1024;
#[cfg(any(target_os = "linux", target_os = "macos"))]
const MAX_TAR_STREAM_BYTES: u64 = 512 * 1024 * 1024;

fn unique_sibling_path(dest_path: &Path, label: &str) -> PathBuf {
    static SIBLING_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let file_name = dest_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("sing-box");
    let seq = SIBLING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    dest_path.with_file_name(format!(
        ".{file_name}.{label}-{}-{seq}-{nonce}",
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
    let had_existing = dest_path.symlink_metadata().is_ok();
    if had_existing {
        fs::rename(dest_path, &backup_path)
            .map_err(|e| format!("Failed to stage the existing core for replacement: {e}"))?;
    }
    if let Err(error) = fs::rename(staged_path, dest_path) {
        if had_existing
            && backup_path.symlink_metadata().is_ok()
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
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("github.com")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
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
    let mut read = 0usize;
    while read < magic.len() {
        match file.read(&mut magic[read..]) {
            Ok(0) => break,
            Ok(n) => read += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(format!("Failed to read binary header: {e}")),
        }
    }

    #[cfg(target_os = "windows")]
    let valid = if read >= 2 && &magic[..2] == b"MZ" {
        let mut offset_bytes = [0u8; 4];
        file.seek(SeekFrom::Start(0x3c)).is_ok() && file.read_exact(&mut offset_bytes).is_ok() && {
            let pe_offset = u32::from_le_bytes(offset_bytes) as u64;
            let mut signature = [0u8; 4];
            (0x40..=16 * 1024 * 1024).contains(&pe_offset)
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

async fn fetch_with_no_proxy_fallback(
    proxy_client: &reqwest::Client,
    direct_client: &reqwest::Client,
    url: &str,
) -> Result<reqwest::Response, reqwest::Error> {
    match proxy_client
        .get(url)
        .header("User-Agent", "sing-box-gui")
        .send()
        .await
    {
        Ok(resp) if !matches!(resp.status().as_u16(), 502..=504) => Ok(resp),
        Err(first_err) => direct_client
            .get(url)
            .header("User-Agent", "sing-box-gui")
            .send()
            .await
            .or(Err(first_err)),
        Ok(bad_gateway_resp) => direct_client
            .get(url)
            .header("User-Agent", "sing-box-gui")
            .send()
            .await
            .or(Ok(bad_gateway_resp)),
    }
}

fn emit_download_stage<F: FnMut(CoreDownloadStage)>(stage: CoreDownloadStage, on_progress: &mut F) {
    let _ = download_progress_tx().send(stage.clone());
    on_progress(stage);
}

/// Download official sing-box into the managed bin folder.
/// When `force` is true, replace an existing binary (reinstall / upgrade pin).
pub async fn download_core(progress_sender: Sender<String>, force: bool) -> Result<(), String> {
    download_core_with_progress(None, Some(&progress_sender), force, |_| {}).await
}

/// Download official sing-box into an explicit `dest_path` (for custom core paths).
#[allow(dead_code)]
pub async fn download_core_to(
    dest_path: &Path,
    progress_sender: Sender<String>,
    force: bool,
) -> Result<(), String> {
    download_core_with_progress(Some(dest_path), Some(&progress_sender), force, |_| {}).await
}

#[cfg(target_os = "windows")]
fn extract_core_from_zip(archive_path: &Path, output_path: &Path) -> Result<(), String> {
    let zip_file = File::open(archive_path).map_err(|e| format!("Failed to open temp zip: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(zip_file).map_err(|e| format!("Invalid zip archive: {e}"))?;

    let mut matches = 0usize;
    let mut created_output = false;
    let res: Result<(), String> = (|| {
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|e| format!("Failed to read zip entry: {e}"))?;
            let Some(path) = entry.enclosed_name() else {
                continue;
            };
            let is_non_regular = entry.unix_mode().is_some_and(|mode| {
                let kind = mode & 0o170000;
                kind != 0 && kind != 0o100000
            });
            if !entry.is_file()
                || entry.is_symlink()
                || is_non_regular
                || !is_safe_core_entry(&path, get_core_filename())
            {
                continue;
            }
            matches += 1;
            if matches > 1 {
                return Err("Core archive contains ambiguous duplicate binaries".to_string());
            }
            let declared_size = entry.size();
            if declared_size == 0 || declared_size > MAX_CORE_BINARY_BYTES {
                return Err("Downloaded core binary has an invalid size".to_string());
            }
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(output_path)
                .map_err(|e| format!("Failed to create staged core: {e}"))?;
            created_output = true;
            let copied = io::copy(
                &mut Read::by_ref(&mut entry).take(MAX_CORE_BINARY_BYTES + 1),
                &mut output,
            )
            .map_err(|e| format!("Failed to extract core: {e}"))?;
            if copied == 0 || copied > MAX_CORE_BINARY_BYTES || copied != declared_size {
                return Err("Downloaded core binary size does not match archive entry".to_string());
            }
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
    })();
    if res.is_err() && created_output {
        let _ = fs::remove_file(output_path);
    }
    res
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn extract_core_from_tar_gz(archive_path: &Path, output_path: &Path) -> Result<(), String> {
    let tar_gz =
        File::open(archive_path).map_err(|e| format!("Failed to open temp archive: {e}"))?;
    let gz = flate2::read::GzDecoder::new(tar_gz).take(MAX_TAR_STREAM_BYTES);
    let mut archive = tar::Archive::new(gz);

    let mut matches = 0usize;
    let mut created_output = false;
    let res: Result<(), String> = (|| {
        for entry in archive
            .entries()
            .map_err(|e| format!("Failed to inspect tar.gz: {e}"))?
        {
            let mut entry = entry.map_err(|e| format!("Failed to read tar entry: {e}"))?;
            let path = entry
                .path()
                .map_err(|e| format!("Invalid tar entry path: {e}"))?;
            let is_core =
                entry.header().entry_type().is_file() && is_safe_core_entry(&path, "sing-box");
            if !is_core {
                continue;
            }
            matches += 1;
            if matches > 1 {
                return Err("Core archive contains ambiguous duplicate binaries".to_string());
            }
            let size = entry.header().size().unwrap_or(0);
            if size == 0 || size > MAX_CORE_BINARY_BYTES {
                return Err("Downloaded core binary has an invalid size".to_string());
            }
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(output_path)
                .map_err(|e| format!("Failed to create staged core: {e}"))?;
            created_output = true;
            let copied = std::io::copy(
                &mut Read::by_ref(&mut entry).take(MAX_CORE_BINARY_BYTES + 1),
                &mut output,
            )
            .map_err(|e| format!("Failed to extract core: {e}"))?;
            if copied == 0 || copied > MAX_CORE_BINARY_BYTES || copied != size {
                return Err("Downloaded core binary size does not match archive entry".to_string());
            }
            output
                .sync_all()
                .map_err(|e| format!("Failed to flush staged core: {e}"))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(output_path, fs::Permissions::from_mode(0o755))
                    .map_err(|e| format!("Failed to mark staged core executable: {e}"))?;
            }
        }
        (matches == 1)
            .then_some(())
            .ok_or_else(|| "Could not find sing-box inside downloaded tar package".to_string())
    })();
    if res.is_err() && created_output {
        let _ = fs::remove_file(output_path);
    }
    res
}

fn is_core_lifecycle_busy() -> bool {
    STARTUP_HANDLE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some()
        || matches!(
            LIFECYCLE_LOCK.try_lock(),
            Err(std::sync::TryLockError::WouldBlock)
        )
}

/// Download official sing-box with structured [`CoreDownloadStage`] callbacks
/// and global watch updates (via [`subscribe_core_download_progress`]).
///
/// When `dest_path` is `None`, installs into [`managed_core_path()`].
pub async fn download_core_with_progress<F>(
    dest_path: Option<&Path>,
    progress_sender: Option<&Sender<String>>,
    force: bool,
    mut on_progress: F,
) -> Result<(), String>
where
    F: FnMut(CoreDownloadStage),
{
    let download_lock = CORE_DOWNLOAD_LOCK.get_or_init(|| tokio::sync::Mutex::new(()));
    let _guard = download_lock
        .try_lock()
        .map_err(|_| "A core download is already in progress".to_string())?;
    if is_core_running() || get_tracked_core_pid().is_some() || is_core_lifecycle_busy() {
        let msg = "Stop the core before reinstalling or replacing it".to_string();
        emit_download_stage(CoreDownloadStage::Failed(msg.clone()), &mut on_progress);
        return Err(msg);
    }

    let dest_path = dest_path
        .map(Path::to_path_buf)
        .unwrap_or_else(managed_core_path);
    let Some(dest_dir) = dest_path.parent() else {
        let msg = "Core destination path has no parent directory".to_string();
        emit_download_stage(CoreDownloadStage::Failed(msg.clone()), &mut on_progress);
        return Err(msg);
    };
    if !dest_dir.as_os_str().is_empty()
        && let Err(e) = fs::create_dir_all(dest_dir)
    {
        let msg = format!("Failed to create bin directory: {e}");
        emit_download_stage(CoreDownloadStage::Failed(msg.clone()), &mut on_progress);
        return Err(msg);
    }

    if dest_path.is_dir() {
        let msg = "Core destination path is a directory".to_string();
        emit_download_stage(CoreDownloadStage::Failed(msg.clone()), &mut on_progress);
        return Err(msg);
    }

    if dest_path.is_file() && !force {
        emit_download_stage(CoreDownloadStage::Completed, &mut on_progress);
        return Ok(());
    }

    emit_download_stage(CoreDownloadStage::Starting, &mut on_progress);
    if let Some(tx) = progress_sender {
        let _ = tx.try_send("Downloading sing-box core...".to_string());
    }

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
        emit_download_stage(CoreDownloadStage::FetchingMetadata, &mut on_progress);
        let proxy_client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| format!("Failed to build core download client: {e}"))?;
        let direct_client = reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| format!("Failed to build direct core download client: {e}"))?;

        let metadata_url =
            format!("https://api.github.com/repos/SagerNet/sing-box/releases/tags/v{version}");
        let metadata_response =
            fetch_with_no_proxy_fallback(&proxy_client, &direct_client, &metadata_url)
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
        let response = fetch_with_no_proxy_fallback(
            &proxy_client,
            &direct_client,
            &asset.browser_download_url,
        )
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
        let mut last_reported_bytes = 0usize;
        let mut hasher = Sha256::new();
        emit_download_stage(
            CoreDownloadStage::Downloading {
                downloaded_bytes: 0,
                total_bytes: Some(asset.size),
            },
            &mut on_progress,
        );
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Failed to read chunk: {}", e))?;
            downloaded = downloaded.saturating_add(chunk.len());
            if downloaded > MAX_CORE_ARCHIVE_BYTES || downloaded as u64 > asset.size {
                return Err(
                    "Downloaded core archive exceeds the expected release asset size".to_string(),
                );
            }
            hasher.update(&chunk);
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to write chunk: {}", e))?;
            if downloaded.saturating_sub(last_reported_bytes) >= 64 * 1024
                || downloaded as u64 == asset.size
            {
                last_reported_bytes = downloaded;
                emit_download_stage(
                    CoreDownloadStage::Downloading {
                        downloaded_bytes: downloaded as u64,
                        total_bytes: Some(asset.size),
                    },
                    &mut on_progress,
                );
            }
        }
        file.flush()
            .await
            .map_err(|e| format!("Failed to flush core archive: {e}"))?;
        file.sync_all()
            .await
            .map_err(|e| format!("Failed to flush core archive: {e}"))?;
        drop(file);

        emit_download_stage(CoreDownloadStage::Verifying, &mut on_progress);
        if let Some(tx) = progress_sender {
            let _ = tx.try_send("Verifying core...".to_string());
        }
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

        emit_download_stage(CoreDownloadStage::Extracting, &mut on_progress);
        if let Some(tx) = progress_sender {
            let _ = tx.try_send("Extracting core...".to_string());
        }

        #[cfg(target_os = "windows")]
        {
            let archive_path = temp_archive_path.clone();
            let output_path = staged_path.clone();
            tokio::task::spawn_blocking(move || extract_core_from_zip(&archive_path, &output_path))
                .await
                .map_err(|e| format!("Core extraction task failed: {e}"))??;
        }

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let archive_path = temp_archive_path.clone();
            let output_path = staged_path.clone();
            tokio::task::spawn_blocking(move || {
                extract_core_from_tar_gz(&archive_path, &output_path)
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

        emit_download_stage(CoreDownloadStage::Installing, &mut on_progress);
        if let Some(tx) = progress_sender {
            let _ = tx.try_send("Installing core...".to_string());
        }
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
            emit_download_stage(CoreDownloadStage::Completed, &mut on_progress);
            if let Some(tx) = progress_sender {
                let _ = tx.try_send("Core installed successfully!".to_string());
            }
            Ok(())
        }
        Err(e) => {
            emit_download_stage(CoreDownloadStage::Failed(e.clone()), &mut on_progress);
            Err(e)
        }
    }
}

/// Build and write `run_config.json` from the active profile + GUI settings.
pub fn prepare_run_config(gui_config: &GuiConfig) -> Result<PathBuf, String> {
    let active_id = gui_config
        .active_profile_id
        .as_ref()
        .ok_or_else(|| "No active profile selected".to_string())?;

    let config_path = get_profile_path(active_id);
    if !config_path.is_file() {
        return Err("Active profile config file not found!".to_string());
    }

    let profile_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read profile file: {}", e))?;

    let normalized_content = profile_content.trim_start_matches('\u{feff}');
    let trimmed = normalized_content.trim();
    let mut final_config = if trimmed.starts_with('{') || trimmed.starts_with('[') {
        crate::config::merge_native_json_profile(normalized_content, gui_config)?
    } else {
        crate::config::convert_clash_to_singbox(normalized_content, gui_config)?
    };

    // Fix known startup footguns (broken rule-set proxies, relative cache path).
    crate::config::mitigate_run_config(&mut final_config);

    let run_config_path = get_app_dir().join("run_config.json");
    let run_config_content = serde_json::to_string_pretty(&final_config)
        .map_err(|e| format!("Failed to serialize final config: {}", e))?;
    crate::config::atomic_write(&run_config_path, run_config_content.as_bytes())?;
    Ok(run_config_path)
}

fn apply_singbox_compat_env(cmd: &mut Command) {
    cmd.env("ENABLE_DEPRECATED_LEGACY_DNS_SERVERS", "true");
    cmd.env("ENABLE_DEPRECATED_MISSING_DOMAIN_RESOLVER", "true");
    cmd.env("ENABLE_DEPRECATED_SPECIAL_OUTBOUNDS", "true");
}

#[cfg(target_os = "linux")]
fn spawn_unix_core(mut cmd: Command, close_on_exit: bool) -> std::io::Result<Child> {
    use std::os::unix::process::CommandExt;
    use std::sync::mpsc;

    type SpawnJob = Box<dyn FnOnce() + Send>;
    static SPAWNER_TX: OnceLock<mpsc::Sender<SpawnJob>> = OnceLock::new();
    let tx = SPAWNER_TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<SpawnJob>();
        let _ = thread::Builder::new()
            .name("sing-box-spawner".to_string())
            .spawn(move || {
                while let Ok(job) = rx.recv() {
                    job();
                }
            });
        tx
    });

    let (res_tx, res_rx) = mpsc::sync_channel(1);
    tx.send(Box::new(move || {
        // Record TGID before fork; the persistent spawner thread outlives
        // transient spawn_blocking workers so PDEATHSIG only fires on GUI exit.
        let parent_pid = unsafe { libc::getpid() };
        unsafe {
            cmd.pre_exec(move || {
                if libc::setpgid(0, 0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if close_on_exit {
                    if libc::prctl(
                        libc::PR_SET_PDEATHSIG,
                        libc::SIGTERM as libc::c_ulong,
                        0,
                        0,
                        0,
                    ) != 0
                    {
                        return Err(std::io::Error::last_os_error());
                    }
                    if libc::getppid() != parent_pid {
                        libc::raise(libc::SIGTERM);
                    }
                }
                Ok(())
            });
        }
        let _ = res_tx.send(cmd.spawn());
    }))
    .map_err(|_| std::io::Error::other("Core spawner thread unavailable"))?;

    res_rx
        .recv()
        .map_err(|_| std::io::Error::other("Core spawner thread did not respond"))?
}

#[cfg(all(unix, not(target_os = "linux")))]
fn spawn_unix_core(mut cmd: Command, _close_on_exit: bool) -> std::io::Result<Child> {
    use std::os::unix::process::CommandExt;
    unsafe {
        cmd.pre_exec(|| {
            if libc::setpgid(0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    cmd.spawn()
}

struct BoundedCommandOutput {
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn read_pipe_bounded<R: Read + Send + 'static>(
    mut pipe: R,
    max_bytes: usize,
) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut collected = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            match pipe.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let room = max_bytes.saturating_sub(collected.len());
                    if room > 0 {
                        collected.extend_from_slice(&buf[..n.min(room)]);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        collected
    })
}

fn run_command_with_timeout(
    mut cmd: Command,
    timeout: Duration,
    max_output_bytes: usize,
    spawn_err_prefix: &str,
) -> Result<BoundedCommandOutput, String> {
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("{spawn_err_prefix}: {e}"))?;
    let stdout_handle = child
        .stdout
        .take()
        .map(|pipe| read_pipe_bounded(pipe, max_output_bytes));
    let stderr_handle = child
        .stderr
        .take()
        .map(|pipe| read_pipe_bounded(pipe, max_output_bytes));

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "{spawn_err_prefix}: timed out after {}s",
                        timeout.as_secs()
                    ));
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{spawn_err_prefix}: {e}"));
            }
        }
    };

    let stdout = stdout_handle
        .and_then(|h| h.join().ok())
        .unwrap_or_default();
    let stderr = stderr_handle
        .and_then(|h| h.join().ok())
        .unwrap_or_default();
    Ok(BoundedCommandOutput {
        status,
        stdout,
        stderr,
    })
}

fn run_config_check(core_path: &Path, run_config_path: &Path) -> Result<(), String> {
    let mut cmd = Command::new(core_path);
    cmd.arg("check").arg("-c").arg(run_config_path.as_os_str());
    cmd.current_dir(get_app_dir());
    apply_singbox_compat_env(&mut cmd);

    let output = run_command_with_timeout(
        cmd,
        Duration::from_secs(15),
        256 * 1024,
        "Failed to run sing-box check",
    )?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = decode_log_line(&output.stderr);
        let stdout = decode_log_line(&output.stdout);
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

fn kill_and_reap_child(child: &mut Child) {
    #[cfg(unix)]
    if let Ok(pid_t) = libc::pid_t::try_from(child.id())
        && pid_t > 0
    {
        unsafe {
            let _ = libc::kill(-pid_t, libc::SIGKILL);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn stop_child_gracefully(child: &mut Child) {
    #[cfg(unix)]
    {
        if let Ok(pid) = libc::pid_t::try_from(child.id())
            && pid > 0
        {
            unsafe {
                let pg_rc = libc::kill(-pid, libc::SIGTERM);
                let result = if pg_rc != 0
                    && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
                {
                    libc::kill(pid, libc::SIGTERM)
                } else {
                    pg_rc
                };
                if result != 0 {
                    let error = std::io::Error::last_os_error();
                    if error.raw_os_error() != Some(libc::ESRCH) {
                        let mut g = get_last_unexpected_exit_lock();
                        *g = Some(format!("Failed to terminate core: {error}"));
                    }
                }
            }
            let start = Instant::now();
            let mut exited = false;
            while start.elapsed() < Duration::from_millis(GRACEFUL_STOP_TIMEOUT_MS) {
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
                unsafe {
                    let _ = libc::kill(-pid, libc::SIGKILL);
                }
                let _ = child.kill();
            }
        } else {
            let _ = child.kill();
        }
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
    }
    let _ = child.wait();
}

/// Run `sing-box check -c run_config.json` after generating config.
/// Also used internally by `start_core`; public for Settings “validate” hooks.
#[allow(dead_code)]
pub fn check_core_config(gui_config: &GuiConfig) -> Result<(), String> {
    let core_path = get_core_path(gui_config);
    if !core_path.is_file() {
        return Err(
            "sing-box core not found! Please download it or specify correct path in Settings."
                .to_string(),
        );
    }
    let run_config_path = prepare_run_config(gui_config)?;
    run_config_check(&core_path, &run_config_path)
}

static LIFECYCLE_LOCK: Mutex<()> = Mutex::new(());

struct StopInFlightGuard;

impl StopInFlightGuard {
    fn new() -> Self {
        STOP_IN_FLIGHT.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

impl Drop for StopInFlightGuard {
    fn drop(&mut self) {
        STOP_IN_FLIGHT.fetch_sub(1, Ordering::SeqCst);
    }
}

pub fn start_core(gui_config: &GuiConfig, log_sender: Sender<String>) -> Result<(), String> {
    let _lifecycle_guard = LIFECYCLE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if STOP_IN_FLIGHT.load(Ordering::SeqCst) > 0 {
        return Err("Core startup cancelled".to_string());
    }
    // Reset the intentional-stop flag up front so a failed restart does not
    // leave a stale flag lingering when no child is being tracked.
    INTENTIONAL_STOP.store(false, Ordering::SeqCst);
    CONSECUTIVE_TRY_WAIT_ERRORS.store(0, Ordering::SeqCst);
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
    if !core_path.is_file() {
        return Err(
            "sing-box core not found! Please download it or specify correct path in Settings."
                .to_string(),
        );
    }

    // Validate generated config before spawning the process.
    // Spawn/check failures are hard errors — never skip validation silently.
    let run_config_path = prepare_run_config(gui_config)?;
    run_config_check(&core_path, &run_config_path)
        .map_err(|msg| format!("Config check failed: {msg}"))?;

    // Stop any orphaned detached instance left from a prior close_core_on_exit == false
    // run only after the new config has passed validation.
    stop_detached_core_internal()?;

    if INTENTIONAL_STOP.load(Ordering::SeqCst) {
        return Err("Core startup cancelled".to_string());
    }

    #[cfg(target_os = "windows")]
    if !gui_config.close_core_on_exit {
        close_current_job_object();
    }

    let app_dir = get_app_dir();
    let mut cmd = Command::new(&core_path);
    cmd.arg("run").arg("-c").arg(run_config_path.as_os_str());
    cmd.current_dir(&app_dir);
    apply_singbox_compat_env(&mut cmd);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    #[cfg(unix)]
    let mut child = spawn_unix_core(cmd, gui_config.close_core_on_exit)
        .map_err(|e| format!("Failed to start process: {}", e))?;
    #[cfg(not(unix))]
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start process: {}", e))?;

    let child_pid = child.id();
    let core_name = core_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_else(|| get_core_filename())
        .to_string();
    {
        let mut startup_lock = STARTUP_HANDLE.lock().unwrap_or_else(|e| e.into_inner());
        *startup_lock = Some(StartupHandle {
            pid: child_pid,
            core_name,
            #[cfg(unix)]
            process_group: true,
        });
    }

    #[cfg(target_os = "windows")]
    if gui_config.close_core_on_exit
        && let Err(error) = bind_child_to_job_object(&child)
    {
        // Without the job object the core can outlive a crashed GUI.
        let _ = log_sender.try_send(format!(
            "[GUI] Warning: failed to bind core to job object: {error}"
        ));
    }

    let (stdout, stderr) = match (child.stdout.take(), child.stderr.take()) {
        (Some(out), Some(err)) => (out, err),
        _ => {
            *STARTUP_HANDLE.lock().unwrap_or_else(|e| e.into_inner()) = None;
            kill_and_reap_child(&mut child);
            #[cfg(target_os = "windows")]
            close_current_job_object();
            return Err("Failed to capture stdout/stderr".to_string());
        }
    };

    let early_buf = Arc::new(Mutex::new(Vec::<String>::new()));
    let capture_early = Arc::new(AtomicBool::new(true));
    spawn_log_forwarder_with_flag(
        stdout,
        log_sender.clone(),
        Arc::clone(&early_buf),
        Arc::clone(&capture_early),
    );
    spawn_log_forwarder_with_flag(
        stderr,
        log_sender,
        Arc::clone(&early_buf),
        Arc::clone(&capture_early),
    );

    // Do not advertise success until the process survives the grace window.
    // On early exit, surface FATAL/ERROR text instead of a silent UI flip.
    let grace_res = wait_core_startup_grace_interruptible(
        &mut child,
        &early_buf,
        STARTUP_GRACE_MS,
        &INTENTIONAL_STOP,
    );
    capture_early.store(false, Ordering::Release);
    {
        let mut buf_guard = early_buf.lock().unwrap_or_else(|e| e.into_inner());
        buf_guard.clear();
        buf_guard.shrink_to_fit();
    }

    if let Err(e) = grace_res {
        *STARTUP_HANDLE.lock().unwrap_or_else(|e| e.into_inner()) = None;
        if INTENTIONAL_STOP.load(Ordering::SeqCst) {
            stop_child_gracefully(&mut child);
        } else {
            kill_and_reap_child(&mut child);
        }
        #[cfg(target_os = "windows")]
        close_current_job_object();
        return Err(e);
    }

    let mut lock = get_process_lock();
    *STARTUP_HANDLE.lock().unwrap_or_else(|e| e.into_inner()) = None;
    if INTENTIONAL_STOP.load(Ordering::SeqCst) {
        stop_child_gracefully(&mut child);
        #[cfg(target_os = "windows")]
        close_current_job_object();
        CORE_RUNNING_CACHED.store(false, Ordering::SeqCst);
        return Err("Core startup cancelled".to_string());
    }

    {
        let mut g = get_last_unexpected_exit_lock();
        *g = None;
    }
    record_running_core_pid(child_pid, &core_path, cfg!(unix));
    CORE_RUNNING_CACHED.store(true, Ordering::SeqCst);
    *lock = Some(child);
    Ok(())
}

pub fn stop_core() {
    let _stop_guard = StopInFlightGuard::new();
    INTENTIONAL_STOP.store(true, Ordering::SeqCst);
    CORE_RUNNING_CACHED.store(false, Ordering::SeqCst);

    // Signal any in-flight startup immediately so `start_core` exits its grace loop promptly.
    let startup = STARTUP_HANDLE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    if let Some(handle) = startup {
        #[cfg(unix)]
        let pg = handle.process_group;
        #[cfg(not(unix))]
        let pg = false;
        let _ = proc_ctl::terminate_pid(handle.pid, Some(&handle.core_name), pg, 0);
    }

    // Wait for any in-flight `start_core` to finish reaping and release `LIFECYCLE_LOCK`.
    let _lifecycle_guard = LIFECYCLE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    CONSECUTIVE_TRY_WAIT_ERRORS.store(0, Ordering::SeqCst);

    let mut had_child = false;
    {
        let mut lock = get_process_lock();
        if let Some(child) = lock.as_mut() {
            had_child = true;
            match child.try_wait() {
                Ok(Some(_)) => {
                    lock.take();
                }
                Ok(None) => {
                    stop_child_gracefully(child);
                    lock.take();
                }
                Err(error) => {
                    let mut g = get_last_unexpected_exit_lock();
                    *g = Some(format!("Failed to inspect core before stopping: {error}"));
                    let _ = child.kill();
                    let _ = child.wait();
                    lock.take();
                }
            }
        }
        CORE_RUNNING_CACHED.store(false, Ordering::SeqCst);
    }

    if !had_child {
        let _ = stop_detached_core_internal();
    } else {
        remove_pid_record_at(&core_pid_file_path());
    }

    // Close the Windows Job Object only after child termination + wait completes.
    #[cfg(target_os = "windows")]
    close_current_job_object();
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
                remove_pid_record_at(&core_pid_file_path());
                #[cfg(target_os = "windows")]
                close_current_job_object();
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
                    if let Some(mut dead_child) = lock.take() {
                        let _ = dead_child.kill();
                        let _ = dead_child.wait();
                    }
                    remove_pid_record_at(&core_pid_file_path());
                    #[cfg(target_os = "windows")]
                    close_current_job_object();
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
/// calling [`is_core_running`]. Use this in per-second subscriptions to avoid
/// contending on the process lock on the GUI thread.
pub fn is_core_running_fast() -> bool {
    CORE_RUNNING_CACHED.load(Ordering::SeqCst)
}

/// Run `sing-box version` and return the first line of stdout.
pub fn get_core_version(gui_config: &GuiConfig) -> Result<String, String> {
    let path = get_core_path(gui_config);
    if !path.is_file() {
        return Err("Core not installed".to_string());
    }
    let mut cmd = Command::new(&path);
    cmd.arg("version");
    let output = run_command_with_timeout(
        cmd,
        Duration::from_secs(10),
        64 * 1024,
        "Failed to run core version",
    )?;
    if !output.status.success() {
        let stderr = decode_log_line(&output.stderr);
        let detail = stderr.trim();
        return Err(if detail.is_empty() {
            format!("Core version check failed (status {})", output.status)
        } else {
            format!("Core version check failed: {detail}")
        });
    }
    let text = decode_log_line(&output.stdout);
    let first = text.lines().next().unwrap_or("").trim().to_string();
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
    fn log_lines_drop_ansi_colour_codes() {
        let raw = b"+0800 2026-10-27 14:30:00 \x1b[36mINFO\x1b[0m [1 0ms] router: ok";
        assert_eq!(
            decode_log_line(raw),
            "+0800 2026-10-27 14:30:00 INFO [1 0ms] router: ok"
        );
        assert_eq!(decode_log_line(b"plain \xe4\xb8\xad"), "plain 中");
        // A truncated trailing escape must not panic or leak the ESC byte.
        assert_eq!(decode_log_line(b"cut \x1b[3"), "cut ");
    }

    /// The release zip is deflated; guards the trimmed `zip` feature set.
    #[test]
    fn core_zip_entries_inflate_with_enabled_features() {
        use std::io::{Cursor, Read, Write};
        let payload = b"MZ sing-box core ".repeat(64);
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        writer
            .start_file("sing-box-1.12.0-windows-amd64/sing-box.exe", options)
            .unwrap();
        writer.write_all(&payload).unwrap();
        let bytes = writer.finish().unwrap().into_inner();

        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut entry = archive.by_index(0).unwrap();
        let path = entry.enclosed_name().unwrap();
        assert!(entry.is_file());
        assert!(is_safe_core_entry(&path, "sing-box.exe"));
        assert_eq!(entry.size(), payload.len() as u64);
        let mut out = Vec::new();
        entry.read_to_end(&mut out).unwrap();
        assert_eq!(out, payload);
    }

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

    #[test]
    fn compat_env_sets_all_deprecated_flags() {
        let mut cmd = Command::new("sing-box");
        apply_singbox_compat_env(&mut cmd);
        let envs: std::collections::HashMap<_, _> = cmd
            .get_envs()
            .filter_map(|(k, v)| Some((k.to_str()?.to_string(), v?.to_str()?.to_string())))
            .collect();
        assert_eq!(
            envs.get("ENABLE_DEPRECATED_LEGACY_DNS_SERVERS")
                .map(String::as_str),
            Some("true")
        );
        assert_eq!(
            envs.get("ENABLE_DEPRECATED_MISSING_DOMAIN_RESOLVER")
                .map(String::as_str),
            Some("true")
        );
        assert_eq!(
            envs.get("ENABLE_DEPRECATED_SPECIAL_OUTBOUNDS")
                .map(String::as_str),
            Some("true")
        );
    }

    #[test]
    fn get_core_path_respects_custom_path_and_blank_fallback() {
        let mut cfg = GuiConfig {
            core_path: Some("  /custom/nonexistent/sing-box  ".to_string()),
            ..Default::default()
        };
        assert_eq!(
            get_core_path(&cfg),
            PathBuf::from("/custom/nonexistent/sing-box")
        );
        assert!(!is_core_installed(&cfg));

        cfg.core_path = Some("   ".to_string());
        assert_eq!(get_core_path(&cfg), managed_core_path());
    }

    #[test]
    fn get_core_version_errors_on_nonzero_exit() {
        #[cfg(target_os = "windows")]
        let (_bin_dir, script_path) = {
            let dir = tempfile::tempdir().expect("tempdir");
            let p = dir.path().join("fake-core.cmd");
            fs::write(&p, "@echo boom 1>&2\r\n@exit /b 42\r\n").expect("write script");
            (dir, p)
        };
        #[cfg(not(target_os = "windows"))]
        let (_bin_dir, script_path) = {
            use std::os::unix::fs::PermissionsExt;
            let dir = tempfile::tempdir().expect("tempdir");
            let p = dir.path().join("fake-core.sh");
            fs::write(&p, "#!/bin/sh\necho boom >&2\nexit 42\n").expect("write script");
            fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).expect("chmod");
            (dir, p)
        };
        let cfg = GuiConfig {
            core_path: Some(script_path.to_string_lossy().into_owned()),
            ..Default::default()
        };
        let err = get_core_version(&cfg).expect_err("non-zero status should error");
        assert!(err.contains("boom") || err.contains("failed"), "err={err}");
    }

    #[test]
    fn log_forwarder_stops_buffering_early_lines_after_grace() {
        let (reader, mut writer) = std::io::pipe().expect("os pipe");
        let early = Arc::new(Mutex::new(Vec::new()));
        let capture_early = Arc::new(AtomicBool::new(true));
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(16);
        spawn_log_forwarder_with_flag(reader, tx, Arc::clone(&early), Arc::clone(&capture_early));

        use std::io::Write;
        writeln!(writer, "grace-line").expect("write");
        writer.flush().expect("flush");
        assert_eq!(rx.blocking_recv().as_deref(), Some("grace-line"));
        assert_eq!(*early.lock().unwrap(), vec!["grace-line".to_string()]);

        capture_early.store(false, Ordering::SeqCst);
        writeln!(writer, "post-grace-line").expect("write");
        writer.flush().expect("flush");
        assert_eq!(rx.blocking_recv().as_deref(), Some("post-grace-line"));
        assert_eq!(*early.lock().unwrap(), vec!["grace-line".to_string()]);
    }

    #[test]
    fn wait_grace_cancels_promptly_on_intentional_stop() {
        #[cfg(target_os = "windows")]
        let mut child = Command::new("cmd")
            .args(["/C", "ping -n 10 127.0.0.1 >nul"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        #[cfg(not(target_os = "windows"))]
        let mut child = Command::new("sh")
            .args(["-c", "sleep 10"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");

        let early = Mutex::new(Vec::new());
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancel_clone = Arc::clone(&cancelled);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(120));
            cancel_clone.store(true, Ordering::SeqCst);
        });

        let started = Instant::now();
        let err = wait_core_startup_grace_interruptible(&mut child, &early, 5000, &cancelled)
            .expect_err("should cancel before full grace window");
        let elapsed = started.elapsed();
        let _ = child.kill();
        let _ = child.wait();
        assert!(err.contains("cancelled"), "err={err}");
        assert!(elapsed < Duration::from_secs(2), "took {elapsed:?}");
    }

    #[test]
    fn pid_record_roundtrip_and_watch_progress_helpers() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pid_path = dir.path().join("core.pid");
        let rec = CorePidRecord {
            pid: 424242,
            core_name: get_core_filename().to_string(),
            process_group: true,
        };
        write_pid_record_to(&pid_path, &rec).expect("write");
        assert_eq!(read_pid_record_from(&pid_path), Some(rec));
        remove_pid_record_at(&pid_path);
        assert!(read_pid_record_from(&pid_path).is_none());

        let watch_rx = subscribe_core_download_progress();
        let _ = download_progress_tx().send(CoreDownloadStage::FetchingMetadata);
        assert!(watch_rx.has_changed().unwrap_or(false));
        assert_eq!(
            current_core_download_progress(),
            CoreDownloadStage::FetchingMetadata
        );
        let _ = download_progress_tx().send(CoreDownloadStage::Idle);
    }

    #[test]
    fn proc_ctl_verifies_image_name_before_terminating_pid() {
        let self_exe = std::env::current_exe().expect("test exe");
        let self_name = self_exe
            .file_name()
            .and_then(|n| n.to_str())
            .expect("exe name");
        let self_pid = std::process::id();
        assert!(proc_ctl::is_pid_alive(self_pid, self_name));
        assert!(!proc_ctl::is_pid_alive(self_pid, "sing-box-unrelated"));

        #[cfg(target_os = "windows")]
        let mut child = Command::new("ping")
            .args(["-n", "10", "127.0.0.1"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn ping");
        #[cfg(not(target_os = "windows"))]
        let mut child = Command::new("sleep")
            .arg("10")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn sleep");

        let pid = child.id();
        #[cfg(target_os = "windows")]
        let expected_name = "PING.EXE".to_string();
        #[cfg(target_os = "linux")]
        let expected_name = fs::read_link(format!("/proc/{pid}/exe"))
            .ok()
            .and_then(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_string))
            .unwrap_or_else(|| "sleep".to_string());
        #[cfg(all(unix, not(target_os = "linux")))]
        let expected_name = "sleep".to_string();

        assert!(!proc_ctl::is_pid_alive(pid, "sing-box-unrelated"));
        assert!(proc_ctl::is_pid_alive(pid, &expected_name));

        // Wrong expected name must refuse to kill.
        assert!(!proc_ctl::terminate_pid(pid, Some("sing-box-unrelated"), false, 500).unwrap());
        assert!(proc_ctl::is_pid_alive(pid, &expected_name));

        // Matching name must terminate and wait cleanly.
        assert!(proc_ctl::terminate_pid(pid, Some(&expected_name), false, 2000).unwrap());
        let _ = child.wait();
        assert!(!proc_ctl::is_pid_alive(pid, &expected_name));

        // Zero and out-of-range PIDs must fail closed without signaling PID 0 or -1.
        assert!(!proc_ctl::is_pid_alive(0, &expected_name));
        assert!(!proc_ctl::is_pid_alive(u32::MAX, &expected_name));
        assert!(!proc_ctl::terminate_pid(0, None, false, 0).unwrap());
        assert!(!proc_ctl::terminate_pid(u32::MAX, None, true, 0).unwrap());
    }

    #[test]
    fn log_forwarder_truncates_on_utf8_boundary_without_replacement_or_gbk_mojibake() {
        // Construct a line > 64 KiB where a 3-byte UTF-8 character ("中") straddles 65536.
        let mut input = vec![b'a'; 64 * 1024 - 1];
        input.extend_from_slice("中文日志内容".as_bytes());
        input.extend_from_slice(b"\n");

        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(4);
        let early = Arc::new(Mutex::new(Vec::new()));
        spawn_log_forwarder(std::io::Cursor::new(input), tx, early);

        let line = rx.blocking_recv().expect("truncated utf8 line");
        assert!(line.ends_with(" … [truncated]"), "line={line:?}");
        assert!(
            !line.contains('\u{fffd}'),
            "split UTF-8 byte leaked replacement char: {line:?}"
        );
        assert!(
            line.starts_with("aaaa"),
            "prefix should remain intact UTF-8: {:?}",
            &line[..16]
        );
    }

    #[test]
    fn poisoned_early_buf_recovers_in_forwarder_and_grace_wait() {
        let early = Arc::new(Mutex::new(vec!["FATAL: poisoned detail".to_string()]));
        let poison_ref = Arc::clone(&early);
        let _ = thread::spawn(move || {
            let _guard = poison_ref.lock().unwrap();
            panic!("simulate panic while holding early_buf");
        })
        .join();
        assert!(early.is_poisoned());

        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(8);
        spawn_log_forwarder(
            std::io::Cursor::new(b"FATAL: after poison\n".to_vec()),
            tx,
            Arc::clone(&early),
        );
        assert_eq!(rx.blocking_recv().as_deref(), Some("FATAL: after poison"));
        let recovered = early.lock().unwrap_or_else(|e| e.into_inner()).clone();
        assert!(recovered.iter().any(|l| l == "FATAL: poisoned detail"));
        assert!(recovered.iter().any(|l| l == "FATAL: after poison"));

        let mut child = Command::new(std::env::current_exe().expect("exe"))
            .arg("--list")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        let err = wait_core_startup_grace(&mut child, &early, 5000).expect_err("exit");
        assert!(
            err.contains("FATAL: after poison"),
            "poisoned buffer should still surface FATAL lines: {err}"
        );
    }

    #[test]
    fn unique_sibling_path_never_collides_and_install_rolls_back_on_missing_stage() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join(get_core_filename());
        let p1 = unique_sibling_path(&dest, "staged");
        let p2 = unique_sibling_path(&dest, "staged");
        assert_ne!(p1, p2);

        fs::write(&dest, b"working core").expect("seed dest");
        let missing_stage = unique_sibling_path(&dest, "missing");
        let err = install_staged_core(&missing_stage, &dest).expect_err("missing staged fails");
        assert!(err.contains("Failed to install"), "err={err}");
        assert_eq!(fs::read(&dest).expect("rollback restored"), b"working core");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn zip_extraction_and_pe_magic_reject_invalid_inputs() {
        use std::io::Write;
        let dir = tempfile::tempdir().expect("tempdir");

        // PE header overlapping DOS header (e_lfanew < 0x40) must be rejected.
        let pe_path = dir.path().join("bad_pe.exe");
        let mut pe = vec![0u8; 0x80];
        pe[..2].copy_from_slice(b"MZ");
        pe[0x3c..0x40].copy_from_slice(&(0x20u32).to_le_bytes());
        pe[0x20..0x24].copy_from_slice(b"PE\0\0");
        fs::write(&pe_path, pe).unwrap();
        assert!(validate_binary_magic(&pe_path).is_err());

        // Duplicate matching binaries in a zip must fail closed.
        let zip_path = dir.path().join("dup.zip");
        let staged_path = dir.path().join("out.exe");
        {
            let file = File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let opts = zip::write::SimpleFileOptions::default();
            writer.start_file("a/sing-box.exe", opts).unwrap();
            writer.write_all(b"MZ1").unwrap();
            writer.start_file("b/sing-box.exe", opts).unwrap();
            writer.write_all(b"MZ2").unwrap();
            writer.finish().unwrap();
        }
        let err = extract_core_from_zip(&zip_path, &staged_path).expect_err("duplicate fails");
        assert!(err.contains("duplicate"), "err={err}");
        assert!(
            !staged_path.exists(),
            "failed zip extraction must clean up staged file"
        );

        // Symlink entry with matching name must be ignored, not extracted.
        let sym_zip = dir.path().join("sym.zip");
        {
            let file = File::create(&sym_zip).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let opts = zip::write::SimpleFileOptions::default();
            writer
                .add_symlink("pkg/sing-box.exe", "/etc/passwd", opts)
                .unwrap();
            writer.finish().unwrap();
        }
        let err = extract_core_from_zip(&sym_zip, &staged_path).expect_err("symlink ignored");
        assert!(err.contains("Could not find"), "err={err}");
        assert!(!staged_path.exists());
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn tar_gz_extraction_rejects_symlinks_traversal_and_duplicates() {
        use flate2::Compression;
        use flate2::write::GzEncoder;

        let dir = tempfile::tempdir().expect("tempdir");
        let staged_path = dir.path().join("staged-sing-box");

        // Valid tar.gz with symlink + real binary extracts only the regular binary with 0o755.
        let valid_tgz = dir.path().join("valid.tar.gz");
        {
            let file = File::create(&valid_tgz).unwrap();
            let gz = GzEncoder::new(file, Compression::default());
            let mut builder = tar::Builder::new(gz);

            let mut sym = tar::Header::new_gnu();
            sym.set_entry_type(tar::EntryType::Symlink);
            sym.set_size(0);
            sym.set_mode(0o777);
            builder
                .append_link(&mut sym, "pkg/sing-box", "/bin/sh")
                .unwrap();

            let payload = b"\x7fELFdemo-sing-box";
            let mut hdr = tar::Header::new_gnu();
            hdr.set_size(payload.len() as u64);
            hdr.set_mode(0o644);
            hdr.set_cksum();
            builder
                .append_data(&mut hdr, "sing-box-1.0/sing-box", &payload[..])
                .unwrap();
            builder.into_inner().unwrap().finish().unwrap();
        }
        extract_core_from_tar_gz(&valid_tgz, &staged_path).expect("valid tar extraction");
        assert_eq!(fs::read(&staged_path).unwrap(), b"\x7fELFdemo-sing-box");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&staged_path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o755);
        }
        let _ = fs::remove_file(&staged_path);

        // Duplicate matching regular files fail closed and clean up staged_path.
        let dup_tgz = dir.path().join("dup.tar.gz");
        {
            let file = File::create(&dup_tgz).unwrap();
            let gz = GzEncoder::new(file, Compression::default());
            let mut builder = tar::Builder::new(gz);
            for path in ["a/sing-box", "b/sing-box"] {
                let payload = b"ELF";
                let mut hdr = tar::Header::new_gnu();
                hdr.set_size(payload.len() as u64);
                hdr.set_mode(0o755);
                hdr.set_cksum();
                builder.append_data(&mut hdr, path, &payload[..]).unwrap();
            }
            builder.into_inner().unwrap().finish().unwrap();
        }
        let err = extract_core_from_tar_gz(&dup_tgz, &staged_path).expect_err("duplicate fails");
        assert!(err.contains("duplicate"), "err={err}");
        assert!(!staged_path.exists());
    }

    #[test]
    fn log_forwarder_trims_trailing_incomplete_utf8_even_after_earlier_invalid_byte() {
        let mut buf = vec![b'a', 0xff, b'b'];
        buf.extend_from_slice(&"中".as_bytes()[..2]);
        trim_incomplete_utf8_tail(&mut buf);
        assert_eq!(buf, vec![b'a', 0xff, b'b']);

        #[cfg(target_os = "windows")]
        {
            // Valid GBK multi-byte tail must not be chopped by UTF-8 fallback.
            let mut gbk = b"hello \xb9\xfa".to_vec();
            trim_incomplete_utf8_tail(&mut gbk);
            assert_eq!(decode_log_line(&gbk), "hello 国");
        }
    }

    #[test]
    fn is_core_installed_rejects_directories_and_url_check_rejects_extras() {
        let dir = tempfile::tempdir().expect("tempdir");
        let subdir = dir.path().join("bin-dir");
        fs::create_dir(&subdir).unwrap();
        let cfg_dir = GuiConfig {
            core_path: Some(subdir.to_string_lossy().into_owned()),
            ..Default::default()
        };
        assert!(!is_core_installed(&cfg_dir));

        let file_path = dir.path().join("sing-box");
        fs::write(&file_path, b"bin").unwrap();
        let cfg_file = GuiConfig {
            core_path: Some(file_path.to_string_lossy().into_owned()),
            ..Default::default()
        };
        assert!(is_core_installed(&cfg_file));

        let name = "sing-box-1.13.14-windows-amd64.zip";
        assert!(!is_official_core_asset_url(
            "https://evil@github.com/SagerNet/sing-box/releases/download/v1.13.14/sing-box-1.13.14-windows-amd64.zip",
            "1.13.14",
            name
        ));
        assert!(!is_official_core_asset_url(
            "https://github.com:8443/SagerNet/sing-box/releases/download/v1.13.14/sing-box-1.13.14-windows-amd64.zip",
            "1.13.14",
            name
        ));
        assert!(!is_official_core_asset_url(
            "https://github.com/SagerNet/sing-box/releases/download/v1.13.14/sing-box-1.13.14-windows-amd64.zip?raw=1",
            "1.13.14",
            name
        ));
        assert!(!is_official_core_asset_url(
            "https://github.com/SagerNet/sing-box/releases/download/v1.13.14/sing-box-1.13.14-windows-amd64.zip#frag",
            "1.13.14",
            name
        ));
    }

    #[test]
    fn run_command_with_timeout_kills_hanging_process_and_strips_ansi_on_error() {
        #[cfg(target_os = "windows")]
        let hang_cmd = {
            let mut c = Command::new("powershell.exe");
            c.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 10"]);
            c
        };
        #[cfg(not(target_os = "windows"))]
        let hang_cmd = {
            let mut c = Command::new("sh");
            c.args(["-c", "exec sleep 10"]);
            c
        };

        let started = Instant::now();
        let err = run_command_with_timeout(
            hang_cmd,
            Duration::from_millis(150),
            1024,
            "Failed to run sing-box check",
        )
        .err()
        .expect("hanging process must time out");
        assert!(err.contains("timed out"), "err={err}");
        assert!(started.elapsed() < Duration::from_secs(3));

        let dir = tempfile::tempdir().expect("tempdir");
        #[cfg(target_os = "windows")]
        let script = {
            let p = dir.path().join("fake-check.cmd");
            fs::write(
                &p,
                b"@echo \x1b[31mFATAL\x1b[0m bad config 1>&2\r\n@exit /b 1\r\n",
            )
            .unwrap();
            p
        };
        #[cfg(not(target_os = "windows"))]
        let script = {
            use std::os::unix::fs::PermissionsExt;
            let p = dir.path().join("fake-check.sh");
            fs::write(
                &p,
                "#!/bin/sh\nprintf '\\033[31mFATAL\\033[0m bad config\\n' >&2\nexit 1\n",
            )
            .unwrap();
            fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
            p
        };
        let cfg_path = dir.path().join("run.json");
        fs::write(&cfg_path, "{}").unwrap();
        let err = run_config_check(&script, &cfg_path).expect_err("non-zero check fails");
        assert!(err.contains("FATAL bad config"), "err={err:?}");
        assert!(!err.contains('\x1b'), "ANSI leaked into error: {err:?}");
    }

    #[test]
    fn start_core_respects_pending_stop_and_early_exit_caps_lines() {
        let many_errors: Vec<String> = (1..=35)
            .map(|i| format!("ERROR[{i:04}] line-{i}"))
            .collect();
        let summary = format_core_early_exit_error(Some(1), &many_errors);
        assert!(!summary.contains("line-1\n"), "should cap to recent lines");
        assert!(summary.contains("line-35"), "summary={summary}");

        let _guard = StopInFlightGuard::new();
        let (tx, _rx) = tokio::sync::mpsc::channel(4);
        let err = start_core(&GuiConfig::default(), tx).expect_err("pending stop cancels start");
        assert_eq!(err, "Core startup cancelled");
    }
}
