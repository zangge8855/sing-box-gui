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

pub fn get_core_path(gui_config: &GuiConfig) -> PathBuf {
    if let Some(ref path) = gui_config.core_path {
        let p = PathBuf::from(path);
        if p.exists() {
            return p;
        }
    }
    get_app_dir().join("bin").join("sing-box.exe")
}

pub fn is_core_installed(gui_config: &GuiConfig) -> bool {
    get_core_path(gui_config).exists()
}

pub fn download_core(progress_sender: UnboundedSender<String>) -> Result<(), String> {
    let app_dir = get_app_dir();
    let bin_dir = app_dir.join("bin");
    let dest_path = bin_dir.join("sing-box.exe");
    
    if dest_path.exists() {
        return Ok(());
    }
    
    let _ = progress_sender.send("Downloading sing-box core...".to_string());
    
    let version = "1.13.14";
    let url = format!(
        "https://github.com/SagerNet/sing-box/releases/download/v{}/sing-box-{}-windows-amd64.zip",
        version, version
    );
    
    let temp_zip_path = app_dir.join("temp_core.zip");
    
    // Download zip using reqwest
    let mut response = reqwest::blocking::get(&url)
        .map_err(|e| format!("Failed to download core: {}", e))?;
        
    if !response.status().is_success() {
        return Err(format!("Server returned error: {}", response.status()));
    }
    
    let mut file = File::create(&temp_zip_path)
        .map_err(|e| format!("Failed to create temp zip file: {}", e))?;
        
    io::copy(&mut response, &mut file)
        .map_err(|e| format!("Failed to write zip file: {}", e))?;
        
    let _ = progress_sender.send("Extracting core...".to_string());
    
    // Extract using zip crate
    let zip_file = File::open(&temp_zip_path)
        .map_err(|e| format!("Failed to open temp zip: {}", e))?;
        
    let mut archive = zip::ZipArchive::new(zip_file)
        .map_err(|e| format!("Invalid zip archive: {}", e))?;
        
    let mut extracted = false;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("Failed to read zip index: {}", e))?;
            
        let name = file.name().to_string();
        if name.ends_with("sing-box.exe") {
            let mut outfile = File::create(&dest_path)
                .map_err(|e| format!("Failed to create target sing-box.exe: {}", e))?;
            io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to extract sing-box.exe: {}", e))?;
            extracted = true;
            break;
        }
    }
    
    // Cleanup temp zip
    let _ = fs::remove_file(temp_zip_path);
    
    if extracted {
        let _ = progress_sender.send("Core installed successfully!".to_string());
        Ok(())
    } else {
        Err("Could not find sing-box.exe inside downloaded zip package".to_string())
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
        
    let final_config = crate::config::convert_clash_to_singbox(&profile_content, gui_config)?;
    
    let run_config_path = get_app_dir().join("run_config.json");
    let run_config_content = serde_json::to_string_pretty(&final_config)
        .map_err(|e| format!("Failed to serialize final config: {}", e))?;
    fs::write(&run_config_path, run_config_content)
        .map_err(|e| format!("Failed to save final run_config: {}", e))?;
        
    let mut cmd = Command::new(&core_path);
    cmd.args(&["run", "-c", &run_config_path.to_string_lossy()]);
    cmd.env("ENABLE_DEPRECATED_LEGACY_DNS_SERVERS", "true");
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
