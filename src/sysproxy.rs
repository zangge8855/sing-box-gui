use std::sync::atomic::{AtomicBool, Ordering};

static SYSTEM_PROXY_OWNED: AtomicBool = AtomicBool::new(false);

#[allow(dead_code)]
fn backup_path() -> std::path::PathBuf {
    crate::config::get_app_dir().join("system-proxy-backup.json")
}

#[allow(dead_code)]
fn save_platform_backup(platform: &str, payload: serde_json::Value) -> Result<(), String> {
    let value = serde_json::json!({ "platform": platform, "payload": payload });
    let bytes = serde_json::to_vec(&value)
        .map_err(|error| format!("Failed to serialize system proxy backup: {error}"))?;
    crate::config::atomic_write(&backup_path(), &bytes)
}

#[allow(dead_code)]
fn load_platform_backup(platform: &str) -> Option<serde_json::Value> {
    let bytes = std::fs::read(backup_path()).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    (value.get("platform")?.as_str()? == platform).then(|| value.get("payload").cloned())?
}

#[allow(dead_code)]
fn clear_platform_backup() {
    let path = backup_path();
    let _ = std::fs::remove_file(&path);
    if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
        let _ = std::fs::remove_file(path.with_file_name(format!("{file_name}.bak")));
    }
}

pub fn set_system_proxy(enable: bool, port: u16) -> Result<(), String> {
    match platform::set_system_proxy(enable, port) {
        Ok(()) => {
            SYSTEM_PROXY_OWNED.store(enable, Ordering::Release);
            Ok(())
        }
        Err(error) => {
            // Enabling is transactional on supported platforms. A failed
            // attempt rolls back to the user's snapshot, so stale ownership
            // must not survive from an earlier app-managed endpoint.
            if enable {
                SYSTEM_PROXY_OWNED.store(false, Ordering::Release);
            }
            Err(error)
        }
    }
}

/// Whether this process successfully enabled the current system proxy.
/// A proxy that was already active before launch is deliberately not owned.
pub fn is_system_proxy_owned() -> bool {
    SYSTEM_PROXY_OWNED.load(Ordering::Acquire)
}

/// Restore ownership information loaded from persisted GUI configuration.
/// This does not touch the operating system; it only tells cleanup code which
/// proxy state belongs to this application instance.
pub fn restore_system_proxy_owned(owned: bool) {
    SYSTEM_PROXY_OWNED.store(owned, Ordering::Release);
}

