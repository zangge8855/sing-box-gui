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

pub fn download_core(progress_sender: UnboundedSender<String>) -> Result<(), String> {
    let app_dir = get_app_dir();
    let bin_dir = app_dir.join("bin");
    let dest_path = bin_dir.join(get_core_filename());
    
    if dest_path.exists() {
        return Ok(());
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
    
    // Download using reqwest
    let mut response = reqwest::blocking::get(&url)
        .map_err(|e| format!("Failed to download core: {}", e))?;
        
    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }
    
    let mut file = File::create(&temp_archive_path)
        .map_err(|e| format!("Failed to create temp archive file: {}", e))?;
        
    io::copy(&mut response, &mut file)
        .map_err(|e| format!("Failed to write archive file: {}", e))?;
        
    let _ = progress_sender.send("Extracting core...".to_string());
    
    #[cfg(target_os = "windows")]
    {
        // Extract using zip crate for Windows
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
        
        let _ = fs::remove_file(temp_archive_path);
        
        if extracted {
            let _ = progress_sender.send("Core installed successfully!".to_string());
            Ok(())
        } else {
            Err(format!("Could not find {} inside downloaded zip package", core_name))
        }
    }
    
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        // Extract using system `tar` command for Linux and macOS
        let status = Command::new("tar")
            .arg("-xzf")
            .arg(&temp_archive_path)
            .arg("-C")
            .arg(&app_dir)
            .status()
            .map_err(|e| format!("Failed to run tar command: {}", e))?;
            
        let _ = fs::remove_file(temp_archive_path);
        
        if !status.success() {
            return Err("Failed to extract tar.gz archive using system tar command".to_string());
        }
        
        // Find extracted binary
        let extracted_dir = app_dir.join(format!("sing-box-{}-{}", version, arch));
        let src_binary = extracted_dir.join("sing-box");
        if src_binary.exists() {
            fs::copy(&src_binary, &dest_path)
                .map_err(|e| format!("Failed to copy sing-box binary: {}", e))?;
            let _ = fs::remove_dir_all(extracted_dir);
            
            // Set permissions
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&dest_path, fs::Permissions::from_mode(0o755));
            
            let _ = progress_sender.send("Core installed successfully!".to_string());
            Ok(())
        } else {
            Err("Could not find sing-box inside extracted tar folder".to_string())
        }
    }
}

pub fn start_core(
    gui_config: &GuiConfig,
    log_sender: UnboundedSender<String>,
) -> Result<(), String> {
    let mut lock = CURRENT_PROCESS.lock().unwrap();
    if lock.is_some() {
        return Ok(());
    }
    
    let core_path = get_core_path(gui_config);
    if !core_path.exists() {
        return Err("sing-box core not found! Please download it or specify correct path in Settings.".to_string());
    }
    
    let active_id = gui_config.active_profile_id.as_ref()
        .ok_or_else(|| "No active profile selected".to_string())?;
        
    let config_path = get_profile_path(active_id);
    if !config_path.exists() {
        return Err("Active profile config file not found!".to_string());
    }
    
    // Generate config JSON before running
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
    fs::write(&run_config_path, run_config_content)
        .map_err(|e| format!("Failed to save final run_config: {}", e))?;
        
    let mut cmd = Command::new(&core_path);
    cmd.args(&["run", "-c", &run_config_path.to_string_lossy()]);
    cmd.env("ENABLE_DEPRECATED_LEGACY_DNS_SERVERS", "true");
    cmd.env("ENABLE_DEPRECATED_MISSING_DOMAIN_RESOLVER", "true");
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    
    // Hide CMD window on Windows
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    
    let mut child = cmd.spawn()
        .map_err(|e| format!("Failed to start process: {}", e))?;
        
    let stdout = child.stdout.take().ok_or_else(|| "Failed to capture stdout".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "Failed to capture stderr".to_string())?;
    
    *lock = Some(child);
    
    // Stream stdout logs
    let sender_stdout = log_sender.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line_str) = line {
                let _ = sender_stdout.send(line_str);
            }
        }
    });
    
    // Stream stderr logs
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
    let mut lock = CURRENT_PROCESS.lock().unwrap();
    if let Some(mut child) = lock.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

pub fn is_core_running() -> bool {
    let mut lock = CURRENT_PROCESS.lock().unwrap();
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
