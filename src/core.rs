use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use tokio::sync::mpsc::UnboundedSender;
use crate::config::{get_app_dir, get_profile_path};
use crate::state::GuiConfig;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

static CURRENT_PROCESS: Mutex<Option<Child>> = Mutex::new(None);

fn get_process_lock() -> std::sync::MutexGuard<'static, Option<Child>> {
    CURRENT_PROCESS.lock().unwrap_or_else(|e| e.into_inner())
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
            
        let bytes = response.bytes().await
            .map_err(|e| format!("Failed to read response bytes: {}", e))?;
            
        io::copy(&mut &bytes[..], &mut file)
            .map_err(|e| format!("Failed to write archive file: {}", e))?;
            
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
            let status = Command::new("tar")
                .arg("-xzf")
                .arg(&temp_archive_path)
                .arg("-C")
                .arg(&app_dir)
                .status()
                .map_err(|e| format!("Failed to run tar command: {}", e))?;
                
            if !status.success() {
                return Err("Failed to extract tar.gz archive using system tar command".to_string());
            }
            
            let extracted_dir = app_dir.join(format!("sing-box-{}-{}", version, arch));
            let src_binary = extracted_dir.join("sing-box");
            if src_binary.exists() {
                fs::copy(&src_binary, &dest_path)
                    .map_err(|e| format!("Failed to copy sing-box binary: {}", e))?;
                let _ = fs::remove_dir_all(extracted_dir);
                
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&dest_path, fs::Permissions::from_mode(0o755));
                Ok(())
            } else {
                Err("Could not find sing-box inside extracted tar folder".to_string())
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
    let final_config = if trimmed.starts_with('{') || trimmed.starts_with('[') {
        crate::config::merge_native_json_profile(&profile_content, gui_config)?
    } else {
        crate::config::convert_clash_to_singbox(&profile_content, gui_config)?
    };

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
        return Ok(());
    }
    
    let core_path = get_core_path(gui_config);
    if !core_path.exists() {
        return Err("sing-box core not found! Please download it or specify correct path in Settings.".to_string());
    }

    // Validate generated config before spawning the process
    let run_config_path = prepare_run_config(gui_config)?;
    {
        let mut check_cmd = Command::new(&core_path);
        check_cmd.args(["check", "-c", &run_config_path.to_string_lossy()]);
        check_cmd.stdout(Stdio::piped());
        check_cmd.stderr(Stdio::piped());
        #[cfg(target_os = "windows")]
        check_cmd.creation_flags(0x08000000);
        if let Ok(output) = check_cmd.output() {
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
    }
        
    let mut cmd = Command::new(&core_path);
    cmd.args(&["run", "-c", &run_config_path.to_string_lossy()]);
    cmd.env("ENABLE_DEPRECATED_LEGACY_DNS_SERVERS", "true");
    cmd.env("ENABLE_DEPRECATED_MISSING_DOMAIN_RESOLVER", "true");
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    
    let mut child = cmd.spawn()
        .map_err(|e| format!("Failed to start process: {}", e))?;
        
    let stdout = child.stdout.take().ok_or_else(|| "Failed to capture stdout".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "Failed to capture stderr".to_string())?;
    
    *lock = Some(child);
    
    let sender_stdout = log_sender.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line_str) = line {
                let _ = sender_stdout.send(line_str);
            }
        }
    });
    
    let sender_stderr = log_sender;
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line_str) = line {
                let _ = sender_stderr.send(line_str);
            }
        }
    });
    
    Ok(())
}

pub fn stop_core() {
    let mut lock = get_process_lock();
    if let Some(mut child) = lock.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

pub fn is_core_running() -> bool {
    let mut lock = get_process_lock();
    if let Some(ref mut child) = *lock {
        match child.try_wait() {
            Ok(None) => true,
            _ => {
                *lock = None;
                false
            }
        }
    } else {
        false
    }
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
