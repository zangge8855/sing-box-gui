use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

static SYSTEM_PROXY_OWNED: AtomicBool = AtomicBool::new(false);
static SYSPROXY_MUTEX: Mutex<()> = Mutex::new(());

#[allow(dead_code)]
fn backup_path() -> std::path::PathBuf {
    crate::config::get_app_dir().join("system-proxy-backup.json")
}

#[allow(dead_code)]
fn save_platform_backup_to(
    path: &std::path::Path,
    platform: &str,
    payload: serde_json::Value,
) -> Result<(), String> {
    let value = serde_json::json!({ "platform": platform, "payload": payload });
    let bytes = serde_json::to_vec(&value)
        .map_err(|error| format!("Failed to serialize system proxy backup: {error}"))?;
    crate::config::atomic_write(path, &bytes)
}

#[allow(dead_code)]
fn save_platform_backup(platform: &str, payload: serde_json::Value) -> Result<(), String> {
    save_platform_backup_to(&backup_path(), platform, payload)
}

#[allow(dead_code)]
fn parse_platform_backup_bytes(bytes: &[u8], platform: &str) -> Option<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    (value.get("platform")?.as_str()? == platform).then(|| value.get("payload").cloned())?
}

#[allow(dead_code)]
fn load_platform_backup_from(path: &std::path::Path, platform: &str) -> Option<serde_json::Value> {
    // Only fall back to `.bak` when the primary file still exists (for example if
    // a crash left a corrupt primary); an absent primary means backup was cleared.
    if !path.exists() {
        return None;
    }
    if let Ok(bytes) = std::fs::read(path)
        && let Some(payload) = parse_platform_backup_bytes(&bytes, platform)
    {
        return Some(payload);
    }
    let file_name = path.file_name().and_then(|name| name.to_str())?;
    let bak_path = path.with_file_name(format!("{file_name}.bak"));
    let bak_bytes = std::fs::read(bak_path).ok()?;
    parse_platform_backup_bytes(&bak_bytes, platform)
}

#[allow(dead_code)]
fn load_platform_backup(platform: &str) -> Option<serde_json::Value> {
    load_platform_backup_from(&backup_path(), platform)
}

#[allow(dead_code)]
fn clear_platform_backup_at(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
        let _ = std::fs::remove_file(path.with_file_name(format!("{file_name}.bak")));
    }
}

#[allow(dead_code)]
fn clear_platform_backup() {
    clear_platform_backup_at(&backup_path());
}

pub fn set_system_proxy(enable: bool, port: u16) -> Result<(), String> {
    if enable && port == 0 {
        return Err("Proxy port must be greater than zero".to_string());
    }
    let _guard = SYSPROXY_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
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
    Ok(enabled && is_loopback_proxy_host(&host) && configured_port == port)
}

#[allow(dead_code)]
fn is_loopback_proxy_host(host: &str) -> bool {
    let host = host.trim().trim_start_matches('[').trim_end_matches(']');
    host == "127.0.0.1" || host.eq_ignore_ascii_case("localhost") || host == "::1"
}

#[allow(dead_code)]
fn is_owned_proxy_endpoint(
    host: &str,
    configured_port: u16,
    expected_port: u16,
    recorded_port: Option<u16>,
) -> bool {
    configured_port > 0
        && is_loopback_proxy_host(host)
        && (expected_port == 0
            || configured_port == expected_port
            || recorded_port == Some(configured_port))
}

#[cfg(any(target_os = "windows", test))]
fn merge_windows_proxy_override(existing: &str) -> String {
    let app_entries = ["localhost", "127.0.0.1", "::1", "<local>"];
    let mut merged: Vec<String> = Vec::new();
    for entry in existing
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        if !merged.iter().any(|seen| seen.eq_ignore_ascii_case(entry)) {
            merged.push(entry.to_string());
        }
    }
    for required in app_entries {
        if !merged
            .iter()
            .any(|seen| seen.eq_ignore_ascii_case(required))
        {
            merged.push(required.to_string());
        }
    }
    merged.join(";")
}

#[cfg(any(target_os = "windows", test))]
fn restore_windows_proxy_override(original: &str, current: &str) -> String {
    let app_entries = ["localhost", "127.0.0.1", "::1", "<local>"];
    let mut merged: Vec<String> = Vec::new();
    for entry in original
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        if !merged.iter().any(|seen| seen.eq_ignore_ascii_case(entry)) {
            merged.push(entry.to_string());
        }
    }
    for entry in current.split(';').map(str::trim).filter(|entry| {
        !entry.is_empty()
            && !app_entries
                .iter()
                .any(|app| app.eq_ignore_ascii_case(entry))
    }) {
        if !merged.iter().any(|seen| seen.eq_ignore_ascii_case(entry)) {
            merged.push(entry.to_string());
        }
    }
    merged.join(";")
}

#[cfg(any(target_os = "linux", test))]
fn parse_gsettings_string_array(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    let trimmed = trimmed.strip_prefix("@as ").unwrap_or(trimmed).trim();
    let Some(inner) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    let mut chars = inner.chars().peekable();
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() || ch == ',' {
            chars.next();
            continue;
        }
        if ch == '\'' || ch == '"' {
            let quote = ch;
            chars.next();
            let mut item = String::new();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    if let Some(next) = chars.next() {
                        item.push(next);
                    }
                } else if c == quote {
                    break;
                } else {
                    item.push(c);
                }
            }
            let item = item.trim();
            if !item.is_empty() {
                items.push(item.to_string());
            }
        } else {
            let mut item = String::new();
            while let Some(&c) = chars.peek() {
                if c == ',' {
                    break;
                }
                item.push(c);
                chars.next();
            }
            let item = item.trim();
            if !item.is_empty() {
                items.push(item.to_string());
            }
        }
    }
    items
}

#[cfg(any(target_os = "linux", test))]
fn format_gsettings_string_array(items: &[String]) -> String {
    if items.is_empty() {
        return "@as []".to_string();
    }
    let quoted: Vec<String> = items
        .iter()
        .map(|item| {
            let escaped = item.replace('\\', "\\\\").replace('\'', "\\'");
            format!("'{escaped}'")
        })
        .collect();
    format!("[{}]", quoted.join(", "))
}

#[allow(dead_code)]
fn merge_linux_ignore_hosts(existing: &[String]) -> Vec<String> {
    let mut merged: Vec<String> = Vec::new();
    for entry in existing.iter().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if !merged.iter().any(|seen| seen.eq_ignore_ascii_case(entry)) {
            merged.push(entry.to_string());
        }
    }
    for required in ["localhost", "127.0.0.1", "::1"] {
        if !merged
            .iter()
            .any(|seen| seen.eq_ignore_ascii_case(required))
        {
            merged.push(required.to_string());
        }
    }
    merged
}

#[allow(dead_code)]
fn restore_linux_ignore_hosts(original: &[String], current: &[String]) -> Vec<String> {
    let app_entries = ["localhost", "127.0.0.1", "::1"];
    let mut merged: Vec<String> = Vec::new();
    for entry in original.iter().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if !merged.iter().any(|seen| seen.eq_ignore_ascii_case(entry)) {
            merged.push(entry.to_string());
        }
    }
    for entry in current.iter().map(|s| s.trim()).filter(|entry| {
        !entry.is_empty()
            && !app_entries
                .iter()
                .any(|app| app.eq_ignore_ascii_case(entry))
    }) {
        if !merged.iter().any(|seen| seen.eq_ignore_ascii_case(entry)) {
            merged.push(entry.to_string());
        }
    }
    merged
}

#[cfg(any(target_os = "linux", test))]
fn merge_kde_no_proxy(existing: &str) -> String {
    let items: Vec<String> = existing
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    merge_linux_ignore_hosts(&items).join(",")
}

#[cfg(any(target_os = "linux", test))]
fn restore_kde_no_proxy(original: &str, current: &str) -> String {
    let orig: Vec<String> = original
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    let curr: Vec<String> = current
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    restore_linux_ignore_hosts(&orig, &curr).join(",")
}

#[cfg(any(target_os = "windows", test))]
fn parse_windows_endpoint(endpoint: &str) -> Option<(String, u16)> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return None;
    }
    let endpoint = endpoint
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(endpoint)
        .trim()
        .trim_end_matches('/');
    if let Some(rest) = endpoint.strip_prefix('[') {
        let (host, port) = rest.split_once("]:")?;
        let host = host.trim();
        let port = port.trim().parse::<u16>().ok().filter(|&p| p > 0)?;
        return (!host.is_empty()).then(|| (host.to_string(), port));
    }
    let (host, port) = endpoint.rsplit_once(':')?;
    let host = host.trim();
    if host.is_empty() || host.contains(':') {
        return None;
    }
    let port = port.trim().parse::<u16>().ok().filter(|&p| p > 0)?;
    Some((host.to_string(), port))
}

#[cfg(any(target_os = "windows", test))]
fn parse_windows_proxy_server(value: &str) -> Option<(String, u16)> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains('=') {
        for preferred in ["http", "https", "socks", "socks5", "socks4"] {
            if let Some(found) = trimmed.split(';').find_map(|part| {
                let (scheme, endpoint) = part.split_once('=')?;
                scheme
                    .trim()
                    .eq_ignore_ascii_case(preferred)
                    .then(|| parse_windows_endpoint(endpoint))?
            }) {
                return Some(found);
            }
        }
        return None;
    }
    parse_windows_endpoint(trimmed)
}

