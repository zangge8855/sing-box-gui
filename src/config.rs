use crate::state::{GuiConfig, ProxyNode};
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use std::sync::OnceLock;
static APP_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn get_app_dir() -> PathBuf {
    APP_DIR
        .get_or_init(|| {
            if let Ok(override_dir) = std::env::var("SING_BOX_GUI_DATA_DIR") {
                let dir = PathBuf::from(override_dir);
                let _ = fs::create_dir_all(&dir);
                let _ = fs::create_dir_all(dir.join("profiles"));
                let _ = fs::create_dir_all(dir.join("bin"));
                return dir;
            }
            let base = dirs::data_dir()
                .or_else(dirs::config_dir)
                .or_else(|| std::env::var("APPDATA").ok().map(PathBuf::from))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let dir = base.join("sing-box-gui");
            let _ = fs::create_dir_all(&dir);
            let _ = fs::create_dir_all(dir.join("profiles"));
            let _ = fs::create_dir_all(dir.join("bin"));
            dir
        })
        .clone()
}

/// Parse Clash subscription-userinfo header values.
/// Example: `upload=123; download=456; total=789; expire=1710000000`
pub fn parse_subscription_userinfo(
    header: &str,
) -> (Option<u64>, Option<u64>, Option<u64>, Option<i64>) {
    let mut upload = None;
    let mut download = None;
    let mut total = None;
    let mut expire = None;
    for part in header.split(';') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            let k = k.trim().to_lowercase();
            let v = v.trim();
            match k.as_str() {
                "upload" => upload = v.parse().ok(),
                "download" => download = v.parse().ok(),
                "total" => total = v.parse().ok(),
                "expire" => expire = v.parse().ok(),
                _ => {}
            }
        }
    }
    (upload, download, total, expire)
}

pub fn get_config_path() -> PathBuf {
    get_app_dir().join("gui_config.json")
}

