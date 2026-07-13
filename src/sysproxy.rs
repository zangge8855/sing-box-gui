use std::sync::atomic::{AtomicBool, Ordering};

static SYSTEM_PROXY_OWNED: AtomicBool = AtomicBool::new(false);

pub fn set_system_proxy(enable: bool, port: u16) -> Result<(), String> {
    platform::set_system_proxy(enable, port)?;
    SYSTEM_PROXY_OWNED.store(enable, Ordering::Release);
    Ok(())
}

/// Whether this process successfully enabled the current system proxy.
/// A proxy that was already active before launch is deliberately not owned.
pub fn is_system_proxy_owned() -> bool {
    SYSTEM_PROXY_OWNED.load(Ordering::Acquire)
}

pub fn check_system_proxy(port: u16) -> Result<bool, String> {
    let (enabled, host, configured_port) = platform::get_system_proxy()?;
    Ok(enabled
        && matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1")
        && configured_port == port)
}

#[cfg(target_os = "windows")]
mod platform {
    use std::ffi::c_void;
    use std::ptr;
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};

    const INTERNET_SETTINGS: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Internet Settings";
    const INTERNET_OPTION_REFRESH: u32 = 37;
    const INTERNET_OPTION_SETTINGS_CHANGED: u32 = 39;

    #[link(name = "wininet")]
    unsafe extern "system" {
        fn InternetSetOptionW(
            internet: *mut c_void,
            option: u32,
            buffer: *mut c_void,
            buffer_length: u32,
        ) -> i32;
    }

    fn notify_proxy_changed() {
        unsafe {
            let _ = InternetSetOptionW(
                ptr::null_mut(),
                INTERNET_OPTION_SETTINGS_CHANGED,
                ptr::null_mut(),
                0,
            );
            let _ =
                InternetSetOptionW(ptr::null_mut(), INTERNET_OPTION_REFRESH, ptr::null_mut(), 0);
        }
    }

    pub(super) fn parse_proxy_server(value: &str) -> Option<(String, u16)> {
        let endpoint = if value.contains('=') {
            value.split(';').find_map(|part| {
                let (scheme, endpoint) = part.split_once('=')?;
                matches!(scheme.trim(), "http" | "https" | "socks").then_some(endpoint.trim())
            })?
        } else {
            value.trim()
        };

        if let Some(rest) = endpoint.strip_prefix('[') {
            let (host, port) = rest.split_once("]:")?;
            return Some((host.to_string(), port.parse().ok()?));
        }
        let (host, port) = endpoint.rsplit_once(':')?;
        Some((host.trim().to_string(), port.trim().parse().ok()?))
    }

    pub fn set_system_proxy(enable: bool, port: u16) -> Result<(), String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let settings = hkcu
            .open_subkey_with_flags(INTERNET_SETTINGS, KEY_SET_VALUE)
            .map_err(|e| format!("Failed to open Windows proxy settings: {e}"))?;
        settings
            .set_value("ProxyEnable", &(u32::from(enable)))
            .map_err(|e| format!("Failed to update Windows proxy state: {e}"))?;
        if enable {
            settings
                .set_value("ProxyServer", &format!("127.0.0.1:{port}"))
                .map_err(|e| format!("Failed to update Windows proxy server: {e}"))?;
            settings
                .set_value("ProxyOverride", &"localhost;127.0.0.1;::1;<local>")
                .map_err(|e| format!("Failed to update Windows proxy bypass list: {e}"))?;
        }
        notify_proxy_changed();
        Ok(())
    }

    pub fn get_system_proxy() -> Result<(bool, String, u16), String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let settings = hkcu
            .open_subkey_with_flags(INTERNET_SETTINGS, KEY_READ)
            .map_err(|e| format!("Failed to read Windows proxy settings: {e}"))?;
        let enabled = settings
            .get_value::<u32, _>("ProxyEnable")
            .unwrap_or_default()
            != 0;
        let server = settings
            .get_value::<String, _>("ProxyServer")
            .unwrap_or_default();
        let (host, port) = parse_proxy_server(&server).unwrap_or_default();
        Ok((enabled, host, port))
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use std::process::{Command, Output};

    const PROXY_SCHEMA: &str = "org.gnome.system.proxy";

    fn run_gsettings(args: &[&str]) -> Result<Output, String> {
        let output = Command::new("gsettings")
            .args(args)
            .output()
            .map_err(|e| format!("Failed to run gsettings: {e}"))?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(format!(
                "gsettings failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }

    fn set_value(schema: &str, key: &str, value: &str) -> Result<(), String> {
        run_gsettings(&["set", schema, key, value]).map(|_| ())
    }

    fn get_value(schema: &str, key: &str) -> Result<String, String> {
        let output = run_gsettings(&["get", schema, key])?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn set_system_proxy(enable: bool, port: u16) -> Result<(), String> {
        if !enable {
            return set_value(PROXY_SCHEMA, "mode", "'none'");
        }

        for service in ["http", "https", "socks"] {
            let schema = format!("{PROXY_SCHEMA}.{service}");
            set_value(&schema, "host", "'127.0.0.1'")?;
            set_value(&schema, "port", &port.to_string())?;
        }
        set_value(
            PROXY_SCHEMA,
            "ignore-hosts",
            "['localhost', '127.0.0.1', '::1']",
        )?;
        set_value(PROXY_SCHEMA, "mode", "'manual'")
    }

    pub fn get_system_proxy() -> Result<(bool, String, u16), String> {
        let enabled = get_value(PROXY_SCHEMA, "mode")? == "'manual'";
        let host = get_value(&format!("{PROXY_SCHEMA}.http"), "host")?
            .trim_matches(['\'', '"'])
            .to_string();
        let port = get_value(&format!("{PROXY_SCHEMA}.http"), "port")?
            .parse::<u16>()
            .map_err(|e| format!("Failed to parse GNOME proxy port: {e}"))?;
        Ok((enabled, host, port))
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::process::{Command, Output};

    fn run(command: &str, args: &[&str]) -> Result<Output, String> {
        let output = Command::new(command)
            .args(args)
            .output()
            .map_err(|e| format!("Failed to run {command}: {e}"))?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(format!(
                "{command} failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }

    fn default_network_service() -> Result<String, String> {
        if let Ok(route) = run("route", &["-n", "get", "default"])
            && let Some(device) = String::from_utf8_lossy(&route.stdout)
                .lines()
                .find_map(|line| line.trim().strip_prefix("interface:"))
                .map(str::trim)
        {
            let ports = run("networksetup", &["-listallhardwareports"])?;
            for block in String::from_utf8_lossy(&ports.stdout).split("\n\n") {
                let mut hardware_port = None;
                let mut block_device = None;
                for line in block.lines().map(str::trim) {
                    if let Some(value) = line.strip_prefix("Hardware Port:") {
                        hardware_port = Some(value.trim());
                    } else if let Some(value) = line.strip_prefix("Device:") {
                        block_device = Some(value.trim());
                    }
                }
                if block_device == Some(device)
                    && let Some(service) = hardware_port
                {
                    return Ok(service.to_string());
                }
            }
        }

        let services = run("networksetup", &["-listallnetworkservices"])?;
        String::from_utf8_lossy(&services.stdout)
            .lines()
            .map(str::trim)
            .find(|line| {
                !line.is_empty() && !line.starts_with("An asterisk") && !line.starts_with('*')
            })
            .map(str::to_string)
            .ok_or_else(|| "No enabled macOS network service was found".to_string())
    }

    fn set_proxy(service: &str, kind: &str, enable: bool, port: u16) -> Result<(), String> {
        run(
            "networksetup",
            &[
                &format!("-set{kind}"),
                service,
                "127.0.0.1",
                &port.to_string(),
            ],
        )?;
        run(
            "networksetup",
            &[
                &format!("-set{kind}state"),
                service,
                if enable { "on" } else { "off" },
            ],
        )?;
        Ok(())
    }

    fn parse_proxy(output: &str) -> Result<(bool, String, u16), String> {
        let value = |key: &str| {
            output
                .lines()
                .find_map(|line| line.trim().strip_prefix(key))
                .map(str::trim)
        };
        let enabled = value("Enabled:") == Some("Yes");
        let host = value("Server:").unwrap_or_default().to_string();
        let port = value("Port:")
            .unwrap_or("0")
            .parse::<u16>()
            .map_err(|e| format!("Failed to parse macOS proxy port: {e}"))?;
        Ok((enabled, host, port))
    }

    pub fn set_system_proxy(enable: bool, port: u16) -> Result<(), String> {
        let service = default_network_service()?;
        for kind in ["webproxy", "securewebproxy", "socksfirewallproxy"] {
            set_proxy(&service, kind, enable, port)?;
        }
        if enable {
            run(
                "networksetup",
                &[
                    "-setproxybypassdomains",
                    &service,
                    "localhost",
                    "127.0.0.1",
                    "::1",
                ],
            )?;
        }
        Ok(())
    }

    pub fn get_system_proxy() -> Result<(bool, String, u16), String> {
        let service = default_network_service()?;
        let output = run("networksetup", &["-getwebproxy", &service])?;
        parse_proxy(&String::from_utf8_lossy(&output.stdout))
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod platform {
    pub fn set_system_proxy(_enable: bool, _port: u16) -> Result<(), String> {
        Err("System proxy is not supported on this platform".to_string())
    }

    pub fn get_system_proxy() -> Result<(bool, String, u16), String> {
        Err("System proxy is not supported on this platform".to_string())
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "windows")]
    #[test]
    fn parses_windows_proxy_server_formats() {
        use super::platform::parse_proxy_server;

        assert_eq!(
            parse_proxy_server("127.0.0.1:2080"),
            Some(("127.0.0.1".to_string(), 2080))
        );
        assert_eq!(
            parse_proxy_server("http=127.0.0.1:2080;https=127.0.0.1:2080"),
            Some(("127.0.0.1".to_string(), 2080))
        );
        assert_eq!(
            parse_proxy_server("[::1]:2080"),
            Some(("::1".to_string(), 2080))
        );
    }
}
