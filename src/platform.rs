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
                fn OpenProcessToken(process: HANDLE, access: u32, token: *mut HANDLE) -> BOOL;
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

#[cfg(any(target_os = "linux", test))]
fn linux_status_has_cap_net_admin(status: &str) -> bool {
    const CAP_NET_ADMIN_BIT: u32 = 12;
    status
        .lines()
        .find_map(|line| {
            let value = line.strip_prefix("CapEff:")?.trim();
            let capabilities = u64::from_str_radix(value, 16).ok()?;
            Some(capabilities & (1u64 << CAP_NET_ADMIN_BIT) != 0)
        })
        .unwrap_or(false)
}

#[cfg(any(target_os = "linux", test))]
fn vfs_cap_has_net_admin(raw: &[u8]) -> bool {
    const VFS_CAP_REVISION_1: u32 = 0x0100_0000;
    const VFS_CAP_REVISION_2: u32 = 0x0200_0000;
    const VFS_CAP_REVISION_3: u32 = 0x0300_0000;
    const VFS_CAP_REVISION_MASK: u32 = 0xFF00_0000;
    const VFS_CAP_FLAGS_EFFECTIVE: u32 = 0x0000_0001;
    const CAP_NET_ADMIN_MASK: u32 = 1u32 << 12;

    if raw.len() < 12 {
        return false;
    }
    let magic_etc = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
    let valid_revision = matches!(
        magic_etc & VFS_CAP_REVISION_MASK,
        VFS_CAP_REVISION_1 | VFS_CAP_REVISION_2 | VFS_CAP_REVISION_3
    );
    if !valid_revision || (magic_etc & VFS_CAP_FLAGS_EFFECTIVE) == 0 {
        return false;
    }
    let permitted_low = u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);
    (permitted_low & CAP_NET_ADMIN_MASK) != 0
}

#[cfg(any(target_os = "linux", test))]
fn getcap_output_has_cap_net_admin(output: &str) -> bool {
    // Handles both older ("path = cap_net_admin+ep") and newer ("path cap_net_admin=ep") formats.
    output.lines().any(|line| {
        line.split_whitespace().any(|token| {
            let lower = token.to_ascii_lowercase();
            let Some((caps, flags)) = lower.split_once(['=', '+']) else {
                return false;
            };
            caps.split(',')
                .any(|cap| cap == "cap_net_admin" || cap == "all")
                && flags.contains('e')
                && flags.contains('p')
        })
    })
}

#[cfg(target_os = "linux")]
fn binary_has_cap_net_admin(path: &std::path::Path) -> bool {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    if let Ok(c_path) = CString::new(path.as_os_str().as_bytes()) {
        let mut buf = [0u8; 64];
        let len = unsafe {
            libc::getxattr(
                c_path.as_ptr(),
                c"security.capability".as_ptr(),
                buf.as_mut_ptr().cast(),
                buf.len(),
            )
        };
        if len > 0 {
            let len = usize::try_from(len).unwrap_or(0).min(buf.len());
            return vfs_cap_has_net_admin(&buf[..len]);
        }
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ENODATA) {
            return false;
        }
    }

    for getcap_bin in ["getcap", "/usr/sbin/getcap", "/sbin/getcap"] {
        if let Ok(output) = std::process::Command::new(getcap_bin).arg(path).output()
            && output.status.success()
        {
            return getcap_output_has_cap_net_admin(&String::from_utf8_lossy(&output.stdout));
        }
    }
    false
}

/// Returns `true` when TUN mode can run without prompting for elevation.
///
/// On Linux this returns `true` when the GUI process is elevated **or** when
/// `core_path` carries `cap_net_admin=ep` in its `security.capability` xattr.
#[allow(dead_code)]
pub fn is_tun_capable(core_path: Option<&std::path::Path>) -> bool {
    if is_running_elevated() {
        return true;
    }
    #[cfg(target_os = "linux")]
    if let Some(path) = core_path
        && path.exists()
        && binary_has_cap_net_admin(path)
    {
        return true;
    }
    #[cfg(not(target_os = "linux"))]
    let _ = core_path;
    false
}