fn load_gui_config_from_paths(
    path: &std::path::Path,
    backup_path: &std::path::Path,
    corrupt_path: &std::path::Path,
) -> GuiConfig {
    let mut migrated_proxy_ownership = false;
    let mut restored_from_backup = false;

    let try_read_backup = || -> Option<GuiConfig> {
        let bytes = fs::read(backup_path).ok()?;
        let text = std::str::from_utf8(&bytes).ok()?;
        serde_json::from_str::<GuiConfig>(text.trim_start_matches('\u{feff}')).ok()
    };

    #[allow(unused_mut)]
    let mut config = if path.exists() {
        match fs::read(path) {
            Ok(bytes) => match std::str::from_utf8(&bytes) {
                Ok(raw_content) => {
                    let content = raw_content.trim_start_matches('\u{feff}');
                    match serde_json::from_str::<GuiConfig>(content) {
                        Ok(mut cfg) => {
                            // Versions before `system_proxy_owned` only persisted the
                            // enabled bit. Treat an enabled legacy endpoint as
                            // application-owned for one migration pass; the async
                            // startup probe below still verifies the actual endpoint
                            // before any cleanup is attempted.
                            if cfg.system_proxy_enabled
                                && serde_json::from_str::<serde_json::Value>(content)
                                    .ok()
                                    .and_then(|value| value.get("system_proxy_owned").cloned())
                                    .is_none()
                            {
                                cfg.system_proxy_owned = true;
                                migrated_proxy_ownership = true;
                            }
                            cfg
                        }
                        Err(e) => {
                            eprintln!(
                                "Failed to parse gui_config.json: {}. Backing up and restoring last good copy.",
                                e
                            );
                            // Keep the corrupt bytes under a name the routine save
                            // backup (`gui_config.json.bak`, rewritten by every
                            // atomic_write) can never overwrite.
                            let _ = fs::write(corrupt_path, &bytes);
                            if let Some(backup_cfg) = try_read_backup() {
                                restored_from_backup = true;
                                // Remove the corrupt primary before atomic_write so
                                // `.bak` is not overwritten with corrupt bytes.
                                let _ = fs::remove_file(path);
                                backup_cfg
                            } else {
                                GuiConfig::default()
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Failed to decode gui_config.json as UTF-8: {}. Backing up and restoring last good copy.",
                        e
                    );
                    let _ = fs::write(corrupt_path, &bytes);
                    if let Some(backup_cfg) = try_read_backup() {
                        restored_from_backup = true;
                        let _ = fs::remove_file(path);
                        backup_cfg
                    } else {
                        GuiConfig::default()
                    }
                }
            },
            Err(e) => {
                eprintln!(
                    "Failed to read gui_config.json: {}. Restoring last good copy.",
                    e
                );
                if let Some(backup_cfg) = try_read_backup() {
                    restored_from_backup = true;
                    backup_cfg
                } else {
                    GuiConfig::default()
                }
            }
        }
    } else if let Some(backup_cfg) = try_read_backup() {
        restored_from_backup = true;
        backup_cfg
    } else {
        GuiConfig::default()
    };

    if config.mixed_port == 0 {
        config.mixed_port = 2080;
    }
    if config.api_port == 0 || config.api_port == config.mixed_port {
        config.api_port = if config.mixed_port == 9090 {
            9091
        } else {
            9090
        };
    }

    if (migrated_proxy_ownership || restored_from_backup)
        && let Ok(serialized) = serde_json::to_string_pretty(&config)
    {
        let _ = atomic_write(path, serialized.as_bytes());
    }

    config
}

pub fn load_gui_config() -> GuiConfig {
    let app_dir = get_app_dir();
    load_gui_config_from_paths(
        &get_config_path(),
        &app_dir.join("gui_config.json.bak"),
        &app_dir.join("gui_config.json.corrupt.bak"),
    )
}

pub fn save_gui_config(config: &GuiConfig) -> Result<(), String> {
    let path = get_config_path();
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    atomic_write(&path, content.as_bytes())?;
    Ok(())
}

fn cleanup_stale_cache_files() {
    let dir = get_app_dir();
    let Ok(entries) = fs::read_dir(&dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let is_profile_cache = name.starts_with("cache_") && name.ends_with("_cache.db");
        let is_legacy_cache = name == "cache.db";
        if !is_profile_cache && !is_legacy_cache {
            continue;
        }
        let max_age_secs = if is_legacy_cache { 86_400 } else { 14 * 86_400 };
        let expired = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > std::time::Duration::from_secs(max_age_secs));
        if expired {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Remove `.{name}.tmp-{pid}-…` litter left behind when a crash or power loss
/// hits between `atomic_write`'s temp-file creation and its commit rename.
/// Only entries from other process ids that have not been touched recently are
/// removed, so a concurrent instance's in-flight write is never deleted.
pub fn cleanup_stale_temp_files() {
    let own_pid = std::process::id().to_string();
    for dir in [get_app_dir(), get_app_dir().join("profiles")] {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if !name.starts_with('.') {
                continue;
            }
            let Some((_, marker)) = name.split_once(".tmp-") else {
                continue;
            };
            if marker.split('-').next() == Some(own_pid.as_str()) {
                continue;
            }
            let recently_touched = entry
                .metadata()
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|modified| modified.elapsed().ok())
                .is_some_and(|age| age < std::time::Duration::from_secs(300));
            if !recently_touched {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    cleanup_stale_cache_files();
}

/// Replace a persisted file without exposing a partially-written destination.
/// This is used for settings and subscription files because a power loss during
/// a direct `fs::write` would otherwise reset the application on next launch.
pub fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    let parent = path
        .parent()
        .ok_or_else(|| "Target file has no parent directory".to_string())?;
    fs::create_dir_all(parent).map_err(|e| format!("Failed to create data directory: {e}"))?;

    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("data");
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp_path = parent.join(format!(
        ".{file_name}.tmp-{}-{sequence}-{nonce}",
        std::process::id(),
    ));
    let backup_path = parent.join(format!("{file_name}.bak"));

    let write_result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|e| format!("Failed to create temporary file: {e}"))?;
        file.write_all(bytes)
            .map_err(|e| format!("Failed to write temporary file: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("Failed to flush temporary file: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o600))
                .map_err(|e| format!("Failed to restrict temporary file permissions: {e}"))?;
        }
        Ok::<(), String>(())
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    if path.exists() {
        if let Err(error) = fs::copy(path, &backup_path) {
            let _ = fs::remove_file(&temp_path);
            return Err(format!("Failed to create recovery backup: {error}"));
        }
        if let Ok(backup) = fs::File::open(&backup_path) {
            let _ = backup.sync_all();
        }
    }

    #[cfg(target_os = "windows")]
    let commit_result = windows_replace_file(&temp_path, path);
    #[cfg(not(target_os = "windows"))]
    let commit_result = fs::rename(&temp_path, path)
        .map_err(|error| format!("Failed to commit temporary file: {error}"));

    if let Err(e) = commit_result {
        let _ = fs::remove_file(&temp_path);
        return Err(e);
    }
    #[cfg(unix)]
    if let Ok(dir) = fs::File::open(parent) {
        let _ = dir.sync_all();
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_replace_file(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x0000_0001;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;

    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(
            existing_file_name: *const u16,
            new_file_name: *const u16,
            flags: u32,
        ) -> i32;
    }

    let source_wide: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination_wide: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let replaced = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        Err(format!(
            "Failed to commit temporary file: {}",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

fn decode_base64_padded(input: &str) -> Option<String> {
    let cleaned: String = input
        .trim_start_matches('\u{feff}')
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let input = cleaned.as_str();
    if input.is_empty() {
        return None;
    }
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    // Attempt standard decode
    if let Ok(bytes) = STANDARD.decode(input)
        && let Ok(s) = String::from_utf8(bytes)
    {
        return Some(s);
    }

    // Attempt with padding
    let mut padded = input.to_string();
    while !padded.len().is_multiple_of(4) {
        padded.push('=');
    }
    if let Ok(bytes) = STANDARD.decode(&padded)
        && let Ok(s) = String::from_utf8(bytes)
    {
        return Some(s);
    }

    // Attempt URL-safe decode
    use base64::engine::general_purpose::URL_SAFE;
    if let Ok(bytes) = URL_SAFE.decode(input)
        && let Ok(s) = String::from_utf8(bytes)
    {
        return Some(s);
    }

    let mut padded_url = input.to_string();
    while !padded_url.len().is_multiple_of(4) {
        padded_url.push('=');
    }
    if let Ok(bytes) = URL_SAFE.decode(&padded_url)
        && let Ok(s) = String::from_utf8(bytes)
    {
        return Some(s);
    }

    None
}

fn parse_query_bool(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn insert_yaml_string(map: &mut serde_yaml::Mapping, key: &str, value: &str) {
    if !value.is_empty() {
        map.insert(
            serde_yaml::Value::String(key.to_string()),
            serde_yaml::Value::String(value.to_string()),
        );
    }
}

fn insert_yaml_number_or_string(map: &mut serde_yaml::Mapping, key: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    let yaml_value = value
        .parse::<u64>()
        .map(|number| serde_yaml::Value::Number(number.into()))
        .unwrap_or_else(|_| serde_yaml::Value::String(value.to_string()));
    map.insert(serde_yaml::Value::String(key.to_string()), yaml_value);
}

fn extract_url_credential(url: &url::Url) -> String {
    let user = urlencoding::decode(url.username())
        .unwrap_or(std::borrow::Cow::Borrowed(url.username()))
        .into_owned();
    match url.password() {
        Some(pass) => {
            let decoded_pass = urlencoding::decode(pass)
                .unwrap_or(std::borrow::Cow::Borrowed(pass))
                .into_owned();
            format!("{user}:{decoded_pass}")
        }
        None => user,
    }
}

fn normalize_wg_key(raw: &str) -> String {
    raw.trim().replace(' ', "+")
}

fn normalize_wg_cidr(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.contains('/') {
        return trimmed.to_string();
    }
    if let Ok(ip) = trimmed.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(_) => format!("{trimmed}/32"),
            std::net::IpAddr::V6(_) => format!("{trimmed}/128"),
        }
    } else {
        trimmed.to_string()
    }
}

fn parse_share_link(link: &str) -> Option<serde_yaml::Mapping> {
    let link = link.trim_start_matches('\u{feff}').trim();
    if link.is_empty() {
        return None;
    }

    use url::Url;
    let url = Url::parse(link).ok()?;
    let scheme = url.scheme();

    let mut map = serde_yaml::Mapping::new();

    let host = url.host_str().unwrap_or("127.0.0.1").to_string();
    let port = url.port().unwrap_or(443);
    let tag = url
        .fragment()
        .map(|f| {
            urlencoding::decode(f)
                .unwrap_or(std::borrow::Cow::Borrowed(f))
                .into_owned()
        })
        .unwrap_or_else(|| format!("{}-{}", scheme, host));

    map.insert(
        serde_yaml::Value::String("name".to_string()),
        serde_yaml::Value::String(tag),
    );
    map.insert(
        serde_yaml::Value::String("server".to_string()),
        serde_yaml::Value::String(host),
    );
    map.insert(
        serde_yaml::Value::String("port".to_string()),
        serde_yaml::Value::Number(port.into()),
    );

    match scheme {
        "ss" => {
            map.insert(
                serde_yaml::Value::String("type".to_string()),
                serde_yaml::Value::String("ss".to_string()),
            );

            let raw_body = link
                .trim_start_matches("ss://")
                .split('#')
                .next()
                .unwrap_or_default()
                .split('?')
                .next()
                .unwrap_or_default();
            if !raw_body.contains('@') {
                if let Some(decoded) = decode_base64_padded(raw_body) {
                    let decoded_link = format!("ss://{}", decoded);
                    if let Ok(temp_url) = Url::parse(&decoded_link) {
                        let cipher = urlencoding::decode(temp_url.username())
                            .unwrap_or(std::borrow::Cow::Borrowed(temp_url.username()))
                            .into_owned();
                        let password = temp_url
                            .password()
                            .and_then(|value| urlencoding::decode(value).ok())
                            .map(|value| value.into_owned())
                            .unwrap_or_default();
                        if !cipher.is_empty() && !password.is_empty() {
                            map.insert(
                                serde_yaml::Value::String("cipher".to_string()),
                                serde_yaml::Value::String(cipher),
                            );
                            map.insert(
                                serde_yaml::Value::String("password".to_string()),
                                serde_yaml::Value::String(password),
                            );
                        }
                        let host = temp_url.host_str().unwrap_or("127.0.0.1").to_string();
                        let port = temp_url.port().unwrap_or(443);
                        map.insert(
                            serde_yaml::Value::String("server".to_string()),
                            serde_yaml::Value::String(host),
                        );
                        map.insert(
                            serde_yaml::Value::String("port".to_string()),
                            serde_yaml::Value::Number(port.into()),
                        );
                    }
                }
            } else if let Some(raw_password) = url.password() {
                let cipher = urlencoding::decode(url.username())
                    .unwrap_or(std::borrow::Cow::Borrowed(url.username()))
                    .into_owned();
                let password = urlencoding::decode(raw_password)
                    .unwrap_or(std::borrow::Cow::Borrowed(raw_password))
                    .into_owned();
                if !cipher.is_empty() && !password.is_empty() {
                    map.insert(
                        serde_yaml::Value::String("cipher".to_string()),
                        serde_yaml::Value::String(cipher),
                    );
                    map.insert(
                        serde_yaml::Value::String("password".to_string()),
                        serde_yaml::Value::String(password),
                    );
                }
            } else {
                let username = urlencoding::decode(url.username())
                    .unwrap_or(std::borrow::Cow::Borrowed(url.username()))
                    .into_owned();
                if let Some(decoded) = decode_base64_padded(&username) {
                    let parts: Vec<&str> = decoded.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let cipher = urlencoding::decode(parts[0])
                            .unwrap_or(std::borrow::Cow::Borrowed(parts[0]))
                            .into_owned();
                        let password = urlencoding::decode(parts[1])
                            .unwrap_or(std::borrow::Cow::Borrowed(parts[1]))
                            .into_owned();
                        map.insert(
                            serde_yaml::Value::String("cipher".to_string()),
                            serde_yaml::Value::String(cipher),
                        );
                        map.insert(
                            serde_yaml::Value::String("password".to_string()),
                            serde_yaml::Value::String(password),
                        );
                    }
                }
            }
            if let Some(plugin) = url
                .query_pairs()
                .find_map(|(k, v)| (k == "plugin").then(|| v.into_owned()))
            {
                let mut parts = plugin.split(';');
                if let Some(name) = parts.next().filter(|s| !s.is_empty()) {
                    map.insert(
                        serde_yaml::Value::String("plugin".to_string()),
                        serde_yaml::Value::String(name.to_string()),
                    );
                    let options = parts.collect::<Vec<_>>().join(";");
                    if !options.is_empty() {
                        map.insert(
                            serde_yaml::Value::String("plugin-opts-raw".to_string()),
                            serde_yaml::Value::String(options),
                        );
                    }
                }
            }
        }
        "ssr" => {
            let body = link
                .trim_start_matches("ssr://")
                .split('#')
                .next()
                .unwrap_or_default();
            let decoded = decode_base64_padded(body)?;
            let (server_part, query_part) = if let Some((s, q)) = decoded.split_once("/?") {
                (s, q)
            } else if let Some((s, q)) = decoded.split_once('?') {
                (s.trim_end_matches('/'), q)
            } else {
                (decoded.trim_end_matches('/'), "")
            };
            let (server, rest) = if server_part.starts_with('[') {
                let (ipv6, rest) = server_part.split_once("]:")?;
                (ipv6.trim_start_matches('[').to_string(), rest)
            } else {
                let (host, rest) = server_part.split_once(':')?;
                (host.to_string(), rest)
            };
            let parts: Vec<&str> = rest.splitn(5, ':').collect();
            if parts.len() != 5 {
                return None;
            }
            let port = parts[0].parse::<u16>().ok()?;
            let protocol = parts[1];
            let method = parts[2];
            let obfs = parts[3];
            let password = decode_base64_padded(parts[4])?;
            if server.is_empty() || method.is_empty() || password.is_empty() {
                return None;
            }

            insert_yaml_string(&mut map, "type", "ssr");
            insert_yaml_string(&mut map, "server", &server);
            map.insert(
                serde_yaml::Value::String("port".to_string()),
                serde_yaml::Value::Number(port.into()),
            );
            insert_yaml_string(&mut map, "cipher", method);
            insert_yaml_string(&mut map, "password", &password);
            insert_yaml_string(&mut map, "protocol", protocol);
            insert_yaml_string(&mut map, "obfs", obfs);

            for (key, value) in url::form_urlencoded::parse(query_part.as_bytes()) {
                match key.as_ref() {
                    "remarks" => {
                        if let Some(remarks) = decode_base64_padded(value.as_ref()) {
                            insert_yaml_string(&mut map, "name", &remarks);
                        }
                    }
                    "obfsparam" => {
                        if let Some(param) = decode_base64_padded(value.as_ref()) {
                            insert_yaml_string(&mut map, "obfs-param", &param);
                        }
                    }
                    "protoparam" => {
                        if let Some(param) = decode_base64_padded(value.as_ref()) {
                            insert_yaml_string(&mut map, "protocol-param", &param);
                        }
                    }
                    _ => {}
                }
            }
        }
        "vmess" => {
            map.insert(
                serde_yaml::Value::String("type".to_string()),
                serde_yaml::Value::String("vmess".to_string()),
            );

            let body_b64 = link
                .trim_start_matches("vmess://")
                .split('?')
                .next()
                .unwrap_or_default()
                .split('#')
                .next()
                .unwrap_or_default();
            if let Some(decoded_json) = decode_base64_padded(body_b64)
                && let Ok(v) = serde_json::from_str::<serde_json::Value>(&decoded_json)
            {
                let name = v
                    .get("ps")
                    .and_then(|x| x.as_str())
                    .unwrap_or("vmess-node")
                    .to_string();
                let server = v
                    .get("add")
                    .and_then(|x| x.as_str())
                    .unwrap_or("127.0.0.1")
                    .to_string();
                let port = v
                    .get("port")
                    .and_then(|x| {
                        x.as_u64()
                            .or_else(|| x.as_str().and_then(|s| s.parse::<u64>().ok()))
                    })
                    .unwrap_or(443);
                let uuid = v
                    .get("id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let alter_id = v
                    .get("aid")
                    .and_then(|x| {
                        x.as_u64()
                            .or_else(|| x.as_str().and_then(|s| s.parse::<u64>().ok()))
                    })
                    .unwrap_or(0);
                let cipher = v
                    .get("scy")
                    .and_then(|x| x.as_str())
                    .unwrap_or("auto")
                    .to_string();
                let network = v
                    .get("net")
                    .and_then(|x| x.as_str())
                    .unwrap_or("tcp")
                    .to_string();
                let tls_enabled = v.get("tls").is_some_and(|val| {
                    val.as_bool().unwrap_or_else(|| {
                        val.as_str().is_some_and(|s| {
                            s.eq_ignore_ascii_case("tls")
                                || s == "1"
                                || s.eq_ignore_ascii_case("true")
                        }) || (val.as_u64() == Some(1))
                    })
                });
                let host_sni = v
                    .get("sni")
                    .and_then(|x| x.as_str())
                    .filter(|s| !s.is_empty())
                    .or_else(|| {
                        v.get("host")
                            .and_then(|x| x.as_str())
                            .filter(|s| !s.is_empty())
                    })
                    .unwrap_or("")
                    .to_string();
                let skip_cert_verify = [
                    "allowInsecure",
                    "skipCertVerify",
                    "skip-cert-verify",
                    "insecure",
                ]
                .iter()
                .filter_map(|k| v.get(*k))
                .any(|val| {
                    val.as_bool().unwrap_or_else(|| {
                        val.as_str().is_some_and(parse_query_bool) || (val.as_u64() == Some(1))
                    })
                });
                let service_name = v
                    .get("serviceName")
                    .or_else(|| v.get("service_name"))
                    .or_else(|| v.get("grpc-service-name"))
                    .and_then(|x| x.as_str())
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .or_else(|| {
                        (network == "grpc")
                            .then(|| {
                                v.get("path")
                                    .and_then(|x| x.as_str())
                                    .filter(|s| !s.is_empty())
                                    .map(str::to_string)
                            })
                            .flatten()
                    });

                map.insert(
                    serde_yaml::Value::String("name".to_string()),
                    serde_yaml::Value::String(name),
                );
                map.insert(
                    serde_yaml::Value::String("server".to_string()),
                    serde_yaml::Value::String(server),
                );
                map.insert(
                    serde_yaml::Value::String("port".to_string()),
                    serde_yaml::Value::Number(port.into()),
                );
                map.insert(
                    serde_yaml::Value::String("uuid".to_string()),
                    serde_yaml::Value::String(uuid),
                );
                map.insert(
                    serde_yaml::Value::String("alterId".to_string()),
                    serde_yaml::Value::Number(alter_id.into()),
                );
                map.insert(
                    serde_yaml::Value::String("cipher".to_string()),
                    serde_yaml::Value::String(cipher),
                );
                map.insert(
                    serde_yaml::Value::String("network".to_string()),
                    serde_yaml::Value::String(network.clone()),
                );
                if let Some(alpn) = v.get("alpn") {
                    if let Some(alpn_str) = alpn.as_str().filter(|value| !value.is_empty()) {
                        insert_yaml_string(&mut map, "alpn", alpn_str);
                    } else if let Some(alpn_arr) = alpn.as_array() {
                        let items: Vec<serde_yaml::Value> = alpn_arr
                            .iter()
                            .filter_map(|item| {
                                item.as_str()
                                    .filter(|s| !s.is_empty())
                                    .map(|s| serde_yaml::Value::String(s.to_string()))
                            })
                            .collect();
                        if !items.is_empty() {
                            map.insert(
                                serde_yaml::Value::String("alpn".to_string()),
                                serde_yaml::Value::Sequence(items),
                            );
                        }
                    }
                }
                if let Some(fingerprint) = v
                    .get("fp")
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                {
                    insert_yaml_string(&mut map, "client-fingerprint", fingerprint);
                }
                if skip_cert_verify {
                    map.insert(
                        serde_yaml::Value::String("skip-cert-verify".to_string()),
                        serde_yaml::Value::Bool(true),
                    );
                }

                if tls_enabled {
                    map.insert(
                        serde_yaml::Value::String("tls".to_string()),
                        serde_yaml::Value::Bool(true),
                    );
                    if !host_sni.is_empty() {
                        map.insert(
                            serde_yaml::Value::String("sni".to_string()),
                            serde_yaml::Value::String(host_sni),
                        );
                    }
                }
                if let Some(path) = v
                    .get("path")
                    .and_then(|x| x.as_str())
                    .filter(|s| !s.is_empty())
                {
                    map.insert(
                        serde_yaml::Value::String("path".to_string()),
                        serde_yaml::Value::String(path.to_string()),
                    );
                }
                if let Some(host) = v
                    .get("host")
                    .and_then(|x| x.as_str())
                    .filter(|s| !s.is_empty())
                {
                    map.insert(
                        serde_yaml::Value::String("host".to_string()),
                        serde_yaml::Value::String(host.to_string()),
                    );
                }
                if let Some(service) = service_name {
                    map.insert(
                        serde_yaml::Value::String("serviceName".to_string()),
                        serde_yaml::Value::String(service),
                    );
                }
            } else {
                return None;
            }
            if map
                .get(serde_yaml::Value::String("uuid".to_string()))
                .and_then(|v| v.as_str())
                .is_none_or(str::is_empty)
            {
                return None;
            }
        }
        "vless" => {
            map.insert(
                serde_yaml::Value::String("type".to_string()),
                serde_yaml::Value::String("vless".to_string()),
            );

            let decoded_uuid = extract_url_credential(&url);
            if decoded_uuid.is_empty() {
                return None;
            }
            map.insert(
                serde_yaml::Value::String("uuid".to_string()),
                serde_yaml::Value::String(decoded_uuid),
            );

            let mut tls_enabled = false;
            let mut reality_enabled = false;
            let mut sni = String::new();
            let mut flow = String::new();
            let mut network = String::new();
            let mut path = String::new();
            let mut host = String::new();
            let mut service_name = String::new();
            let mut skip_cert_verify = false;
            let mut public_key = String::new();
            let mut short_id = String::new();
            let mut fingerprint = String::new();

            for (k, v) in url.query_pairs() {
                if k == "security" {
                    if v == "tls" {
                        tls_enabled = true;
                    } else if v == "reality" {
                        tls_enabled = true;
                        reality_enabled = true;
                    }
                }
                if k == "sni" || k == "peer" || k == "servername" {
                    sni = v.to_string();
                }
                if k == "flow" {
                    flow = v.to_string();
                }
                if k == "type" {
                    network = v.to_string();
                }
                if k == "path" {
                    path = v.to_string();
                }
                if k == "host" {
                    host = v.to_string();
                }
                if k == "serviceName" || k == "service_name" || k == "grpc-service-name" {
                    service_name = v.to_string();
                }
                if k == "alpn" {
                    insert_yaml_string(&mut map, "alpn", v.as_ref());
                }
                if k == "fp" || k == "fingerprint" {
                    insert_yaml_string(&mut map, "client-fingerprint", v.as_ref());
                }
                if k == "pbk" {
                    public_key = v.to_string();
                }
                if k == "sid" {
                    short_id = v.to_string();
                }
                if k == "fp" {
                    fingerprint = v.to_string();
                }
                if matches!(
                    k.as_ref(),
                    "skipCertVerify" | "allowInsecure" | "insecure" | "skip-cert-verify"
                ) {
                    skip_cert_verify |= parse_query_bool(v.as_ref());
                }
            }
            if network == "grpc" && service_name.is_empty() && !path.is_empty() {
                service_name = path.clone();
            }
            if !public_key.is_empty() {
                reality_enabled = true;
                tls_enabled = true;
            }
            if tls_enabled {
                map.insert(
                    serde_yaml::Value::String("tls".to_string()),
                    serde_yaml::Value::Bool(true),
                );
                if !sni.is_empty() {
                    map.insert(
                        serde_yaml::Value::String("sni".to_string()),
                        serde_yaml::Value::String(sni),
                    );
                }
            }
            if reality_enabled {
                map.insert(
                    serde_yaml::Value::String("reality".to_string()),
                    serde_yaml::Value::Bool(true),
                );
                if !public_key.is_empty() {
                    map.insert(
                        serde_yaml::Value::String("public-key".to_string()),
                        serde_yaml::Value::String(public_key),
                    );
                }
                if !short_id.is_empty() {
                    map.insert(
                        serde_yaml::Value::String("short-id".to_string()),
                        serde_yaml::Value::String(short_id),
                    );
                }
            }
            if !fingerprint.is_empty() {
                map.insert(
                    serde_yaml::Value::String("fingerprint".to_string()),
                    serde_yaml::Value::String(fingerprint),
                );
            }
            if !flow.is_empty() {
                map.insert(
                    serde_yaml::Value::String("flow".to_string()),
                    serde_yaml::Value::String(flow),
                );
            }
            if !network.is_empty() {
                map.insert(
                    serde_yaml::Value::String("network".to_string()),
                    serde_yaml::Value::String(network),
                );
            }
            if !path.is_empty() {
                map.insert(
                    serde_yaml::Value::String("path".to_string()),
                    serde_yaml::Value::String(path),
                );
            }
            if !host.is_empty() {
                map.insert(
                    serde_yaml::Value::String("host".to_string()),
                    serde_yaml::Value::String(host),
                );
            }
            if !service_name.is_empty() {
                map.insert(
                    serde_yaml::Value::String("serviceName".to_string()),
                    serde_yaml::Value::String(service_name),
                );
            }
            if skip_cert_verify {
                map.insert(
                    serde_yaml::Value::String("skip-cert-verify".to_string()),
                    serde_yaml::Value::Bool(true),
                );
            }
        }
        "trojan" | "trojan-go" => {
            map.insert(
                serde_yaml::Value::String("type".to_string()),
                serde_yaml::Value::String("trojan".to_string()),
            );

            let decoded_password = extract_url_credential(&url);
            if decoded_password.is_empty() {
                return None;
            }
            map.insert(
                serde_yaml::Value::String("password".to_string()),
                serde_yaml::Value::String(decoded_password),
            );

            let mut tls_enabled = true;
            let mut sni = String::new();
            let mut network = String::new();
            let mut path = String::new();
            let mut host = String::new();
            let mut service_name = String::new();
            let mut skip_cert_verify = false;

            for (k, v) in url.query_pairs() {
                if k == "security" && v == "none" {
                    tls_enabled = false;
                }
                if k == "sni" || k == "peer" || k == "servername" {
                    sni = v.to_string();
                }
                if k == "type" {
                    network = v.to_string();
                }
                if k == "path" {
                    path = v.to_string();
                }
                if k == "host" {
                    host = v.to_string();
                }
                if k == "serviceName" || k == "service_name" || k == "grpc-service-name" {
                    service_name = v.to_string();
                }
                if k == "alpn" {
                    insert_yaml_string(&mut map, "alpn", v.as_ref());
                }
                if k == "fp" || k == "fingerprint" {
                    insert_yaml_string(&mut map, "client-fingerprint", v.as_ref());
                }
                if matches!(
                    k.as_ref(),
                    "skipCertVerify" | "allowInsecure" | "insecure" | "skip-cert-verify"
                ) {
                    skip_cert_verify |= parse_query_bool(v.as_ref());
                }
            }
            if network == "grpc" && service_name.is_empty() && !path.is_empty() {
                service_name = path.clone();
            }
            if tls_enabled {
                map.insert(
                    serde_yaml::Value::String("tls".to_string()),
                    serde_yaml::Value::Bool(true),
                );
                if !sni.is_empty() {
                    map.insert(
                        serde_yaml::Value::String("sni".to_string()),
                        serde_yaml::Value::String(sni),
                    );
                }
            }
            if !network.is_empty() {
                map.insert(
                    serde_yaml::Value::String("network".to_string()),
                    serde_yaml::Value::String(network),
                );
            }
            if !path.is_empty() {
                map.insert(
                    serde_yaml::Value::String("path".to_string()),
                    serde_yaml::Value::String(path),
                );
            }
            if !host.is_empty() {
                map.insert(
                    serde_yaml::Value::String("host".to_string()),
                    serde_yaml::Value::String(host),
                );
            }
            if !service_name.is_empty() {
                map.insert(
                    serde_yaml::Value::String("serviceName".to_string()),
                    serde_yaml::Value::String(service_name),
                );
            }
            if skip_cert_verify {
                map.insert(
                    serde_yaml::Value::String("skip-cert-verify".to_string()),
                    serde_yaml::Value::Bool(true),
                );
            }
        }
        "hysteria" => {
            map.insert(
                serde_yaml::Value::String("type".to_string()),
                serde_yaml::Value::String("hysteria".to_string()),
            );
            let decoded_password = extract_url_credential(&url);
            if decoded_password.is_empty() {
                return None;
            }
            map.insert(
                serde_yaml::Value::String("auth-str".to_string()),
                serde_yaml::Value::String(decoded_password),
            );

            for (k, v) in url.query_pairs() {
                match k.as_ref() {
                    "sni" | "peer" | "servername" => {
                        insert_yaml_string(&mut map, "sni", v.as_ref())
                    }
                    "obfs" => insert_yaml_string(&mut map, "obfs", v.as_ref()),
                    "upmbps" | "up_mbps" | "up" => {
                        insert_yaml_number_or_string(&mut map, "up_mbps", v.as_ref())
                    }
                    "downmbps" | "down_mbps" | "down" => {
                        insert_yaml_number_or_string(&mut map, "down_mbps", v.as_ref())
                    }
                    "alpn" => insert_yaml_string(&mut map, "alpn", v.as_ref()),
                    "fp" | "fingerprint" => {
                        insert_yaml_string(&mut map, "client-fingerprint", v.as_ref())
                    }
                    "mport" | "ports" => insert_yaml_string(&mut map, "ports", v.as_ref()),
                    "insecure" | "allowInsecure" | "skipCertVerify" | "skip-cert-verify"
                        if parse_query_bool(v.as_ref()) =>
                    {
                        map.insert(
                            serde_yaml::Value::String("skip-cert-verify".to_string()),
                            serde_yaml::Value::Bool(true),
                        );
                    }
                    _ => {}
                }
            }
        }
        "hysteria2" | "hy2" => {
            map.insert(
                serde_yaml::Value::String("type".to_string()),
                serde_yaml::Value::String("hysteria2".to_string()),
            );
            let decoded_password = extract_url_credential(&url);
            if decoded_password.is_empty() {
                return None;
            }
            map.insert(
                serde_yaml::Value::String("password".to_string()),
                serde_yaml::Value::String(decoded_password),
            );

            for (k, v) in url.query_pairs() {
                match k.as_ref() {
                    "sni" | "peer" | "servername" => {
                        insert_yaml_string(&mut map, "sni", v.as_ref())
                    }
                    "obfs" => insert_yaml_string(&mut map, "obfs", v.as_ref()),
                    "obfs-password" | "obfs_password" => {
                        insert_yaml_string(&mut map, "obfs-password", v.as_ref())
                    }
                    "upmbps" | "up_mbps" | "up" => {
                        insert_yaml_number_or_string(&mut map, "up_mbps", v.as_ref())
                    }
                    "downmbps" | "down_mbps" | "down" => {
                        insert_yaml_number_or_string(&mut map, "down_mbps", v.as_ref())
                    }
                    "alpn" => insert_yaml_string(&mut map, "alpn", v.as_ref()),
                    "fp" | "fingerprint" => {
                        insert_yaml_string(&mut map, "client-fingerprint", v.as_ref())
                    }
                    "mport" | "ports" => insert_yaml_string(&mut map, "ports", v.as_ref()),
                    "insecure" | "allowInsecure" | "skipCertVerify" | "skip-cert-verify"
                        if parse_query_bool(v.as_ref()) =>
                    {
                        map.insert(
                            serde_yaml::Value::String("skip-cert-verify".to_string()),
                            serde_yaml::Value::Bool(true),
                        );
                    }
                    _ => {}
                }
            }
        }
        "tuic" => {
            map.insert(
                serde_yaml::Value::String("type".to_string()),
                serde_yaml::Value::String("tuic".to_string()),
            );
            let uuid = url.username().to_string();
            let decoded_uuid = urlencoding::decode(&uuid)
                .unwrap_or(std::borrow::Cow::Borrowed(&uuid))
                .into_owned();
            let password = url.password().unwrap_or("").to_string();
            let decoded_password = urlencoding::decode(&password)
                .unwrap_or(std::borrow::Cow::Borrowed(&password))
                .into_owned();
            if decoded_uuid.is_empty() || decoded_password.is_empty() {
                return None;
            }

            map.insert(
                serde_yaml::Value::String("uuid".to_string()),
                serde_yaml::Value::String(decoded_uuid),
            );
            if !decoded_password.is_empty() {
                map.insert(
                    serde_yaml::Value::String("password".to_string()),
                    serde_yaml::Value::String(decoded_password),
                );
            }

            for (k, v) in url.query_pairs() {
                match k.as_ref() {
                    "sni" | "peer" | "servername" => {
                        insert_yaml_string(&mut map, "sni", v.as_ref())
                    }
                    "congestion_control" | "congestion-control" => {
                        insert_yaml_string(&mut map, "congestion_control", v.as_ref())
                    }
                    "udp_relay_mode" | "udp-relay-mode" => {
                        insert_yaml_string(&mut map, "udp_relay_mode", v.as_ref())
                    }
                    "heartbeat" => insert_yaml_string(&mut map, "heartbeat", v.as_ref()),
                    "alpn" => insert_yaml_string(&mut map, "alpn", v.as_ref()),
                    "fp" | "fingerprint" => {
                        insert_yaml_string(&mut map, "client-fingerprint", v.as_ref())
                    }
                    "reduce-rtt" | "reduce_rtt" | "zero_rtt_handshake"
                        if parse_query_bool(v.as_ref()) =>
                    {
                        map.insert(
                            serde_yaml::Value::String("zero_rtt_handshake".to_string()),
                            serde_yaml::Value::Bool(true),
                        );
                    }
                    "udp_over_stream" if parse_query_bool(v.as_ref()) => {
                        map.insert(
                            serde_yaml::Value::String("udp_over_stream".to_string()),
                            serde_yaml::Value::Bool(true),
                        );
                    }
                    "insecure" | "allowInsecure" | "skipCertVerify" | "skip-cert-verify"
                        if parse_query_bool(v.as_ref()) =>
                    {
                        map.insert(
                            serde_yaml::Value::String("skip-cert-verify".to_string()),
                            serde_yaml::Value::Bool(true),
                        );
                    }
                    _ => {}
                }
            }
        }
        "anytls" => {
            map.insert(
                serde_yaml::Value::String("type".to_string()),
                serde_yaml::Value::String("anytls".to_string()),
            );
            let password = extract_url_credential(&url);
            if password.is_empty() {
                return None;
            }
            map.insert(
                serde_yaml::Value::String("password".to_string()),
                serde_yaml::Value::String(password),
            );
            for (k, v) in url.query_pairs() {
                match k.as_ref() {
                    "sni" | "peer" | "servername" => {
                        map.insert(
                            serde_yaml::Value::String("sni".to_string()),
                            serde_yaml::Value::String(v.into_owned()),
                        );
                    }
                    "alpn" => insert_yaml_string(&mut map, "alpn", v.as_ref()),
                    "fp" | "fingerprint" => {
                        insert_yaml_string(&mut map, "client-fingerprint", v.as_ref())
                    }
                    "insecure" | "allowInsecure" | "skipCertVerify" | "skip-cert-verify"
                        if parse_query_bool(v.as_ref()) =>
                    {
                        map.insert(
                            serde_yaml::Value::String("skip-cert-verify".to_string()),
                            serde_yaml::Value::Bool(true),
                        );
                    }
                    _ => {}
                }
            }
        }
        "naive+https" | "naive" => {
            insert_yaml_string(&mut map, "type", "naive");
            let username = urlencoding::decode(url.username())
                .unwrap_or(std::borrow::Cow::Borrowed(url.username()))
                .into_owned();
            let password = url
                .password()
                .and_then(|value| urlencoding::decode(value).ok())
                .map(|value| value.into_owned())
                .unwrap_or_default();
            if username.is_empty() || password.is_empty() {
                return None;
            }
            insert_yaml_string(&mut map, "username", &username);
            insert_yaml_string(&mut map, "password", &password);
            map.insert(
                serde_yaml::Value::String("tls".to_string()),
                serde_yaml::Value::Bool(true),
            );
            for (key, value) in url.query_pairs() {
                match key.as_ref() {
                    "sni" | "peer" => insert_yaml_string(&mut map, "sni", value.as_ref()),
                    "alpn" => insert_yaml_string(&mut map, "alpn", value.as_ref()),
                    "fp" | "fingerprint" => {
                        insert_yaml_string(&mut map, "client-fingerprint", value.as_ref())
                    }
                    "insecure" | "allowInsecure" | "skipCertVerify"
                        if parse_query_bool(value.as_ref()) =>
                    {
                        map.insert(
                            serde_yaml::Value::String("skip-cert-verify".to_string()),
                            serde_yaml::Value::Bool(true),
                        );
                    }
                    _ => {}
                }
            }
        }
        "ssh" => {
            insert_yaml_string(&mut map, "type", "ssh");
            if url.port().is_none() {
                map.insert(
                    serde_yaml::Value::String("port".to_string()),
                    serde_yaml::Value::Number(22.into()),
                );
            }
            let user = urlencoding::decode(url.username())
                .unwrap_or(std::borrow::Cow::Borrowed(url.username()))
                .into_owned();
            let password = url
                .password()
                .and_then(|value| urlencoding::decode(value).ok())
                .map(|value| value.into_owned())
                .unwrap_or_default();
            if user.is_empty() || password.is_empty() {
                return None;
            }
            insert_yaml_string(&mut map, "user", &user);
            insert_yaml_string(&mut map, "password", &password);
        }
        "shadowtls" | "shadow-tls" => {
            insert_yaml_string(&mut map, "type", "shadowtls");
            let password = extract_url_credential(&url);
            let mut version = 3u64;
            for (key, value) in url.query_pairs() {
                match key.as_ref() {
                    "version" => version = value.parse::<u64>().unwrap_or(3),
                    "sni" | "peer" | "servername" => {
                        insert_yaml_string(&mut map, "sni", value.as_ref())
                    }
                    "alpn" => insert_yaml_string(&mut map, "alpn", value.as_ref()),
                    "fp" | "fingerprint" => {
                        insert_yaml_string(&mut map, "client-fingerprint", value.as_ref())
                    }
                    "insecure" | "allowInsecure" | "skipCertVerify" | "skip-cert-verify"
                        if parse_query_bool(value.as_ref()) =>
                    {
                        map.insert(
                            serde_yaml::Value::String("skip-cert-verify".to_string()),
                            serde_yaml::Value::Bool(true),
                        );
                    }
                    _ => {}
                }
            }
            if version >= 3 && password.is_empty() {
                return None;
            }
            map.insert(
                serde_yaml::Value::String("version".to_string()),
                serde_yaml::Value::Number(version.into()),
            );
            insert_yaml_string(&mut map, "password", &password);
        }
        "socks" | "socks5" => {
            insert_yaml_string(&mut map, "type", "socks5");
            if url.port().is_none() {
                map.insert(
                    serde_yaml::Value::String("port".to_string()),
                    serde_yaml::Value::Number(1080.into()),
                );
            }
            let mut username = urlencoding::decode(url.username())
                .unwrap_or(std::borrow::Cow::Borrowed(url.username()))
                .into_owned();
            let mut password = url
                .password()
                .and_then(|value| urlencoding::decode(value).ok())
                .map(|value| value.into_owned())
                .unwrap_or_default();
            if url.password().is_none()
                && !username.is_empty()
                && let Some(decoded) = decode_base64_padded(&username)
                && !decoded.chars().any(|c| c.is_control())
                && let Some((decoded_user, decoded_pass)) = decoded.split_once(':')
            {
                username = decoded_user.to_string();
                password = decoded_pass.to_string();
            }
            insert_yaml_string(&mut map, "username", &username);
            insert_yaml_string(&mut map, "password", &password);
        }
        "wg" | "wireguard" => {
            insert_yaml_string(&mut map, "type", "wireguard");
            if url.port().is_none() {
                map.insert(
                    serde_yaml::Value::String("port".to_string()),
                    serde_yaml::Value::Number(51820.into()),
                );
            }
            let mut private_key = normalize_wg_key(&extract_url_credential(&url));
            let mut public_key = String::new();
            let mut pre_shared_key = String::new();
            let mut addresses: Vec<serde_yaml::Value> = Vec::new();
            let mut reserved: Vec<serde_yaml::Value> = Vec::new();
            let mut mtu: Option<u64> = None;

            for (key, value) in url.query_pairs() {
                match key.as_ref() {
                    "privatekey" | "private-key" | "private_key" if private_key.is_empty() => {
                        private_key = normalize_wg_key(value.as_ref());
                    }
                    "publickey" | "public-key" | "public_key" | "pk" => {
                        public_key = normalize_wg_key(value.as_ref());
                    }
                    "presharedkey" | "pre-shared-key" | "pre_shared_key" | "psk" => {
                        pre_shared_key = normalize_wg_key(value.as_ref());
                    }
                    "address" | "ip" => {
                        for part in value.split(',') {
                            let cidr = normalize_wg_cidr(part);
                            if !cidr.is_empty() {
                                addresses.push(serde_yaml::Value::String(cidr));
                            }
                        }
                    }
                    "mtu" => {
                        if let Ok(parsed) = value.trim().parse::<u64>()
                            && parsed > 0
                        {
                            mtu = Some(parsed);
                        }
                    }
                    "reserved" => {
                        let trimmed = value.trim().trim_start_matches('[').trim_end_matches(']');
                        for part in trimmed.split(',') {
                            if let Ok(byte) = part.trim().parse::<u8>() {
                                reserved.push(serde_yaml::Value::Number(u64::from(byte).into()));
                            }
                        }
                    }
                    _ => {}
                }
            }

            if private_key.is_empty() || public_key.is_empty() {
                return None;
            }
            insert_yaml_string(&mut map, "private-key", &private_key);
            insert_yaml_string(&mut map, "public-key", &public_key);
            if !pre_shared_key.is_empty() {
                insert_yaml_string(&mut map, "pre-shared-key", &pre_shared_key);
            }
            if !addresses.is_empty() {
                map.insert(
                    serde_yaml::Value::String("address".to_string()),
                    serde_yaml::Value::Sequence(addresses),
                );
            }
            if let Some(mtu) = mtu {
                map.insert(
                    serde_yaml::Value::String("mtu".to_string()),
                    serde_yaml::Value::Number(mtu.into()),
                );
            }
            if !reserved.is_empty() {
                map.insert(
                    serde_yaml::Value::String("reserved".to_string()),
                    serde_yaml::Value::Sequence(reserved),
                );
            }
        }
        "http" | "https" => {
            insert_yaml_string(&mut map, "type", "http");
            if url.port().is_none() {
                let default_port = if scheme == "https" { 443 } else { 80 };
                map.insert(
                    serde_yaml::Value::String("port".to_string()),
                    serde_yaml::Value::Number(default_port.into()),
                );
            }
            let username = urlencoding::decode(url.username())
                .unwrap_or(std::borrow::Cow::Borrowed(url.username()))
                .into_owned();
            let password = url
                .password()
                .and_then(|value| urlencoding::decode(value).ok())
                .map(|value| value.into_owned())
                .unwrap_or_default();
            insert_yaml_string(&mut map, "username", &username);
            insert_yaml_string(&mut map, "password", &password);
            if scheme == "https" {
                map.insert(
                    serde_yaml::Value::String("tls".to_string()),
                    serde_yaml::Value::Bool(true),
                );
            }
            for (key, value) in url.query_pairs() {
                match key.as_ref() {
                    "sni" | "peer" | "servername" => {
                        insert_yaml_string(&mut map, "sni", value.as_ref())
                    }
                    "alpn" => insert_yaml_string(&mut map, "alpn", value.as_ref()),
                    "fp" | "fingerprint" => {
                        insert_yaml_string(&mut map, "client-fingerprint", value.as_ref())
                    }
                    "insecure" | "allowInsecure" | "skipCertVerify" | "skip-cert-verify"
                        if parse_query_bool(value.as_ref()) =>
                    {
                        map.insert(
                            serde_yaml::Value::String("skip-cert-verify".to_string()),
                            serde_yaml::Value::Bool(true),
                        );
                    }
                    _ => {}
                }
            }
        }
        _ => return None,
    }

    Some(map)
}

fn canonical_clash_proxy_type(node_type: &str) -> String {
    match node_type.trim().to_ascii_lowercase().as_str() {
        "shadowsocks" => "ss".to_string(),
        "hy2" => "hysteria2".to_string(),
        "shadow-tls" | "shadow_tls" => "shadowtls".to_string(),
        "wg" => "wireguard".to_string(),
        value => value.to_string(),
    }
}

fn is_supported_clash_proxy_type(node_type: &str) -> bool {
    matches!(
        node_type,
        "ss" | "ssr"
            | "vmess"
            | "vless"
            | "trojan"
            | "hysteria"
            | "hysteria2"
            | "socks"
            | "socks5"
            | "http"
            | "tuic"
            | "anytls"
            | "naive"
            | "ssh"
            | "shadowtls"
            | "wireguard"
    )
}

fn clash_proxy_has_required_credentials(map: &serde_yaml::Mapping, node_type: &str) -> bool {
    let non_empty =
        |keys: &[&str]| yaml_string_any(map, keys).is_some_and(|value| !value.trim().is_empty());
    match node_type {
        "ss" | "ssr" => non_empty(&["cipher"]) && non_empty(&["password"]),
        "vmess" | "vless" => non_empty(&["uuid"]),
        "trojan" | "hysteria2" | "anytls" => non_empty(&["password"]),
        "hysteria" => non_empty(&["auth-str", "auth_str", "password"]),
        "tuic" => non_empty(&["uuid"]) && non_empty(&["password"]),
        "naive" => non_empty(&["username", "user"]) && non_empty(&["password"]),
        "ssh" => {
            non_empty(&["user", "username"])
                && (non_empty(&["password"])
                    || non_empty(&[
                        "private-key",
                        "private_key",
                        "private-key-path",
                        "private_key_path",
                    ]))
        }
        "shadowtls" => yaml_u64_any(map, &["version"]).unwrap_or(3) < 3 || non_empty(&["password"]),
        "wireguard" => {
            non_empty(&["private-key", "private_key"])
                && (non_empty(&["public-key", "public_key"])
                    || yaml_value_any(map, &["peers"])
                        .and_then(|v| v.as_sequence())
                        .is_some_and(|peers| !peers.is_empty()))
        }
        "socks" | "socks5" | "http" => true,
        _ => false,
    }
}

fn strip_bom_trimmed(content: &str) -> &str {
    content.trim_start_matches('\u{feff}').trim()
}

fn normalize_profile_content_inner(content: &str, allow_base64_decode: bool) -> String {
    let content = strip_bom_trimmed(content);
    if content.starts_with('{') || content.starts_with('[') {
        return content.to_string();
    }

    let mut proxies = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(map) = parse_share_link(line) {
            proxies.push(map);
        }
    }

    if !proxies.is_empty() {
        let mut root = serde_yaml::Mapping::new();
        root.insert(
            serde_yaml::Value::String("proxies".to_string()),
            serde_yaml::Value::Sequence(
                proxies
                    .into_iter()
                    .map(serde_yaml::Value::Mapping)
                    .collect(),
            ),
        );
        if let Ok(yaml_str) = serde_yaml::to_string(&root) {
            return yaml_str;
        }
    }

    // Many subscription providers return one Base64-encoded blob containing
    // newline-separated sharing links (or, less commonly, encoded YAML/JSON).
    // Decode at most once to avoid recursive ambiguity on arbitrary text.
    if allow_base64_decode && let Some(decoded) = decode_base64_padded(content) {
        let decoded = strip_bom_trimmed(&decoded);
        let looks_like_profile = decoded.starts_with('{')
            || decoded.starts_with('[')
            || decoded.contains("proxies:")
            || decoded.lines().any(|line| {
                matches!(
                    line.trim().split_once("://").map(|(scheme, _)| scheme),
                    Some(
                        "ss" | "ssr"
                            | "vmess"
                            | "vless"
                            | "trojan"
                            | "trojan-go"
                            | "hysteria"
                            | "hysteria2"
                            | "hy2"
                            | "tuic"
                            | "anytls"
                            | "naive+https"
                            | "naive"
                            | "ssh"
                            | "shadowtls"
                            | "shadow-tls"
                            | "socks"
                            | "socks5"
                            | "http"
                            | "https"
                            | "wg"
                            | "wireguard"
                    )
                )
            });
        if looks_like_profile {
            return normalize_profile_content_inner(decoded, false);
        }
    }

    content.to_string()
}

pub fn normalize_profile_content(content: &str) -> String {
    normalize_profile_content_inner(content, true)
}

pub fn get_profile_path(profile_id: &str) -> PathBuf {
    let sanitized: String = profile_id
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    get_app_dir()
        .join("profiles")
        .join(format!("{}.json", sanitized))
}

/// Returns `true` when `content` (after UTF-8 BOM stripping and optional
/// base64-blob decoding) begins with a JSON object or array delimiter.
#[allow(dead_code)] // Exported for Wave 2 main/UI wiring
pub fn is_native_json_profile(content: &str) -> bool {
    let normalized = normalize_profile_content(content);
    let trimmed = strip_bom_trimmed(&normalized);
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

/// Parse proxy nodes from either a native sing-box JSON profile or a
/// Clash YAML / share-link / base64 subscription, normalizing BOM and
/// base64-encoded JSON automatically before dispatch.
#[allow(dead_code)] // Exported for Wave 2 main/UI wiring
pub fn parse_nodes_from_content(content: &str) -> Result<Vec<ProxyNode>, String> {
    let normalized = normalize_profile_content(content);
    let trimmed = strip_bom_trimmed(&normalized);
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        parse_native_json_nodes(trimmed)
    } else {
        parse_clash_yaml_nodes(trimmed)
    }
}

/// Outbound tags emitted by `convert_clash_to_singbox` itself. Node names must
/// never collide with these or the generated config fails sing-box's duplicate
/// tag check; both the converter and the UI node parser dedup against them so
/// displayed names stay in sync with generated tags.
const RESERVED_OUTBOUND_TAGS: &[&str] = &["Proxy", "Auto", "direct"];

fn parse_server_ports_value(map: &serde_yaml::Mapping) -> Option<Vec<String>> {
    fn push_segment(items: &mut Vec<String>, seg: &str) {
        let s = seg.trim();
        if s.is_empty() {
            return;
        }
        if let Some((start, end)) = s.split_once('-').or_else(|| s.split_once(':')) {
            if let (Ok(a), Ok(b)) = (start.trim().parse::<u16>(), end.trim().parse::<u16>())
                && a > 0
                && b > 0
            {
                let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
                items.push(format!("{lo}:{hi}"));
            }
        } else if let Ok(p) = s.parse::<u16>()
            && p > 0
        {
            items.push(format!("{p}:{p}"));
        }
    }

    let val = yaml_value_any(map, &["ports", "server_ports", "server-ports"])?;
    let mut items = Vec::new();
    if let Some(s) = val.as_str() {
        for part in s.split(',') {
            push_segment(&mut items, part);
        }
    } else if let Some(n) = val.as_u64() {
        if let Ok(p) = u16::try_from(n)
            && p > 0
        {
            items.push(format!("{p}:{p}"));
        }
    } else if let Some(seq) = val.as_sequence() {
        for entry in seq {
            if let Some(s) = entry.as_str() {
                for part in s.split(',') {
                    push_segment(&mut items, part);
                }
            } else if let Some(n) = entry.as_u64()
                && let Ok(p) = u16::try_from(n)
                && p > 0
            {
                items.push(format!("{p}:{p}"));
            }
        }
    }
    (!items.is_empty()).then_some(items)
}

fn first_port_from_server_ports(ports: &[String]) -> Option<u16> {
    ports.iter().find_map(|range| {
        range
            .split(':')
            .next()
            .and_then(|s| s.parse::<u16>().ok())
            .filter(|&p| p > 0)
    })
}

fn extract_clash_endpoint(map: &serde_yaml::Mapping, node_type: &str) -> Option<(String, u16)> {
    let first_peer = if node_type == "wireguard" {
        yaml_value_any(map, &["peers"])
            .and_then(|v| v.as_sequence())
            .and_then(|peers| peers.first())
            .and_then(|v| v.as_mapping())
    } else {
        None
    };

    let read_server = |m: &serde_yaml::Mapping| {
        yaml_value_any(m, &["server", "address"]).and_then(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_i64().map(|i| i.to_string()))
        })
    };
    let read_port = |m: &serde_yaml::Mapping| {
        yaml_value_any(m, &["port", "server_port"]).and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_str().and_then(|text| text.parse::<u64>().ok()))
        })
    };

    let server = read_server(map)
        .or_else(|| first_peer.and_then(read_server))
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let port_u64 = read_port(map)
        .or_else(|| first_peer.and_then(read_port))
        .or_else(|| {
            parse_server_ports_value(map)
                .as_deref()
                .and_then(first_port_from_server_ports)
                .map(u64::from)
        })
        .unwrap_or(0);

    if port_u64 == 0 || port_u64 > 65535 {
        return None;
    }
    Some((server, port_u64 as u16))
}