#[cfg(any(target_os = "windows", test))]
const CONN_VERSION_IE6: u32 = 0x3C;
#[cfg(any(target_os = "windows", test))]
const CONN_VERSION_IE7: u32 = 0x46;
#[cfg(any(target_os = "windows", test))]
const PROXY_TYPE_DIRECT: u32 = 0x01;
#[cfg(any(target_os = "windows", test))]
const PROXY_TYPE_PROXY: u32 = 0x02;
#[cfg(any(target_os = "windows", test))]
const PROXY_TYPE_AUTO_PROXY_URL: u32 = 0x04;
#[cfg(any(target_os = "windows", test))]
const PROXY_TYPE_AUTO_DETECT: u32 = 0x08;
#[cfg(any(target_os = "windows", test))]
const MAX_CONN_STRING_LEN: usize = 65_536;

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowsConnectionSettings {
    version: u32,
    counter: u32,
    flags: u32,
    proxy_server: Vec<u8>,
    proxy_override: Vec<u8>,
    autoconfig_url: Vec<u8>,
    footer: Vec<u8>,
}

#[cfg(any(target_os = "windows", test))]
impl WindowsConnectionSettings {
    fn parse(blob: &[u8]) -> Option<Self> {
        if blob.len() < 24 {
            return None;
        }
        let version = u32::from_le_bytes(blob[0..4].try_into().ok()?);
        if !matches!(version, CONN_VERSION_IE6 | CONN_VERSION_IE7) {
            return None;
        }
        let counter = u32::from_le_bytes(blob[4..8].try_into().ok()?);
        let flags = u32::from_le_bytes(blob[8..12].try_into().ok()?);
        let mut cursor = 12usize;

        let read_len_field = |cursor: &mut usize| -> Option<Vec<u8>> {
            let len_end = cursor.checked_add(4)?;
            let len_bytes: [u8; 4] = blob.get(*cursor..len_end)?.try_into().ok()?;
            let len = usize::try_from(u32::from_le_bytes(len_bytes)).ok()?;
            if len > MAX_CONN_STRING_LEN {
                return None;
            }
            let data_end = len_end.checked_add(len)?;
            let data = blob.get(len_end..data_end)?.to_vec();
            *cursor = data_end;
            Some(data)
        };

        let proxy_server = read_len_field(&mut cursor)?;
        let proxy_override = read_len_field(&mut cursor)?;
        let autoconfig_url = read_len_field(&mut cursor)?;
        let footer = blob.get(cursor..)?.to_vec();

        Some(Self {
            version,
            counter,
            flags,
            proxy_server,
            proxy_override,
            autoconfig_url,
            footer,
        })
    }

    fn encode(&self) -> Result<Vec<u8>, String> {
        let write_len_field = |out: &mut Vec<u8>, bytes: &[u8]| -> Result<(), String> {
            if bytes.len() > MAX_CONN_STRING_LEN {
                return Err("Windows connection setting string exceeds maximum length".to_string());
            }
            let len = u32::try_from(bytes.len())
                .map_err(|_| "Windows connection setting length overflow".to_string())?;
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(bytes);
            Ok(())
        };

        let total = 12usize
            .saturating_add(12)
            .saturating_add(self.proxy_server.len())
            .saturating_add(self.proxy_override.len())
            .saturating_add(self.autoconfig_url.len())
            .saturating_add(self.footer.len());
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&self.counter.to_le_bytes());
        out.extend_from_slice(&self.flags.to_le_bytes());
        write_len_field(&mut out, &self.proxy_server)?;
        write_len_field(&mut out, &self.proxy_override)?;
        write_len_field(&mut out, &self.autoconfig_url)?;
        out.extend_from_slice(&self.footer);
        Ok(out)
    }

    fn default_with_proxy(port: u16) -> Self {
        Self {
            version: CONN_VERSION_IE7,
            counter: 1,
            flags: PROXY_TYPE_DIRECT | PROXY_TYPE_PROXY,
            proxy_server: format!("127.0.0.1:{port}").into_bytes(),
            proxy_override: merge_windows_proxy_override("").into_bytes(),
            autoconfig_url: Vec::new(),
            footer: vec![0u8; 32],
        }
    }

    fn apply_proxy(&mut self, port: u16) {
        self.counter = self.counter.wrapping_add(1);
        self.flags = (self.flags & !(PROXY_TYPE_AUTO_PROXY_URL | PROXY_TYPE_AUTO_DETECT))
            | PROXY_TYPE_DIRECT
            | PROXY_TYPE_PROXY;
        self.proxy_server = format!("127.0.0.1:{port}").into_bytes();
        let current_override = String::from_utf8_lossy(&self.proxy_override);
        self.proxy_override = merge_windows_proxy_override(&current_override).into_bytes();
        self.autoconfig_url.clear();
    }

    fn clear_proxy(&mut self) {
        self.counter = self.counter.wrapping_add(1);
        self.flags = (self.flags & !PROXY_TYPE_PROXY) | PROXY_TYPE_DIRECT;
    }

    fn is_owned_endpoint(&self, expected_port: u16, recorded_port: Option<u16>) -> bool {
        if (self.flags & PROXY_TYPE_PROXY) == 0 {
            return false;
        }
        let server = String::from_utf8_lossy(&self.proxy_server);
        parse_windows_proxy_server(&server).is_some_and(|(host, port)| {
            is_owned_proxy_endpoint(&host, port, expected_port, recorded_port)
        })
    }
}

#[cfg(any(target_os = "linux", test))]
fn parse_kde_proxy_endpoint(raw: &str) -> Option<(String, u16)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let without_scheme = trimmed
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed)
        .trim()
        .trim_end_matches('/');
    if let Some(rest) = without_scheme.strip_prefix('[') {
        let (host, after) = rest.split_once(']')?;
        let host = host.trim();
        let after = after.trim();
        let port_str = after.strip_prefix(':').unwrap_or(after).trim();
        let port = port_str.parse::<u16>().ok().filter(|&p| p > 0)?;
        return (!host.is_empty()).then(|| (host.to_string(), port));
    }
    if let Some((host, port)) = without_scheme.rsplit_once(':') {
        let host = host.trim();
        if !host.contains(':') {
            let port = port.trim().parse::<u16>().ok().filter(|&p| p > 0)?;
            return (!host.is_empty()).then(|| (host.to_string(), port));
        }
    }
    let mut parts = without_scheme.split_whitespace();
    let host = parts.next()?.trim();
    let port = parts
        .next()?
        .trim()
        .parse::<u16>()
        .ok()
        .filter(|&p| p > 0)?;
    if parts.next().is_some() {
        return None;
    }
    (!host.is_empty()).then(|| (host.to_string(), port))
}