/// Install or remove per-user launch integration on the current platform.
pub fn set_autostart(enable: bool) -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("Failed to resolve current executable: {e}"))?;

    #[cfg(target_os = "windows")]
    {
        use winreg::RegKey;
        use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};

        const RUN_KEY_PATH: &str = r#"Software\Microsoft\Windows\CurrentVersion\Run"#;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if enable {
            let (run_key, _) = hkcu
                .create_subkey(RUN_KEY_PATH)
                .map_err(|e| format!("Failed to open autostart registry key: {e}"))?;
            let value = format_windows_autostart_value(&exe.to_string_lossy());
            run_key
                .set_value("sing-box-gui", &value)
                .map_err(|e| format!("Failed to write autostart registry: {e}"))?;
        } else {
            let run_key = match hkcu.open_subkey_with_flags(RUN_KEY_PATH, KEY_WRITE) {
                Ok(key) => key,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(err) => return Err(format!("Failed to open autostart registry key: {err}")),
            };
            if let Err(err) = run_key.delete_value("sing-box-gui")
                && err.kind() != std::io::ErrorKind::NotFound
            {
                return Err(format!("Failed to delete autostart registry value: {err}"));
            }
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
            let plist = format_macos_autostart_plist(&exe.to_string_lossy());
            crate::config::atomic_write(&path, plist.as_bytes())?;
        } else if let Err(err) = std::fs::remove_file(&path)
            && err.kind() != std::io::ErrorKind::NotFound
        {
            return Err(format!("Failed to remove LaunchAgent: {err}"));
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let config =
            dirs::config_dir().ok_or_else(|| "Config directory is unavailable".to_string())?;
        let autostart = config.join("autostart");
        let path = autostart.join("sing-box-gui.desktop");
        if enable {
            std::fs::create_dir_all(&autostart)
                .map_err(|e| format!("Failed to create autostart directory: {e}"))?;
            let entry = format_linux_autostart_entry(&exe.to_string_lossy());
            crate::config::atomic_write(&path, entry.as_bytes())?;
        } else if let Err(err) = std::fs::remove_file(&path)
            && err.kind() != std::io::ErrorKind::NotFound
        {
            return Err(format!("Failed to remove autostart entry: {err}"));
        }
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("Autostart is not supported on this platform".to_string())
}

#[cfg(any(target_os = "windows", test))]
fn format_windows_autostart_value(exe: &str) -> String {
    let escaped = exe.replace('"', "\\\"");
    format!("\"{escaped}\" --minimized")
}

#[cfg(any(target_os = "macos", test))]
fn format_macos_autostart_plist(exe: &str) -> String {
    let exe = xml_escape(exe);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>Label</key><string>io.github.zangge8855.sing-box-gui</string><key>ProgramArguments</key><array><string>{exe}</string><string>--minimized</string></array><key>RunAtLoad</key><true/></dict></plist>\n"
    )
}

#[cfg(any(target_os = "linux", test))]
fn format_linux_autostart_entry(exe: &str) -> String {
    let exec = desktop_escape(exe);
    format!(
        "[Desktop Entry]\nType=Application\nName=sing-box GUI\nComment=Start sing-box GUI with the desktop session\nExec=\"{exec}\" --minimized\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
    )
}