pub fn parse_clash_yaml_nodes(content: &str) -> Result<Vec<ProxyNode>, String> {
    let key_name = serde_yaml::Value::String("name".into());
    let key_type = serde_yaml::Value::String("type".into());
    let normalized = normalize_profile_content(content);
    let val: serde_yaml::Value =
        serde_yaml::from_str(&normalized).map_err(|e| format!("YAML parsing failed: {}", e))?;

    let proxies = val
        .get("proxies")
        .and_then(|v| v.as_sequence())
        .ok_or_else(|| "No 'proxies' key found in config file".to_string())?;

    let mut nodes = Vec::new();
    // Mirror the converter's reserved-tag dedup so UI names match run tags.
    let mut used_names: BTreeSet<String> = RESERVED_OUTBOUND_TAGS
        .iter()
        .map(|tag| tag.to_string())
        .collect();
    for item in proxies {
        if let Some(map) = item.as_mapping() {
            let name = map
                .get(&key_name)
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed")
                .to_string();

            let node_type = map
                .get(&key_type)
                .and_then(|v| v.as_str())
                .map(canonical_clash_proxy_type)
                .unwrap_or_else(|| "unknown".to_string());

            if !is_supported_clash_proxy_type(&node_type) {
                continue;
            }
            if !clash_proxy_has_required_credentials(map, &node_type) {
                continue;
            }

            let Some((server, port)) = extract_clash_endpoint(map, &node_type) else {
                continue;
            };

            let clean_name: String = name
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect::<String>()
                .trim()
                .to_string();
            let base_name = if clean_name.is_empty() {
                "Unnamed".to_string()
            } else {
                clean_name
            };
            let mut unique_name = base_name.clone();
            let mut suffix = 2usize;
            while !used_names.insert(unique_name.clone()) {
                unique_name = format!("{base_name} ({suffix})");
                suffix += 1;
            }

            nodes.push(ProxyNode {
                name: unique_name,
                node_type,
                server,
                port,
                latency: None,
                selectable: true,
            });
        }
    }
    Ok(nodes)
}

fn extract_port_from_json_server_ports(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Option<u16> {
    let parse_entry = |s: &str| -> Option<u16> {
        let first = s.trim().split(':').next()?.trim();
        let p = first.parse::<u16>().ok()?;
        (p > 0).then_some(p)
    };
    let val = obj.get("server_ports")?;
    if let Some(s) = val.as_str() {
        return s.split(',').find_map(parse_entry);
    }
    val.as_array()?.iter().find_map(|entry| {
        entry.as_str().and_then(parse_entry).or_else(|| {
            entry
                .as_u64()
                .and_then(|n| u16::try_from(n).ok())
                .filter(|&p| p > 0)
        })
    })
}

pub fn parse_native_json_nodes(json_content: &str) -> Result<Vec<ProxyNode>, String> {
    let normalized = normalize_profile_content(json_content);
    let trimmed = strip_bom_trimmed(&normalized);
    let val: serde_json::Value =
        serde_json::from_str(trimmed).map_err(|e| format!("JSON parsing failed: {}", e))?;

    let mut nodes = Vec::new();
    let outbounds = val
        .get("outbounds")
        .and_then(|value| value.as_array())
        .map(Vec::as_slice)
        .unwrap_or_default();
    for item in outbounds {
        if let Some(obj) = item.as_object() {
            let node_type = obj
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            if node_type == "selector"
                || node_type == "direct"
                || node_type == "dns"
                || node_type == "block"
                || node_type == "urltest"
            {
                continue;
            }

            let name = obj
                .get("tag")
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed")
                .to_string();

            let server = obj
                .get("server")
                .and_then(|v| {
                    v.as_str()
                        .map(|s| s.to_string())
                        .or_else(|| v.as_i64().map(|i| i.to_string()))
                })
                .unwrap_or_else(|| "127.0.0.1".to_string());

            let port_u64 = obj
                .get("server_port")
                .and_then(|v| v.as_u64())
                .or_else(|| extract_port_from_json_server_ports(obj).map(u64::from))
                .unwrap_or(0);

            if port_u64 == 0 || port_u64 > 65535 {
                continue;
            }
            let port = port_u64 as u16;

            if port > 0 {
                nodes.push(ProxyNode {
                    name,
                    node_type,
                    server,
                    port,
                    latency: None,
                    selectable: true,
                });
            }
        }
    }

    if let Some(endpoints) = val.get("endpoints").and_then(|value| value.as_array()) {
        for item in endpoints {
            let Some(obj) = item.as_object() else {
                continue;
            };
            let endpoint_type = obj
                .get("type")
                .and_then(|value| value.as_str())
                .unwrap_or("endpoint");
            let name = obj
                .get("tag")
                .and_then(|value| value.as_str())
                .unwrap_or("Unnamed endpoint")
                .to_string();
            let peer = obj
                .get("peers")
                .and_then(|value| value.as_array())
                .and_then(|peers| peers.first())
                .and_then(|value| value.as_object());
            let server = peer
                .and_then(|peer| peer.get("address").or_else(|| peer.get("server")))
                .and_then(|value| value.as_str())
                .or_else(|| obj.get("server").and_then(|value| value.as_str()));
            let port = peer
                .and_then(|peer| peer.get("port").or_else(|| peer.get("server_port")))
                .and_then(|value| value.as_u64())
                .or_else(|| obj.get("server_port").and_then(|value| value.as_u64()));
            if let (Some(server), Some(port)) = (server, port)
                && (1..=65535).contains(&port)
            {
                nodes.push(ProxyNode {
                    name,
                    node_type: format!("{endpoint_type} endpoint"),
                    server: server.to_string(),
                    port: port as u16,
                    latency: None,
                    selectable: false,
                });
            }
        }
    }
    Ok(nodes)
}

pub fn validate_profile_content(content: &str) -> Result<(), String> {
    let normalized = normalize_profile_content(content);
    let trimmed = strip_bom_trimmed(&normalized);
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        let value: serde_json::Value =
            serde_json::from_str(trimmed).map_err(|e| format!("Invalid JSON structure: {}", e))?;
        let object = value
            .as_object()
            .ok_or_else(|| "Native sing-box JSON must be a configuration object".to_string())?;
        let has_outbounds = object
            .get("outbounds")
            .is_some_and(|value| value.is_array());
        let has_endpoints = object
            .get("endpoints")
            .is_some_and(|value| value.is_array());
        if !has_outbounds && !has_endpoints {
            return Err(
                "Native sing-box JSON must contain an 'outbounds' or 'endpoints' array".to_string(),
            );
        }
        Ok(())
    } else {
        // First validate the YAML itself — catches malformed indentation etc.
        let _: serde_yaml::Value =
            serde_yaml::from_str(trimmed).map_err(|e| format!("Invalid YAML structure: {}", e))?;
        // Then ensure at least one proxy node was actually parsed.
        let nodes = parse_clash_yaml_nodes(trimmed)
            .map_err(|e| format!("Invalid Clash configuration format: {}", e))?;
        if nodes.is_empty() {
            return Err(
                "Profile contained no parseable proxy nodes (unsupported schemes or bad links)."
                    .to_string(),
            );
        }
        Ok(())
    }
}

fn yaml_value_any<'a>(
    map: &'a serde_yaml::Mapping,
    keys: &[&str],
) -> Option<&'a serde_yaml::Value> {
    keys.iter()
        .find_map(|key| map.get(serde_yaml::Value::String((*key).to_string())))
}

fn yaml_string_any<'a>(map: &'a serde_yaml::Mapping, keys: &[&str]) -> Option<&'a str> {
    yaml_value_any(map, keys).and_then(|value| value.as_str())
}

fn yaml_bool_any(map: &serde_yaml::Mapping, keys: &[&str]) -> Option<bool> {
    yaml_value_any(map, keys).and_then(|value| {
        value
            .as_bool()
            .or_else(|| value.as_str().map(parse_query_bool))
    })
}

fn yaml_u64_any(map: &serde_yaml::Mapping, keys: &[&str]) -> Option<u64> {
    yaml_value_any(map, keys).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_str().and_then(|text| text.parse::<u64>().ok()))
    })
}

fn yaml_string_list_any(map: &serde_yaml::Mapping, keys: &[&str]) -> Vec<String> {
    yaml_value_any(map, keys)
        .map(|value| {
            if let Some(text) = value.as_str() {
                text.split(',')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(str::to_string)
                    .collect()
            } else {
                value
                    .as_sequence()
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(|entry| entry.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default()
            }
        })
        .unwrap_or_default()
}

fn enrich_tls_options(
    tls: &mut serde_json::Map<String, serde_json::Value>,
    map: &serde_yaml::Mapping,
) {
    let alpn = yaml_string_list_any(map, &["alpn"]);
    if !alpn.is_empty() {
        tls.insert("alpn".to_string(), json!(alpn));
    }
    if let Some(fingerprint) = yaml_string_any(map, &["client-fingerprint", "fingerprint"])
        .filter(|value| !value.is_empty() && *value != "none")
    {
        tls.insert(
            "utls".to_string(),
            json!({ "enabled": true, "fingerprint": fingerprint }),
        );
    }
}

fn extract_ws_early_data_from_path(raw_path: &str) -> (String, Option<u64>) {
    let Some((base, query)) = raw_path.split_once('?') else {
        return (raw_path.to_string(), None);
    };
    let mut ed: Option<u64> = None;
    let mut kept_params: Vec<&str> = Vec::new();
    for part in query.split('&') {
        if part.is_empty() {
            continue;
        }
        let (key, value) = part.split_once('=').unwrap_or((part, ""));
        if key == "ed" {
            if let Ok(parsed) = value.parse::<u64>()
                && parsed > 0
            {
                ed = Some(parsed);
            }
        } else {
            kept_params.push(part);
        }
    }
    let clean_path = if kept_params.is_empty() {
        base.to_string()
    } else {
        format!("{base}?{}", kept_params.join("&"))
    };
    (clean_path, ed)
}

fn get_transport_block(map: &serde_yaml::Mapping) -> Option<serde_json::Value> {
    let network = yaml_string_any(map, &["network", "type"]).unwrap_or("tcp");

    if network == "ws" {
        let mut raw_path = "/".to_string();
        let mut headers_map = serde_json::Map::new();
        let mut max_early_data: Option<u64> = None;
        let mut early_data_header_name: Option<String> = None;

        if let Some(ws_opts) =
            yaml_value_any(map, &["ws-opts", "ws_opts"]).and_then(|v| v.as_mapping())
        {
            if let Some(p) = yaml_string_any(ws_opts, &["path"]).filter(|s| !s.is_empty()) {
                raw_path = p.to_string();
            }
            if let Some(headers) =
                yaml_value_any(ws_opts, &["headers"]).and_then(|v| v.as_mapping())
            {
                for (k, v) in headers {
                    if let (Some(ks), Some(vs)) = (k.as_str(), v.as_str())
                        && !vs.is_empty()
                    {
                        headers_map.insert(ks.to_string(), json!(vs));
                    }
                }
            }
            max_early_data =
                yaml_u64_any(ws_opts, &["max-early-data", "max_early_data", "early-data"])
                    .filter(|&n| n > 0);
            early_data_header_name = yaml_string_any(
                ws_opts,
                &["early-data-header-name", "early_data_header_name"],
            )
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        } else {
            if let Some(p) = yaml_string_any(map, &["path"]).filter(|s| !s.is_empty()) {
                raw_path = p.to_string();
            }
            if let Some(h) = yaml_string_any(map, &["host"]).filter(|s| !s.is_empty()) {
                headers_map.insert("Host".to_string(), json!(h));
            }
        }

        let (path, ed_from_path) = extract_ws_early_data_from_path(&raw_path);
        let early_data = max_early_data.or(ed_from_path);

        let mut transport = serde_json::Map::new();
        transport.insert("type".to_string(), json!("ws"));
        transport.insert("path".to_string(), json!(path));
        if !headers_map.is_empty() {
            transport.insert(
                "headers".to_string(),
                serde_json::Value::Object(headers_map),
            );
        }
        if let Some(ed) = early_data {
            transport.insert("max_early_data".to_string(), json!(ed));
            transport.insert(
                "early_data_header_name".to_string(),
                json!(
                    early_data_header_name.unwrap_or_else(|| "Sec-WebSocket-Protocol".to_string())
                ),
            );
        }
        Some(serde_json::Value::Object(transport))
    } else if network == "grpc" {
        let mut service_name = String::new();
        if let Some(grpc_opts) =
            yaml_value_any(map, &["grpc-opts", "grpc_opts"]).and_then(|v| v.as_mapping())
        {
            if let Some(s) = yaml_string_any(
                grpc_opts,
                &[
                    "grpc-service-name",
                    "grpc_service_name",
                    "serviceName",
                    "service_name",
                ],
            ) {
                service_name = s.to_string();
            }
        } else if let Some(s) = yaml_string_any(
            map,
            &[
                "grpc-service-name",
                "grpc_service_name",
                "serviceName",
                "service_name",
            ],
        ) {
            service_name = s.to_string();
        }

        let mut transport = serde_json::Map::new();
        transport.insert("type".to_string(), json!("grpc"));
        if !service_name.is_empty() {
            transport.insert("service_name".to_string(), json!(service_name));
        }
        Some(serde_json::Value::Object(transport))
    } else if network == "http" || network == "h2" {
        let opts = if network == "h2" {
            yaml_value_any(map, &["h2-opts", "h2_opts"]).and_then(|v| v.as_mapping())
        } else {
            yaml_value_any(map, &["http-opts", "http_opts"]).and_then(|v| v.as_mapping())
        };
        let path = opts
            .and_then(|o| {
                yaml_string_any(o, &["path"])
                    .map(str::to_string)
                    .or_else(|| {
                        yaml_value_any(o, &["path"])
                            .and_then(|v| v.as_sequence())
                            .and_then(|s| s.first())
                            .and_then(|v| v.as_str())
                            .map(str::to_string)
                    })
            })
            .or_else(|| yaml_string_any(map, &["path"]).map(str::to_string))
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| "/".to_string());
        let hosts: Vec<String> = opts
            .and_then(|o| yaml_value_any(o, &["host"]))
            .map(|v| {
                if let Some(host) = v.as_str() {
                    host.split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect()
                } else {
                    v.as_sequence()
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(|v| v.as_str())
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .map(str::to_string)
                                .collect()
                        })
                        .unwrap_or_default()
                }
            })
            .or_else(|| {
                yaml_string_any(map, &["host"]).map(|host| {
                    host.split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect()
                })
            })
            .unwrap_or_default();
        let method = opts
            .and_then(|o| yaml_string_any(o, &["method"]))
            .map(str::trim)
            .filter(|m| !m.is_empty());

        let mut transport = serde_json::Map::new();
        transport.insert("type".to_string(), json!("http"));
        transport.insert("path".to_string(), json!(path));
        if !hosts.is_empty() {
            transport.insert("host".to_string(), json!(hosts));
        }
        if let Some(method) = method {
            transport.insert("method".to_string(), json!(method));
        }
        Some(serde_json::Value::Object(transport))
    } else if network == "httpupgrade" || network == "http-upgrade" {
        let opts = yaml_value_any(map, &["http-upgrade-opts", "http_upgrade_opts"])
            .and_then(|v| v.as_mapping());
        let path = opts
            .and_then(|o| yaml_string_any(o, &["path"]))
            .or_else(|| yaml_string_any(map, &["path"]))
            .filter(|p| !p.is_empty())
            .unwrap_or("/");
        let host = opts
            .and_then(|o| yaml_string_any(o, &["host"]))
            .or_else(|| yaml_string_any(map, &["host"]))
            .map(str::trim)
            .filter(|h| !h.is_empty());
        let mut transport = serde_json::Map::new();
        transport.insert("type".to_string(), json!("httpupgrade"));
        transport.insert("path".to_string(), json!(path));
        if let Some(host) = host {
            transport.insert("host".to_string(), json!(host));
        }
        Some(serde_json::Value::Object(transport))
    } else if network == "quic" {
        Some(json!({ "type": "quic" }))
    } else {
        None
    }
}

fn extract_multiplex_block(map: &serde_yaml::Mapping) -> Option<serde_json::Value> {
    let mux = yaml_value_any(map, &["smux", "multiplex"])?.as_mapping()?;
    if !yaml_bool_any(mux, &["enabled"]).unwrap_or(false) {
        return None;
    }
    let mut obj = serde_json::Map::new();
    obj.insert("enabled".to_string(), json!(true));
    if let Some(protocol) = yaml_string_any(mux, &["protocol"]).filter(|s| !s.is_empty()) {
        obj.insert("protocol".to_string(), json!(protocol));
    }
    if let Some(max_conn) = yaml_u64_any(mux, &["max-connections", "max_connections"]) {
        obj.insert("max_connections".to_string(), json!(max_conn));
    }
    if let Some(min_streams) = yaml_u64_any(mux, &["min-streams", "min_streams"]) {
        obj.insert("min_streams".to_string(), json!(min_streams));
    }
    if let Some(max_streams) = yaml_u64_any(mux, &["max-streams", "max_streams"]) {
        obj.insert("max_streams".to_string(), json!(max_streams));
    }
    if let Some(padding) = yaml_bool_any(mux, &["padding"]) {
        obj.insert("padding".to_string(), json!(padding));
    }
    if let Some(brutal) = yaml_value_any(mux, &["brutal"]).and_then(|v| v.as_mapping())
        && yaml_bool_any(brutal, &["enabled"]).unwrap_or(false)
    {
        let mut b = serde_json::Map::new();
        b.insert("enabled".to_string(), json!(true));
        if let Some(up) = yaml_u64_any(brutal, &["up-mbps", "up_mbps"]) {
            b.insert("up_mbps".to_string(), json!(up));
        }
        if let Some(down) = yaml_u64_any(brutal, &["down-mbps", "down_mbps"]) {
            b.insert("down_mbps".to_string(), json!(down));
        }
        obj.insert("brutal".to_string(), serde_json::Value::Object(b));
    }
    Some(serde_json::Value::Object(obj))
}