#[cfg(any(target_os = "macos", test))]
fn is_virtual_interface(device: &str) -> bool {
    let lower = device.trim().to_ascii_lowercase();
    [
        "utun", "tun", "tap", "ppp", "ipsec", "gif", "stf", "lo", "awdl", "llw", "bridge", "anpi",
        "ap", "vmnet", "vmenet", "vboxnet", "feth", "pktap",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
}

#[cfg(any(target_os = "macos", test))]
fn is_secondary_service(service_name: &str) -> bool {
    let lower = service_name.trim().to_ascii_lowercase();
    lower.contains("thunderbolt bridge")
        || lower.contains("iphone usb")
        || lower.contains("bluetooth pan")
        || lower.contains("firewire")
}

#[cfg(any(target_os = "macos", test))]
fn parse_network_service_order(order: &str) -> Vec<(String, Option<String>)> {
    let mut services: Vec<(String, Option<String>)> = Vec::new();
    let mut pending_idx: Option<usize> = None;

    for line in order.lines().map(str::trim) {
        if line.starts_with('(') && !line.starts_with("(Hardware Port:") {
            if let Some((prefix, raw_name)) = line.split_once(") ") {
                let prefix_inner = prefix.trim_start_matches('(').trim();
                let trimmed = raw_name.trim();
                if prefix_inner == "*" || trimmed.starts_with('*') {
                    pending_idx = None;
                } else if !trimmed.is_empty() {
                    services.push((trimmed.to_string(), None));
                    pending_idx = Some(services.len() - 1);
                }
            }
        } else if let Some(idx) = pending_idx
            && let Some((_, device_value)) = line.split_once("Device:")
        {
            let device = device_value.trim().trim_end_matches(')').trim();
            if !device.is_empty() && !device.eq_ignore_ascii_case("none") {
                services[idx].1 = Some(device.to_string());
            }
        }
    }
    services
}

#[cfg(any(target_os = "macos", test))]
fn service_for_interface_in_order(order: &str, device: &str) -> Option<String> {
    parse_network_service_order(order)
        .into_iter()
        .find_map(|(service, dev)| (dev.as_deref() == Some(device)).then_some(service))
}

#[cfg(any(target_os = "macos", test))]
fn parse_active_physical_devices_from_ifconfig(output: &str) -> Vec<String> {
    let mut active = Vec::new();
    let mut current_iface: Option<String> = None;
    let mut is_up_running = false;
    let mut has_routable_ip = false;
    let mut status_inactive = false;

    let flush = |iface: Option<String>,
                 up_running: bool,
                 routable: bool,
                 inactive: bool,
                 out: &mut Vec<String>| {
        if let Some(name) = iface
            && up_running
            && routable
            && !inactive
            && !is_virtual_interface(&name)
            && !out.contains(&name)
        {
            out.push(name);
        }
    };

    for raw_line in output.lines() {
        if !raw_line.starts_with([' ', '\t']) && raw_line.contains(": flags=") {
            flush(
                current_iface.take(),
                is_up_running,
                has_routable_ip,
                status_inactive,
                &mut active,
            );
            let iface = raw_line.split(':').next().unwrap_or("").trim().to_string();
            current_iface = (!iface.is_empty()).then_some(iface);
            is_up_running = false;
            if let Some(flags_start) = raw_line.find('<')
                && let Some(flags_end) = raw_line[flags_start..].find('>')
            {
                let flags = &raw_line[flags_start + 1..flags_start + flags_end];
                let tokens: Vec<&str> = flags.split(',').collect();
                is_up_running = tokens.contains(&"UP") && tokens.contains(&"RUNNING");
            }
            has_routable_ip = false;
            status_inactive = false;
            continue;
        }

        let line = raw_line.trim();
        if let Some(rest) = line.strip_prefix("status:") {
            if rest.trim().eq_ignore_ascii_case("inactive") {
                status_inactive = true;
            }
        } else if let Some(rest) = line.strip_prefix("inet ") {
            let ip = rest.split_whitespace().next().unwrap_or("");
            if let Ok(addr) = ip.parse::<std::net::Ipv4Addr>()
                && !addr.is_loopback()
                && !addr.is_link_local()
                && !addr.is_unspecified()
            {
                has_routable_ip = true;
            }
        } else if let Some(rest) = line.strip_prefix("inet6 ") {
            let ip = rest.split(['%', ' ']).next().unwrap_or("");
            if let Ok(addr) = ip.parse::<std::net::Ipv6Addr>()
                && !addr.is_loopback()
                && !addr.is_unspecified()
                && !addr.is_multicast()
                && (addr.segments()[0] & 0xffc0) != 0xfe80
            {
                has_routable_ip = true;
            }
        }
    }

    flush(
        current_iface,
        is_up_running,
        has_routable_ip,
        status_inactive,
        &mut active,
    );
    active
}

#[cfg(any(target_os = "macos", test))]
fn select_macos_service(
    services: &[(String, Option<String>)],
    route_device: Option<&str>,
    active_devices: &[String],
) -> Option<String> {
    if let Some(dev) = route_device
        && !is_virtual_interface(dev)
        && let Some((service, _)) = services.iter().find(|(service, s_dev)| {
            s_dev.as_deref() == Some(dev) && !is_secondary_service(service)
        })
    {
        return Some(service.clone());
    }

    if let Some((service, _)) = services.iter().find(|(service, s_dev)| {
        !is_secondary_service(service)
            && s_dev.as_deref().is_some_and(|dev| {
                !is_virtual_interface(dev) && active_devices.iter().any(|active| active == dev)
            })
    }) {
        return Some(service.clone());
    }

    if let Some((service, _)) = services.iter().find(|(_service, s_dev)| {
        s_dev
            .as_deref()
            .is_some_and(|dev| active_devices.iter().any(|active| active == dev))
    }) {
        return Some(service.clone());
    }

    if let Some((service, _)) = services.iter().find(|(service, s_dev)| {
        !is_secondary_service(service)
            && s_dev
                .as_deref()
                .is_some_and(|dev| !is_virtual_interface(dev))
    }) {
        return Some(service.clone());
    }

    if let Some((service, _)) = services.iter().find(|(service, s_dev)| {
        !is_secondary_service(service) && !s_dev.as_deref().is_some_and(is_virtual_interface)
    }) {
        return Some(service.clone());
    }

    services.first().map(|(service, _)| service.clone())
}

#[cfg(any(target_os = "macos", test))]
fn parse_macos_bypass_domains(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("There aren't"))
        .map(str::to_string)
        .collect()
}

#[cfg(any(target_os = "macos", test))]
fn merge_macos_bypass_domains(raw_existing: &str) -> Vec<String> {
    merge_linux_ignore_hosts(&parse_macos_bypass_domains(raw_existing))
}

#[cfg(any(target_os = "macos", test))]
fn parse_macos_proxy_output(output: &str) -> Result<(bool, String, u16), String> {
    let value = |key: &str| {
        output
            .lines()
            .find_map(|line| line.trim().strip_prefix(key))
            .map(str::trim)
    };
    let enabled = value("Enabled:").is_some_and(|v| v.eq_ignore_ascii_case("yes"));
    let host = value("Server:").unwrap_or_default().to_string();
    let port = value("Port:")
        .filter(|s| !s.is_empty())
        .unwrap_or("0")
        .parse::<u16>()
        .map_err(|e| format!("Failed to parse macOS proxy port: {e}"))?;
    Ok((enabled, host, port))
}