#[cfg(any(target_os = "macos", test))]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(any(target_os = "linux", test))]
fn desktop_escape(value: &str) -> String {
    // The Exec value is unescaped twice: the Desktop Entry string escape runs
    // before quoted-argument parsing, `%` is a field-code prefix, and `` ` ``
    // and `$` are reserved inside quoted arguments. A literal backslash must
    // survive both levels, hence four backslashes.
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\\\\\"),
            '"' => escaped.push_str("\\\""),
            '$' => escaped.push_str("\\$"),
            '`' => escaped.push_str("\\`"),
            '%' => escaped.push_str("%%"),
            _ => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    #[test]
    fn desktop_escape_covers_exec_reserved_characters() {
        assert_eq!(
            super::desktop_escape("/usr/local/bin/sing-box-gui"),
            "/usr/local/bin/sing-box-gui"
        );
        assert_eq!(
            super::desktop_escape("/home/u/100%backup/app"),
            "/home/u/100%%backup/app"
        );
        assert_eq!(super::desktop_escape("a$b`c\"d"), "a\\$b\\`c\\\"d");
        assert_eq!(super::desktop_escape("a\\b"), "a\\\\\\\\b");
    }

    #[test]
    fn autostart_entries_include_minimized_flag() {
        assert_eq!(
            super::format_windows_autostart_value(r"C:\Program Files\sing-box\gui.exe"),
            "\"C:\\Program Files\\sing-box\\gui.exe\" --minimized"
        );
        let plist = super::format_macos_autostart_plist("/Applications/sing&box.app/gui");
        assert!(plist.contains(
            "<string>/Applications/sing&amp;box.app/gui</string><string>--minimized</string>"
        ));
        let desktop = super::format_linux_autostart_entry("/usr/bin/sing-box-gui");
        assert!(desktop.contains("Exec=\"/usr/bin/sing-box-gui\" --minimized\n"));
    }

    #[test]
    fn detects_linux_net_admin_capability() {
        assert!(super::linux_status_has_cap_net_admin(
            "Name:\ttest\nCapEff:\t0000000000001000\n"
        ));
        assert!(!super::linux_status_has_cap_net_admin(
            "Name:\ttest\nCapEff:\t0000000000000000\n"
        ));
    }

    #[test]
    fn parses_linux_vfs_capability_xattr_and_getcap_text() {
        // v2 header with effective bit set (0x0200_0001) + CAP_NET_ADMIN bit 12 (0x1000) in permitted
        let mut raw = [0u8; 20];
        raw[0..4].copy_from_slice(&0x0200_0001u32.to_le_bytes());
        raw[4..8].copy_from_slice(&(1u32 << 12).to_le_bytes());
        assert!(super::vfs_cap_has_net_admin(&raw));

        // Permitted without effective bit should be false
        raw[0..4].copy_from_slice(&0x0200_0000u32.to_le_bytes());
        assert!(!super::vfs_cap_has_net_admin(&raw));

        // Different capability only (CAP_NET_BIND_SERVICE = bit 10)
        raw[0..4].copy_from_slice(&0x0200_0001u32.to_le_bytes());
        raw[4..8].copy_from_slice(&(1u32 << 10).to_le_bytes());
        assert!(!super::vfs_cap_has_net_admin(&raw));

        assert!(super::getcap_output_has_cap_net_admin(
            "/opt/sing-box cap_net_admin=ep\n"
        ));
        assert!(super::getcap_output_has_cap_net_admin(
            "/opt/sing-box = cap_net_bind_service,cap_net_admin+ep\n"
        ));
        assert!(!super::getcap_output_has_cap_net_admin(
            "/opt/sing-box cap_net_bind_service=ep\n"
        ));
        assert!(!super::getcap_output_has_cap_net_admin(
            "/opt/sing-box cap_net_admin=p cap_net_bind_service=ep\n"
        ));
    }

    #[test]
    fn tun_capable_matches_elevated_without_core_path() {
        assert_eq!(super::is_tun_capable(None), super::is_running_elevated());
    }

    #[test]
    fn vfs_cap_edge_cases_and_xml_escape_work() {
        // Short buffer and invalid revision fail closed.
        assert!(!super::vfs_cap_has_net_admin(&[0u8; 8]));
        let mut v1 = [0u8; 12];
        v1[0..4].copy_from_slice(&0x0100_0001u32.to_le_bytes());
        v1[4..8].copy_from_slice(&(1u32 << 12).to_le_bytes());
        assert!(super::vfs_cap_has_net_admin(&v1));

        let mut v3 = [0u8; 24];
        v3[0..4].copy_from_slice(&0x0300_0001u32.to_le_bytes());
        v3[4..8].copy_from_slice(&(1u32 << 12).to_le_bytes());
        assert!(super::vfs_cap_has_net_admin(&v3));

        let mut bad_rev = [0u8; 20];
        bad_rev[0..4].copy_from_slice(&0x0400_0001u32.to_le_bytes());
        bad_rev[4..8].copy_from_slice(&(1u32 << 12).to_le_bytes());
        assert!(!super::vfs_cap_has_net_admin(&bad_rev));

        assert!(super::getcap_output_has_cap_net_admin(
            "/opt/sing-box all=ep\n"
        ));
        assert_eq!(
            super::xml_escape("a<b>c'd\"e&f"),
            "a&lt;b&gt;c&apos;d&quot;e&amp;f"
        );
    }
}