fn extract_sni<'a>(map: &'a serde_yaml::Mapping, fallback: &'a str) -> &'a str {
    yaml_string_any(map, &["sni", "servername"])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback)
}

fn yaml_scalar_string_any(map: &serde_yaml::Mapping, keys: &[&str]) -> Option<String> {
    yaml_value_any(map, keys).and_then(|value| {
        value
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| value.as_u64().map(|n| n.to_string()))
            .or_else(|| value.as_i64().map(|n| n.to_string()))
    })
}

fn yaml_mbps_any(map: &serde_yaml::Mapping, keys: &[&str]) -> Option<u64> {
    let value = yaml_value_any(map, keys)?;
    if let Some(n) = value.as_u64() {
        return (n > 0).then_some(n);
    }
    let text = value.as_str()?.trim();
    if text.is_empty() {
        return None;
    }
    let lower = text.to_ascii_lowercase();
    let num_end = lower
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(lower.len());
    let number: f64 = lower[..num_end].trim().parse().ok()?;
    let unit = lower[num_end..].trim();
    let mbps = match unit {
        "" | "m" | "mbps" => number,
        "g" | "gbps" => number * 1000.0,
        "k" | "kbps" => (number / 1000.0).max(1.0),
        _ => return None,
    };
    (mbps >= 1.0).then_some(mbps.round() as u64)
}

