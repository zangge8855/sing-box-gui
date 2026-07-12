pub fn is_running_elevated() -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        unsafe {
            // Use OpenProcessToken + GetTokenInformation(TokenElevation).
            #[allow(clippy::upper_case_acronyms)]
            type BOOL = i32;
            #[allow(clippy::upper_case_acronyms)]
            type HANDLE = *mut c_void;
            unsafe extern "system" {
                fn GetCurrentProcess() -> HANDLE;
                fn OpenProcessToken(
                    process: HANDLE,
                    access: u32,
                    token: *mut HANDLE,
                ) -> BOOL;
                fn GetTokenInformation(
                    token: HANDLE,
                    class: u32,
                    buf: *mut c_void,
                    len: u32,
                    ret_len: *mut u32,
                ) -> BOOL;
                fn CloseHandle(h: HANDLE) -> BOOL;
            }
            // TOKEN_QUERY = 0x0008 ; TokenElevation = 20
            let mut token: HANDLE = std::ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), 0x0008, &mut token) == 0 {
                return false;
            }
            let mut elevated: i32 = 0;
            let mut ret = 0u32;
            let ok = GetTokenInformation(
                token,
                20,
                &mut elevated as *mut _ as *mut c_void,
                std::mem::size_of::<i32>() as u32,
                &mut ret,
            );
            CloseHandle(token);
            ok != 0 && elevated != 0
        }
    }
    #[cfg(target_os = "linux")]
    {
        if unsafe { libc::geteuid() == 0 } {
            return true;
        }
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .is_some_and(|status| linux_status_has_cap_net_admin(&status))
    }
    #[cfg(all(unix, not(target_os = "linux")))]
    {
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(any(target_os = "windows", unix)))]
    {
        true // assume elevated where we cannot check
    }
}

#[cfg(target_os = "linux")]
fn linux_status_has_cap_net_admin(status: &str) -> bool {
    const CAP_NET_ADMIN_BIT: u32 = 12;
    status.lines().find_map(|line| {
        let value = line.strip_prefix("CapEff:")?.trim();
        let capabilities = u64::from_str_radix(value, 16).ok()?;
        Some(capabilities & (1u64 << CAP_NET_ADMIN_BIT) != 0)
    }).unwrap_or(false)
}

/// Install or remove per-user launch integration on the current platform.
pub fn set_autostart(enable: bool) -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("Failed to resolve current executable: {e}"))?;

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let run_key = hkcu
            .open_subkey_with_flags(
                r#"Software\Microsoft\Windows\CurrentVersion\Run"#,
                KEY_WRITE,
            )
            .map_err(|e| format!("Failed to open autostart registry key: {e}"))?;
        if enable {
            let quoted = format!("\"{}\"", exe.to_string_lossy());
            run_key
                .set_value("sing-box-gui", &quoted)
                .map_err(|e| format!("Failed to write autostart registry: {e}"))?;
        } else {
            let _ = run_key.delete_value("sing-box-gui");
        }
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir().ok_or_else(|| "Home directory is unavailable".to_string())?;
        let agents = home.join("Library/LaunchAgents");
        let path = agents.join("io.github.zangge8855.sing-box-gui.plist");
        if enable {
            std::fs::create_dir_all(&agents)
                .map_err(|e| format!("Failed to create LaunchAgents directory: {e}"))?;
            let exe = xml_escape(&exe.to_string_lossy());
            let plist = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>Label</key><string>io.github.zangge8855.sing-box-gui</string><key>ProgramArguments</key><array><string>{exe}</string></array><key>RunAtLoad</key><true/></dict></plist>\n"
            );
            crate::config::atomic_write(&path, plist.as_bytes())?;
        } else if path.exists() {
            std::fs::remove_file(path)
                .map_err(|e| format!("Failed to remove LaunchAgent: {e}"))?;
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let config = dirs::config_dir().ok_or_else(|| "Config directory is unavailable".to_string())?;
        let autostart = config.join("autostart");
        let path = autostart.join("sing-box-gui.desktop");
        if enable {
            std::fs::create_dir_all(&autostart)
                .map_err(|e| format!("Failed to create autostart directory: {e}"))?;
            let exec = desktop_escape(&exe.to_string_lossy());
            let entry = format!(
                "[Desktop Entry]\nType=Application\nName=sing-box GUI\nComment=Start sing-box GUI with the desktop session\nExec=\"{exec}\"\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
            );
            crate::config::atomic_write(&path, entry.as_bytes())?;
        } else if path.exists() {
            std::fs::remove_file(path)
                .map_err(|e| format!("Failed to remove autostart entry: {e}"))?;
        }
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("Autostart is not supported on this platform".to_string())
}

#[cfg(target_os = "macos")]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(target_os = "linux")]
fn desktop_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    #[test]
    fn detects_linux_net_admin_capability() {
        assert!(super::linux_status_has_cap_net_admin(
            "Name:\ttest\nCapEff:\t0000000000001000\n"
        ));
        assert!(!super::linux_status_has_cap_net_admin(
            "Name:\ttest\nCapEff:\t0000000000000000\n"
        ));
    }
}