/// Whether a durable platform snapshot exists from an earlier app-owned
/// proxy session. This is intentionally read-only; callers use it as a final
/// cleanup safety net after an abnormal process exit.
pub fn has_persisted_backup() -> bool {
    platform::has_persisted_backup()
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
    const BACKUP_MARKER: &str = "SingBoxGuiProxyBackupVersion";
    const BACKUP_ENABLE: &str = "SingBoxGuiProxyBackupEnable";
    const BACKUP_SERVER: &str = "SingBoxGuiProxyBackupServer";
    const BACKUP_OVERRIDE: &str = "SingBoxGuiProxyBackupOverride";

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

    fn clear_proxy_backup(settings: &RegKey) -> Result<(), String> {
        let _ = settings.delete_value(BACKUP_ENABLE);
        let _ = settings.delete_value(BACKUP_SERVER);
        let _ = settings.delete_value(BACKUP_OVERRIDE);
        settings
            .delete_value(BACKUP_MARKER)
            .map_err(|e| format!("Failed to clear Windows proxy backup marker: {e}"))
    }

    pub fn has_persisted_backup() -> bool {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        hkcu.open_subkey_with_flags(INTERNET_SETTINGS, KEY_READ)
            .ok()
            .and_then(|settings| settings.get_value::<u32, _>(BACKUP_MARKER).ok())
            .is_some()
    }

    fn restore_proxy_backup(
        settings: &RegKey,
        expected_port: u16,
        force: bool,
    ) -> Result<bool, String> {
        if settings.get_value::<u32, _>(BACKUP_MARKER).is_err() {
            return Ok(false);
        }
        let current_enable: u32 = settings.get_value("ProxyEnable").unwrap_or_default();
        let current_server: String = settings.get_value("ProxyServer").unwrap_or_default();
        let still_owned = current_enable != 0
            && parse_proxy_server(&current_server).is_some_and(|(host, port)| {
                matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1") && port == expected_port
            });
        if !force && !still_owned {
            clear_proxy_backup(settings)?;
            return Ok(true);
        }
        let original_enable: u32 = settings.get_value(BACKUP_ENABLE).unwrap_or_default();
        let original_server: String = settings.get_value(BACKUP_SERVER).unwrap_or_default();
        let original = settings
            .get_value::<String, _>(BACKUP_OVERRIDE)
            .unwrap_or_default();
        let current: String = settings.get_value("ProxyOverride").unwrap_or_default();
        let app_entries = ["localhost", "127.0.0.1", "::1", "<local>"];
        let mut merged: Vec<String> = original
            .split(';')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(str::to_string)
            .collect();
        for entry in current.split(';').map(str::trim).filter(|entry| {
            !entry.is_empty()
                && !app_entries
                    .iter()
                    .any(|app| app.eq_ignore_ascii_case(entry))
        }) {
            if !merged
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(entry))
            {
                merged.push(entry.to_string());
            }
        }
        settings
            .set_value("ProxyOverride", &merged.join(";"))
            .map_err(|e| format!("Failed to restore Windows bypass list: {e}"))?;

        if force || still_owned {
            settings
                .set_value("ProxyServer", &original_server)
                .map_err(|e| format!("Failed to restore Windows proxy server: {e}"))?;
            settings
                .set_value("ProxyEnable", &original_enable)
                .map_err(|e| format!("Failed to restore Windows proxy state: {e}"))?;
        }

        clear_proxy_backup(settings)?;
        Ok(true)
    }

    pub fn set_system_proxy(enable: bool, port: u16) -> Result<(), String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let settings = hkcu
            .open_subkey_with_flags(INTERNET_SETTINGS, KEY_READ | KEY_SET_VALUE)
            .map_err(|e| format!("Failed to open Windows proxy settings: {e}"))?;
        if enable {
            // Snapshot all user-controlled proxy values before the first
            // enable. The backup survives an app restart and lets shutdown
            // restore an already-enabled corporate proxy instead of blindly
            // forcing Windows back to "direct".
            let marker: Option<u32> = settings.get_value(BACKUP_MARKER).ok();
            let merged_override = if marker.is_none() {
                let original_enable: u32 = settings.get_value("ProxyEnable").unwrap_or_default();
                let original_server: String = settings.get_value("ProxyServer").unwrap_or_default();
                let original: String = settings.get_value("ProxyOverride").unwrap_or_default();
                settings
                    .set_value(BACKUP_ENABLE, &original_enable)
                    .map_err(|e| format!("Failed to back up Windows proxy state: {e}"))?;
                settings
                    .set_value(BACKUP_SERVER, &original_server)
                    .map_err(|e| format!("Failed to back up Windows proxy server: {e}"))?;
                settings
                    .set_value(BACKUP_OVERRIDE, &original)
                    .map_err(|e| format!("Failed to back up Windows bypass list: {e}"))?;
                settings
                    .set_value(BACKUP_MARKER, &1u32)
                    .map_err(|e| format!("Failed to mark Windows proxy backup: {e}"))?;
                Some(if original.trim().is_empty() {
                    "localhost;127.0.0.1;::1;<local>".to_string()
                } else {
                    format!("{original};localhost;127.0.0.1;::1;<local>")
                })
            } else {
                None
            };
            let apply_result = (|| {
                if let Some(merged) = merged_override {
                    settings
                        .set_value("ProxyOverride", &merged)
                        .map_err(|e| format!("Failed to update Windows proxy bypass list: {e}"))?;
                }
                settings
                    .set_value("ProxyServer", &format!("127.0.0.1:{port}"))
                    .map_err(|e| format!("Failed to update Windows proxy server: {e}"))?;
                settings
                    .set_value("ProxyEnable", &1u32)
                    .map_err(|e| format!("Failed to update Windows proxy state: {e}"))?;
                Ok::<(), String>(())
            })();
            if let Err(error) = apply_result {
                return match restore_proxy_backup(&settings, port, true) {
                    Ok(_) => Err(error),
                    Err(rollback) => Err(format!("{error}; rollback failed: {rollback}")),
                };
            }
        } else {
            // Restore the complete user state if we previously backed it up.
            if !restore_proxy_backup(&settings, port, false)? {
                let current_server: String = settings.get_value("ProxyServer").unwrap_or_default();
                if parse_proxy_server(&current_server) == Some(("127.0.0.1".to_string(), port)) {
                    settings
                        .set_value("ProxyEnable", &0u32)
                        .map_err(|e| format!("Failed to disable Windows proxy: {e}"))?;
                }
            }
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

    fn restore_backup(expected_port: u16, force: bool) -> Result<bool, String> {
        let Some(payload) = super::load_platform_backup("linux") else {
            return Ok(false);
        };
        if !force {
            let (enabled, host, configured_port) = get_system_proxy()?;
            let still_owned = enabled
                && matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1")
                && configured_port == expected_port;
            if !still_owned {
                super::clear_platform_backup();
                return Ok(true);
            }
        }
        let values = payload
            .as_object()
            .ok_or_else(|| "Invalid Linux proxy backup".to_string())?;
        for service in ["http", "https", "socks"] {
            let schema = format!("{PROXY_SCHEMA}.{service}");
            let host = values
                .get(&format!("{service}_host"))
                .and_then(|v| v.as_str())
                .unwrap_or("'127.0.0.1'");
            let service_port = values
                .get(&format!("{service}_port"))
                .and_then(|v| v.as_str())
                .unwrap_or("0");
            set_value(&schema, "host", host)?;
            set_value(&schema, "port", service_port)?;
        }
        if let Some(ignore_hosts) = values.get("ignore_hosts").and_then(|v| v.as_str()) {
            set_value(PROXY_SCHEMA, "ignore-hosts", ignore_hosts)?;
        }
        let mode = values
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("'none'");
        set_value(PROXY_SCHEMA, "mode", mode)?;
        super::clear_platform_backup();
        Ok(true)
    }

    pub fn has_persisted_backup() -> bool {
        super::load_platform_backup("linux").is_some()
    }

    pub fn set_system_proxy(enable: bool, port: u16) -> Result<(), String> {
        if enable {
            if super::load_platform_backup("linux").is_none() {
                let mut values = serde_json::Map::new();
                values.insert(
                    "mode".to_string(),
                    serde_json::Value::String(get_value(PROXY_SCHEMA, "mode")?),
                );
                values.insert(
                    "ignore_hosts".to_string(),
                    serde_json::Value::String(get_value(PROXY_SCHEMA, "ignore-hosts")?),
                );
                for service in ["http", "https", "socks"] {
                    let schema = format!("{PROXY_SCHEMA}.{service}");
                    values.insert(
                        format!("{service}_host"),
                        serde_json::Value::String(get_value(&schema, "host")?),
                    );
                    values.insert(
                        format!("{service}_port"),
                        serde_json::Value::String(get_value(&schema, "port")?),
                    );
                }
                super::save_platform_backup("linux", serde_json::Value::Object(values))?;
            }

            let apply_result = (|| {
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
            })();
            if let Err(error) = apply_result {
                return match restore_backup(port, true) {
                    Ok(_) => Err(error),
                    Err(rollback) => Err(format!("{error}; rollback failed: {rollback}")),
                };
            }
            Ok(())
        } else if restore_backup(port, false)? {
            Ok(())
        } else {
            // No durable snapshot means this process did not own the proxy.
            // Avoid disabling an unrelated manual proxy after a crash.
            let (enabled, host, configured_port) = get_system_proxy()?;
            if enabled && host == "127.0.0.1" && configured_port == port {
                set_value(PROXY_SCHEMA, "mode", "'none'")?;
            }
            Ok(())
        }
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

    #[cfg_attr(not(test), allow(dead_code))]
    fn service_for_interface(order: &str, device: &str) -> Option<String> {
        let mut current_service = None;
        for line in order.lines().map(str::trim) {
            // `-listnetworkserviceorder` emits entries such as
            // `(1) Wi-Fi` followed by `(Hardware Port: Wi-Fi, Device: en0)`.
            if line.starts_with('(')
                && !line.starts_with("(Hardware Port:")
                && let Some((_, name)) = line.split_once(") ")
            {
                let name = name.trim().trim_start_matches('*').trim();
                if !name.is_empty() {
                    current_service = Some(name.to_string());
                }
            }
            if let Some(device_value) = line.split_once("Device:").map(|(_, value)| value) {
                let device_value = device_value.trim().trim_end_matches(')').trim();
                if device_value == device {
                    return current_service;
                }
            }
        }
        None
    }

    fn default_network_service() -> Result<String, String> {
        if let Ok(route) = run("route", &["-n", "get", "default"])
            && let Some(device) = String::from_utf8_lossy(&route.stdout)
                .lines()
                .find_map(|line| line.trim().strip_prefix("interface:"))
                .map(str::trim)
        {
            if let Ok(order) = run("networksetup", &["-listnetworkserviceorder"])
                && let Some(service) =
                    service_for_interface(&String::from_utf8_lossy(&order.stdout), device)
            {
                return Ok(service);
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

    fn backup_service() -> Result<Option<String>, String> {
        let Some(payload) = super::load_platform_backup("macos") else {
            return Ok(None);
        };
        let service = payload
            .get("service")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "Invalid macOS proxy backup service".to_string())?;
        Ok(Some(service.to_string()))
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

    fn set_proxy_state(service: &str, kind: &str, enable: bool) -> Result<(), String> {
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

    fn restore_backup(expected_port: u16, force: bool) -> Result<bool, String> {
        let Some(payload) = super::load_platform_backup("macos") else {
            return Ok(false);
        };
        let service = payload
            .get("service")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Invalid macOS proxy backup service".to_string())?;
        if !force {
            let output = run("networksetup", &["-getwebproxy", service])?;
            let (enabled, host, configured_port) =
                parse_proxy(&String::from_utf8_lossy(&output.stdout))?;
            let still_owned = enabled
                && matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1")
                && configured_port == expected_port;
            if !still_owned {
                super::clear_platform_backup();
                return Ok(true);
            }
        }
        let kinds = payload
            .get("kinds")
            .and_then(|v| v.as_object())
            .ok_or_else(|| "Invalid macOS proxy backup entries".to_string())?;
        for kind in ["webproxy", "securewebproxy", "socksfirewallproxy"] {
            if let Some(raw) = kinds.get(kind).and_then(|v| v.as_str()) {
                let (was_enabled, host, old_port) = parse_proxy(raw)?;
                if !host.is_empty() && old_port > 0 {
                    run(
                        "networksetup",
                        &[
                            &format!("-set{kind}"),
                            service,
                            &host,
                            &old_port.to_string(),
                        ],
                    )?;
                }
                set_proxy_state(service, kind, was_enabled)?;
            }
        }
        let bypass = payload
            .get("bypass")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let domains: Vec<String> = bypass
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("There aren't"))
            .map(str::to_string)
            .collect();
        let mut args = vec!["-setproxybypassdomains".to_string(), service.to_string()];
        if domains.is_empty() {
            args.push("Empty".to_string());
        } else {
            args.extend(domains);
        }
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        run("networksetup", &refs)?;
        super::clear_platform_backup();
        Ok(true)
    }

    pub fn has_persisted_backup() -> bool {
        super::load_platform_backup("macos").is_some()
    }

    pub fn set_system_proxy(enable: bool, port: u16) -> Result<(), String> {
        if enable {
            // If the process is reapplying a proxy after a restart or a port
            // change, keep using the service captured in the durable snapshot.
            // Resolving the current default service here could otherwise leave
            // an old service configured and restore the wrong one on exit.
            let service = if let Some(saved) = backup_service()? {
                saved
            } else {
                default_network_service()?
            };
            if super::load_platform_backup("macos").is_none() {
                let mut kinds = serde_json::Map::new();
                for kind in ["webproxy", "securewebproxy", "socksfirewallproxy"] {
                    let output = run("networksetup", &[&format!("-get{kind}"), &service])?;
                    kinds.insert(
                        kind.to_string(),
                        serde_json::Value::String(String::from_utf8_lossy(&output.stdout).into()),
                    );
                }
                let bypass = run("networksetup", &["-getproxybypassdomains", &service])
                    .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
                    .unwrap_or_default();
                super::save_platform_backup(
                    "macos",
                    serde_json::json!({
                        "service": service,
                        "kinds": kinds,
                        "bypass": bypass,
                    }),
                )?;
            }
            let apply_result = (|| {
                for kind in ["webproxy", "securewebproxy", "socksfirewallproxy"] {
                    set_proxy(&service, kind, true, port)?;
                }
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
                Ok::<(), String>(())
            })();
            if let Err(error) = apply_result {
                return match restore_backup(port, true) {
                    Ok(_) => Err(error),
                    Err(rollback) => Err(format!("{error}; rollback failed: {rollback}")),
                };
            }
            Ok(())
        } else {
            // Restoring a saved service must not depend on the current network
            // route still being available (for example after Wi-Fi changes).
            if restore_backup(port, false)? {
                return Ok(());
            }
            let service = default_network_service()?;
            let output = run("networksetup", &["-getwebproxy", &service])?;
            let (enabled, host, configured_port) =
                parse_proxy(&String::from_utf8_lossy(&output.stdout))?;
            if enabled && host == "127.0.0.1" && configured_port == port {
                for kind in ["webproxy", "securewebproxy", "socksfirewallproxy"] {
                    set_proxy_state(&service, kind, false)?;
                }
            }
            Ok(())
        }
    }

    pub fn get_system_proxy() -> Result<(bool, String, u16), String> {
        let service = default_network_service()?;
        let output = run("networksetup", &["-getwebproxy", &service])?;
        parse_proxy(&String::from_utf8_lossy(&output.stdout))
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod platform {
    pub fn has_persisted_backup() -> bool {
        false
    }

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

    #[cfg(target_os = "macos")]
    #[test]
    fn maps_macos_interface_to_network_service_order() {
        use super::platform::service_for_interface;

        let order = "An asterisk (*) denotes that a network service is disabled.\n(1) Office Wi-Fi\n(Hardware Port: Wi-Fi, Device: en0)\n(2) *USB\n(Hardware Port: USB 10/100/1000 LAN, Device: en7)\n";
        assert_eq!(
            service_for_interface(order, "en0"),
            Some("Office Wi-Fi".to_string())
        );
        assert_eq!(service_for_interface(order, "en9"), None);
    }
}