fn translate_ss_plugin(
    plugin: &str,
    raw_opts: Option<&str>,
    opts_map: Option<&serde_yaml::Mapping>,
) -> (String, String) {
    let plugin_lower = plugin.trim().to_ascii_lowercase();
    match plugin_lower.as_str() {
        "obfs" | "simple-obfs" | "obfs-local" => {
            if let Some(opts) = opts_map {
                let mut pairs = Vec::new();
                if let Some(mode) = yaml_string_any(opts, &["obfs", "mode"])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    pairs.push(format!("obfs={mode}"));
                }
                if let Some(host) = yaml_string_any(opts, &["obfs-host", "host"])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    pairs.push(format!("obfs-host={host}"));
                }
                ("obfs-local".to_string(), pairs.join(";"))
            } else if let Some(raw) = raw_opts {
                let normalized = raw
                    .split(';')
                    .map(str::trim)
                    .filter(|p| !p.is_empty())
                    .map(|part| {
                        if let Some(val) = part.strip_prefix("mode=") {
                            format!("obfs={val}")
                        } else if let Some(val) = part.strip_prefix("host=") {
                            format!("obfs-host={val}")
                        } else {
                            part.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(";");
                ("obfs-local".to_string(), normalized)
            } else {
                ("obfs-local".to_string(), String::new())
            }
        }
        "v2ray-plugin" => {
            if let Some(opts) = opts_map {
                let mut parts = Vec::new();
                for (k, v) in opts {
                    let Some(key) = k.as_str() else { continue };
                    match key {
                        "tls" | "skip-cert-verify" | "fast-open" | "mux" => {
                            let enabled = v
                                .as_bool()
                                .or_else(|| v.as_str().map(parse_query_bool))
                                .unwrap_or(false);
                            if enabled {
                                parts.push(key.to_string());
                            }
                        }
                        "headers" => {
                            if let Some(headers) = v.as_mapping()
                                && let Some(host) = yaml_string_any(headers, &["Host", "host"])
                                    .map(str::trim)
                                    .filter(|s| !s.is_empty())
                            {
                                parts.push(format!("host={host}"));
                            }
                        }
                        _ => {
                            if let Some(val) = v
                                .as_str()
                                .map(str::to_string)
                                .or_else(|| v.as_bool().map(|b| b.to_string()))
                                .or_else(|| v.as_i64().map(|n| n.to_string()))
                                .filter(|s| !s.is_empty())
                            {
                                parts.push(format!("{key}={val}"));
                            }
                        }
                    }
                }
                ("v2ray-plugin".to_string(), parts.join(";"))
            } else {
                (
                    "v2ray-plugin".to_string(),
                    raw_opts.unwrap_or_default().to_string(),
                )
            }
        }
        _ => (
            plugin.to_string(),
            raw_opts
                .map(str::to_string)
                .or_else(|| {
                    opts_map.map(|opts| {
                        opts.iter()
                            .filter_map(|(k, v)| {
                                let key = k.as_str()?;
                                let value = v
                                    .as_str()
                                    .map(str::to_string)
                                    .or_else(|| v.as_bool().map(|b| b.to_string()))
                                    .or_else(|| v.as_i64().map(|n| n.to_string()))?;
                                Some(format!("{key}={value}"))
                            })
                            .collect::<Vec<_>>()
                            .join(";")
                    })
                })
                .unwrap_or_default(),
        ),
    }
}

fn split_dns_host_port(authority: &str) -> (String, Option<u16>) {
    let authority = authority.trim();
    if let Some(rest) = authority.strip_prefix('[') {
        if let Some((ip, after)) = rest.split_once(']') {
            let port = after.strip_prefix(':').and_then(|p| p.parse::<u16>().ok());
            return (ip.to_string(), port);
        }
        return (authority.to_string(), None);
    }
    if authority.parse::<std::net::IpAddr>().is_ok() {
        return (authority.to_string(), None);
    }
    if let Some((host, port_str)) = authority.rsplit_once(':')
        && !host.contains(':')
        && let Ok(port) = port_str.parse::<u16>()
    {
        return (host.to_string(), Some(port));
    }
    (authority.to_string(), None)
}

/// Convert a GUI DNS server address into a sing-box 1.12+/1.13+ typed DNS server object.
/// Returns `(server_json, server_host_is_domain)`.
fn dns_address_to_server_json(
    tag: &str,
    raw_addr: &str,
    detour: Option<&str>,
    domain_resolver: Option<&str>,
) -> (serde_json::Value, bool) {
    let addr = raw_addr.trim();
    if addr.eq_ignore_ascii_case("fakeip") {
        return (
            json!({
                "tag": tag,
                "type": "fakeip",
                "inet4_range": "198.18.0.0/15",
                "inet6_range": "fc00::/18"
            }),
            false,
        );
    }
    if addr.eq_ignore_ascii_case("local") || addr.eq_ignore_ascii_case("local://") {
        return (json!({ "tag": tag, "type": "local" }), false);
    }
    if let Some(iface) = addr.strip_prefix("dhcp://") {
        let mut obj = serde_json::Map::new();
        obj.insert("tag".to_string(), json!(tag));
        obj.insert("type".to_string(), json!("dhcp"));
        let iface = iface.trim();
        if !iface.is_empty() && !iface.eq_ignore_ascii_case("auto") {
            obj.insert("interface".to_string(), json!(iface));
        }
        return (serde_json::Value::Object(obj), false);
    }

    let (dns_type, rest) = if let Some((scheme, after)) = addr.split_once("://") {
        let schema = match scheme.to_ascii_lowercase().as_str() {
            "udp" | "tcp" | "tls" | "https" | "h3" | "quic" => scheme.to_ascii_lowercase(),
            _ => "udp".to_string(),
        };
        (schema, after)
    } else {
        ("udp".to_string(), addr)
    };

    let (authority, path) = match rest.split_once('/') {
        Some((a, p)) => (a, Some(format!("/{p}"))),
        None => (rest, None),
    };
    let (host, port) = split_dns_host_port(authority);
    let server_host = if host.is_empty() {
        "8.8.8.8".to_string()
    } else {
        host
    };
    let is_domain = server_host.parse::<std::net::IpAddr>().is_err();

    let mut obj = serde_json::Map::new();
    obj.insert("tag".to_string(), json!(tag));
    obj.insert("type".to_string(), json!(dns_type));
    obj.insert("server".to_string(), json!(server_host));
    if let Some(port) = port {
        obj.insert("server_port".to_string(), json!(port));
    }
    if matches!(dns_type.as_str(), "https" | "h3")
        && let Some(path) = path.filter(|p| !p.is_empty())
    {
        obj.insert("path".to_string(), json!(path));
    }
    if is_domain && let Some(resolver) = domain_resolver {
        obj.insert("domain_resolver".to_string(), json!(resolver));
    }
    if let Some(detour) = detour.filter(|d| !d.is_empty()) {
        obj.insert("detour".to_string(), json!(detour));
    }
    (serde_json::Value::Object(obj), is_domain)
}

pub fn convert_clash_to_singbox(
    yaml_content: &str,
    gui_config: &GuiConfig,
) -> Result<serde_json::Value, String> {
    let key_alter_id = serde_yaml::Value::String("alterId".into());
    let key_auth_str = serde_yaml::Value::String("auth_str".into());
    let key_auth_str_dash = serde_yaml::Value::String("auth-str".into());
    let key_cipher = serde_yaml::Value::String("cipher".into());
    let key_flow = serde_yaml::Value::String("flow".into());
    let key_name = serde_yaml::Value::String("name".into());
    let key_password = serde_yaml::Value::String("password".into());
    let key_plugin = serde_yaml::Value::String("plugin".into());
    let key_plugin_opts = serde_yaml::Value::String("plugin-opts".into());
    let key_plugin_opts_raw = serde_yaml::Value::String("plugin-opts-raw".into());
    let key_protocol = serde_yaml::Value::String("protocol".into());
    let key_protocol_param = serde_yaml::Value::String("protocol-param".into());
    let key_reality_opts = serde_yaml::Value::String("reality-opts".into());
    let key_skip_cert_verify = serde_yaml::Value::String("skip-cert-verify".into());
    let key_type = serde_yaml::Value::String("type".into());
    let key_uuid = serde_yaml::Value::String("uuid".into());
    let normalized = normalize_profile_content(yaml_content);
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&normalized).map_err(|e| format!("YAML parsing failed: {}", e))?;

    let proxies_val = yaml
        .get("proxies")
        .ok_or_else(|| "No 'proxies' key found in Clash config".to_string())?;

    let proxies_arr = proxies_val
        .as_sequence()
        .ok_or_else(|| "'proxies' must be a sequence/list".to_string())?;

    let mut outbounds = Vec::new();
    let mut endpoints = Vec::new();
    let mut node_tags = Vec::new();
    // Reserve the tags this generator emits itself ("Proxy" selector, "Auto"
    // urltest, standard direct) so a node with one of these names gets
    // a " (2)" suffix instead of producing a duplicate outbound tag that
    // sing-box rejects at startup.
    let mut used_tags: BTreeSet<String> = RESERVED_OUTBOUND_TAGS
        .iter()
        .map(|tag| tag.to_string())
        .collect();
    let mut unsupported_types = BTreeSet::new();

    for item in proxies_arr {
        let mut outbound = serde_json::Map::new();

        let map = match item.as_mapping() {
            Some(m) => m,
            None => continue,
        };

        let name = map
            .get(&key_name)
            .and_then(|v| v.as_str())
            .unwrap_or("Unnamed")
            .to_string();

        let node_type = map
            .get(&key_type)
            .and_then(|v| v.as_str())
            .map(canonical_clash_proxy_type)
            .unwrap_or_else(|| "unknown".to_string());

        if !is_supported_clash_proxy_type(&node_type) {
            unsupported_types.insert(node_type.clone());
            continue;
        }

        let Some((server, port)) = extract_clash_endpoint(map, &node_type) else {
            continue;
        };

        let skip_cert_verify = map
            .get(&key_skip_cert_verify)
            .and_then(|v| v.as_bool().or_else(|| v.as_str().map(parse_query_bool)))
            .unwrap_or(false);

        let mut wireguard_endpoint: Option<serde_json::Map<String, serde_json::Value>> = None;

        match node_type.as_str() {
            "ss" => {
                outbound.insert("type".to_string(), json!("shadowsocks"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));

                let cipher = map.get(&key_cipher).and_then(|v| v.as_str()).unwrap_or("");
                let password = map
                    .get(&key_password)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if cipher.is_empty() || password.is_empty() {
                    continue;
                }

                outbound.insert("method".to_string(), json!(cipher));
                outbound.insert("password".to_string(), json!(password));

                let has_plugin = if let Some(plugin) = map.get(&key_plugin).and_then(|v| v.as_str())
                {
                    let raw_opts = map.get(&key_plugin_opts_raw).and_then(|v| v.as_str());
                    let opts_map = map.get(&key_plugin_opts).and_then(|v| v.as_mapping());
                    let (translated_plugin, translated_opts) =
                        translate_ss_plugin(plugin, raw_opts, opts_map);
                    outbound.insert("plugin".to_string(), json!(translated_plugin));
                    if !translated_opts.is_empty() {
                        outbound.insert("plugin_opts".to_string(), json!(translated_opts));
                    }
                    true
                } else {
                    false
                };

                if !has_plugin && let Some(multiplex) = extract_multiplex_block(map) {
                    outbound.insert("multiplex".to_string(), multiplex);
                }
            }
            "ssr" => {
                outbound.insert("type".to_string(), json!("shadowsocksr"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));

                let method = map
                    .get(&key_cipher)
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                let password = map
                    .get(&key_password)
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                if method.is_empty() || password.is_empty() {
                    continue;
                }
                outbound.insert("method".to_string(), json!(method));
                outbound.insert("password".to_string(), json!(password));

                if let Some(protocol) = map
                    .get(&key_protocol)
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                {
                    outbound.insert("protocol".to_string(), json!(protocol));
                }
                if let Some(param) = map
                    .get(&key_protocol_param)
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                {
                    outbound.insert("protocol_param".to_string(), json!(param));
                }
                if let Some(obfs) =
                    yaml_string_any(map, &["obfs"]).filter(|value| !value.is_empty())
                {
                    outbound.insert("obfs".to_string(), json!(obfs));
                }
                if let Some(param) = yaml_string_any(map, &["obfs-param", "obfs_param"])
                    .filter(|value| !value.is_empty())
                {
                    outbound.insert("obfs_param".to_string(), json!(param));
                }
            }
            "vmess" => {
                outbound.insert("type".to_string(), json!("vmess"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));

                let uuid = map.get(&key_uuid).and_then(|v| v.as_str()).unwrap_or("");
                let security = map
                    .get(&key_cipher)
                    .and_then(|v| v.as_str())
                    .unwrap_or("auto");
                let alter_id = map.get(&key_alter_id).and_then(|v| v.as_i64()).unwrap_or(0);
                if uuid.is_empty() {
                    continue;
                }

                outbound.insert("uuid".to_string(), json!(uuid));
                outbound.insert("security".to_string(), json!(security));
                if alter_id > 0 {
                    outbound.insert("alter_id".to_string(), json!(alter_id));
                }

                let tls_enabled = yaml_bool_any(map, &["tls"]).unwrap_or(false);
                let sni = extract_sni(map, &server);

                if tls_enabled {
                    let mut tls_opts = serde_json::Map::new();
                    tls_opts.insert("enabled".to_string(), json!(true));
                    tls_opts.insert("server_name".to_string(), json!(sni));
                    tls_opts.insert("insecure".to_string(), json!(skip_cert_verify));
                    enrich_tls_options(&mut tls_opts, map);
                    outbound.insert("tls".to_string(), serde_json::Value::Object(tls_opts));
                }

                if let Some(trans) = get_transport_block(map) {
                    outbound.insert("transport".to_string(), trans);
                }
                if let Some(multiplex) = extract_multiplex_block(map) {
                    outbound.insert("multiplex".to_string(), multiplex);
                }
            }
            "vless" => {
                outbound.insert("type".to_string(), json!("vless"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));

                let uuid = map.get(&key_uuid).and_then(|v| v.as_str()).unwrap_or("");
                let flow = map.get(&key_flow).and_then(|v| v.as_str()).unwrap_or("");
                if uuid.is_empty() {
                    continue;
                }

                outbound.insert("uuid".to_string(), json!(uuid));
                if !flow.is_empty() {
                    outbound.insert("flow".to_string(), json!(flow));
                }

                let reality_opts_map = map.get(&key_reality_opts).and_then(|v| v.as_mapping());
                let public_key = reality_opts_map
                    .and_then(|m| yaml_scalar_string_any(m, &["public-key", "public_key", "pbk"]))
                    .or_else(|| yaml_scalar_string_any(map, &["public-key", "public_key", "pbk"]))
                    .unwrap_or_default();
                let short_id = reality_opts_map
                    .and_then(|m| yaml_scalar_string_any(m, &["short-id", "short_id", "sid"]))
                    .or_else(|| yaml_scalar_string_any(map, &["short-id", "short_id", "sid"]))
                    .unwrap_or_default();

                let reality_enabled = yaml_bool_any(map, &["reality"]).unwrap_or(false)
                    || reality_opts_map.is_some()
                    || !public_key.is_empty();
                let tls_enabled = yaml_bool_any(map, &["tls"]).unwrap_or(false) || reality_enabled;
                let sni = extract_sni(map, &server);

                if tls_enabled {
                    let mut tls_opts = serde_json::Map::new();
                    tls_opts.insert("enabled".to_string(), json!(true));
                    tls_opts.insert("server_name".to_string(), json!(sni));
                    tls_opts.insert("insecure".to_string(), json!(skip_cert_verify));
                    enrich_tls_options(&mut tls_opts, map);

                    if reality_enabled {
                        let mut reality_json = serde_json::Map::new();
                        reality_json.insert("enabled".to_string(), json!(true));
                        reality_json.insert("public_key".to_string(), json!(public_key));
                        if !short_id.is_empty() {
                            reality_json.insert("short_id".to_string(), json!(short_id));
                        }
                        tls_opts.insert(
                            "reality".to_string(),
                            serde_json::Value::Object(reality_json),
                        );

                        tls_opts.entry("utls".to_string()).or_insert_with(|| {
                            json!({
                                "enabled": true,
                                "fingerprint": "chrome"
                            })
                        });
                    }

                    outbound.insert("tls".to_string(), serde_json::Value::Object(tls_opts));
                }

                if let Some(trans) = get_transport_block(map) {
                    outbound.insert("transport".to_string(), trans);
                }
                if let Some(multiplex) = extract_multiplex_block(map) {
                    outbound.insert("multiplex".to_string(), multiplex);
                }
            }
            "trojan" => {
                outbound.insert("type".to_string(), json!("trojan"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));

                let password = map
                    .get(&key_password)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if password.is_empty() {
                    continue;
                }
                outbound.insert("password".to_string(), json!(password));

                let tls_enabled = yaml_bool_any(map, &["tls"]).unwrap_or(true);
                let sni = extract_sni(map, &server);

                if tls_enabled {
                    let mut tls_opts = serde_json::Map::new();
                    tls_opts.insert("enabled".to_string(), json!(true));
                    tls_opts.insert("server_name".to_string(), json!(sni));
                    tls_opts.insert("insecure".to_string(), json!(skip_cert_verify));
                    enrich_tls_options(&mut tls_opts, map);
                    outbound.insert("tls".to_string(), serde_json::Value::Object(tls_opts));
                }

                if let Some(trans) = get_transport_block(map) {
                    outbound.insert("transport".to_string(), trans);
                }
                if let Some(multiplex) = extract_multiplex_block(map) {
                    outbound.insert("multiplex".to_string(), multiplex);
                }
            }
            "hysteria" => {
                outbound.insert("type".to_string(), json!("hysteria"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));

                let auth = map
                    .get(&key_auth_str)
                    .and_then(|v| v.as_str())
                    .or_else(|| map.get(&key_auth_str_dash).and_then(|v| v.as_str()))
                    .or_else(|| map.get(&key_password).and_then(|v| v.as_str()))
                    .unwrap_or("");
                if auth.is_empty() {
                    continue;
                }
                outbound.insert("auth_str".to_string(), json!(auth));

                if let Some(obfs) =
                    yaml_string_any(map, &["obfs"]).filter(|value| !value.is_empty())
                {
                    outbound.insert("obfs".to_string(), json!(obfs));
                }
                if let Some(up_mbps) = yaml_mbps_any(map, &["up_mbps", "up-mbps", "up", "up-speed"])
                {
                    outbound.insert("up_mbps".to_string(), json!(up_mbps));
                } else if let Some(up) = yaml_value_any(map, &["up", "up-speed"])
                    && let Ok(value) = serde_json::to_value(up)
                {
                    outbound.insert("up".to_string(), value);
                }
                if let Some(down_mbps) =
                    yaml_mbps_any(map, &["down_mbps", "down-mbps", "down", "down-speed"])
                {
                    outbound.insert("down_mbps".to_string(), json!(down_mbps));
                } else if let Some(down) = yaml_value_any(map, &["down", "down-speed"])
                    && let Ok(value) = serde_json::to_value(down)
                {
                    outbound.insert("down".to_string(), value);
                }
                if let Some(server_ports) = parse_server_ports_value(map) {
                    outbound.insert("server_ports".to_string(), json!(server_ports));
                }
                if let Some(interval) = yaml_string_any(map, &["hop-interval", "hop_interval"])
                    .filter(|value| !value.is_empty())
                {
                    outbound.insert("hop_interval".to_string(), json!(interval));
                }

                let sni = extract_sni(map, &server);
                let mut tls_opts = serde_json::Map::new();
                tls_opts.insert("enabled".to_string(), json!(true));
                tls_opts.insert("server_name".to_string(), json!(sni));
                tls_opts.insert("insecure".to_string(), json!(skip_cert_verify));
                enrich_tls_options(&mut tls_opts, map);
                outbound.insert("tls".to_string(), serde_json::Value::Object(tls_opts));
            }
            "hysteria2" => {
                outbound.insert("type".to_string(), json!("hysteria2"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));

                let password = map
                    .get(&key_password)
                    .and_then(|v| v.as_str())
                    .or_else(|| map.get(&key_auth_str).and_then(|v| v.as_str()))
                    .unwrap_or("");
                if password.is_empty() {
                    continue;
                }
                outbound.insert("password".to_string(), json!(password));

                if let Some(up_mbps) = yaml_mbps_any(map, &["up_mbps", "up-mbps", "up"]) {
                    outbound.insert("up_mbps".to_string(), json!(up_mbps));
                }
                if let Some(down_mbps) = yaml_mbps_any(map, &["down_mbps", "down-mbps", "down"]) {
                    outbound.insert("down_mbps".to_string(), json!(down_mbps));
                }
                let (obfs_type, obfs_password) = if let Some(obfs_map) =
                    yaml_value_any(map, &["obfs"]).and_then(|v| v.as_mapping())
                {
                    let t = yaml_string_any(obfs_map, &["type"])
                        .unwrap_or("")
                        .to_string();
                    let p = yaml_string_any(obfs_map, &["password"])
                        .unwrap_or("")
                        .to_string();
                    (t, p)
                } else {
                    let t = yaml_string_any(map, &["obfs"]).unwrap_or("").to_string();
                    let p = yaml_string_any(map, &["obfs-password", "obfs_password"])
                        .unwrap_or("")
                        .to_string();
                    (t, p)
                };
                if !obfs_type.is_empty() && obfs_type != "none" {
                    outbound.insert(
                        "obfs".to_string(),
                        json!({ "type": obfs_type, "password": obfs_password }),
                    );
                }
                if let Some(server_ports) = parse_server_ports_value(map) {
                    outbound.insert("server_ports".to_string(), json!(server_ports));
                }
                if let Some(interval) = yaml_string_any(map, &["hop-interval", "hop_interval"])
                    .filter(|value| !value.is_empty())
                {
                    outbound.insert("hop_interval".to_string(), json!(interval));
                }

                let sni = extract_sni(map, &server);
                let mut tls_opts = serde_json::Map::new();
                tls_opts.insert("enabled".to_string(), json!(true));
                tls_opts.insert("server_name".to_string(), json!(sni));
                tls_opts.insert("insecure".to_string(), json!(skip_cert_verify));
                enrich_tls_options(&mut tls_opts, map);
                outbound.insert("tls".to_string(), serde_json::Value::Object(tls_opts));
            }
            "socks" | "socks5" => {
                outbound.insert("type".to_string(), json!("socks"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));

                let username = yaml_string_any(map, &["username", "user"]).unwrap_or("");
                let password = map
                    .get(&key_password)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if !username.is_empty() {
                    outbound.insert("username".to_string(), json!(username));
                }
                if !password.is_empty() {
                    outbound.insert("password".to_string(), json!(password));
                }
            }
            "http" => {
                outbound.insert("type".to_string(), json!("http"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));

                let username = yaml_string_any(map, &["username", "user"]).unwrap_or("");
                let password = map
                    .get(&key_password)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if !username.is_empty() {
                    outbound.insert("username".to_string(), json!(username));
                }
                if !password.is_empty() {
                    outbound.insert("password".to_string(), json!(password));
                }

                let tls_enabled = yaml_bool_any(map, &["tls"]).unwrap_or(false);
                let sni = extract_sni(map, &server);

                if tls_enabled {
                    let mut tls_opts = serde_json::Map::new();
                    tls_opts.insert("enabled".to_string(), json!(true));
                    tls_opts.insert("server_name".to_string(), json!(sni));
                    tls_opts.insert("insecure".to_string(), json!(skip_cert_verify));
                    enrich_tls_options(&mut tls_opts, map);
                    outbound.insert("tls".to_string(), serde_json::Value::Object(tls_opts));
                }
            }
            "tuic" => {
                outbound.insert("type".to_string(), json!("tuic"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));

                let uuid = map.get(&key_uuid).and_then(|v| v.as_str()).unwrap_or("");
                let password = map
                    .get(&key_password)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if uuid.is_empty() || password.is_empty() {
                    continue;
                }

                outbound.insert("uuid".to_string(), json!(uuid));
                if !password.is_empty() {
                    outbound.insert("password".to_string(), json!(password));
                }

                let congestion = yaml_string_any(
                    map,
                    &["congestion_control", "congestion-control", "congestion"],
                )
                .unwrap_or("cubic");
                outbound.insert("congestion_control".to_string(), json!(congestion));
                if let Some(mode) = yaml_string_any(map, &["udp_relay_mode", "udp-relay-mode"])
                    .filter(|value| !value.is_empty())
                {
                    outbound.insert("udp_relay_mode".to_string(), json!(mode));
                }
                if yaml_bool_any(map, &["udp_over_stream", "udp-over-stream"]).unwrap_or(false) {
                    outbound.insert("udp_over_stream".to_string(), json!(true));
                }
                if yaml_bool_any(
                    map,
                    &[
                        "zero_rtt_handshake",
                        "zero-rtt-handshake",
                        "reduce_rtt",
                        "reduce-rtt",
                    ],
                )
                .unwrap_or(false)
                {
                    outbound.insert("zero_rtt_handshake".to_string(), json!(true));
                }
                if let Some(hb_val) = yaml_value_any(map, &["heartbeat"]) {
                    if let Some(hb_num) = hb_val.as_u64() {
                        if hb_num > 0 {
                            outbound.insert("heartbeat".to_string(), json!(format!("{hb_num}ms")));
                        }
                    } else if let Some(hb_str) =
                        hb_val.as_str().map(str::trim).filter(|s| !s.is_empty())
                    {
                        if hb_str.chars().all(|c| c.is_ascii_digit()) {
                            outbound.insert("heartbeat".to_string(), json!(format!("{hb_str}ms")));
                        } else {
                            outbound.insert("heartbeat".to_string(), json!(hb_str));
                        }
                    }
                }

                let sni = extract_sni(map, &server);
                let mut tls_opts = serde_json::Map::new();
                tls_opts.insert("enabled".to_string(), json!(true));
                tls_opts.insert("server_name".to_string(), json!(sni));
                tls_opts.insert("insecure".to_string(), json!(skip_cert_verify));
                enrich_tls_options(&mut tls_opts, map);
                outbound.insert("tls".to_string(), serde_json::Value::Object(tls_opts));
            }
            "anytls" => {
                outbound.insert("type".to_string(), json!("anytls"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                let password = map
                    .get(&key_password)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if password.is_empty() {
                    continue;
                }
                outbound.insert("password".to_string(), json!(password));
                let sni = extract_sni(map, &server);
                let mut tls_opts = serde_json::Map::new();
                tls_opts.insert("enabled".to_string(), json!(true));
                tls_opts.insert("server_name".to_string(), json!(sni));
                tls_opts.insert("insecure".to_string(), json!(skip_cert_verify));
                enrich_tls_options(&mut tls_opts, map);
                outbound.insert("tls".to_string(), serde_json::Value::Object(tls_opts));
            }
            "naive" => {
                outbound.insert("type".to_string(), json!("naive"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                let username = yaml_string_any(map, &["username", "user"]).unwrap_or("");
                let password = map
                    .get(&key_password)
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                if username.is_empty() || password.is_empty() {
                    continue;
                }
                outbound.insert("username".to_string(), json!(username));
                outbound.insert("password".to_string(), json!(password));
                if let Some(concurrency) =
                    yaml_u64_any(map, &["insecure-concurrency", "insecure_concurrency"])
                {
                    outbound.insert("insecure_concurrency".to_string(), json!(concurrency));
                }
                let sni = extract_sni(map, &server);
                let mut tls_opts = serde_json::Map::new();
                tls_opts.insert("enabled".to_string(), json!(true));
                tls_opts.insert("server_name".to_string(), json!(sni));
                tls_opts.insert("insecure".to_string(), json!(skip_cert_verify));
                enrich_tls_options(&mut tls_opts, map);
                outbound.insert("tls".to_string(), serde_json::Value::Object(tls_opts));
            }
            "ssh" => {
                outbound.insert("type".to_string(), json!("ssh"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                let user = yaml_string_any(map, &["user", "username"]).unwrap_or("");
                if user.is_empty() {
                    continue;
                }
                outbound.insert("user".to_string(), json!(user));
                if let Some(password) = map
                    .get(&key_password)
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.is_empty())
                {
                    outbound.insert("password".to_string(), json!(password));
                }
                if let Some(private_key) = yaml_string_any(map, &["private-key", "private_key"])
                    .filter(|value| !value.is_empty())
                {
                    outbound.insert("private_key".to_string(), json!(private_key));
                }
                if let Some(path) = yaml_string_any(map, &["private-key-path", "private_key_path"])
                    .filter(|value| !value.is_empty())
                {
                    outbound.insert("private_key_path".to_string(), json!(path));
                }
                if let Some(passphrase) =
                    yaml_string_any(map, &["private-key-passphrase", "private_key_passphrase"])
                        .filter(|value| !value.is_empty())
                {
                    outbound.insert("private_key_passphrase".to_string(), json!(passphrase));
                }
                let host_keys = yaml_string_list_any(map, &["host-key", "host_key"]);
                if !host_keys.is_empty() {
                    outbound.insert("host_key".to_string(), json!(host_keys));
                }
                let host_key_algorithms =
                    yaml_string_list_any(map, &["host-key-algorithms", "host_key_algorithms"]);
                if !host_key_algorithms.is_empty() {
                    outbound.insert(
                        "host_key_algorithms".to_string(),
                        json!(host_key_algorithms),
                    );
                }
                if let Some(version) = yaml_string_any(map, &["client-version", "client_version"])
                    .filter(|value| !value.is_empty())
                {
                    outbound.insert("client_version".to_string(), json!(version));
                }
                if outbound.get("password").is_none()
                    && outbound.get("private_key").is_none()
                    && outbound.get("private_key_path").is_none()
                {
                    continue;
                }
            }
            "shadowtls" => {
                outbound.insert("type".to_string(), json!("shadowtls"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                let version = yaml_u64_any(map, &["version"]).unwrap_or(3);
                if !(1..=3).contains(&version) {
                    continue;
                }
                outbound.insert("version".to_string(), json!(version));
                let password = map
                    .get(&key_password)
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                if version >= 3 && password.is_empty() {
                    continue;
                }
                if !password.is_empty() {
                    outbound.insert("password".to_string(), json!(password));
                }
                let sni = extract_sni(map, &server);
                let mut tls_opts = serde_json::Map::new();
                tls_opts.insert("enabled".to_string(), json!(true));
                tls_opts.insert("server_name".to_string(), json!(sni));
                tls_opts.insert("insecure".to_string(), json!(skip_cert_verify));
                enrich_tls_options(&mut tls_opts, map);
                outbound.insert("tls".to_string(), serde_json::Value::Object(tls_opts));
            }
            "wireguard" => {
                let Some(private_key) = yaml_string_any(map, &["private-key", "private_key"])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                else {
                    continue;
                };

                let mut addresses: Vec<String> = Vec::new();
                for addr_item in yaml_string_list_any(map, &["address", "ip"]) {
                    let cidr = normalize_wg_cidr(&addr_item);
                    if !cidr.is_empty() && !addresses.contains(&cidr) {
                        addresses.push(cidr);
                    }
                }
                if let Some(ipv6) = yaml_string_any(map, &["ipv6"]) {
                    let cidr = normalize_wg_cidr(ipv6);
                    if !cidr.is_empty() && !addresses.contains(&cidr) {
                        addresses.push(cidr);
                    }
                }
                if addresses.is_empty() {
                    addresses.push("10.0.0.2/32".to_string());
                }

                let parse_reserved = |m: &serde_yaml::Mapping| -> Option<serde_json::Value> {
                    let val = yaml_value_any(m, &["reserved"])?;
                    if let Some(seq) = val.as_sequence() {
                        let bytes: Vec<u64> = seq.iter().filter_map(|x| x.as_u64()).collect();
                        (!bytes.is_empty()).then_some(json!(bytes))
                    } else if let Some(s) = val.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                        let inner = s.trim_start_matches('[').trim_end_matches(']');
                        let bytes: Vec<u64> = inner
                            .split(',')
                            .filter_map(|part| part.trim().parse::<u64>().ok())
                            .collect();
                        if !bytes.is_empty() {
                            Some(json!(bytes))
                        } else {
                            Some(json!(s))
                        }
                    } else {
                        None
                    }
                };

                let build_peer = |peer_map: &serde_yaml::Mapping,
                                  fallback_addr: Option<&str>,
                                  fallback_port: Option<u16>|
                 -> Option<serde_json::Value> {
                    let addr = yaml_string_any(peer_map, &["address", "server"])
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .or(fallback_addr)?;
                    let peer_port = yaml_u64_any(peer_map, &["port", "server_port"])
                        .and_then(|p| u16::try_from(p).ok())
                        .filter(|&p| p > 0)
                        .or(fallback_port)?;
                    let pub_key = yaml_string_any(peer_map, &["public-key", "public_key"])
                        .map(str::trim)
                        .filter(|s| !s.is_empty())?;

                    let mut peer = serde_json::Map::new();
                    peer.insert("address".to_string(), json!(addr));
                    peer.insert("port".to_string(), json!(peer_port));
                    peer.insert("public_key".to_string(), json!(pub_key));
                    if let Some(psk) = yaml_string_any(
                        peer_map,
                        &["pre-shared-key", "pre_shared_key", "preshared-key"],
                    )
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    {
                        peer.insert("pre_shared_key".to_string(), json!(psk));
                    }
                    let allowed = yaml_string_list_any(peer_map, &["allowed-ips", "allowed_ips"]);
                    peer.insert(
                        "allowed_ips".to_string(),
                        if allowed.is_empty() {
                            json!(["0.0.0.0/0", "::/0"])
                        } else {
                            json!(allowed)
                        },
                    );
                    if let Some(res) = parse_reserved(peer_map).or_else(|| parse_reserved(map)) {
                        peer.insert("reserved".to_string(), res);
                    }
                    Some(serde_json::Value::Object(peer))
                };

                let mut peers_json = Vec::new();
                if let Some(peers_seq) =
                    yaml_value_any(map, &["peers"]).and_then(|v| v.as_sequence())
                {
                    for p in peers_seq {
                        if let Some(pm) = p.as_mapping()
                            && let Some(peer_val) = build_peer(pm, Some(&server), Some(port))
                        {
                            peers_json.push(peer_val);
                        }
                    }
                }
                if peers_json.is_empty()
                    && let Some(peer_val) = build_peer(map, Some(&server), Some(port))
                {
                    peers_json.push(peer_val);
                }
                if peers_json.is_empty() {
                    continue;
                }

                let mut ep = serde_json::Map::new();
                ep.insert("type".to_string(), json!("wireguard"));
                ep.insert("address".to_string(), json!(addresses));
                ep.insert("private_key".to_string(), json!(private_key));
                ep.insert("peers".to_string(), json!(peers_json));
                if let Some(mtu) = yaml_u64_any(map, &["mtu"]).filter(|&m| m > 0) {
                    ep.insert("mtu".to_string(), json!(mtu));
                }
                wireguard_endpoint = Some(ep);
            }
            _ => {
                continue;
            }
        }

        let clean_name: String = name
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect::<String>()
            .trim()
            .to_string();
        let base_tag = if clean_name.is_empty() {
            "Unnamed".to_string()
        } else {
            clean_name
        };
        let mut tag = base_tag.clone();
        let mut suffix = 2usize;
        while !used_tags.insert(tag.clone()) {
            tag = format!("{base_tag} ({suffix})");
            suffix += 1;
        }
        node_tags.push(tag.clone());

        if let Some(mut ep) = wireguard_endpoint {
            ep.insert("tag".to_string(), json!(tag));
            endpoints.push(serde_json::Value::Object(ep));
        } else {
            if gui_config.tcp_fast_open {
                outbound.insert("tcp_fast_open".to_string(), json!(true));
            }
            if gui_config.tcp_multipath {
                outbound.insert("tcp_multipath".to_string(), json!(true));
            }
            outbound.insert("tag".to_string(), json!(tag));
            outbounds.push(serde_json::Value::Object(outbound));
        }
    }

    if outbounds.is_empty() && endpoints.is_empty() {
        let suffix = if unsupported_types.is_empty() {
            String::new()
        } else {
            format!(
                " Unsupported proxy types: {}.",
                unsupported_types.into_iter().collect::<Vec<_>>().join(", ")
            )
        };
        return Err(format!(
            "No supported proxy nodes could be converted.{suffix} Use a native sing-box JSON profile for endpoint-only or custom outbound types."
        ));
    }

    // Master Selector "Proxy"
    let mut proxy_selector = serde_json::Map::new();
    proxy_selector.insert("type".to_string(), json!("selector"));
    proxy_selector.insert("tag".to_string(), json!("Proxy"));
    let mut selector_outbounds = vec!["Auto".to_string()];
    selector_outbounds.extend(node_tags.clone());
    proxy_selector.insert("outbounds".to_string(), json!(selector_outbounds));

    // Remember selected node tag (Store Cache)
    let default_node = gui_config
        .selected_node_tag
        .as_ref()
        .filter(|tag| node_tags.contains(tag) || *tag == "Auto")
        .cloned()
        .unwrap_or_else(|| "Auto".to_string());
    proxy_selector.insert("default".to_string(), json!(default_node));

    // Master URLTest "Auto"
    let mut auto_urltest = serde_json::Map::new();
    auto_urltest.insert("type".to_string(), json!("urltest"));
    auto_urltest.insert("tag".to_string(), json!("Auto"));
    auto_urltest.insert("outbounds".to_string(), json!(node_tags));
    auto_urltest.insert(
        "url".to_string(),
        json!("http://cp.cloudflare.com/generate_204"),
    );
    auto_urltest.insert("interval".to_string(), json!("3m"));

    let mut final_outbounds = vec![
        serde_json::Value::Object(proxy_selector),
        serde_json::Value::Object(auto_urltest),
    ];
    final_outbounds.extend(outbounds);

    // Standard direct outbound (modern 1.13+ rules use `action: "reject"` instead of a block outbound)
    final_outbounds.push(json!({ "type": "direct", "tag": "direct" }));

    // Build modern 1.12+/1.13+ typed DNS servers
    let (local_dns_server, local_needs_bootstrap) = dns_address_to_server_json(
        "dns_local",
        &gui_config.dns_server_local,
        None,
        Some("dns_bootstrap"),
    );
    let (remote_dns_server, _) = dns_address_to_server_json(
        "dns_remote",
        &gui_config.dns_server_remote,
        Some("Proxy"),
        Some("dns_local"),
    );

    let mut dns_servers = Vec::new();
    if local_needs_bootstrap {
        dns_servers.push(json!({ "tag": "dns_bootstrap", "type": "local" }));
    }
    dns_servers.push(remote_dns_server);
    dns_servers.push(local_dns_server);

    let mut dns_rules = vec![json!({ "clash_mode": "Direct", "server": "dns_local" })];

    // Resolve custom bypass domains via local DNS (prevents leaks and improves CDN speed)
    if !gui_config.custom_bypass_domains.is_empty() {
        dns_rules.push(json!({
            "domain_suffix": gui_config.custom_bypass_domains,
            "server": "dns_local"
        }));
    }

    // Resolve custom proxy domains via Fake-IP (when enabled) or remote DNS
    if !gui_config.custom_proxy_domains.is_empty() {
        if gui_config.fake_ip {
            dns_rules.push(json!({
                "domain_suffix": gui_config.custom_proxy_domains,
                "query_type": ["A", "AAAA"],
                "server": "dns_fakeip"
            }));
            dns_rules.push(json!({
                "domain_suffix": gui_config.custom_proxy_domains,
                "server": "dns_remote"
            }));
        } else {
            dns_rules.push(json!({
                "domain_suffix": gui_config.custom_proxy_domains,
                "server": "dns_remote"
            }));
        }
    }

    if gui_config.fake_ip {
        dns_servers.push(json!({
            "tag": "dns_fakeip",
            "type": "fakeip",
            "inet4_range": "198.18.0.0/15",
            "inet6_range": "fc00::/18"
        }));
        // Global mode must override geosite-cn / direct rules cleanly.
        dns_rules.push(json!({
            "clash_mode": "Global",
            "query_type": ["A", "AAAA"],
            "server": "dns_fakeip"
        }));
        dns_rules.push(json!({ "clash_mode": "Global", "server": "dns_remote" }));
        dns_rules.push(json!({ "rule_set": ["geosite-cn"], "server": "dns_local" }));
        dns_rules.push(json!({ "query_type": ["A", "AAAA"], "server": "dns_fakeip" }));
    } else {
        dns_rules.push(json!({ "clash_mode": "Global", "server": "dns_remote" }));
        dns_rules.push(json!({ "rule_set": ["geosite-cn"], "server": "dns_local" }));
    }

    let dns = json!({
        "servers": dns_servers,
        "rules": dns_rules,
        "final": "dns_remote"
    });

    // Build Inbounds
    let mut inbounds = vec![json!({
        "type": "mixed",
        "tag": "mixed-in",
        "listen": "127.0.0.1",
        "listen_port": gui_config.mixed_port
    })];

    if gui_config.tun_mode {
        inbounds.push(default_tun_inbound());
    }

    // Build Rules — custom explicit bypass/proxy rules and LAN (`ip_is_private`) stay
    // aligned with DNS rules by running before `clash_mode: "Global"`, while
    // `geosite-cn` / `geoip-cn` stay after `clash_mode: "Global"`.
    let mut rules_list = vec![
        json!({ "action": "sniff", "sniffer": ["http", "tls", "quic", "dns"] }),
        json!({ "protocol": "dns", "action": "hijack-dns" }),
        json!({ "port": 53, "action": "hijack-dns" }),
        json!({ "clash_mode": "Direct", "outbound": "direct" }),
    ];

    if !gui_config.custom_bypass_domains.is_empty() {
        rules_list.push(json!({
            "domain_suffix": gui_config.custom_bypass_domains,
            "outbound": "direct"
        }));
    }
    if !gui_config.custom_proxy_domains.is_empty() {
        rules_list.push(json!({
            "domain_suffix": gui_config.custom_proxy_domains,
            "outbound": "Proxy"
        }));
    }
    if !gui_config.custom_bypass_ips.is_empty() {
        rules_list.push(json!({
            "ip_cidr": gui_config.custom_bypass_ips,
            "outbound": "direct"
        }));
    }
    if !gui_config.custom_proxy_ips.is_empty() {
        rules_list.push(json!({
            "ip_cidr": gui_config.custom_proxy_ips,
            "outbound": "Proxy"
        }));
    }

    rules_list.push(json!({ "ip_is_private": true, "outbound": "direct" }));
    rules_list.push(json!({ "clash_mode": "Global", "outbound": "Proxy" }));
    rules_list.push(json!({ "rule_set": ["geosite-cn", "geoip-cn"], "outbound": "direct" }));

    let rules = json!({
        "rules": rules_list,
        "rule_set": [
            {
                "tag": "geosite-cn",
                "type": "remote",
                "format": "binary",
                "url": "https://raw.githubusercontent.com/SagerNet/sing-geosite/rule-set/geosite-cn.srs",
                "download_detour": "direct"
            },
            {
                "tag": "geoip-cn",
                "type": "remote",
                "format": "binary",
                "url": "https://raw.githubusercontent.com/SagerNet/sing-geoip/rule-set/geoip-cn.srs",
                "download_detour": "direct"
            }
        ],
        "auto_detect_interface": true,
        "default_domain_resolver": "dns_local",
        "final": "Proxy"
    });

    let mut root = serde_json::Map::new();
    root.insert(
        "log".to_string(),
        json!({
            "level": "info",
            "timestamp": true
        }),
    );
    root.insert("dns".to_string(), dns);
    root.insert("inbounds".to_string(), serde_json::Value::Array(inbounds));
    if !endpoints.is_empty() {
        root.insert("endpoints".to_string(), serde_json::Value::Array(endpoints));
    }
    root.insert(
        "outbounds".to_string(),
        serde_json::Value::Array(final_outbounds),
    );
    root.insert("route".to_string(), rules);
    root.insert(
        "experimental".to_string(),
        json!({
            "clash_api": {
                "external_controller": format!("127.0.0.1:{}", gui_config.api_port),
                "secret": "",
                "default_mode": gui_config.routing_mode.as_clash_mode()
            }
        }),
    );

    Ok(serde_json::Value::Object(root))
}

/// Strip known-broken GitHub proxy frontends from remote rule-set URLs.
/// Subscriptions often wrap GitHub with `gh-proxy.com`, which can return 403
/// and crash the core on first rule-set init even when `sing-box check` passes.
pub fn rewrite_remote_rule_set_url(url: &str) -> String {
    const PREFIXES: &[&str] = &[
        "https://gh-proxy.com/",
        "http://gh-proxy.com/",
        "https://mirror.ghproxy.com/",
        "http://mirror.ghproxy.com/",
        "https://ghproxy.com/",
        "http://ghproxy.com/",
        "https://ghproxy.net/",
        "http://ghproxy.net/",
        "https://gh.ddlc.top/",
        "http://gh.ddlc.top/",
    ];
    let mut current = url.trim().to_string();
    // Nested prefixes are uncommon but cheap to peel.
    for _ in 0..3 {
        let mut stripped = false;
        for prefix in PREFIXES {
            if let Some(rest) = current.strip_prefix(prefix)
                && (rest.starts_with("http://") || rest.starts_with("https://"))
            {
                current = rest.to_string();
                stripped = true;
                break;
            }
        }
        if !stripped {
            break;
        }
    }
    if let Some(rest) = current.strip_prefix("https://raw.githubusercontent.com/") {
        let parts: Vec<&str> = rest.splitn(4, '/').collect();
        let rewritten = if parts.len() == 4 {
            if parts[2] == "refs" {
                // GitHub's UI emits fully-qualified refs (`refs/heads/<branch>`
                // / `refs/tags/<tag>`); jsDelivr only accepts the bare ref
                // name, so peel the prefix. Unknown refs forms stay untouched
                // rather than being rewritten into a guaranteed 404.
                parts[3]
                    .strip_prefix("heads/")
                    .or_else(|| parts[3].strip_prefix("tags/"))
                    .and_then(|ref_and_path| ref_and_path.split_once('/'))
                    .map(|(version, path)| {
                        format!(
                            "https://testingcf.jsdelivr.net/gh/{}/{}@{}/{}",
                            parts[0], parts[1], version, path
                        )
                    })
            } else {
                Some(format!(
                    "https://testingcf.jsdelivr.net/gh/{}/{}@{}/{}",
                    parts[0], parts[1], parts[2], parts[3]
                ))
            }
        } else {
            None
        };
        if let Some(new_url) = rewritten {
            current = new_url;
        }
    }
    current
}

/// Collect outbound tags present in the config.
pub fn collect_outbound_tags(config: &serde_json::Value) -> Vec<String> {
    config
        .get("outbounds")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|o| o.get("tag").and_then(|t| t.as_str()).map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Resolve a usable direct outbound tag for rule-set downloads.
/// Prefers an existing `type: direct` outbound (tag `direct` / `DIRECT` first),
/// otherwise inserts `{ "type": "direct", "tag": "direct" }` and returns `"direct"`.
pub fn ensure_direct_outbound_tag(config: &mut serde_json::Value) -> String {
    let mut preferred: Option<String> = None;
    let mut any_direct: Option<String> = None;

    if let Some(arr) = config.get("outbounds").and_then(|v| v.as_array()) {
        for o in arr {
            let typ = o.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if typ != "direct" {
                continue;
            }
            let tag = match o.get("tag").and_then(|t| t.as_str()) {
                Some(t) if !t.is_empty() => t.to_string(),
                _ => continue,
            };
            if tag.eq_ignore_ascii_case("direct") {
                preferred = Some(tag);
                break;
            }
            if any_direct.is_none() {
                any_direct = Some(tag);
            }
        }
    }

    if let Some(tag) = preferred.or(any_direct) {
        return tag;
    }

    // No direct outbound in profile — inject a standard one (Clash path already has it).
    let existing_tags = collect_outbound_tags(config);
    let mut candidate = "direct".to_string();
    let mut suffix = 2usize;
    while existing_tags.iter().any(|t| t == &candidate) {
        candidate = format!("direct-{}", suffix);
        suffix += 1;
    }
    let outbound = json!({ "type": "direct", "tag": &candidate });
    match config.get_mut("outbounds") {
        Some(serde_json::Value::Array(arr)) => arr.push(outbound),
        _ => {
            if let Some(root) = config.as_object_mut() {
                root.insert("outbounds".to_string(), json!([outbound]));
            }
        }
    }
    candidate
}

fn fnv1a_64_feed(hash: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *hash ^= u64::from(b);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    *hash ^= 0xff;
    *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
}

fn profile_cache_db_path(config: &serde_json::Value) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    if let Some(outbounds) = config.get("outbounds").and_then(|v| v.as_array()) {
        for ob in outbounds {
            let typ = ob.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if matches!(typ, "selector" | "urltest" | "direct" | "block" | "dns") {
                continue;
            }
            let tag = ob.get("tag").and_then(|v| v.as_str()).unwrap_or("");
            let server = ob.get("server").and_then(|v| v.as_str()).unwrap_or("");
            fnv1a_64_feed(&mut hash, typ.as_bytes());
            fnv1a_64_feed(&mut hash, tag.as_bytes());
            fnv1a_64_feed(&mut hash, server.as_bytes());
        }
    }
    if let Some(endpoints) = config.get("endpoints").and_then(|v| v.as_array()) {
        for ep in endpoints {
            let typ = ep.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let tag = ep.get("tag").and_then(|v| v.as_str()).unwrap_or("");
            fnv1a_64_feed(&mut hash, typ.as_bytes());
            fnv1a_64_feed(&mut hash, tag.as_bytes());
        }
    }
    if let Some(rule_sets) = config.pointer("/route/rule_set").and_then(|v| v.as_array()) {
        for rs in rule_sets {
            let tag = rs.get("tag").and_then(|v| v.as_str()).unwrap_or("");
            let url = rs.get("url").and_then(|v| v.as_str()).unwrap_or("");
            fnv1a_64_feed(&mut hash, tag.as_bytes());
            fnv1a_64_feed(&mut hash, url.as_bytes());
        }
    }
    get_app_dir()
        .join(format!("cache_{hash:016x}_cache.db"))
        .to_string_lossy()
        .to_string()
}

/// Apply in-app startup mitigations to a generated sing-box config:
/// - rewrite broken remote rule-set proxy URLs
/// - set `download_detour` to a **real** direct outbound tag (profiles often use `DIRECT`)
/// - pin an isolated per-profile `cache_file.path` under the app data directory
/// - enable `store_fakeip` when a Fake-IP DNS server is configured
pub fn mitigate_run_config(config: &mut serde_json::Value) {
    // Resolve / inject direct outbound first so rule-set detours reference a real tag.
    let direct_tag = ensure_direct_outbound_tag(config);
    let outbound_tags = collect_outbound_tags(config);

    // 1) Remote rule-set URL / detour fixes
    if let Some(rule_sets) = config
        .pointer_mut("/route/rule_set")
        .and_then(|v| v.as_array_mut())
    {
        for entry in rule_sets.iter_mut() {
            let Some(obj) = entry.as_object_mut() else {
                continue;
            };
            let is_remote = obj
                .get("type")
                .and_then(|t| t.as_str())
                .map(|t| t == "remote")
                .unwrap_or(false);
            if !is_remote {
                continue;
            }
            if let Some(url) = obj.get("url").and_then(|u| u.as_str()) {
                let rewritten = rewrite_remote_rule_set_url(url);
                if rewritten != url {
                    obj.insert("url".to_string(), json!(rewritten));
                }
            }
            // Initial rule-set download must work before any proxy is up.
            // Use an outbound tag that actually exists (native profiles often use "DIRECT").
            let detour_ok = obj
                .get("download_detour")
                .and_then(|d| d.as_str())
                .map(|d| outbound_tags.iter().any(|t| t == d))
                .unwrap_or(false);
            if !detour_ok {
                obj.insert("download_detour".to_string(), json!(direct_tag.clone()));
            }
        }
    }

    // 2) Per-profile isolated cache_file path under app data dir + Fake-IP persistence
    let has_fakeip = config
        .pointer("/dns/servers")
        .and_then(|v| v.as_array())
        .is_some_and(|servers| {
            servers.iter().any(|server| {
                server.get("type").and_then(|t| t.as_str()) == Some("fakeip")
                    || server.get("address").and_then(|a| a.as_str()) == Some("fakeip")
            })
        });
    let cache_path = profile_cache_db_path(config);
    if let Some(root) = config.as_object_mut()
        && let Some(exp) = root
            .entry("experimental")
            .or_insert_with(|| json!({}))
            .as_object_mut()
    {
        let cache = exp
            .entry("cache_file")
            .or_insert_with(|| json!({ "enabled": true }));
        if let Some(cache_obj) = cache.as_object_mut() {
            cache_obj.entry("enabled").or_insert_with(|| json!(true));
            cache_obj.insert("path".to_string(), json!(cache_path));
            if has_fakeip {
                cache_obj
                    .entry("store_fakeip")
                    .or_insert_with(|| json!(true));
            }
        }
    }
}

fn default_tun_inbound() -> serde_json::Value {
    #[allow(unused_mut)]
    let mut inbound = json!({
        "type": "tun", "tag": "tun-in", "address": ["172.19.0.1/30"],
        "auto_route": true, "strict_route": true, "stack": "system"
    });
    #[cfg(not(target_os = "macos"))]
    {
        inbound["interface_name"] = json!("singbox-tun");
    }
    inbound
}

/// Report GUI settings that are ignored because a native sing-box JSON profile
/// manages its own routing, DNS, or dial options directly.
#[allow(dead_code)] // Exported for Wave 2 main/UI wiring
pub fn native_json_ignored_settings(raw: &str, config: &GuiConfig) -> Vec<&'static str> {
    if !is_native_json_profile(raw) {
        return Vec::new();
    }
    let normalized = normalize_profile_content(raw);
    if serde_json::from_str::<serde_json::Value>(strip_bom_trimmed(&normalized)).is_err() {
        return Vec::new();
    }
    let defaults = GuiConfig::default();
    let mut ignored = Vec::new();
    if !config.custom_bypass_domains.is_empty()
        || !config.custom_proxy_domains.is_empty()
        || !config.custom_bypass_ips.is_empty()
        || !config.custom_proxy_ips.is_empty()
    {
        ignored.push("Custom Rules");
    }
    if config.fake_ip
        || config.dns_server_remote != defaults.dns_server_remote
        || config.dns_server_local != defaults.dns_server_local
    {
        ignored.push("DNS / Fake-IP");
    }
    if config.tcp_fast_open {
        ignored.push("TCP Fast Open");
    }
    if config.tcp_multipath {
        ignored.push("TCP Multipath");
    }
    ignored
}

pub fn merge_native_json_profile(
    json_content: &str,
    gui_config: &GuiConfig,
) -> Result<serde_json::Value, String> {
    let normalized = normalize_profile_content(json_content);
    let mut config: serde_json::Value = serde_json::from_str(strip_bom_trimmed(&normalized))
        .map_err(|e| format!("Invalid native Sing-Box JSON: {}", e))?;

    let config_obj = config
        .as_object_mut()
        .ok_or_else(|| "Native Sing-Box JSON must be a JSON object".to_string())?;

    // 1. Inbounds setup
    let inbounds = config_obj
        .get_mut("inbounds")
        .and_then(|v| v.as_array_mut());

    let mut mixed_found = false;
    let mut tun_found = false;

    if let Some(arr) = inbounds {
        for val in arr.iter_mut() {
            if let Some(obj) = val.as_object_mut()
                && let Some(t) = obj.get("type").and_then(|t| t.as_str())
            {
                if t == "mixed" && !mixed_found {
                    // System proxy / local clients connect via 127.0.0.1; ensure
                    // the mixed inbound listens on an IPv4 loopback-reachable address.
                    let listen_ok = obj
                        .get("listen")
                        .and_then(|v| v.as_str())
                        .is_some_and(|addr| addr == "127.0.0.1" || addr == "0.0.0.0");
                    if !listen_ok {
                        obj.insert("listen".to_string(), json!("127.0.0.1"));
                    }
                    obj.insert("listen_port".to_string(), json!(gui_config.mixed_port));
                    mixed_found = true;
                } else if t == "tun" {
                    #[cfg(not(target_os = "macos"))]
                    obj.entry("interface_name".to_string())
                        .or_insert(json!("singbox-tun"));
                    obj.entry("auto_route".to_string()).or_insert(json!(true));
                    obj.entry("strict_route".to_string()).or_insert(json!(true));
                    obj.entry("stack".to_string()).or_insert(json!("system"));
                    tun_found = true;
                }
            }
        }

        // Add if not found
        if !mixed_found {
            arr.push(json!({
                "type": "mixed",
                "tag": "mixed-in",
                "listen": "127.0.0.1",
                "listen_port": gui_config.mixed_port
            }));
        }
        if gui_config.tun_mode && !tun_found {
            arr.push(default_tun_inbound());
        } else if !gui_config.tun_mode && tun_found {
            arr.retain(|val| val.get("type").and_then(|t| t.as_str()) != Some("tun"));
        }
    } else {
        let mut arr = vec![json!({
            "type": "mixed",
            "tag": "mixed-in",
            "listen": "127.0.0.1",
            "listen_port": gui_config.mixed_port
        })];
        if gui_config.tun_mode {
            arr.push(default_tun_inbound());
        }
        config_obj.insert("inbounds".to_string(), serde_json::Value::Array(arr));
    }

    // 2. Apply the remembered node to the first selector that contains it.
    // Native profiles commonly use a selector name other than `Proxy`, so the
    // membership check is more reliable than hard-coding a tag.
    if let Some(selected) = gui_config.selected_node_tag.as_deref()
        && let Some(outbounds) = config_obj
            .get_mut("outbounds")
            .and_then(|v| v.as_array_mut())
    {
        for outbound in outbounds {
            let Some(obj) = outbound.as_object_mut() else {
                continue;
            };
            if obj.get("type").and_then(|v| v.as_str()) != Some("selector") {
                continue;
            }
            let contains_selected = obj
                .get("outbounds")
                .and_then(|v| v.as_array())
                .is_some_and(|items| items.iter().any(|v| v.as_str() == Some(selected)));
            if contains_selected {
                obj.insert("default".to_string(), json!(selected));
                break;
            }
        }
    }

    // 3. Clash API / experimental settings setup
    let default_mode = gui_config.routing_mode.as_clash_mode();
    let experimental = config_obj
        .get_mut("experimental")
        .and_then(|v| v.as_object_mut());

    if let Some(exp_obj) = experimental {
        if let Some(clash_api_val) = exp_obj.get_mut("clash_api") {
            if let Some(clash_api_obj) = clash_api_val.as_object_mut() {
                clash_api_obj.insert(
                    "external_controller".to_string(),
                    json!(format!("127.0.0.1:{}", gui_config.api_port)),
                );
                clash_api_obj.insert("default_mode".to_string(), json!(default_mode));
            } else {
                // `"clash_api": null` (or any non-object) would otherwise be
                // kept verbatim, leaving the core without an API endpoint
                // while the GUI polls it forever.
                *clash_api_val = json!({
                    "external_controller": format!("127.0.0.1:{}", gui_config.api_port),
                    "secret": "",
                    "default_mode": default_mode
                });
            }
        } else {
            exp_obj.insert(
                "clash_api".to_string(),
                json!({
                    "external_controller": format!("127.0.0.1:{}", gui_config.api_port),
                    "secret": "",
                    "default_mode": default_mode
                }),
            );
        }
    } else {
        config_obj.insert(
            "experimental".to_string(),
            json!({
                "clash_api": {
                    "external_controller": format!("127.0.0.1:{}", gui_config.api_port),
                    "secret": "",
                    "default_mode": default_mode
                }
            }),
        );
    }

    Ok(config)
}

/// Read the `experimental.clash_api.secret` value (if any) from the on-disk
/// `run_config.json` produced by `prepare_run_config`. Lets `api.rs` request
/// handlers (proxies / connections / mode) attach the `Authorization: Bearer
/// <secret>` header when a native profile was authored with a custom secret.
fn read_clash_api_secret_from_path(
    path: &std::path::Path,
    cache: &std::sync::Mutex<(Vec<u8>, String)>,
) -> String {
    let Ok(bytes) = std::fs::read(path) else {
        return String::new();
    };
    let mut g = cache.lock().unwrap_or_else(|e| e.into_inner());
    if !g.0.is_empty() && g.0 == bytes {
        return g.1.clone();
    }
    let secret = serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|v| {
            v.pointer("/experimental/clash_api/secret")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();
    *g = (bytes, secret.clone());
    secret
}

pub fn get_clash_api_secret() -> String {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<(Vec<u8>, String)>> =
        std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new((Vec::new(), String::new())));
    let path = get_app_dir().join("run_config.json");
    read_clash_api_secret_from_path(&path, cache)
}

pub fn generate_preview_config(gui_config: &GuiConfig) -> String {
    let active_id = match &gui_config.active_profile_id {
        Some(id) => id,
        None => return "No active profile selected.".to_string(),
    };
    let path = get_profile_path(active_id);
    if !path.exists() {
        return "Active profile configuration file not found.".to_string();
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => return format!("Failed to read profile: {}", e),
    };
    let normalized = normalize_profile_content(&content);
    let trimmed = strip_bom_trimmed(&normalized);
    let res = if trimmed.starts_with('{') || trimmed.starts_with('[') {
        merge_native_json_profile(trimmed, gui_config)
    } else {
        convert_clash_to_singbox(trimmed, gui_config)
    };
    match res {
        Ok(mut val) => {
            mitigate_run_config(&mut val);
            redact_sensitive_json(&mut val);
            match serde_json::to_string_pretty(&val) {
                Ok(json_str) => json_str,
                Err(e) => format!("Failed to serialize config: {}", e),
            }
        }
        Err(e) => format!("Failed to generate preview: {}", e),
    }
}

fn is_sensitive_json_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase().replace(['-', ' '], "_");
    key.contains("secret")
        || key.contains("token")
        || key.contains("password")
        || key.contains("passwd")
        || key.contains("private_key")
        || key.contains("passphrase")
        || key.contains("credential")
        || key.contains("auth")
        || key.contains("uuid")
        || key == "key"
        || key == "psk"
}

pub(crate) fn redact_sensitive_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if is_sensitive_json_key(key) {
                    *child = serde_json::Value::String("******".to_string());
                } else {
                    redact_sensitive_json(child);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                redact_sensitive_json(item);
            }
        }
        serde_json::Value::String(text) => {
            if let Ok(mut parsed) = url::Url::parse(text)
                && matches!(parsed.scheme(), "http" | "https")
            {
                let mut changed = false;
                if parsed.password().is_some() {
                    let _ = parsed.set_password(Some("******"));
                    changed = true;
                }
                let pairs: Vec<(String, String)> = parsed
                    .query_pairs()
                    .map(|(key, value)| {
                        if is_sensitive_json_key(&key) {
                            changed = true;
                            (key.into_owned(), "******".to_string())
                        } else {
                            (key.into_owned(), value.into_owned())
                        }
                    })
                    .collect();
                if !pairs.is_empty() {
                    let mut query = parsed.query_pairs_mut();
                    query.clear();
                    for (key, value) in pairs {
                        query.append_pair(&key, &value);
                    }
                }
                if let Some(fragment) = parsed.fragment().map(str::to_string)
                    && (is_sensitive_json_key(&fragment) || fragment.len() > 32)
                {
                    parsed.set_fragment(Some("******"));
                    changed = true;
                }
                if changed {
                    *text = parsed.to_string();
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::RoutingMode;
    use base64::Engine;

    #[test]
    fn generated_tun_uses_a_platform_valid_interface_name() {
        let config = GuiConfig {
            tun_mode: true,
            ..GuiConfig::default()
        };
        let merged = merge_native_json_profile(
            r#"{"outbounds":[{"type":"direct","tag":"direct"}]}"#,
            &config,
        )
        .unwrap();
        let tun = merged["inbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|inbound| inbound["type"] == "tun")
            .unwrap();
        assert_eq!(tun["auto_route"], true);
        #[cfg(target_os = "macos")]
        assert!(
            tun.get("interface_name").is_none(),
            "macOS must allocate its own utunN interface"
        );
        #[cfg(not(target_os = "macos"))]
        assert_eq!(tun["interface_name"], "singbox-tun");
        let native = merge_native_json_profile(r#"{"inbounds":[{"type":"tun","interface_name":"utun123"}],"outbounds":[{"type":"direct","tag":"direct"}]}"#, &config).unwrap();
        assert_eq!(native["inbounds"][0]["interface_name"], "utun123");
    }

    #[test]
    fn test_parse_subscription_userinfo() {
        let (u, d, t, e) =
            parse_subscription_userinfo("upload=100; download=200; total=1000; expire=1710000000");
        assert_eq!(u, Some(100));
        assert_eq!(d, Some(200));
        assert_eq!(t, Some(1000));
        assert_eq!(e, Some(1710000000));
    }

    #[test]
    fn test_rewrite_remote_rule_set_url_strips_gh_proxy() {
        let input = "https://gh-proxy.com/https://github.com/MetaCubeX/meta-rules-dat/raw/refs/heads/sing/geo/geoip/telegram.srs";
        let out = rewrite_remote_rule_set_url(input);
        assert_eq!(
            out,
            "https://github.com/MetaCubeX/meta-rules-dat/raw/refs/heads/sing/geo/geoip/telegram.srs"
        );
        // Unproxied URLs are left alone.
        assert_eq!(
            rewrite_remote_rule_set_url("https://cdn.jsdelivr.net/gh/x@y/z.srs"),
            "https://cdn.jsdelivr.net/gh/x@y/z.srs"
        );
    }

    #[test]
    fn rewrite_raw_github_refs_urls_use_bare_ref_for_jsdelivr() {
        // Plain branch form.
        assert_eq!(
            rewrite_remote_rule_set_url("https://raw.githubusercontent.com/o/r/main/f.srs"),
            "https://testingcf.jsdelivr.net/gh/o/r@main/f.srs"
        );
        // Fully-qualified refs/heads form emitted by GitHub's UI.
        assert_eq!(
            rewrite_remote_rule_set_url(
                "https://raw.githubusercontent.com/SagerNet/sing-geosite/refs/heads/rule-set/geosite-cn.srs"
            ),
            "https://testingcf.jsdelivr.net/gh/SagerNet/sing-geosite@rule-set/geosite-cn.srs"
        );
        assert_eq!(
            rewrite_remote_rule_set_url(
                "https://raw.githubusercontent.com/o/r/refs/tags/v1.0/f.srs"
            ),
            "https://testingcf.jsdelivr.net/gh/o/r@v1.0/f.srs"
        );
        // Unknown refs forms must not be rewritten into a guaranteed 404.
        assert_eq!(
            rewrite_remote_rule_set_url("https://raw.githubusercontent.com/o/r/refs/weird"),
            "https://raw.githubusercontent.com/o/r/refs/weird"
        );
    }

    #[test]
    fn node_names_matching_reserved_tags_get_suffixed() {
        let yaml = r#"
proxies:
  - {name: direct, type: ss, server: a.com, port: 443, cipher: aes-128-gcm, password: x}
  - {name: Auto, type: trojan, server: b.com, port: 443, password: y}
"#;
        let config = convert_clash_to_singbox(yaml, &GuiConfig::default()).unwrap();
        let tags = collect_outbound_tags(&config);
        let unique: BTreeSet<&String> = tags.iter().collect();
        assert_eq!(
            tags.len(),
            unique.len(),
            "duplicate outbound tags: {tags:?}"
        );
        assert!(tags.iter().any(|tag| tag == "direct (2)"), "tags: {tags:?}");
        assert!(tags.iter().any(|tag| tag == "Auto (2)"), "tags: {tags:?}");

        // The UI node parser must produce the same display names.
        let nodes = parse_clash_yaml_nodes(yaml).unwrap();
        let names: Vec<&str> = nodes.iter().map(|node| node.name.as_str()).collect();
        assert_eq!(names, ["direct (2)", "Auto (2)"]);
    }

    #[test]
    fn merge_native_json_profile_replaces_non_object_clash_api() {
        let profile = r#"{
            "outbounds": [{"type": "direct", "tag": "direct"}],
            "experimental": {"clash_api": null}
        }"#;
        let gui_config = GuiConfig::default();
        let merged = merge_native_json_profile(profile, &gui_config).unwrap();
        assert_eq!(
            merged["experimental"]["clash_api"]["external_controller"].as_str(),
            Some(format!("127.0.0.1:{}", gui_config.api_port).as_str())
        );
    }

    #[test]
    fn preview_redaction_masks_sensitive_keys_and_url_credentials() {
        let mut value = serde_json::json!({
            "password": "secret",
            "nested": {"private_key": "pem"},
            "url": "https://user:pass@example.com/sub?token=abc&x=1#access-token-value"
        });
        redact_sensitive_json(&mut value);
        assert_eq!(value["password"], "******");
        assert_eq!(value["nested"]["private_key"], "******");
        let url = value["url"].as_str().unwrap();
        assert!(url.contains("user:******@example.com"));
        assert!(url.contains("token=******"));
        assert!(url.ends_with("#******"), "url={url}");
    }

    #[test]
    fn test_mitigate_run_config_rewrites_rule_sets_and_cache_path() {
        let mut config = json!({
            "route": {
                "rule_set": [
                    {
                        "type": "remote",
                        "tag": "telegram-ip",
                        "format": "binary",
                        "url": "https://gh-proxy.com/https://github.com/MetaCubeX/meta-rules-dat/raw/refs/heads/sing/geo/geoip/telegram.srs"
                    },
                    {
                        "type": "local",
                        "tag": "local-rs",
                        "path": "rules.srs"
                    }
                ]
            },
            "experimental": {
                "cache_file": { "enabled": true, "store_fakeip": true }
            }
        });
        mitigate_run_config(&mut config);
        let url = config["route"]["rule_set"][0]["url"].as_str().unwrap();
        assert!(
            url.starts_with("https://github.com/"),
            "expected github origin, got {url}"
        );
        assert!(!url.contains("gh-proxy.com"));
        // No outbounds → inject type=direct tag=direct
        assert_eq!(
            config["route"]["rule_set"][0]["download_detour"].as_str(),
            Some("direct")
        );
        assert!(
            config["outbounds"].as_array().unwrap().iter().any(|o| o
                .get("tag")
                .and_then(|t| t.as_str())
                == Some("direct")
                && o.get("type").and_then(|t| t.as_str()) == Some("direct")),
            "should inject direct outbound"
        );
        // Local rule-sets are untouched.
        assert_eq!(config["route"]["rule_set"][1]["tag"], "local-rs");
        let path = config["experimental"]["cache_file"]["path"]
            .as_str()
            .expect("cache path set");
        assert!(
            path.ends_with("cache.db"),
            "cache path should end with cache.db: {path}"
        );
        assert!(
            path.contains("sing-box-gui"),
            "cache path should live under app dir: {path}"
        );
    }

    #[test]
    fn test_mitigate_uses_existing_direct_outbound_tag() {
        // Mirrors native subscription profiles that tag the direct outbound "DIRECT".
        let mut config = json!({
            "outbounds": [
                { "type": "selector", "tag": "proxy", "outbounds": ["US"] },
                { "type": "direct", "tag": "DIRECT" },
                { "type": "vless", "tag": "US", "server": "1.2.3.4", "server_port": 443, "uuid": "00000000-0000-0000-0000-000000000000" }
            ],
            "route": {
                "rule_set": [
                    {
                        "type": "remote",
                        "tag": "telegram-ip",
                        "format": "binary",
                        "url": "https://gh-proxy.com/https://github.com/MetaCubeX/meta-rules-dat/raw/refs/heads/sing/geo/geoip/telegram.srs"
                    }
                ]
            },
            "experimental": {
                "cache_file": { "enabled": true }
            }
        });
        mitigate_run_config(&mut config);
        assert_eq!(
            config["route"]["rule_set"][0]["download_detour"].as_str(),
            Some("DIRECT"),
            "must use existing DIRECT tag, not lowercase 'direct'"
        );
        // Must not inject a second direct outbound when DIRECT already exists.
        let direct_count = config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| o.get("type").and_then(|t| t.as_str()) == Some("direct"))
            .count();
        assert_eq!(direct_count, 1);
        // Bad hardcoded "direct" detour should be rewritten to DIRECT.
        config["route"]["rule_set"][0]["download_detour"] = json!("direct");
        mitigate_run_config(&mut config);
        assert_eq!(
            config["route"]["rule_set"][0]["download_detour"].as_str(),
            Some("DIRECT")
        );
    }

    #[test]
    fn test_mitigate_full_path_matches_active_profile_shape() {
        // Optional end-to-end check for developers. Never read the user's real
        // application data or launch their installed core during normal tests.
        let Some(src) =
            std::env::var_os("SING_BOX_GUI_INTEGRATION_CONFIG").map(std::path::PathBuf::from)
        else {
            // Hermetic unit-shape tests above cover mitigation in normal CI.
            return;
        };
        let Some(core) =
            std::env::var_os("SING_BOX_GUI_INTEGRATION_CORE").map(std::path::PathBuf::from)
        else {
            return;
        };
        if !src.is_file() || !core.is_file() {
            return;
        }
        let raw = std::fs::read_to_string(&src).expect("read run_config");
        let mut config: serde_json::Value = serde_json::from_str(&raw).expect("parse run_config");
        mitigate_run_config(&mut config);

        let detour = config["route"]["rule_set"][0]["download_detour"]
            .as_str()
            .expect("download_detour set");
        let tags = collect_outbound_tags(&config);
        assert!(
            tags.iter().any(|t| t == detour),
            "download_detour `{detour}` must exist in outbounds {tags:?}"
        );
        assert_ne!(
            detour, "direct",
            "active profile uses DIRECT; detour must not be the missing lowercase tag"
        );
        assert_eq!(detour, "DIRECT");

        let url = config["route"]["rule_set"][0]["url"].as_str().unwrap_or("");
        assert!(!url.contains("gh-proxy.com"), "url still proxied: {url}");

        // Persist full mitigate output for start-verify (same JSON start_core would write).
        let scratch = std::env::temp_dir().join("sing-box-gui-integration-test");
        let _ = std::fs::create_dir_all(&scratch);
        let out_path = scratch.join("run_config_mitigated_full.json");
        let pretty = serde_json::to_string_pretty(&config).unwrap();
        std::fs::write(&out_path, &pretty).expect("write mitigated config");

        // Explicit integration core: require `sing-box check` + short `run` survival.
        let check = std::process::Command::new(&core)
            .args(["check", "-c", &out_path.to_string_lossy()])
            .output()
            .expect("run check");
        assert!(
            check.status.success(),
            "sing-box check failed: {}",
            String::from_utf8_lossy(&check.stderr)
        );

        let workdir = src.parent().unwrap_or_else(|| std::path::Path::new("."));
        let mut child = std::process::Command::new(&core)
            .args(["run", "-c", &out_path.to_string_lossy()])
            .current_dir(workdir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("spawn sing-box run");
        std::thread::sleep(std::time::Duration::from_secs(5));
        match child.try_wait() {
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
            }
            Ok(Some(status)) => {
                let mut err = String::new();
                if let Some(mut e) = child.stderr.take() {
                    use std::io::Read;
                    let _ = e.read_to_string(&mut err);
                }
                let tail = if err.len() > 2000 {
                    &err[err.len() - 2000..]
                } else {
                    &err
                };
                panic!("core exited during startup with {status}; stderr tail:\n{tail}");
            }
            Err(e) => panic!("try_wait failed: {e}"),
        }
    }

    #[test]
    fn test_default_mode_in_generated_config() {
        let gui_config = GuiConfig {
            routing_mode: RoutingMode::Global,
            ..GuiConfig::default()
        };
        let clash_yaml = r#"
proxies:
  - name: "test-node"
    type: ss
    server: 1.2.3.4
    port: 443
    cipher: aes-256-gcm
    password: "pass"
"#;
        let res = convert_clash_to_singbox(clash_yaml, &gui_config).unwrap();
        let mode = res
            .get("experimental")
            .and_then(|e| e.get("clash_api"))
            .and_then(|c| c.get("default_mode"))
            .and_then(|m| m.as_str());
        assert_eq!(mode, Some("Global"));
    }

    #[test]
    fn test_parse_clash_yaml() {
        let yaml = r#"
proxies:
  - name: "ss-node"
    type: ss
    server: 1.2.3.4
    port: 443
    cipher: aes-256-gcm
    password: "pass"
  - name: "vmess-node"
    type: vmess
    server: 5.6.7.8
    port: 10086
    uuid: "some-uuid"
        "#;

        let nodes = parse_clash_yaml_nodes(yaml).unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].name, "ss-node");
        assert_eq!(nodes[0].node_type, "ss");
        assert_eq!(nodes[0].server, "1.2.3.4");
        assert_eq!(nodes[0].port, 443);

        assert_eq!(nodes[1].name, "vmess-node");
        assert_eq!(nodes[1].node_type, "vmess");
        assert_eq!(nodes[1].server, "5.6.7.8");
        assert_eq!(nodes[1].port, 10086);
    }

    #[test]
    fn test_parse_share_links() {
        let vmess_link = "vmess://eyJ2IjoiMiIsInBzIjoidGVzdC12bWVzcyIsImFkZCI6IjEuMS4xLjEiLCJwb3J0IjoxMDA4NiwiaWQiOiJzb21lLXV1aWQiLCJhaWQiOjAsInNjeSI6ImF1dG8iLCJuZXQiOiJ3cyIsInR5cGUiOiJub25lIiwiaG9zdCI6IiIsInBhdGgiOiIvd3MiLCJ0bHMiOiJ0bHMiLCJzbmkiOiJteS1zbmkuY29tIn0=";
        let node_yaml = normalize_profile_content(vmess_link);
        let nodes = parse_clash_yaml_nodes(&node_yaml).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].name, "test-vmess");
        assert_eq!(nodes[0].node_type, "vmess");
        assert_eq!(nodes[0].server, "1.1.1.1");
        assert_eq!(nodes[0].port, 10086);

        let ss_link = "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ=@2.2.2.2:443#my-ss";
        let node_yaml_ss = normalize_profile_content(ss_link);
        let nodes_ss = parse_clash_yaml_nodes(&node_yaml_ss).unwrap();
        assert_eq!(nodes_ss.len(), 1);
        assert_eq!(nodes_ss[0].name, "my-ss");
        assert_eq!(nodes_ss[0].node_type, "ss");
        assert_eq!(nodes_ss[0].server, "2.2.2.2");
        assert_eq!(nodes_ss[0].port, 443);
    }

    #[test]
    fn shadowsocks_compact_and_plain_userinfo_links_keep_credentials() {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

        let compact = format!(
            "ss://{}#compact",
            URL_SAFE_NO_PAD.encode("aes-128-gcm:secret@compact.example.com:8388")
        );
        let plain = "ss://aes-256-gcm:plain-secret@plain.example.com:443#plain";
        let content = format!("{compact}\n{plain}");
        let config = convert_clash_to_singbox(&content, &GuiConfig::default()).unwrap();
        let nodes: Vec<&serde_json::Value> = config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|outbound| outbound["type"] == "shadowsocks")
            .collect();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0]["method"], "aes-128-gcm");
        assert_eq!(nodes[0]["password"], "secret");
        assert_eq!(nodes[1]["method"], "aes-256-gcm");
        assert_eq!(nodes[1]["password"], "plain-secret");
    }

    #[test]
    fn test_custom_domains_dns_injection() {
        let gui_config = GuiConfig {
            custom_bypass_domains: vec!["bypass.me".to_string()],
            custom_proxy_domains: vec!["proxy.me".to_string()],
            ..GuiConfig::default()
        };

        let clash_yaml = r#"
proxies:
  - name: "test-node"
    type: ss
    server: 127.0.0.1
    port: 443
    cipher: aes-256-gcm
    password: "pass"
"#;
        let res = convert_clash_to_singbox(clash_yaml, &gui_config).unwrap();

        // Extract dns.rules
        let dns_rules = res
            .get("dns")
            .unwrap()
            .get("rules")
            .unwrap()
            .as_array()
            .unwrap();

        // Find custom bypass domains rule
        let bypass_rule = dns_rules
            .iter()
            .find(|r| {
                r.get("domain_suffix")
                    .and_then(|ds| ds.as_array())
                    .map(|arr| arr.iter().any(|v| v.as_str() == Some("bypass.me")))
                    .unwrap_or(false)
            })
            .unwrap();
        assert_eq!(
            bypass_rule.get("server").unwrap().as_str(),
            Some("dns_local")
        );

        // Find custom proxy domains rule
        let proxy_rule = dns_rules
            .iter()
            .find(|r| {
                r.get("domain_suffix")
                    .and_then(|ds| ds.as_array())
                    .map(|arr| arr.iter().any(|v| v.as_str() == Some("proxy.me")))
                    .unwrap_or(false)
            })
            .unwrap();
        assert_eq!(
            proxy_rule.get("server").unwrap().as_str(),
            Some("dns_remote")
        );
    }

    #[test]
    fn validate_profile_rejects_no_node_text() {
        // Loose YAML / comments / plain text — no parseable proxy node lines.
        let res = validate_profile_content("# just comments\nfoo: bar\n");
        assert!(res.is_err(), "should reject content with zero nodes");
        let msg = res.unwrap_err();
        assert!(
            msg.contains("no parseable proxy nodes") || msg.contains("Invalid"),
            "msg={msg}"
        );
    }

    #[test]
    fn parse_skips_nodes_with_zero_port() {
        // Two valid ss proxies + a fake one with port 0 → only 2 should be parsed.
        let yaml = r#"
proxies:
  - name: "ok1"
    type: ss
    server: 1.1.1.1
    port: 443
    cipher: aes-256-gcm
    password: "x"
  - name: "ok2"
    type: ss
    server: 2.2.2.2
    port: 0
    cipher: aes-256-gcm
    password: "y"
  - name: "ok3"
    type: ss
    server: 3.3.3.3
    port: 9999
    cipher: aes-256-gcm
    password: "z"
"#;
        let nodes = parse_clash_yaml_nodes(yaml).unwrap();
        let names: Vec<&str> = nodes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, ["ok1", "ok3"]);
    }

    #[test]
    fn atomic_write_replaces_content_and_keeps_backup() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        std::fs::write(&path, b"old").expect("seed file");

        atomic_write(&path, b"new").expect("atomic write");

        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert_eq!(
            std::fs::read(dir.path().join("settings.json.bak")).unwrap(),
            b"old"
        );
        assert!(!dir.path().join(".settings.json.tmp").exists());
    }

    #[test]
    fn unsupported_clash_nodes_do_not_leak_into_selector() {
        let yaml = r#"
proxies:
  - name: unsupported
    type: snell
    server: 1.1.1.1
    port: 51820
  - name: supported
    type: ss
    server: 2.2.2.2
    port: 8388
    cipher: aes-128-gcm
    password: secret
"#;
        let config = convert_clash_to_singbox(yaml, &GuiConfig::default()).unwrap();
        let selector = config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["type"] == "selector")
            .unwrap();
        let tags = selector["outbounds"].as_array().unwrap();
        assert!(tags.iter().any(|v| v == "supported"));
        assert!(!tags.iter().any(|v| v == "unsupported"));
    }

    #[test]
    fn only_unsupported_clash_nodes_return_actionable_error() {
        let yaml = r#"
proxies:
  - name: unsupported-snell
    type: snell
    server: 1.1.1.1
    port: 51820
"#;
        let err = convert_clash_to_singbox(yaml, &GuiConfig::default()).unwrap_err();
        assert!(err.contains("snell"), "err={err}");
        assert!(err.contains("native sing-box JSON"), "err={err}");
    }

    #[test]
    fn anytls_share_link_converts_to_tls_outbound() {
        let yaml = normalize_profile_content(
            "anytls://secret@example.com:443?sni=edge.example.com#AnyTLS",
        );
        let config = convert_clash_to_singbox(&yaml, &GuiConfig::default()).unwrap();
        let outbound = config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["type"] == "anytls")
            .unwrap();
        assert_eq!(outbound["password"], "secret");
        assert_eq!(outbound["tls"]["server_name"], "edge.example.com");
    }

    #[test]
    fn hysteria_share_link_preserves_auth_string() {
        let yaml =
            normalize_profile_content("hysteria://secret@example.com:443?sni=edge.example.com#HY");
        let config = convert_clash_to_singbox(&yaml, &GuiConfig::default()).unwrap();
        let outbound = config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["type"] == "hysteria")
            .unwrap();
        assert_eq!(outbound["auth_str"], "secret");
    }

    #[test]
    fn hysteria2_share_link_preserves_obfs_bandwidth_and_alpn() {
        let yaml = normalize_profile_content(
            "hy2://secret@example.com:443?sni=edge.example.com&obfs=salamander&obfs-password=mask&upmbps=80&downmbps=200&alpn=h3&insecure=1#HY2",
        );
        let config = convert_clash_to_singbox(&yaml, &GuiConfig::default()).unwrap();
        let outbound = config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|outbound| outbound["type"] == "hysteria2")
            .unwrap();
        assert_eq!(outbound["obfs"]["type"], "salamander");
        assert_eq!(outbound["obfs"]["password"], "mask");
        assert_eq!(outbound["up_mbps"], 80);
        assert_eq!(outbound["down_mbps"], 200);
        assert_eq!(outbound["tls"]["alpn"][0], "h3");
        assert_eq!(outbound["tls"]["insecure"], true);
    }

    #[test]
    fn tuic_share_link_preserves_transport_tuning() {
        let yaml = normalize_profile_content(
            "tuic://00000000-0000-0000-0000-000000000000:secret@example.com:443?congestion_control=bbr&udp_relay_mode=quic&zero_rtt_handshake=1&heartbeat=10s&alpn=h3#TUIC",
        );
        let config = convert_clash_to_singbox(&yaml, &GuiConfig::default()).unwrap();
        let outbound = config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|outbound| outbound["type"] == "tuic")
            .unwrap();
        assert_eq!(outbound["congestion_control"], "bbr");
        assert_eq!(outbound["udp_relay_mode"], "quic");
        assert_eq!(outbound["zero_rtt_handshake"], true);
        assert_eq!(outbound["heartbeat"], "10s");
        assert_eq!(outbound["tls"]["alpn"][0], "h3");
    }

    #[test]
    fn clash_string_port_and_mixed_case_type_are_accepted() {
        let yaml = r#"
proxies:
  - name: string-port
    type: VLESS
    server: edge.example.com
    port: "443"
    uuid: 00000000-0000-0000-0000-000000000000
    tls: true
"#;
        let nodes = parse_clash_yaml_nodes(yaml).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].port, 443);
        assert_eq!(nodes[0].node_type, "vless");
        let config = convert_clash_to_singbox(yaml, &GuiConfig::default()).unwrap();
        assert!(
            config["outbounds"]
                .as_array()
                .unwrap()
                .iter()
                .any(|outbound| outbound["type"] == "vless")
        );
    }

    #[test]
    fn whole_subscription_base64_bundle_is_decoded() {
        use base64::{Engine as _, engine::general_purpose::STANDARD};

        let links = concat!(
            "vless://00000000-0000-0000-0000-000000000000@one.example.com:443?security=tls#One\n",
            "trojan://secret@two.example.com:443?sni=two.example.com#Two\n"
        );
        let encoded = STANDARD.encode(links);
        let normalized = normalize_profile_content(&encoded);
        let nodes = parse_clash_yaml_nodes(&normalized).unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].name, "One");
        assert_eq!(nodes[1].name, "Two");
    }

    #[test]
    fn shadowsocksr_link_preserves_protocol_and_obfuscation() {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

        let password = URL_SAFE_NO_PAD.encode("secret");
        let remarks = URL_SAFE_NO_PAD.encode("SSR 香港");
        let obfs_param = URL_SAFE_NO_PAD.encode("cdn.example.com");
        let protocol_param = URL_SAFE_NO_PAD.encode("42:token");
        let payload = format!(
            "server.example.com:443:auth_aes128_md5:aes-256-cfb:tls1.2_ticket_auth:{password}/?remarks={remarks}&obfsparam={obfs_param}&protoparam={protocol_param}"
        );
        let link = format!("ssr://{}", URL_SAFE_NO_PAD.encode(payload));
        let normalized = normalize_profile_content(&link);
        let config = convert_clash_to_singbox(&normalized, &GuiConfig::default()).unwrap();
        let outbound = config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|outbound| outbound["type"] == "shadowsocksr")
            .unwrap();
        assert_eq!(outbound["tag"], "SSR 香港");
        assert_eq!(outbound["method"], "aes-256-cfb");
        assert_eq!(outbound["protocol"], "auth_aes128_md5");
        assert_eq!(outbound["protocol_param"], "42:token");
        assert_eq!(outbound["obfs"], "tls1.2_ticket_auth");
        assert_eq!(outbound["obfs_param"], "cdn.example.com");
    }

    #[test]
    fn naive_and_ssh_links_convert_to_native_outbounds() {
        let content = concat!(
            "naive+https://user:secret@naive.example.com:443?sni=edge.example.com&alpn=h3#Naive\n",
            "ssh://alice:password@ssh.example.com#SSH\n"
        );
        let config = convert_clash_to_singbox(content, &GuiConfig::default()).unwrap();
        let outbounds = config["outbounds"].as_array().unwrap();
        let naive = outbounds
            .iter()
            .find(|outbound| outbound["type"] == "naive")
            .unwrap();
        assert_eq!(naive["username"], "user");
        assert_eq!(naive["password"], "secret");
        assert_eq!(naive["tls"]["server_name"], "edge.example.com");
        assert_eq!(naive["tls"]["alpn"][0], "h3");

        let ssh = outbounds
            .iter()
            .find(|outbound| outbound["type"] == "ssh")
            .unwrap();
        assert_eq!(ssh["user"], "alice");
        assert_eq!(ssh["password"], "password");
        assert_eq!(ssh["server_port"], 22);
    }

    #[test]
    fn shadowtls_link_and_clash_aliases_are_supported() {
        let link = "shadowtls://secret@shadow.example.com:443?version=3&sni=cover.example.com&fp=chrome#ShadowTLS";
        let config = convert_clash_to_singbox(link, &GuiConfig::default()).unwrap();
        let shadowtls = config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|outbound| outbound["type"] == "shadowtls")
            .unwrap();
        assert_eq!(shadowtls["password"], "secret");
        assert_eq!(shadowtls["version"], 3);
        assert_eq!(shadowtls["tls"]["server_name"], "cover.example.com");
        assert_eq!(shadowtls["tls"]["utls"]["fingerprint"], "chrome");

        let yaml = r#"