#[cfg(target_os = "windows")]
mod platform {
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD;
    use std::collections::BTreeMap;
    use std::ffi::c_void;
    use std::ptr;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, RegType};
    use winreg::{RegKey, RegValue};

    const INTERNET_SETTINGS: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Internet Settings";
    const CONNECTIONS_SUBKEY: &str = "Connections";
    const DEFAULT_CONNECTION_SETTINGS: &str = "DefaultConnectionSettings";
    const INTERNET_OPTION_REFRESH: u32 = 37;
    const INTERNET_OPTION_SETTINGS_CHANGED: u32 = 39;
    const BACKUP_MARKER: &str = "SingBoxGuiProxyBackupVersion";
    const BACKUP_ENABLE: &str = "SingBoxGuiProxyBackupEnable";
    const BACKUP_SERVER: &str = "SingBoxGuiProxyBackupServer";
    const BACKUP_OVERRIDE: &str = "SingBoxGuiProxyBackupOverride";
    const BACKUP_AUTOCONFIGURL: &str = "SingBoxGuiProxyBackupAutoConfigUrl";
    const BACKUP_APPLIED_PORT: &str = "SingBoxGuiProxyBackupAppliedPort";
    const BACKUP_CONNECTIONS: &str = "SingBoxGuiProxyBackupConnections";

    #[derive(serde::Serialize, serde::Deserialize, Default)]
    struct ConnectionsBackup {
        entries: BTreeMap<String, String>,
        #[serde(default)]
        created: Vec<String>,
    }

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

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn parse_proxy_server(value: &str) -> Option<(String, u16)> {
        super::parse_windows_proxy_server(value)
    }

    fn clear_proxy_backup(settings: &RegKey) -> Result<(), String> {
        let _ = settings.delete_value(BACKUP_ENABLE);
        let _ = settings.delete_value(BACKUP_SERVER);
        let _ = settings.delete_value(BACKUP_OVERRIDE);
        let _ = settings.delete_value(BACKUP_AUTOCONFIGURL);
        let _ = settings.delete_value(BACKUP_APPLIED_PORT);
        let _ = settings.delete_value(BACKUP_CONNECTIONS);
        match settings.delete_value(BACKUP_MARKER) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(format!(
                "Failed to clear Windows proxy backup marker: {err}"
            )),
        }
    }

    fn snapshot_connections(settings: &RegKey) -> Result<String, String> {
        let mut backup = ConnectionsBackup::default();
        match settings.open_subkey_with_flags(CONNECTIONS_SUBKEY, KEY_READ) {
            Ok(conns) => {
                for item in conns.enum_values() {
                    if let Ok((name, value)) = item
                        && value.vtype == RegType::REG_BINARY
                    {
                        backup.entries.insert(name, STANDARD.encode(&value.bytes));
                    }
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(format!(
                    "Failed to read Windows proxy connections settings: {err}"
                ));
            }
        }
        if !backup.entries.contains_key(DEFAULT_CONNECTION_SETTINGS) {
            backup.created.push(DEFAULT_CONNECTION_SETTINGS.to_string());
        }
        serde_json::to_string(&backup)
            .map_err(|e| format!("Failed to serialize Windows connections backup: {e}"))
    }

    fn apply_connections_proxy(settings: &RegKey, port: u16) -> Result<(), String> {
        let (conns, _) = settings
            .create_subkey_with_flags(CONNECTIONS_SUBKEY, KEY_READ | KEY_SET_VALUE)
            .map_err(|e| format!("Failed to open Windows proxy connections key: {e}"))?;
        let mut updates: Vec<(String, Vec<u8>)> = Vec::new();
        let mut has_default = false;
        for item in conns.enum_values() {
            let (name, value) = match item {
                Ok(pair) => pair,
                Err(_) => continue,
            };
            if value.vtype != RegType::REG_BINARY {
                continue;
            }
            if name == DEFAULT_CONNECTION_SETTINGS {
                has_default = true;
            }
            if let Some(mut parsed) = super::WindowsConnectionSettings::parse(&value.bytes) {
                parsed.apply_proxy(port);
                updates.push((name, parsed.encode()?));
            }
        }
        if !has_default {
            let encoded = super::WindowsConnectionSettings::default_with_proxy(port).encode()?;
            updates.push((DEFAULT_CONNECTION_SETTINGS.to_string(), encoded));
        }
        for (name, bytes) in updates {
            conns
                .set_raw_value(
                    &name,
                    &RegValue {
                        bytes,
                        vtype: RegType::REG_BINARY,
                    },
                )
                .map_err(|e| format!("Failed to update Windows connection '{name}': {e}"))?;
        }
        Ok(())
    }

    fn restore_connections_backup(
        settings: &RegKey,
        expected_port: u16,
        recorded_port: Option<u16>,
        force: bool,
    ) -> Result<(), String> {
        let Ok(raw_json) = settings.get_value::<String, _>(BACKUP_CONNECTIONS) else {
            return Ok(());
        };
        let backup: ConnectionsBackup = serde_json::from_str(&raw_json)
            .map_err(|e| format!("Failed to parse Windows connections backup: {e}"))?;
        let conns =
            match settings.open_subkey_with_flags(CONNECTIONS_SUBKEY, KEY_READ | KEY_SET_VALUE) {
                Ok(key) => key,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(err) => {
                    return Err(format!(
                        "Failed to open Windows proxy connections key for restore: {err}"
                    ));
                }
            };

        for (name, encoded_bytes) in backup.entries {
            let bytes = STANDARD
                .decode(encoded_bytes.as_bytes())
                .map_err(|e| format!("Failed to decode Windows connection backup '{name}': {e}"))?;
            if !force
                && let Ok(current) = conns.get_raw_value(&name)
                && current.vtype == RegType::REG_BINARY
                && let Some(parsed) = super::WindowsConnectionSettings::parse(&current.bytes)
                && !parsed.is_owned_endpoint(expected_port, recorded_port)
            {
                continue;
            }
            conns
                .set_raw_value(
                    &name,
                    &RegValue {
                        bytes,
                        vtype: RegType::REG_BINARY,
                    },
                )
                .map_err(|e| format!("Failed to restore Windows connection '{name}': {e}"))?;
        }

        for name in backup.created {
            if !force
                && let Ok(current) = conns.get_raw_value(&name)
                && current.vtype == RegType::REG_BINARY
                && let Some(parsed) = super::WindowsConnectionSettings::parse(&current.bytes)
                && !parsed.is_owned_endpoint(expected_port, recorded_port)
            {
                continue;
            }
            if let Err(err) = conns.delete_value(&name)
                && err.kind() != std::io::ErrorKind::NotFound
            {
                return Err(format!(
                    "Failed to clean up Windows connection '{name}': {err}"
                ));
            }
        }
        Ok(())
    }

    fn clear_unowned_connections(settings: &RegKey, port: u16) -> Result<(), String> {
        if port == 0 {
            return Ok(());
        }
        let conns =
            match settings.open_subkey_with_flags(CONNECTIONS_SUBKEY, KEY_READ | KEY_SET_VALUE) {
                Ok(key) => key,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(err) => {
                    return Err(format!(
                        "Failed to open Windows proxy connections key: {err}"
                    ));
                }
            };
        let mut updates = Vec::new();
        for item in conns.enum_values() {
            let (name, value) = match item {
                Ok(pair) => pair,
                Err(_) => continue,
            };
            if value.vtype != RegType::REG_BINARY {
                continue;
            }
            if let Some(mut parsed) = super::WindowsConnectionSettings::parse(&value.bytes)
                && parsed.is_owned_endpoint(port, None)
            {
                parsed.clear_proxy();
                updates.push((name, parsed.encode()?));
            }
        }
        for (name, bytes) in updates {
            conns
                .set_raw_value(
                    &name,
                    &RegValue {
                        bytes,
                        vtype: RegType::REG_BINARY,
                    },
                )
                .map_err(|e| format!("Failed to disable Windows connection '{name}': {e}"))?;
        }
        Ok(())
    }

    pub fn has_persisted_backup() -> bool {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        hkcu.open_subkey_with_flags(INTERNET_SETTINGS, KEY_READ)
            .ok()
            .and_then(|settings| settings.get_value::<u32, _>(BACKUP_MARKER).ok())
            .is_some()
    }

    fn is_existing_backup_still_owned(settings: &RegKey, expected_port: u16) -> bool {
        let current_enable: u32 = settings.get_value("ProxyEnable").unwrap_or_default();
        if current_enable == 0 {
            return false;
        }
        let current_server: String = settings.get_value("ProxyServer").unwrap_or_default();
        let recorded_port: Option<u16> = settings
            .get_value::<u32, _>(BACKUP_APPLIED_PORT)
            .ok()
            .and_then(|p| u16::try_from(p).ok())
            .filter(|&p| p > 0);
        parse_proxy_server(&current_server).is_some_and(|(host, port)| {
            super::is_owned_proxy_endpoint(&host, port, expected_port, recorded_port)
        })
    }

    fn restore_proxy_backup(
        settings: &RegKey,
        expected_port: u16,
        force: bool,
    ) -> Result<bool, String> {
        if settings.get_value::<u32, _>(BACKUP_MARKER).is_err() {
            return Ok(false);
        }
        let recorded_port: Option<u16> = settings
            .get_value::<u32, _>(BACKUP_APPLIED_PORT)
            .ok()
            .and_then(|p| u16::try_from(p).ok())
            .filter(|&p| p > 0);
        let still_owned = is_existing_backup_still_owned(settings, expected_port);
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
        let restored_override = super::restore_windows_proxy_override(&original, &current);
        settings
            .set_value("ProxyOverride", &restored_override)
            .map_err(|e| format!("Failed to restore Windows bypass list: {e}"))?;

        if let Ok(original_pac) = settings.get_value::<String, _>(BACKUP_AUTOCONFIGURL)
            && !original_pac.is_empty()
        {
            settings
                .set_value("AutoConfigURL", &original_pac)
                .map_err(|e| format!("Failed to restore Windows proxy state: {e}"))?;
        }

        if force || still_owned {
            settings
                .set_value("ProxyServer", &original_server)
                .map_err(|e| format!("Failed to restore Windows proxy server: {e}"))?;
            settings
                .set_value("ProxyEnable", &original_enable)
                .map_err(|e| format!("Failed to restore Windows proxy state: {e}"))?;
            restore_connections_backup(settings, expected_port, recorded_port, force)?;
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
            // enable. If a leftover backup exists from an earlier crash but the
            // user has since disabled or changed the proxy, discard the stale
            // backup and capture the current settings instead.
            let mut marker: Option<u32> = settings.get_value(BACKUP_MARKER).ok();
            if marker.is_some() && !is_existing_backup_still_owned(&settings, port) {
                clear_proxy_backup(&settings)?;
                marker = None;
            }
            let current_override: String = settings.get_value("ProxyOverride").unwrap_or_default();
            let pac_to_clear = settings
                .get_value::<String, _>("AutoConfigURL")
                .ok()
                .filter(|pac| !pac.trim().is_empty());
            if marker.is_none() {
                let original_enable: u32 = settings.get_value("ProxyEnable").unwrap_or_default();
                let original_server: String = settings.get_value("ProxyServer").unwrap_or_default();
                let connections_json = snapshot_connections(&settings)?;
                let snapshot_result = (|| {
                    if let Some(ref pac) = pac_to_clear {
                        settings
                            .set_value(BACKUP_AUTOCONFIGURL, pac)
                            .map_err(|e| format!("Failed to back up Windows proxy state: {e}"))?;
                    }
                    settings
                        .set_value(BACKUP_ENABLE, &original_enable)
                        .map_err(|e| format!("Failed to back up Windows proxy state: {e}"))?;
                    settings
                        .set_value(BACKUP_SERVER, &original_server)
                        .map_err(|e| format!("Failed to back up Windows proxy server: {e}"))?;
                    settings
                        .set_value(BACKUP_OVERRIDE, &current_override)
                        .map_err(|e| format!("Failed to back up Windows bypass list: {e}"))?;
                    settings
                        .set_value(BACKUP_CONNECTIONS, &connections_json)
                        .map_err(|e| format!("Failed to back up Windows connections: {e}"))?;
                    settings
                        .set_value(BACKUP_MARKER, &1u32)
                        .map_err(|e| format!("Failed to mark Windows proxy backup: {e}"))?;
                    Ok::<(), String>(())
                })();
                if let Err(err) = snapshot_result {
                    let _ = clear_proxy_backup(&settings);
                    return Err(err);
                }
            } else if let Some(ref pac) = pac_to_clear
                && settings
                    .get_value::<String, _>(BACKUP_AUTOCONFIGURL)
                    .is_err()
            {
                settings
                    .set_value(BACKUP_AUTOCONFIGURL, pac)
                    .map_err(|e| format!("Failed to back up Windows proxy state: {e}"))?;
            }
            let merged_override = super::merge_windows_proxy_override(&current_override);
            let apply_result = (|| {
                if pac_to_clear.is_some()
                    && let Err(err) = settings.delete_value("AutoConfigURL")
                    && err.kind() != std::io::ErrorKind::NotFound
                {
                    return Err(format!("Failed to update Windows proxy state: {err}"));
                }
                if merged_override != current_override {
                    settings
                        .set_value("ProxyOverride", &merged_override)
                        .map_err(|e| format!("Failed to update Windows proxy bypass list: {e}"))?;
                }
                settings
                    .set_value("ProxyServer", &format!("127.0.0.1:{port}"))
                    .map_err(|e| format!("Failed to update Windows proxy server: {e}"))?;
                settings
                    .set_value(BACKUP_APPLIED_PORT, &u32::from(port))
                    .map_err(|e| format!("Failed to record Windows applied proxy port: {e}"))?;
                settings
                    .set_value("ProxyEnable", &1u32)
                    .map_err(|e| format!("Failed to update Windows proxy state: {e}"))?;
                apply_connections_proxy(&settings, port)?;
                Ok::<(), String>(())
            })();
            if let Err(error) = apply_result {
                let rollback_res = restore_proxy_backup(&settings, port, true);
                notify_proxy_changed();
                return match rollback_res {
                    Ok(_) => Err(error),
                    Err(rollback) => Err(format!("{error}; rollback failed: {rollback}")),
                };
            }
        } else {
            // Restore the complete user state if we previously backed it up.
            if !restore_proxy_backup(&settings, port, false)? {
                let current_enable: u32 = settings.get_value("ProxyEnable").unwrap_or_default();
                let current_server: String = settings.get_value("ProxyServer").unwrap_or_default();
                if current_enable != 0
                    && parse_proxy_server(&current_server).is_some_and(|(host, current_port)| {
                        super::is_loopback_proxy_host(&host) && port > 0 && current_port == port
                    })
                {
                    settings
                        .set_value("ProxyEnable", &0u32)
                        .map_err(|e| format!("Failed to disable Windows proxy: {e}"))?;
                    clear_unowned_connections(&settings, port)?;
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
    use std::io::ErrorKind;
    use std::process::{Command, Output};

    const PROXY_SCHEMA: &str = "org.gnome.system.proxy";
    const KIOSLAVERC: &str = "kioslaverc";
    const KDE_PROXY_GROUP: &str = "Proxy Settings";

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

    fn set_gnome_value(schema: &str, key: &str, value: &str) -> Result<(), String> {
        run_gsettings(&["set", schema, key, value]).map(|_| ())
    }

    fn get_gnome_value(schema: &str, key: &str) -> Result<String, String> {
        let output = run_gsettings(&["get", schema, key])?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn gnome_is_usable() -> bool {
        get_gnome_value(PROXY_SCHEMA, "mode").is_ok()
    }

    fn kde_is_session() -> bool {
        for var in [
            "XDG_CURRENT_DESKTOP",
            "XDG_SESSION_DESKTOP",
            "DESKTOP_SESSION",
        ] {
            if let Ok(val) = std::env::var(var) {
                let lower = val.to_ascii_lowercase();
                if lower.contains("kde") || lower.contains("plasma") {
                    return true;
                }
            }
        }
        std::env::var("KDE_FULL_SESSION")
            .ok()
            .is_some_and(|v| !v.trim().is_empty() && v != "0" && !v.eq_ignore_ascii_case("false"))
    }

    fn run_kde_cmd(candidates: &[&str], args: &[&str]) -> Result<Output, String> {
        let mut last_err = None;
        for bin in candidates {
            match Command::new(bin).args(args).output() {
                Ok(output) if output.status.success() => return Ok(output),
                Ok(output) => {
                    last_err = Some(format!(
                        "{bin} failed: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    ));
                }
                Err(e) if e.kind() == ErrorKind::NotFound => continue,
                Err(e) => {
                    last_err = Some(format!("Failed to run {bin}: {e}"));
                }
            }
        }
        Err(last_err.unwrap_or_else(|| format!("{} not found", candidates.join("/"))))
    }

    fn kde_read_key(key: &str) -> Result<String, String> {
        let output = run_kde_cmd(
            &["kreadconfig6", "kreadconfig5"],
            &[
                "--file",
                KIOSLAVERC,
                "--group",
                KDE_PROXY_GROUP,
                "--key",
                key,
            ],
        )?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn kde_write_key(key: &str, value: Option<&str>) -> Result<(), String> {
        match value {
            Some(v) if !v.is_empty() => run_kde_cmd(
                &["kwriteconfig6", "kwriteconfig5"],
                &[
                    "--file",
                    KIOSLAVERC,
                    "--group",
                    KDE_PROXY_GROUP,
                    "--key",
                    key,
                    v,
                ],
            )
            .map(|_| ()),
            _ => run_kde_cmd(
                &["kwriteconfig6", "kwriteconfig5"],
                &[
                    "--file",
                    KIOSLAVERC,
                    "--group",
                    KDE_PROXY_GROUP,
                    "--key",
                    key,
                    "--delete",
                ],
            )
            .map(|_| ()),
        }
    }

    fn kde_notify_reparse() {
        for signal in [
            ("org.kde.KIO.Scheduler.reparseSlaveConfiguration", true),
            ("org.kde.KIO.Scheduler.reparseConfiguration", false),
        ] {
            let mut cmd = Command::new("dbus-send");
            cmd.args(["--session", "--type=signal", "/KIO/Scheduler", signal.0]);
            if signal.1 {
                cmd.arg("string:");
            }
            let _ = cmd.output();
        }
    }

    fn kde_is_usable() -> bool {
        kde_read_key("ProxyType").is_ok()
    }

    fn get_gnome_system_proxy() -> Result<(bool, String, u16), String> {
        let enabled = get_gnome_value(PROXY_SCHEMA, "mode")? == "'manual'";
        if !enabled {
            return Ok((false, String::new(), 0));
        }
        let host = get_gnome_value(&format!("{PROXY_SCHEMA}.http"), "host")?
            .trim_matches(['\'', '"'])
            .to_string();
        let port = get_gnome_value(&format!("{PROXY_SCHEMA}.http"), "port")?
            .parse::<u16>()
            .map_err(|e| format!("Failed to parse GNOME proxy port: {e}"))?;
        Ok((true, host, port))
    }

    fn get_kde_system_proxy() -> Result<(bool, String, u16), String> {
        let proxy_type = kde_read_key("ProxyType")?;
        if proxy_type != "1" {
            return Ok((false, String::new(), 0));
        }
        let raw_http = kde_read_key("httpProxy")?;
        let (host, port) = super::parse_kde_proxy_endpoint(&raw_http).unwrap_or_default();
        Ok((true, host, port))
    }

    fn snapshot_gnome() -> Result<serde_json::Value, String> {
        let mut values = serde_json::Map::new();
        values.insert(
            "mode".to_string(),
            serde_json::Value::String(get_gnome_value(PROXY_SCHEMA, "mode")?),
        );
        values.insert(
            "ignore_hosts".to_string(),
            serde_json::Value::String(get_gnome_value(PROXY_SCHEMA, "ignore-hosts")?),
        );
        for service in ["http", "https", "socks"] {
            let schema = format!("{PROXY_SCHEMA}.{service}");
            values.insert(
                format!("{service}_host"),
                serde_json::Value::String(get_gnome_value(&schema, "host")?),
            );
            values.insert(
                format!("{service}_port"),
                serde_json::Value::String(get_gnome_value(&schema, "port")?),
            );
        }
        Ok(serde_json::Value::Object(values))
    }

    fn snapshot_kde() -> Result<serde_json::Value, String> {
        let mut values = serde_json::Map::new();
        values.insert(
            "proxy_type".to_string(),
            serde_json::Value::String(kde_read_key("ProxyType")?),
        );
        for key in [
            "httpProxy",
            "httpsProxy",
            "socksProxy",
            "NoProxyFor",
            "ReversedException",
        ] {
            values.insert(
                key.to_string(),
                serde_json::Value::String(kde_read_key(key)?),
            );
        }
        Ok(serde_json::Value::Object(values))
    }

    fn apply_gnome(port: u16) -> Result<(), String> {
        let existing_raw = get_gnome_value(PROXY_SCHEMA, "ignore-hosts")?;
        for service in ["http", "https", "socks"] {
            let schema = format!("{PROXY_SCHEMA}.{service}");
            set_gnome_value(&schema, "host", "'127.0.0.1'")?;
            set_gnome_value(&schema, "port", &port.to_string())?;
        }
        let merged =
            super::merge_linux_ignore_hosts(&super::parse_gsettings_string_array(&existing_raw));
        set_gnome_value(
            PROXY_SCHEMA,
            "ignore-hosts",
            &super::format_gsettings_string_array(&merged),
        )?;
        set_gnome_value(PROXY_SCHEMA, "mode", "'manual'")
    }

    fn apply_kde(port: u16) -> Result<(), String> {
        let http_url = format!("http://127.0.0.1:{port}");
        let socks_url = format!("socks://127.0.0.1:{port}");
        let existing_no_proxy = kde_read_key("NoProxyFor")?;
        let merged_no_proxy = super::merge_kde_no_proxy(&existing_no_proxy);

        kde_write_key("httpProxy", Some(&http_url))?;
        kde_write_key("httpsProxy", Some(&http_url))?;
        kde_write_key("socksProxy", Some(&socks_url))?;
        kde_write_key("NoProxyFor", Some(&merged_no_proxy))?;
        kde_write_key("ReversedException", None)?;
        kde_write_key("ProxyType", Some("1"))?;
        kde_notify_reparse();
        Ok(())
    }

    fn restore_gnome_object(
        values: &serde_json::Map<String, serde_json::Value>,
        expected_port: u16,
        recorded_port: Option<u16>,
        force: bool,
    ) -> Result<(), String> {
        if !force {
            let (enabled, host, configured_port) = get_gnome_system_proxy()?;
            let still_owned = enabled
                && super::is_owned_proxy_endpoint(
                    &host,
                    configured_port,
                    expected_port,
                    recorded_port,
                );
            if !still_owned {
                return Ok(());
            }
        }
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
            set_gnome_value(&schema, "host", host)?;
            set_gnome_value(&schema, "port", service_port)?;
        }
        if let Some(orig_raw) = values.get("ignore_hosts").and_then(|v| v.as_str()) {
            let current_raw = get_gnome_value(PROXY_SCHEMA, "ignore-hosts").unwrap_or_default();
            let orig_items = super::parse_gsettings_string_array(orig_raw);
            let curr_items = super::parse_gsettings_string_array(&current_raw);
            let restored = super::restore_linux_ignore_hosts(&orig_items, &curr_items);
            set_gnome_value(
                PROXY_SCHEMA,
                "ignore-hosts",
                &super::format_gsettings_string_array(&restored),
            )?;
        }
        let mode = values
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("'none'");
        set_gnome_value(PROXY_SCHEMA, "mode", mode)
    }

    fn restore_kde_object(
        values: &serde_json::Map<String, serde_json::Value>,
        expected_port: u16,
        recorded_port: Option<u16>,
        force: bool,
    ) -> Result<(), String> {
        if !force {
            let (enabled, host, configured_port) = get_kde_system_proxy()?;
            let still_owned = enabled
                && super::is_owned_proxy_endpoint(
                    &host,
                    configured_port,
                    expected_port,
                    recorded_port,
                );
            if !still_owned {
                return Ok(());
            }
        }
        for key in ["httpProxy", "httpsProxy", "socksProxy"] {
            let orig = values.get(key).and_then(|v| v.as_str()).unwrap_or("");
            kde_write_key(key, (!orig.is_empty()).then_some(orig))?;
        }
        if let Some(orig_reversed) = values.get("ReversedException").and_then(|v| v.as_str()) {
            kde_write_key(
                "ReversedException",
                (!orig_reversed.is_empty()).then_some(orig_reversed),
            )?;
        }
        if let Some(orig_no_proxy) = values.get("NoProxyFor").and_then(|v| v.as_str()) {
            let current_no_proxy = kde_read_key("NoProxyFor").unwrap_or_default();
            let restored = super::restore_kde_no_proxy(orig_no_proxy, &current_no_proxy);
            kde_write_key(
                "NoProxyFor",
                (!restored.is_empty()).then_some(restored.as_str()),
            )?;
        }
        let proxy_type = values
            .get("proxy_type")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        kde_write_key(
            "ProxyType",
            Some(if proxy_type.is_empty() {
                "0"
            } else {
                proxy_type
            }),
        )?;
        kde_notify_reparse();
        Ok(())
    }

    fn is_existing_linux_backup_still_owned(
        obj: &serde_json::Map<String, serde_json::Value>,
        expected_port: u16,
    ) -> bool {
        let recorded_port = obj
            .get("applied_port")
            .and_then(|v| v.as_u64())
            .and_then(|v| u16::try_from(v).ok())
            .filter(|&p| p > 0);
        let gnome_owned = (obj.contains_key("gnome") || obj.contains_key("mode"))
            && get_gnome_system_proxy().is_ok_and(|(enabled, host, configured_port)| {
                enabled
                    && super::is_owned_proxy_endpoint(
                        &host,
                        configured_port,
                        expected_port,
                        recorded_port,
                    )
            });
        let kde_owned = obj.contains_key("kde")
            && get_kde_system_proxy().is_ok_and(|(enabled, host, configured_port)| {
                enabled
                    && super::is_owned_proxy_endpoint(
                        &host,
                        configured_port,
                        expected_port,
                        recorded_port,
                    )
            });
        gnome_owned || kde_owned
    }

    fn restore_backup(expected_port: u16, force: bool) -> Result<bool, String> {
        let Some(payload) = super::load_platform_backup("linux") else {
            return Ok(false);
        };
        let obj = payload
            .as_object()
            .ok_or_else(|| "Invalid Linux proxy backup".to_string())?;
        let recorded_port = obj
            .get("applied_port")
            .and_then(|v| v.as_u64())
            .and_then(|v| u16::try_from(v).ok())
            .filter(|&p| p > 0);

        // Support legacy flat GNOME backup objects alongside multi-desktop backups.
        if obj.contains_key("mode") {
            restore_gnome_object(obj, expected_port, recorded_port, force)?;
            super::clear_platform_backup();
            return Ok(true);
        }

        let mut errors = Vec::new();
        if let Some(gnome_obj) = obj.get("gnome").and_then(|v| v.as_object())
            && let Err(err) = restore_gnome_object(gnome_obj, expected_port, recorded_port, force)
        {
            errors.push(err);
        }
        if let Some(kde_obj) = obj.get("kde").and_then(|v| v.as_object())
            && let Err(err) = restore_kde_object(kde_obj, expected_port, recorded_port, force)
        {
            errors.push(err);
        }
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }

        super::clear_platform_backup();
        Ok(true)
    }

    pub fn has_persisted_backup() -> bool {
        super::load_platform_backup("linux").is_some()
    }

    pub fn set_system_proxy(enable: bool, port: u16) -> Result<(), String> {
        if enable {
            let use_gnome = gnome_is_usable();
            let use_kde = (kde_is_session() || !use_gnome) && kde_is_usable();
            if !use_gnome && !use_kde {
                return Err(
                    "No supported Linux desktop proxy backend found (checked gsettings and kwriteconfig6/kwriteconfig5)"
                        .to_string(),
                );
            }

            let reusable_backup = super::load_platform_backup("linux").filter(|existing| {
                existing
                    .as_object()
                    .is_some_and(|obj| is_existing_linux_backup_still_owned(obj, port))
            });
            if let Some(mut existing) = reusable_backup {
                if let Some(obj) = existing.as_object_mut() {
                    let mut dirty = false;
                    if obj.get("applied_port").and_then(|v| v.as_u64()) != Some(u64::from(port)) {
                        obj.insert("applied_port".to_string(), serde_json::json!(port));
                        dirty = true;
                    }
                    if use_gnome && !obj.contains_key("gnome") && !obj.contains_key("mode") {
                        obj.insert("gnome".to_string(), snapshot_gnome()?);
                        dirty = true;
                    }
                    if use_kde && !obj.contains_key("kde") {
                        obj.insert("kde".to_string(), snapshot_kde()?);
                        dirty = true;
                    }
                    if dirty {
                        super::save_platform_backup("linux", existing)?;
                    }
                }
            } else {
                if super::load_platform_backup("linux").is_some() {
                    super::clear_platform_backup();
                }
                let mut payload = serde_json::Map::new();
                payload.insert("applied_port".to_string(), serde_json::json!(port));
                if use_gnome {
                    payload.insert("gnome".to_string(), snapshot_gnome()?);
                }
                if use_kde {
                    payload.insert("kde".to_string(), snapshot_kde()?);
                }
                super::save_platform_backup("linux", serde_json::Value::Object(payload))?;
            }

            let apply_result = (|| {
                if use_gnome {
                    apply_gnome(port)?;
                }
                if use_kde {
                    apply_kde(port)?;
                }
                Ok::<(), String>(())
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
            // Clean up only if configured to our exact localhost endpoint.
            let mut errors = Vec::new();
            if let Ok((enabled, host, configured_port)) = get_gnome_system_proxy()
                && enabled
                && super::is_loopback_proxy_host(&host)
                && port > 0
                && configured_port == port
                && let Err(err) = set_gnome_value(PROXY_SCHEMA, "mode", "'none'")
            {
                errors.push(err);
            }
            if (kde_is_session() || !gnome_is_usable())
                && let Ok((enabled, host, configured_port)) = get_kde_system_proxy()
                && enabled
                && super::is_loopback_proxy_host(&host)
                && port > 0
                && configured_port == port
            {
                if let Err(err) = kde_write_key("ProxyType", Some("0")) {
                    errors.push(err);
                } else {
                    kde_notify_reparse();
                }
            }
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors.join("; "))
            }
        }
    }

    pub fn get_system_proxy() -> Result<(bool, String, u16), String> {
        if kde_is_session()
            && let Ok(kde_state) = get_kde_system_proxy()
        {
            return Ok(kde_state);
        }
        match get_gnome_system_proxy() {
            Ok(state) => Ok(state),
            Err(gnome_err) => get_kde_system_proxy().map_err(|_| gnome_err),
        }
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
    pub(super) fn service_for_interface(order: &str, device: &str) -> Option<String> {
        super::service_for_interface_in_order(order, device)
    }

    fn default_route_interface(args: &[&str]) -> Option<String> {
        run("route", args).ok().and_then(|route| {
            String::from_utf8_lossy(&route.stdout)
                .lines()
                .find_map(|line| line.trim().strip_prefix("interface:"))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
    }

    fn default_network_service() -> Result<String, String> {
        let route_device = default_route_interface(&["-n", "get", "default"])
            .or_else(|| default_route_interface(&["-n", "get", "-inet6", "default"]));

        let order = run("networksetup", &["-listnetworkserviceorder"])?;
        let services = super::parse_network_service_order(&String::from_utf8_lossy(&order.stdout));

        if let Some(ref dev) = route_device
            && !super::is_virtual_interface(dev)
            && let Some((service, _)) = services.iter().find(|(service, s_dev)| {
                s_dev.as_deref() == Some(dev.as_str()) && !super::is_secondary_service(service)
            })
        {
            return Ok(service.clone());
        }

        // When TUN (utun*) owns the default route, scan active physical interfaces once
        // instead of blindly falling back to Thunderbolt Bridge / iPhone USB.
        let active_devices = run("ifconfig", &[])
            .ok()
            .map(|out| {
                super::parse_active_physical_devices_from_ifconfig(&String::from_utf8_lossy(
                    &out.stdout,
                ))
            })
            .unwrap_or_default();

        super::select_macos_service(&services, route_device.as_deref(), &active_devices)
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
        super::parse_macos_proxy_output(output)
    }

    fn is_existing_macos_backup_still_owned(
        payload: &serde_json::Value,
        expected_port: u16,
    ) -> Option<String> {
        let service = payload
            .get("service")
            .and_then(|v| v.as_str())
            .filter(|v| !v.trim().is_empty())?;
        let recorded_port = payload
            .get("applied_port")
            .and_then(|v| v.as_u64())
            .and_then(|v| u16::try_from(v).ok())
            .filter(|&p| p > 0);
        let output = run("networksetup", &["-getwebproxy", service]).ok()?;
        let (enabled, host, configured_port) =
            parse_proxy(&String::from_utf8_lossy(&output.stdout)).ok()?;
        (enabled
            && super::is_owned_proxy_endpoint(&host, configured_port, expected_port, recorded_port))
        .then(|| service.to_string())
    }

    fn restore_backup(expected_port: u16, force: bool) -> Result<bool, String> {
        let Some(payload) = super::load_platform_backup("macos") else {
            return Ok(false);
        };
        let service = payload
            .get("service")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Invalid macOS proxy backup service".to_string())?;
        let recorded_port = payload
            .get("applied_port")
            .and_then(|v| v.as_u64())
            .and_then(|v| u16::try_from(v).ok())
            .filter(|&p| p > 0);
        if !force {
            let output = run("networksetup", &["-getwebproxy", service])?;
            let (enabled, host, configured_port) =
                parse_proxy(&String::from_utf8_lossy(&output.stdout))?;
            let still_owned = enabled
                && super::is_owned_proxy_endpoint(
                    &host,
                    configured_port,
                    expected_port,
                    recorded_port,
                );
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
        let orig_domains = super::parse_macos_bypass_domains(bypass);
        let domains = match run("networksetup", &["-getproxybypassdomains", service]) {
            Ok(out) => {
                let curr_domains =
                    super::parse_macos_bypass_domains(&String::from_utf8_lossy(&out.stdout));
                super::restore_linux_ignore_hosts(&orig_domains, &curr_domains)
            }
            Err(_) => orig_domains,
        };
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
            // change, keep using the service captured in the durable snapshot,
            // provided that service still holds our owned proxy endpoint.
            let reusable = super::load_platform_backup("macos").and_then(|existing| {
                is_existing_macos_backup_still_owned(&existing, port)
                    .map(|service| (service, existing))
            });
            let (service, existing_backup) = match reusable {
                Some((service, existing)) => (service, Some(existing)),
                None => {
                    if super::load_platform_backup("macos").is_some() {
                        super::clear_platform_backup();
                    }
                    (default_network_service()?, None)
                }
            };
            let mut snapshot_parsed: Option<Vec<(&'static str, (bool, String, u16))>> = None;
            let current_bypass_raw = if let Some(mut existing) = existing_backup {
                if let Some(obj) = existing.as_object_mut()
                    && obj.get("applied_port").and_then(|v| v.as_u64()) != Some(u64::from(port))
                {
                    obj.insert("applied_port".to_string(), serde_json::json!(port));
                    super::save_platform_backup("macos", existing)?;
                }
                let output = run("networksetup", &["-getproxybypassdomains", &service])?;
                String::from_utf8_lossy(&output.stdout).into_owned()
            } else {
                let mut kinds = serde_json::Map::new();
                let mut parsed = Vec::with_capacity(3);
                for kind in ["webproxy", "securewebproxy", "socksfirewallproxy"] {
                    let output = run("networksetup", &[&format!("-get{kind}"), &service])?;
                    let raw = String::from_utf8_lossy(&output.stdout).into_owned();
                    if let Ok(state) = parse_proxy(&raw) {
                        parsed.push((kind, state));
                    }
                    kinds.insert(kind.to_string(), serde_json::Value::String(raw));
                }
                snapshot_parsed = Some(parsed);
                let bypass_output = run("networksetup", &["-getproxybypassdomains", &service])?;
                let bypass = String::from_utf8_lossy(&bypass_output.stdout).into_owned();
                super::save_platform_backup(
                    "macos",
                    serde_json::json!({
                        "service": service,
                        "applied_port": port,
                        "kinds": kinds,
                        "bypass": bypass,
                    }),
                )?;
                bypass
            };
            let existing_domains = super::parse_macos_bypass_domains(&current_bypass_raw);
            let merged_domains = super::merge_macos_bypass_domains(&current_bypass_raw);
            let apply_result = (|| {
                for kind in ["webproxy", "securewebproxy", "socksfirewallproxy"] {
                    let current = snapshot_parsed
                        .as_ref()
                        .and_then(|list| list.iter().find(|(k, _)| *k == kind))
                        .map(|(_, s)| s);
                    match current {
                        Some((true, host, p)) if host == "127.0.0.1" && *p == port => {}
                        Some((false, host, p)) if host == "127.0.0.1" && *p == port => {
                            set_proxy_state(&service, kind, true)?;
                        }
                        _ => {
                            set_proxy(&service, kind, true, port)?;
                        }
                    }
                }
                if merged_domains != existing_domains {
                    let mut args = vec!["-setproxybypassdomains".to_string(), service.clone()];
                    args.extend(merged_domains.iter().cloned());
                    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
                    run("networksetup", &refs)?;
                }
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
            if enabled
                && super::is_loopback_proxy_host(&host)
                && port > 0
                && configured_port == port
            {
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
    use super::*;

    #[test]
    fn ownership_port_check_rejects_foreign_local_proxy() {
        // Expected port must match unless expected_port == 0 or recorded_port matches.
        assert!(is_owned_proxy_endpoint("127.0.0.1", 2080, 2080, None));
        assert!(!is_owned_proxy_endpoint("127.0.0.1", 7890, 2080, None));
        assert!(is_owned_proxy_endpoint("localhost", 7890, 0, None));
        assert!(is_owned_proxy_endpoint("::1", 2080, 2081, Some(2080)));
        assert!(!is_owned_proxy_endpoint("10.0.0.1", 2080, 2080, None));
    }

    #[test]
    fn windows_override_merge_and_restore_preserve_user_entries() {
        let merged = merge_windows_proxy_override("*.corp.local;10.0.0.0/8;LOCALHOST");
        assert_eq!(
            merged,
            "*.corp.local;10.0.0.0/8;LOCALHOST;127.0.0.1;::1;<local>"
        );

        let restored = restore_windows_proxy_override(
            "*.corp.local",
            "*.corp.local;10.0.0.0/8;localhost;127.0.0.1;::1;<local>",
        );
        assert_eq!(restored, "*.corp.local;10.0.0.0/8");
    }

    #[test]
    fn linux_gnome_and_kde_bypass_merge_and_endpoint_parse() {
        let existing = parse_gsettings_string_array("['*.corp.local', '10.0.0.0/8']");
        assert_eq!(existing, vec!["*.corp.local", "10.0.0.0/8"]);
        let merged = merge_linux_ignore_hosts(&existing);
        assert_eq!(
            format_gsettings_string_array(&merged),
            "['*.corp.local', '10.0.0.0/8', 'localhost', '127.0.0.1', '::1']"
        );
        assert_eq!(parse_gsettings_string_array("@as []"), Vec::<String>::new());
        assert_eq!(format_gsettings_string_array(&[]), "@as []");

        assert_eq!(
            merge_kde_no_proxy("*.internal, 192.168.0.0/16, localhost"),
            "*.internal,192.168.0.0/16,localhost,127.0.0.1,::1"
        );
        assert_eq!(
            restore_kde_no_proxy("*.internal", "*.internal,localhost,127.0.0.1,*.added"),
            "*.internal,*.added"
        );
        assert_eq!(
            parse_kde_proxy_endpoint("http://127.0.0.1:2080"),
            Some(("127.0.0.1".to_string(), 2080))
        );
        assert_eq!(
            parse_kde_proxy_endpoint("socks://[::1]:1080/"),
            Some(("::1".to_string(), 1080))
        );
        assert_eq!(
            parse_kde_proxy_endpoint("127.0.0.1 2080"),
            Some(("127.0.0.1".to_string(), 2080))
        );
    }

    #[test]
    fn macos_service_selection_skips_virtual_and_inactive_interfaces() {
        let order = "\
An asterisk (*) denotes that a network service is disabled.
(1) Thunderbolt Bridge
(Hardware Port: Thunderbolt Bridge, Device: bridge0)
(2) *Disabled USB
(Hardware Port: USB LAN, Device: en5)
(3) iPhone USB
(Hardware Port: iPhone USB, Device: en9)
(4) Wi-Fi
(Hardware Port: Wi-Fi, Device: en0)
";
        let services = parse_network_service_order(order);
        assert_eq!(
            service_for_interface_in_order(order, "en0"),
            Some("Wi-Fi".to_string())
        );
        assert_eq!(service_for_interface_in_order(order, "en5"), None);

        let ifconfig = "\
lo0: flags=8049<UP,LOOPBACK,RUNNING,MULTICAST> mtu 16384
\tinet 127.0.0.1 netmask 0xff000000
bridge0: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500
\tstatus: inactive
en9: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500
\tstatus: inactive
en0: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500
\tinet 192.168.1.42 netmask 0xffffff00 broadcast 192.168.1.255
\tstatus: active
utun4: flags=8051<UP,POINTOPOINT,RUNNING,MULTICAST> mtu 1500
\tinet 172.19.0.1 --> 172.19.0.2 netmask 0xfffffffc
";
        let active = parse_active_physical_devices_from_ifconfig(ifconfig);
        assert_eq!(active, vec!["en0".to_string()]);
        assert_eq!(
            select_macos_service(&services, Some("utun4"), &active),
            Some("Wi-Fi".to_string())
        );

        let bypass = merge_macos_bypass_domains("*.local\n169.254/16\nlocalhost\n");
        assert_eq!(
            bypass,
            vec!["*.local", "169.254/16", "localhost", "127.0.0.1", "::1"]
        );
        assert_eq!(
            merge_macos_bypass_domains("There aren't any bypass domains set on Wi-Fi.\n"),
            vec!["localhost", "127.0.0.1", "::1"]
        );
    }

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
        assert_eq!(
            parse_proxy_server("HTTP=http://127.0.0.1:2080/"),
            Some(("127.0.0.1".to_string(), 2080))
        );
        assert_eq!(parse_proxy_server(":2080"), None);
        assert_eq!(parse_proxy_server("127.0.0.1:0"), None);
    }

    #[test]
    fn loopback_host_and_service_order_edge_cases() {
        assert!(is_loopback_proxy_host("127.0.0.1"));
        assert!(is_loopback_proxy_host("LOCALHOST"));
        assert!(is_loopback_proxy_host("[::1]"));
        assert!(!is_loopback_proxy_host("10.0.0.1"));

        let order = "\
An asterisk (*) denotes that a network service is disabled.
(*) Disabled Wi-Fi
(Hardware Port: Wi-Fi, Device: en0)
(2) Ethernet
(Hardware Port: Ethernet, Device: en1)
";
        assert_eq!(service_for_interface_in_order(order, "en0"), None);
        assert_eq!(
            service_for_interface_in_order(order, "en1"),
            Some("Ethernet".to_string())
        );

        let ifconfig = "\
en0: flags=8843<UP,BROADCAST,SIMPLEX,MULTICAST> mtu 1500
\tinet 192.168.1.42 netmask 0xffffff00
\tstatus: active
en1: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500
\tinet 192.168.1.43 netmask 0xffffff00
\tstatus: active
";
        assert_eq!(
            parse_active_physical_devices_from_ifconfig(ifconfig),
            vec!["en1".to_string()]
        );

        assert_eq!(parse_kde_proxy_endpoint(":2080"), None);
        assert_eq!(parse_kde_proxy_endpoint("127.0.0.1:0"), None);
        assert_eq!(parse_kde_proxy_endpoint("::1"), None);
        assert_eq!(parse_kde_proxy_endpoint("fe80::1"), None);
        assert_eq!(
            parse_kde_proxy_endpoint("[::1] 2080"),
            Some(("::1".to_string(), 2080))
        );
        assert_eq!(
            parse_kde_proxy_endpoint("::1 2080"),
            Some(("::1".to_string(), 2080))
        );
    }

    #[test]
    fn reject_zero_port_and_zero_configured_ownership() {
        assert!(!is_owned_proxy_endpoint("127.0.0.1", 0, 0, None));
        assert!(!is_owned_proxy_endpoint("127.0.0.1", 0, 2080, Some(0)));
        assert!(set_system_proxy(true, 0).is_err());
    }

    #[test]
    fn platform_backup_atomic_persistence_and_corrupt_primary_recovery() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("system-proxy-backup.json");
        let bak_path = dir.path().join("system-proxy-backup.json.bak");

        save_platform_backup_to(&path, "linux", serde_json::json!({"applied_port": 2080}))
            .expect("first save");
        save_platform_backup_to(&path, "linux", serde_json::json!({"applied_port": 2081}))
            .expect("second save creates .bak");
        assert!(bak_path.exists());
        assert_eq!(
            load_platform_backup_from(&path, "linux")
                .and_then(|v| v.get("applied_port").and_then(|p| p.as_u64())),
            Some(2081)
        );
        assert!(load_platform_backup_from(&path, "macos").is_none());

        // Corrupt primary falls back to `.bak` while primary still exists.
        std::fs::write(&path, b"{broken").expect("corrupt primary");
        assert_eq!(
            load_platform_backup_from(&path, "linux")
                .and_then(|v| v.get("applied_port").and_then(|p| p.as_u64())),
            Some(2080)
        );

        // Once cleared, even a lingering `.bak` must never resurrect the backup.
        clear_platform_backup_at(&path);
        assert!(!path.exists());
        assert!(!bak_path.exists());
        std::fs::write(
            &bak_path,
            br#"{"platform":"linux","payload":{"applied_port":9999}}"#,
        )
        .expect("write stray bak");
        assert!(load_platform_backup_from(&path, "linux").is_none());
    }

    #[test]
    fn windows_proxy_server_priority_and_ipv6_rejections() {
        assert_eq!(
            parse_windows_proxy_server("https=10.0.0.1:8443;http=127.0.0.1:2080"),
            Some(("127.0.0.1".to_string(), 2080))
        );
        assert_eq!(
            parse_windows_proxy_server("http=invalid;https=127.0.0.1:2081"),
            Some(("127.0.0.1".to_string(), 2081))
        );
        assert_eq!(
            parse_windows_proxy_server("socks5=127.0.0.1:1080"),
            Some(("127.0.0.1".to_string(), 1080))
        );
        assert_eq!(
            parse_windows_proxy_server("[::1]:2080"),
            Some(("::1".to_string(), 2080))
        );
        assert_eq!(parse_windows_proxy_server("::1"), None);
        assert_eq!(parse_windows_proxy_server("fe80::1"), None);
        assert_eq!(parse_windows_proxy_server("ftp=127.0.0.1:21"), None);
    }

    #[test]
    fn windows_connections_binary_parse_bounds_and_mutation() {
        let mut blob = Vec::new();
        blob.extend_from_slice(&CONN_VERSION_IE7.to_le_bytes());
        blob.extend_from_slice(&u32::MAX.to_le_bytes());
        blob.extend_from_slice(
            &(PROXY_TYPE_DIRECT | PROXY_TYPE_AUTO_DETECT | PROXY_TYPE_AUTO_PROXY_URL).to_le_bytes(),
        );
        let pac = b"http://example.com/proxy.pac";
        let ovr = b"*.corp.local";
        blob.extend_from_slice(&0u32.to_le_bytes()); // proxy_len = 0
        blob.extend_from_slice(&(ovr.len() as u32).to_le_bytes());
        blob.extend_from_slice(ovr);
        blob.extend_from_slice(&(pac.len() as u32).to_le_bytes());
        blob.extend_from_slice(pac);
        blob.extend_from_slice(&[0xAA; 32]);

        let mut parsed = WindowsConnectionSettings::parse(&blob).expect("valid 0x46 blob");
        assert_eq!(parsed.version, CONN_VERSION_IE7);
        assert_eq!(parsed.counter, u32::MAX);
        assert!(!parsed.is_owned_endpoint(2080, None));

        parsed.apply_proxy(2080);
        assert_eq!(parsed.counter, 0);
        assert_eq!(parsed.flags, PROXY_TYPE_DIRECT | PROXY_TYPE_PROXY);
        assert_eq!(parsed.proxy_server, b"127.0.0.1:2080");
        assert!(parsed.autoconfig_url.is_empty());
        assert_eq!(parsed.footer, vec![0xAA; 32]);
        assert!(parsed.is_owned_endpoint(2080, None));
        assert!(parsed.is_owned_endpoint(2081, Some(2080)));
        assert!(!parsed.is_owned_endpoint(7890, Some(7890)));

        let mut foreign = parsed.clone();
        foreign.proxy_server = b"127.0.0.1:7890".to_vec();
        assert!(!foreign.is_owned_endpoint(2080, Some(2080)));

        let encoded = parsed.encode().expect("encode");
        let reparsed = WindowsConnectionSettings::parse(&encoded).expect("reparse");
        assert_eq!(reparsed, parsed);

        parsed.clear_proxy();
        assert_eq!(parsed.flags, PROXY_TYPE_DIRECT);
        assert!(!parsed.is_owned_endpoint(2080, None));

        let fresh = WindowsConnectionSettings::default_with_proxy(3080);
        assert!(fresh.is_owned_endpoint(3080, None));
        assert_eq!(fresh.footer.len(), 32);

        // IE6 (0x3C) with 28-byte footer round-trips cleanly.
        let mut ie6 = vec![0u8; 24 + 28];
        ie6[0..4].copy_from_slice(&CONN_VERSION_IE6.to_le_bytes());
        ie6[4..8].copy_from_slice(&7u32.to_le_bytes());
        ie6[8..12].copy_from_slice(&PROXY_TYPE_DIRECT.to_le_bytes());
        let parsed_ie6 = WindowsConnectionSettings::parse(&ie6).expect("valid 0x3C");
        assert_eq!(parsed_ie6.footer.len(), 28);

        // Bounds and version safety: reject short buffers, unknown versions, and length overflows.
        assert!(WindowsConnectionSettings::parse(&[0u8; 23]).is_none());
        let mut bad_ver = ie6.clone();
        bad_ver[0..4].copy_from_slice(&0x99u32.to_le_bytes());
        assert!(WindowsConnectionSettings::parse(&bad_ver).is_none());
        let mut bad_len = vec![0u8; 24];
        bad_len[0..4].copy_from_slice(&CONN_VERSION_IE7.to_le_bytes());
        bad_len[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(WindowsConnectionSettings::parse(&bad_len).is_none());
        bad_len[12..16].copy_from_slice(&10u32.to_le_bytes());
        assert!(WindowsConnectionSettings::parse(&bad_len).is_none());
    }

    #[test]
    fn macos_proxy_output_and_ipv6_interface_filtering() {
        assert_eq!(
            parse_macos_proxy_output("Enabled: Yes\nServer: 127.0.0.1\nPort: 2080\n"),
            Ok((true, "127.0.0.1".to_string(), 2080))
        );
        assert_eq!(
            parse_macos_proxy_output("Enabled: No\nServer:\nPort:\n"),
            Ok((false, String::new(), 0))
        );
        assert!(parse_macos_proxy_output("Enabled: Yes\nServer: 127.0.0.1\nPort: bad\n").is_err());

        let ifconfig = "\
vmenet0: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500
\tinet 192.168.64.1 netmask 0xffffff00
en0: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500
\tinet6 fe80::1%en0 prefixlen 64 scopeid 0x4
\tinet6 fea0::2 prefixlen 64
\tstatus: active
en1: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500
\tinet6 2001:db8::42 prefixlen 64 autoconf secured
\tstatus: active
";
        assert_eq!(
            parse_active_physical_devices_from_ifconfig(ifconfig),
            vec!["en1".to_string()]
        );

        let services = vec![
            ("Corporate VPN".to_string(), None),
            ("Wi-Fi".to_string(), Some("en0".to_string())),
        ];
        assert_eq!(
            select_macos_service(&services, None, &[]),
            Some("Wi-Fi".to_string())
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