proxies:
  - name: ss-alias
    type: shadowsocks
    server: ss.example.com
    port: 8388
    cipher: aes-128-gcm
    password: secret
  - name: hy2-alias
    type: hy2
    server: hy.example.com
    port: 443
    password: secret
"#;
        let nodes = parse_clash_yaml_nodes(yaml).unwrap();
        assert_eq!(nodes[0].node_type, "ss");
        assert_eq!(nodes[1].node_type, "hysteria2");
    }

    #[test]
    fn socks_https_and_trojan_go_share_links_are_imported() {
        let content = concat!(
            "socks5://user:secret@socks.example.com:1081#SOCKS\n",
            "https://web:proxy@http.example.com?sni=cover.example.com#HTTPS\n",
            "trojan-go://password@trojan.example.com:443?sni=trojan.example.com&type=ws&path=%2Fws#TrojanGo\n"
        );
        let config = convert_clash_to_singbox(content, &GuiConfig::default()).unwrap();
        let outbounds = config["outbounds"].as_array().unwrap();

        let socks = outbounds
            .iter()
            .find(|outbound| outbound["tag"] == "SOCKS")
            .unwrap();
        assert_eq!(socks["type"], "socks");
        assert_eq!(socks["server_port"], 1081);
        assert_eq!(socks["username"], "user");

        let http = outbounds
            .iter()
            .find(|outbound| outbound["tag"] == "HTTPS")
            .unwrap();
        assert_eq!(http["type"], "http");
        assert_eq!(http["server_port"], 443);
        assert_eq!(http["tls"]["server_name"], "cover.example.com");

        let trojan = outbounds
            .iter()
            .find(|outbound| outbound["tag"] == "TrojanGo")
            .unwrap();
        assert_eq!(trojan["type"], "trojan");
        assert_eq!(trojan["transport"]["type"], "ws");
        assert_eq!(trojan["transport"]["path"], "/ws");
    }

    #[test]
    fn duplicate_clash_names_receive_stable_unique_tags() {
        let yaml = r#"
proxies:
  - { name: duplicate, type: ss, server: one.example.com, port: 8388, cipher: aes-128-gcm, password: one }
  - { name: duplicate, type: ss, server: two.example.com, port: 8388, cipher: aes-128-gcm, password: two }
"#;
        let nodes = parse_clash_yaml_nodes(yaml).unwrap();
        assert_eq!(nodes[0].name, "duplicate");
        assert_eq!(nodes[1].name, "duplicate (2)");

        let config = convert_clash_to_singbox(yaml, &GuiConfig::default()).unwrap();
        let tags: Vec<&str> = config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|outbound| outbound["type"] == "shadowsocks")
            .filter_map(|outbound| outbound["tag"].as_str())
            .collect();
        assert_eq!(tags, ["duplicate", "duplicate (2)"]);
    }

    #[test]
    fn native_json_requires_config_object_and_outbounds_or_endpoints() {
        assert!(validate_profile_content("[]").is_err());
        assert!(validate_profile_content(r#"{"log":{"level":"info"}}"#).is_err());
        assert!(validate_profile_content(r#"{"endpoints":[]}"#).is_ok());
    }

    #[test]
    fn native_wireguard_endpoint_is_visible_but_not_directly_selectable() {
        let json = r#"{
          "endpoints": [{
            "type": "wireguard",
            "tag": "wg-home",
            "address": ["10.0.0.2/32"],
            "private_key": "secret",
            "peers": [{
              "server": "wg.example.com",
              "server_port": 51820,
              "public_key": "peer-key"
            }]
          }]
        }"#;
        let nodes = parse_native_json_nodes(json).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].name, "wg-home");
        assert_eq!(nodes[0].node_type, "wireguard endpoint");
        assert_eq!(nodes[0].server, "wg.example.com");
        assert_eq!(nodes[0].port, 51820);
        assert!(!nodes[0].selectable);
    }

    #[test]
    fn plain_vless_link_does_not_force_tls() {
        let yaml = normalize_profile_content(
            "vless://00000000-0000-0000-0000-000000000000@example.com:80?security=none&type=tcp#plain",
        );
        let config = convert_clash_to_singbox(&yaml, &GuiConfig::default()).unwrap();
        let outbound = config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["type"] == "vless")
            .unwrap();
        assert!(outbound.get("tls").is_none());
    }

    #[test]
    fn sharing_links_preserve_alpn_and_utls_fingerprints() {
        let content = concat!(
            "vless://00000000-0000-0000-0000-000000000000@vless.example.com:443?security=tls&sni=cover.example.com&alpn=h2%2Ch3&fp=firefox#VLESS-TLS\n",
            "anytls://secret@any.example.com:443?sni=any.example.com&alpn=h2&fp=chrome#AnyTLS-FP\n"
        );
        let config = convert_clash_to_singbox(content, &GuiConfig::default()).unwrap();
        let outbounds = config["outbounds"].as_array().unwrap();
        let vless = outbounds
            .iter()
            .find(|outbound| outbound["tag"] == "VLESS-TLS")
            .unwrap();
        assert_eq!(vless["tls"]["alpn"][0], "h2");
        assert_eq!(vless["tls"]["alpn"][1], "h3");
        assert_eq!(vless["tls"]["utls"]["fingerprint"], "firefox");

        let anytls = outbounds
            .iter()
            .find(|outbound| outbound["tag"] == "AnyTLS-FP")
            .unwrap();
        assert_eq!(anytls["tls"]["alpn"][0], "h2");
        assert_eq!(anytls["tls"]["utls"]["fingerprint"], "chrome");
    }

    #[test]
    fn native_selector_uses_remembered_node() {
        let json = r#"{
          "outbounds": [
            {"type":"selector","tag":"custom","outbounds":["a","b"]},
            {"type":"shadowsocks","tag":"a","server":"1.1.1.1","server_port":8388,"method":"aes-128-gcm","password":"x"},
            {"type":"shadowsocks","tag":"b","server":"2.2.2.2","server_port":8388,"method":"aes-128-gcm","password":"y"}
          ]
        }"#;
        let gui = GuiConfig {
            selected_node_tag: Some("b".to_string()),
            ..GuiConfig::default()
        };
        let merged = merge_native_json_profile(json, &gui).unwrap();
        assert_eq!(merged["outbounds"][0]["default"], "b");
    }

    #[test]
    fn ss_and_ssr_parsing_edge_cases() {
        // SS with explicit user:password in URL
        let ss_link = "ss://YWVzLTEyOC1nY206dGVzdHBhc3M=@1.2.3.4:8388#SS-Node";
        let config = convert_clash_to_singbox(ss_link, &GuiConfig::default()).unwrap();
        let outbounds = config["outbounds"].as_array().unwrap();
        let ss = outbounds.iter().find(|o| o["tag"] == "SS-Node").unwrap();
        assert_eq!(ss["method"], "aes-128-gcm");
        assert_eq!(ss["password"], "testpass");

        // SSR with bracketed IPv6 and query
        let ssr_raw =
            "[2001:db8::1]:8388:origin:aes-128-cfb:plain:dGVzdA/?obfsparam=&remarks=U1NSLU5vZGU";
        let ssr_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(ssr_raw.as_bytes());
        let ssr_link = format!("ssr://{ssr_b64}");
        let config_ssr = convert_clash_to_singbox(&ssr_link, &GuiConfig::default()).unwrap();
        let outbounds_ssr = config_ssr["outbounds"].as_array().unwrap();
        let ssr = outbounds_ssr
            .iter()
            .find(|o| o["tag"] == "SSR-Node")
            .unwrap();
        assert_eq!(ssr["server"], "2001:db8::1");
        assert_eq!(ssr["server_port"], 8388);
    }

    #[test]
    fn vmess_vless_hy2_tuic_edge_cases() {
        // VMess with tls: 1
        let vmess_json = r#"{"v":"2","ps":"VMess-Node","add":"example.com","port":"443","id":"00000000-0000-0000-0000-000000000000","aid":"0","net":"ws","type":"none","host":"example.com","path":"/ws","tls":1}"#;
        let vmess_b64 = base64::engine::general_purpose::STANDARD.encode(vmess_json.as_bytes());
        let vmess_link = format!("vmess://{vmess_b64}");
        let config_vmess = convert_clash_to_singbox(&vmess_link, &GuiConfig::default()).unwrap();
        let outbounds_vmess = config_vmess["outbounds"].as_array().unwrap();
        let vmess = outbounds_vmess
            .iter()
            .find(|o| o["tag"] == "VMess-Node")
            .unwrap();
        assert_eq!(vmess["tls"]["enabled"], true);

        // VLESS with reality public key
        let vless_link = "vless://00000000-0000-0000-0000-000000000000@vless.example.com:443?pbk=abcdef123456&sni=target.com#VLESS-Reality";
        let config_vless = convert_clash_to_singbox(vless_link, &GuiConfig::default()).unwrap();
        let outbounds_vless = config_vless["outbounds"].as_array().unwrap();
        let vless = outbounds_vless
            .iter()
            .find(|o| o["tag"] == "VLESS-Reality")
            .unwrap();
        assert_eq!(vless["tls"]["enabled"], true);
        assert_eq!(vless["tls"]["reality"]["enabled"], true);
        assert_eq!(vless["tls"]["reality"]["public_key"], "abcdef123456");

        // Hy2 with YAML mapping obfs
        let hy2_yaml = r#"
proxies:
  - name: Hy2-Node
    type: hysteria2
    server: hy2.example.com
    port: 443
    password: pass
    obfs:
      type: salamander
      password: obfspassword
"#;
        let config_hy2 = convert_clash_to_singbox(hy2_yaml, &GuiConfig::default()).unwrap();
        let outbounds_hy2 = config_hy2["outbounds"].as_array().unwrap();
        let hy2 = outbounds_hy2
            .iter()
            .find(|o| o["tag"] == "Hy2-Node")
            .unwrap();
        assert_eq!(hy2["obfs"]["type"], "salamander");
        assert_eq!(hy2["obfs"]["password"], "obfspassword");

        // TUIC with congestion-control
        let tuic_yaml = r#"
proxies:
  - name: TUIC-Node
    type: tuic
    server: tuic.example.com
    port: 8443
    uuid: 00000000-0000-0000-0000-000000000000
    password: pass
    congestion-control: bbr
"#;
        let config_tuic = convert_clash_to_singbox(tuic_yaml, &GuiConfig::default()).unwrap();
        let outbounds_tuic = config_tuic["outbounds"].as_array().unwrap();
        let tuic = outbounds_tuic
            .iter()
            .find(|o| o["tag"] == "TUIC-Node")
            .unwrap();
        assert_eq!(tuic["congestion_control"], "bbr");
    }

    #[test]
    fn modern_1_13_schema_typed_dns_fakeip_global_order_and_no_block() {
        let gui = GuiConfig {
            fake_ip: true,
            routing_mode: crate::state::RoutingMode::Global,
            dns_server_local: "https://dns.alidns.com/dns-query".to_string(),
            dns_server_remote: "https://dns.google/dns-query".to_string(),
            custom_bypass_domains: vec!["bypass.local".to_string()],
            custom_proxy_domains: vec!["proxy.local".to_string()],
            custom_bypass_ips: vec!["10.88.0.0/16".to_string()],
            ..GuiConfig::default()
        };
        let yaml = r#"
proxies:
  - name: "NodeA"
    type: ss
    server: 1.2.3.4
    port: 8388
    cipher: aes-128-gcm
    password: secret
"#;
        let mut config = convert_clash_to_singbox(yaml, &gui).unwrap();
        let outbounds = config["outbounds"].as_array().unwrap();
        assert!(
            !outbounds.iter().any(|o| o["type"] == "block"),
            "unused special block outbound must not be emitted"
        );

        let servers = config["dns"]["servers"].as_array().unwrap();
        let find_dns = |tag: &str| servers.iter().find(|s| s["tag"] == tag).unwrap();
        assert_eq!(find_dns("dns_bootstrap")["type"], "local");
        assert_eq!(find_dns("dns_local")["type"], "https");
        assert_eq!(find_dns("dns_local")["server"], "dns.alidns.com");
        assert_eq!(find_dns("dns_local")["path"], "/dns-query");
        assert_eq!(find_dns("dns_local")["domain_resolver"], "dns_bootstrap");
        assert!(find_dns("dns_local").get("address").is_none());

        assert_eq!(find_dns("dns_remote")["type"], "https");
        assert_eq!(find_dns("dns_remote")["server"], "dns.google");
        assert_eq!(find_dns("dns_remote")["domain_resolver"], "dns_local");
        assert_eq!(find_dns("dns_remote")["detour"], "Proxy");

        assert_eq!(find_dns("dns_fakeip")["type"], "fakeip");
        assert_eq!(find_dns("dns_fakeip")["inet4_range"], "198.18.0.0/15");
        assert_eq!(find_dns("dns_fakeip")["inet6_range"], "fc00::/18");
        assert_eq!(config["dns"]["final"], "dns_remote");

        let dns_rules = config["dns"]["rules"].as_array().unwrap();
        let bypass_dns_idx = dns_rules
            .iter()
            .position(|r| r.get("domain_suffix") == Some(&json!(["bypass.local"])))
            .unwrap();
        let global_fakeip_idx = dns_rules
            .iter()
            .position(|r| {
                r.get("clash_mode") == Some(&json!("Global"))
                    && r.get("server") == Some(&json!("dns_fakeip"))
                    && r.get("query_type") == Some(&json!(["A", "AAAA"]))
            })
            .unwrap();
        let geosite_dns_idx = dns_rules
            .iter()
            .position(|r| r.get("rule_set") == Some(&json!(["geosite-cn"])))
            .unwrap();
        assert!(bypass_dns_idx < global_fakeip_idx);
        assert!(
            global_fakeip_idx < geosite_dns_idx,
            "Global FakeIP rule must run before geosite-cn local DNS rule"
        );

        let route_rules = config["route"]["rules"].as_array().unwrap();
        let bypass_route_idx = route_rules
            .iter()
            .position(|r| r.get("domain_suffix") == Some(&json!(["bypass.local"])))
            .unwrap();
        let bypass_ip_idx = route_rules
            .iter()
            .position(|r| r.get("ip_cidr") == Some(&json!(["10.88.0.0/16"])))
            .unwrap();
        let private_idx = route_rules
            .iter()
            .position(|r| r.get("ip_is_private") == Some(&json!(true)))
            .unwrap();
        let global_route_idx = route_rules
            .iter()
            .position(|r| r.get("clash_mode") == Some(&json!("Global")))
            .unwrap();
        let cn_route_idx = route_rules
            .iter()
            .position(|r| r.get("rule_set") == Some(&json!(["geosite-cn", "geoip-cn"])))
            .unwrap();
        assert!(bypass_route_idx < global_route_idx);
        assert!(bypass_ip_idx < global_route_idx);
        assert!(private_idx < global_route_idx);
        assert!(global_route_idx < cn_route_idx);
        assert_eq!(config["route"]["default_domain_resolver"], "dns_local");
        assert_eq!(config["route"]["final"], "Proxy");

        mitigate_run_config(&mut config);
        assert_eq!(config["experimental"]["cache_file"]["store_fakeip"], true);
        let cache_a = config["experimental"]["cache_file"]["path"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(cache_a.ends_with("cache.db"));

        // Different profile topology receives an isolated cache path
        let yaml_b = yaml.replace("NodeA", "NodeB").replace("1.2.3.4", "5.6.7.8");
        let mut config_b = convert_clash_to_singbox(&yaml_b, &gui).unwrap();
        mitigate_run_config(&mut config_b);
        let cache_b = config_b["experimental"]["cache_file"]["path"]
            .as_str()
            .unwrap()
            .to_string();
        assert_ne!(cache_a, cache_b);
    }

    #[test]
    fn bom_json_array_and_multiline_base64_json_dispatch() {
        let native_json = r#"{
          "outbounds": [
            {"type":"vless","tag":"Native-VLESS","server":"9.9.9.9","server_port":443,"uuid":"00000000-0000-0000-0000-000000000000"}
          ]
        }"#;
        let with_bom = format!("\u{feff}  {native_json}");
        assert!(is_native_json_profile(&with_bom));
        assert!(validate_profile_content(&with_bom).is_ok());
        assert_eq!(parse_nodes_from_content(&with_bom).unwrap().len(), 1);
        assert!(merge_native_json_profile(&with_bom, &GuiConfig::default()).is_ok());

        let array_with_bom = "\u{feff} [{\"bad\": true}]";
        assert!(is_native_json_profile(array_with_bom));
        let err = validate_profile_content(array_with_bom).unwrap_err();
        assert!(err.contains("configuration object"), "err={err}");

        let encoded = base64::engine::general_purpose::STANDARD.encode(with_bom.as_bytes());
        let wrapped = encoded
            .as_bytes()
            .chunks(16)
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect::<Vec<_>>()
            .join("\r\n  ");
        assert!(is_native_json_profile(&wrapped));
        assert!(validate_profile_content(&wrapped).is_ok());
        let nodes = parse_nodes_from_content(&wrapped).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].name, "Native-VLESS");

        // Multiline MIME base64 share links
        let links = "ss://YWVzLTEyOC1nY206cGFzcw==@1.1.1.1:8388#SS1\nss://YWVzLTEyOC1nY206cGFzczI=@2.2.2.2:8388#SS2\n";
        let wrapped_links = base64::engine::general_purpose::STANDARD
            .encode(links.as_bytes())
            .as_bytes()
            .chunks(20)
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect::<Vec<_>>()
            .join("\n ");
        let parsed = parse_nodes_from_content(&wrapped_links).unwrap();
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn hysteria_server_ports_bandwidths_and_native_wireguard_address_port() {
        let yaml = r#"
proxies:
  - name: Hy1-Ports
    type: hysteria
    server: hy1.example.com
    auth_str: secret1
    ports: "20000-20010,30000"
    up: "50 Mbps"
    down: "200 Mbps"
    servername: sni1.example.com
  - name: Hy2-Ports
    type: hysteria2
    server: hy2.example.com
    password: secret2
    ports:
      - "40000-40010"
      - "50000"
    up: "100 Mbps"
    down: "1 Gbps"
    servername: sni2.example.com
"#;
        let nodes = parse_clash_yaml_nodes(yaml).unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].port, 20000);
        assert_eq!(nodes[1].port, 40000);

        let config = convert_clash_to_singbox(yaml, &GuiConfig::default()).unwrap();
        let outbounds = config["outbounds"].as_array().unwrap();
        let hy1 = outbounds.iter().find(|o| o["tag"] == "Hy1-Ports").unwrap();
        assert_eq!(hy1["server_ports"], json!(["20000:20010", "30000:30000"]));
        assert_eq!(hy1["up_mbps"], 50);
        assert_eq!(hy1["down_mbps"], 200);
        assert_eq!(hy1["tls"]["server_name"], "sni1.example.com");

        let hy2 = outbounds.iter().find(|o| o["tag"] == "Hy2-Ports").unwrap();
        assert_eq!(hy2["server_ports"], json!(["40000:40010", "50000:50000"]));
        assert_eq!(hy2["up_mbps"], 100);
        assert_eq!(hy2["down_mbps"], 1000);
        assert_eq!(hy2["tls"]["server_name"], "sni2.example.com");

        let native = r#"{
          "outbounds": [
            { "type": "hysteria2", "tag": "Native-Hy2", "server": "h.example.com", "server_ports": ["21000:22000"], "password": "pw" }
          ],
          "endpoints": [
            {
              "type": "wireguard",
              "tag": "wg-modern",
              "address": ["10.0.0.2/32"],
              "private_key": "priv",
              "peers": [{ "address": "wg.modern.test", "port": 51821, "public_key": "pub" }]
            }
          ]
        }"#;
        let native_nodes = parse_native_json_nodes(native).unwrap();
        assert_eq!(native_nodes.len(), 2);
        assert_eq!(native_nodes[0].port, 21000);
        assert_eq!(native_nodes[1].server, "wg.modern.test");
        assert_eq!(native_nodes[1].port, 51821);
    }

    #[test]
    fn clash_reality_servername_string_tls_ss_plugins_transports_and_tuic() {
        let yaml = r#"
proxies:
  - name: VLESS-Reality-Int
    type: vless
    server: 1.2.3.4
    port: 443
    uuid: 00000000-0000-0000-0000-000000000000
    servername: reality.example.com
    tls: "true"
    reality-opts:
      public-key: pbk12345
      short-id: 12345678
    client-fingerprint: safari
    smux:
      enabled: true
      protocol: smux
      max-connections: 4
      padding: true
  - name: VMess-WS-ED
    type: vmess
    server: vmess.example.com
    port: 443
    uuid: 00000000-0000-0000-0000-000000000000
    tls: "true"
    servername: cdn.vmess.example.com
    network: ws
    ws-opts:
      path: /chat?foo=1&ed=2048
  - name: Trojan-GRPC-Empty
    type: trojan
    server: tr.example.com
    port: 443
    password: secret
    servername: tr-sni.example.com
    network: grpc
  - name: HTTP-StrTLS
    type: http
    server: http.example.com
    port: 443
    tls: "true"
    servername: http-sni.example.com
  - name: SS-Obfs
    type: ss
    server: ss1.example.com
    port: 8388
    cipher: aes-128-gcm
    password: pw1
    plugin: obfs
    plugin-opts:
      mode: tls
      host: bing.com
  - name: SS-V2Ray
    type: ss
    server: ss2.example.com
    port: 8388
    cipher: aes-128-gcm
    password: pw2
    plugin: v2ray-plugin
    plugin-opts:
      mode: websocket
      tls: true
      host: ws.example.com
      path: /v2ray
      mux: false
  - name: TUIC-Aliases
    type: tuic
    server: tuic.example.com
    port: 8443
    uuid: 00000000-0000-0000-0000-000000000000
    password: pw
    servername: tuic-sni.example.com
    reduce-rtt: true
    heartbeat: 10000
"#;
        let config = convert_clash_to_singbox(yaml, &GuiConfig::default()).unwrap();
        let outbounds = config["outbounds"].as_array().unwrap();
        let find = |tag: &str| outbounds.iter().find(|o| o["tag"] == tag).unwrap();

        let vless = find("VLESS-Reality-Int");
        assert_eq!(vless["tls"]["server_name"], "reality.example.com");
        assert_eq!(vless["tls"]["reality"]["public_key"], "pbk12345");
        assert_eq!(vless["tls"]["reality"]["short_id"], "12345678");
        assert_eq!(vless["tls"]["utls"]["fingerprint"], "safari");
        assert_eq!(vless["multiplex"]["enabled"], true);
        assert_eq!(vless["multiplex"]["max_connections"], 4);

        let vmess = find("VMess-WS-ED");
        assert_eq!(vmess["tls"]["enabled"], true);
        assert_eq!(vmess["tls"]["server_name"], "cdn.vmess.example.com");
        assert_eq!(vmess["transport"]["path"], "/chat?foo=1");
        assert_eq!(vmess["transport"]["max_early_data"], 2048);
        assert_eq!(
            vmess["transport"]["early_data_header_name"],
            "Sec-WebSocket-Protocol"
        );
        assert!(vmess["transport"].get("headers").is_none());

        let trojan = find("Trojan-GRPC-Empty");
        assert_eq!(trojan["tls"]["server_name"], "tr-sni.example.com");
        assert_eq!(trojan["transport"]["type"], "grpc");
        assert!(trojan["transport"].get("service_name").is_none());

        let http = find("HTTP-StrTLS");
        assert_eq!(http["tls"]["enabled"], true);
        assert_eq!(http["tls"]["server_name"], "http-sni.example.com");

        let ss_obfs = find("SS-Obfs");
        assert_eq!(ss_obfs["plugin"], "obfs-local");
        assert_eq!(ss_obfs["plugin_opts"], "obfs=tls;obfs-host=bing.com");

        let ss_v2ray = find("SS-V2Ray");
        assert_eq!(ss_v2ray["plugin"], "v2ray-plugin");
        let opts = ss_v2ray["plugin_opts"].as_str().unwrap();
        assert!(opts.contains("tls"), "opts={opts}");
        assert!(!opts.contains("mux=false"), "opts={opts}");

        let tuic = find("TUIC-Aliases");
        assert_eq!(tuic["tls"]["server_name"], "tuic-sni.example.com");
        assert_eq!(tuic["zero_rtt_handshake"], true);
        assert_eq!(tuic["heartbeat"], "10000ms");
    }

    #[test]
    fn share_links_colons_bools_grpc_socks_b64_and_wireguard() {
        let vmess_payload = r#"{"v":"2","ps":"VM-Link","add":"vm.example.com","port":443,"id":"00000000-0000-0000-0000-000000000000","aid":0,"net":"grpc","host":"cdn.vm.example.com","path":"unused","serviceName":"vm-svc","sni":"","tls":"tls","allowInsecure":1}"#;
        let vmess_link = format!(
            "vmess://{}",
            base64::engine::general_purpose::STANDARD.encode(vmess_payload.as_bytes())
        );
        let socks_cred = base64::engine::general_purpose::STANDARD.encode("socksUser:p@ss:123");
        let content = format!(
            concat!(
                "trojan://user:pass:colon@tr.example.com:443?allowInsecure=1&type=grpc&path=tr-svc#TrLink\n",
                "vless://00000000-0000-0000-0000-000000000000@vl.example.com:443?security=tls&skipCertVerify=1&type=grpc&path=vl-svc#VlLink\n",
                "{vmess_link}\n",
                "socks://{socks_cred}@1.2.3.4:1080#SocksB64\n",
                "wg://priv%2Bkey%3D@wg.example.com:51820/?publickey=pub+key%3D&address=10.0.0.2,fd00::2&mtu=1420&reserved=1,2,3#WGLink\n"
            ),
            vmess_link = vmess_link,
            socks_cred = socks_cred
        );

        let config = convert_clash_to_singbox(&content, &GuiConfig::default()).unwrap();
        let outbounds = config["outbounds"].as_array().unwrap();
        let find_ob = |tag: &str| outbounds.iter().find(|o| o["tag"] == tag).unwrap();

        let tr = find_ob("TrLink");
        assert_eq!(tr["password"], "user:pass:colon");
        assert_eq!(tr["tls"]["insecure"], true);
        assert_eq!(tr["transport"]["type"], "grpc");
        assert_eq!(tr["transport"]["service_name"], "tr-svc");

        let vl = find_ob("VlLink");
        assert_eq!(vl["tls"]["insecure"], true);
        assert_eq!(vl["transport"]["service_name"], "vl-svc");

        let vm = find_ob("VM-Link");
        assert_eq!(vm["tls"]["server_name"], "cdn.vm.example.com");
        assert_eq!(vm["tls"]["insecure"], true);
        assert_eq!(vm["transport"]["service_name"], "vm-svc");

        let sk = find_ob("SocksB64");
        assert_eq!(sk["username"], "socksUser");
        assert_eq!(sk["password"], "p@ss:123");

        let endpoints = config["endpoints"].as_array().unwrap();
        assert_eq!(endpoints.len(), 1);
        let wg = &endpoints[0];
        assert_eq!(wg["type"], "wireguard");
        assert_eq!(wg["tag"], "WGLink");
        assert_eq!(wg["address"], json!(["10.0.0.2/32", "fd00::2/128"]));
        assert_eq!(wg["private_key"], "priv+key=");
        assert_eq!(wg["mtu"], 1420);
        assert_eq!(wg["peers"][0]["address"], "wg.example.com");
        assert_eq!(wg["peers"][0]["port"], 51820);
        assert_eq!(wg["peers"][0]["public_key"], "pub+key=");
        assert_eq!(wg["peers"][0]["reserved"], json!([1, 2, 3]));

        let proxy_outbounds = find_ob("Proxy")["outbounds"].as_array().unwrap();
        assert!(proxy_outbounds.iter().any(|v| v == "WGLink"));
    }

    #[test]
    fn native_json_loopback_listen_and_ignored_settings_warnings() {
        let raw_native = r#"{
          "inbounds": [
            { "type": "mixed", "tag": "mixed-in", "listen": "::1", "listen_port": 1080 }
          ],
          "outbounds": [
            { "type": "direct", "tag": "direct" }
          ]
        }"#;
        let merged = merge_native_json_profile(raw_native, &GuiConfig::default()).unwrap();
        assert_eq!(merged["inbounds"][0]["listen"], "127.0.0.1");
        assert_eq!(merged["inbounds"][0]["listen_port"], 2080);

        // `0.0.0.0` also accepts 127.0.0.1 loopback connections cleanly.
        let lan_native = raw_native.replace("::1", "0.0.0.0");
        let merged_lan = merge_native_json_profile(&lan_native, &GuiConfig::default()).unwrap();
        assert_eq!(merged_lan["inbounds"][0]["listen"], "0.0.0.0");

        assert!(native_json_ignored_settings(raw_native, &GuiConfig::default()).is_empty());

        let customized = GuiConfig {
            fake_ip: true,
            tcp_fast_open: true,
            tcp_multipath: true,
            custom_bypass_domains: vec!["example.cn".to_string()],
            ..GuiConfig::default()
        };
        let ignored = native_json_ignored_settings(raw_native, &customized);
        assert_eq!(
            ignored,
            vec![
                "Custom Rules",
                "DNS / Fake-IP",
                "TCP Fast Open",
                "TCP Multipath"
            ]
        );
    }

    #[test]
    fn load_gui_config_recovers_from_backup_and_persists_without_clobbering_bak() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gui_config.json");
        let backup = dir.path().join("gui_config.json.bak");
        let corrupt = dir.path().join("gui_config.json.corrupt.bak");

        let saved_cfg = GuiConfig {
            mixed_port: 3080,
            api_port: 9191,
            fake_ip: true,
            ..GuiConfig::default()
        };
        let good_json = serde_json::to_string_pretty(&saved_cfg).unwrap();
        std::fs::write(&backup, &good_json).unwrap();
        // Write invalid non-UTF8 bytes as corrupt primary config.
        std::fs::write(&path, [0xff, 0xfe, 0x00, 0x80]).unwrap();

        let loaded = load_gui_config_from_paths(&path, &backup, &corrupt);
        assert_eq!(loaded.mixed_port, 3080);
        assert_eq!(loaded.api_port, 9191);
        assert!(loaded.fake_ip);
        assert!(corrupt.exists(), "corrupt bytes should be preserved");

        // Primary config must now be persisted valid JSON and `.bak` must remain valid.
        let primary_after: GuiConfig =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let backup_after: GuiConfig =
            serde_json::from_str(&std::fs::read_to_string(&backup).unwrap()).unwrap();
        assert_eq!(primary_after.mixed_port, 3080);
        assert_eq!(backup_after.mixed_port, 3080);
    }

    #[test]
    fn clash_api_secret_cache_updates_without_mtime_change() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("run_config.json");
        let cache = std::sync::Mutex::new((Vec::new(), String::new()));

        std::fs::write(&path, r#"{"experimental":{"clash_api":{"secret":"abc1"}}}"#).unwrap();
        assert_eq!(read_clash_api_secret_from_path(&path, &cache), "abc1");

        // Same byte length written immediately within the same coarse mtime tick.
        std::fs::write(&path, r#"{"experimental":{"clash_api":{"secret":"xyz2"}}}"#).unwrap();
        assert_eq!(read_clash_api_secret_from_path(&path, &cache), "xyz2");
    }
}
