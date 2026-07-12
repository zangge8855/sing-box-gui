use std::fs;
use std::path::PathBuf;
use serde_json::json;
use crate::state::{GuiConfig, ProxyNode};

use std::sync::OnceLock;
static APP_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn get_app_dir() -> PathBuf {
    APP_DIR.get_or_init(|| {
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
            .unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            });
        let dir = base.join("sing-box-gui");
        let _ = fs::create_dir_all(&dir);
        let _ = fs::create_dir_all(dir.join("profiles"));
        let _ = fs::create_dir_all(dir.join("bin"));
        dir
    }).clone()
}

/// Parse Clash subscription-userinfo header values.
/// Example: `upload=123; download=456; total=789; expire=1710000000`
pub fn parse_subscription_userinfo(header: &str) -> (Option<u64>, Option<u64>, Option<u64>, Option<i64>) {
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

pub fn load_gui_config() -> GuiConfig {
    let path = get_config_path();
    let mut config = if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            match serde_json::from_str::<GuiConfig>(&content) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("Failed to parse gui_config.json: {}. Backing up and resetting to default.", e);
                    let backup_path = get_app_dir().join("gui_config.json.bak");
                    let _ = fs::write(&backup_path, content);
                    GuiConfig::default()
                }
            }
        } else {
            GuiConfig::default()
        }
    } else {
        GuiConfig::default()
    };

    // One-time migration for existing users to match system locale if not migrated yet
    let migration_file = get_app_dir().join(".migrated_locale");
    if !migration_file.exists() {
        #[cfg(target_os = "windows")]
        {
            use winreg::RegKey;
            use winreg::enums::HKEY_CURRENT_USER;
            if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Control Panel\\International")
                && let Ok(locale) = hkcu.get_value::<String, _>("LocaleName")
                    && locale.to_lowercase().starts_with("zh") {
                        config.language = crate::state::Language::Zh;
                        let _ = save_gui_config(&config);
                    }
        }
        let _ = fs::write(migration_file, "done");
    }
    
    config
}

pub fn save_gui_config(config: &GuiConfig) -> Result<(), String> {
    let path = get_config_path();
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    atomic_write(&path, content.as_bytes())?;
    Ok(())
}

/// Replace a persisted file without exposing a partially-written destination.
/// This is used for settings and subscription files because a power loss during
/// a direct `fs::write` would otherwise reset the application on next launch.
pub fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;

    let parent = path
        .parent()
        .ok_or_else(|| "Target file has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("Failed to create data directory: {e}"))?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("data");
    let temp_path = parent.join(format!(".{file_name}.tmp"));
    let backup_path = parent.join(format!("{file_name}.bak"));

    {
        let mut file = fs::File::create(&temp_path)
            .map_err(|e| format!("Failed to create temporary file: {e}"))?;
        file.write_all(bytes)
            .map_err(|e| format!("Failed to write temporary file: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("Failed to flush temporary file: {e}"))?;
    }

    if path.exists() {
        let _ = fs::copy(path, &backup_path);
    }

    #[cfg(target_os = "windows")]
    if path.exists() {
        fs::remove_file(path)
            .map_err(|e| format!("Failed to replace existing file: {e}"))?;
    }

    if let Err(e) = fs::rename(&temp_path, path) {
        #[cfg(target_os = "windows")]
        if backup_path.exists() && !path.exists() {
            let _ = fs::copy(&backup_path, path);
        }
        return Err(format!("Failed to commit temporary file: {e}"));
    }
    Ok(())
}

fn decode_base64_padded(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    
    // Attempt standard decode
    if let Ok(bytes) = STANDARD.decode(input)
        && let Ok(s) = String::from_utf8(bytes) {
            return Some(s);
        }
    
    // Attempt with padding
    let mut padded = input.to_string();
    while !padded.len().is_multiple_of(4) {
        padded.push('=');
    }
    if let Ok(bytes) = STANDARD.decode(&padded)
        && let Ok(s) = String::from_utf8(bytes) {
            return Some(s);
        }
    
    // Attempt URL-safe decode
    use base64::engine::general_purpose::URL_SAFE;
    if let Ok(bytes) = URL_SAFE.decode(input)
        && let Ok(s) = String::from_utf8(bytes) {
            return Some(s);
        }
    
    let mut padded_url = input.to_string();
    while !padded_url.len().is_multiple_of(4) {
        padded_url.push('=');
    }
    if let Ok(bytes) = URL_SAFE.decode(&padded_url)
        && let Ok(s) = String::from_utf8(bytes) {
            return Some(s);
        }
    
    None
}

fn parse_share_link(link: &str) -> Option<serde_yaml::Mapping> {
    let link = link.trim();
    if link.is_empty() {
        return None;
    }
    
    use url::Url;
    let url = Url::parse(link).ok()?;
    let scheme = url.scheme();
    
    let mut map = serde_yaml::Mapping::new();
    
    let host = url.host_str().unwrap_or("127.0.0.1").to_string();
    let port = url.port().unwrap_or(443);
    let tag = url.fragment().map(|f| urlencoding::decode(f).unwrap_or(std::borrow::Cow::Borrowed(f)).into_owned()).unwrap_or_else(|| format!("{}-{}", scheme, host));
    
    map.insert(serde_yaml::Value::String("name".to_string()), serde_yaml::Value::String(tag));
    map.insert(serde_yaml::Value::String("server".to_string()), serde_yaml::Value::String(host));
    map.insert(serde_yaml::Value::String("port".to_string()), serde_yaml::Value::Number(port.into()));
    
    match scheme {
        "ss" => {
            map.insert(serde_yaml::Value::String("type".to_string()), serde_yaml::Value::String("ss".to_string()));
            
            let raw_body = link.trim_start_matches("ss://").split('#').next().unwrap().split('?').next().unwrap();
            if !raw_body.contains('@') {
                if let Some(decoded) = decode_base64_padded(raw_body) {
                    let decoded_link = format!("ss://{}", decoded);
                    if let Ok(temp_url) = Url::parse(&decoded_link) {
                        let userinfo = temp_url.username();
                        let parts: Vec<&str> = userinfo.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let cipher = urlencoding::decode(parts[0]).unwrap_or(std::borrow::Cow::Borrowed(parts[0])).into_owned();
                            let password = urlencoding::decode(parts[1]).unwrap_or(std::borrow::Cow::Borrowed(parts[1])).into_owned();
                            map.insert(serde_yaml::Value::String("cipher".to_string()), serde_yaml::Value::String(cipher));
                            map.insert(serde_yaml::Value::String("password".to_string()), serde_yaml::Value::String(password));
                        }
                        let host = temp_url.host_str().unwrap_or("127.0.0.1").to_string();
                        let port = temp_url.port().unwrap_or(443);
                        map.insert(serde_yaml::Value::String("server".to_string()), serde_yaml::Value::String(host));
                        map.insert(serde_yaml::Value::String("port".to_string()), serde_yaml::Value::Number(port.into()));
                    }
                }
            } else {
                let userinfo_b64 = url.username();
                if let Some(decoded) = decode_base64_padded(userinfo_b64) {
                    let parts: Vec<&str> = decoded.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let cipher = urlencoding::decode(parts[0]).unwrap_or(std::borrow::Cow::Borrowed(parts[0])).into_owned();
                        let password = urlencoding::decode(parts[1]).unwrap_or(std::borrow::Cow::Borrowed(parts[1])).into_owned();
                        map.insert(serde_yaml::Value::String("cipher".to_string()), serde_yaml::Value::String(cipher));
                        map.insert(serde_yaml::Value::String("password".to_string()), serde_yaml::Value::String(password));
                    }
                }
            }
        }
        "vmess" => {
            map.insert(serde_yaml::Value::String("type".to_string()), serde_yaml::Value::String("vmess".to_string()));
            
            let body_b64 = link.trim_start_matches("vmess://").split('?').next().unwrap().split('#').next().unwrap();
            if let Some(decoded_json) = decode_base64_padded(body_b64)
                && let Ok(v) = serde_json::from_str::<serde_json::Value>(&decoded_json) {
                    let name = v.get("ps").and_then(|x| x.as_str()).unwrap_or("vmess-node").to_string();
                    let server = v.get("add").and_then(|x| x.as_str()).unwrap_or("127.0.0.1").to_string();
                    let port = v.get("port").and_then(|x| x.as_u64().or_else(|| x.as_str().and_then(|s| s.parse::<u64>().ok()))).unwrap_or(443);
                    let uuid = v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let alter_id = v.get("aid").and_then(|x| x.as_u64().or_else(|| x.as_str().and_then(|s| s.parse::<u64>().ok()))).unwrap_or(0);
                    let cipher = v.get("scy").and_then(|x| x.as_str()).unwrap_or("auto").to_string();
                    let network = v.get("net").and_then(|x| x.as_str()).unwrap_or("tcp").to_string();
                    let tls = v.get("tls").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let host_sni = v.get("sni").and_then(|x| x.as_str()).or_else(|| v.get("host").and_then(|x| x.as_str())).unwrap_or("").to_string();
                    
                    map.insert(serde_yaml::Value::String("name".to_string()), serde_yaml::Value::String(name));
                    map.insert(serde_yaml::Value::String("server".to_string()), serde_yaml::Value::String(server));
                    map.insert(serde_yaml::Value::String("port".to_string()), serde_yaml::Value::Number(port.into()));
                    map.insert(serde_yaml::Value::String("uuid".to_string()), serde_yaml::Value::String(uuid));
                    map.insert(serde_yaml::Value::String("alterId".to_string()), serde_yaml::Value::Number(alter_id.into()));
                    map.insert(serde_yaml::Value::String("cipher".to_string()), serde_yaml::Value::String(cipher));
                    map.insert(serde_yaml::Value::String("network".to_string()), serde_yaml::Value::String(network));
                    
                    if tls == "tls" {
                        map.insert(serde_yaml::Value::String("tls".to_string()), serde_yaml::Value::Bool(true));
                        if !host_sni.is_empty() {
                            map.insert(serde_yaml::Value::String("sni".to_string()), serde_yaml::Value::String(host_sni));
                        }
                    }
                }
        }
        "vless" => {
            map.insert(serde_yaml::Value::String("type".to_string()), serde_yaml::Value::String("vless".to_string()));
            
            let uuid = url.username().to_string();
            let decoded_uuid = urlencoding::decode(&uuid).unwrap_or(std::borrow::Cow::Borrowed(&uuid)).into_owned();
            map.insert(serde_yaml::Value::String("uuid".to_string()), serde_yaml::Value::String(decoded_uuid));
            
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
                if k == "sni" {
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
                if k == "serviceName" {
                    service_name = v.to_string();
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
                if k == "skipCertVerify" || k == "allowInsecure" {
                    skip_cert_verify = v.parse::<bool>().unwrap_or(false);
                }
            }
            if tls_enabled {
                map.insert(serde_yaml::Value::String("tls".to_string()), serde_yaml::Value::Bool(true));
                if !sni.is_empty() {
                    map.insert(serde_yaml::Value::String("sni".to_string()), serde_yaml::Value::String(sni));
                }
            }
            if reality_enabled {
                map.insert(serde_yaml::Value::String("reality".to_string()), serde_yaml::Value::Bool(true));
                if !public_key.is_empty() {
                    map.insert(serde_yaml::Value::String("public-key".to_string()), serde_yaml::Value::String(public_key));
                }
                if !short_id.is_empty() {
                    map.insert(serde_yaml::Value::String("short-id".to_string()), serde_yaml::Value::String(short_id));
                }
                if !fingerprint.is_empty() {
                    map.insert(serde_yaml::Value::String("fingerprint".to_string()), serde_yaml::Value::String(fingerprint));
                }
            }
            if !flow.is_empty() {
                map.insert(serde_yaml::Value::String("flow".to_string()), serde_yaml::Value::String(flow));
            }
            if !network.is_empty() {
                map.insert(serde_yaml::Value::String("network".to_string()), serde_yaml::Value::String(network));
            }
            if !path.is_empty() {
                map.insert(serde_yaml::Value::String("path".to_string()), serde_yaml::Value::String(path));
            }
            if !host.is_empty() {
                map.insert(serde_yaml::Value::String("host".to_string()), serde_yaml::Value::String(host));
            }
            if !service_name.is_empty() {
                map.insert(serde_yaml::Value::String("serviceName".to_string()), serde_yaml::Value::String(service_name));
            }
            if skip_cert_verify {
                map.insert(serde_yaml::Value::String("skip-cert-verify".to_string()), serde_yaml::Value::Bool(true));
            }
        }
        "trojan" => {
            map.insert(serde_yaml::Value::String("type".to_string()), serde_yaml::Value::String("trojan".to_string()));
            
            let password = url.username().to_string();
            let decoded_password = urlencoding::decode(&password).unwrap_or(std::borrow::Cow::Borrowed(&password)).into_owned();
            map.insert(serde_yaml::Value::String("password".to_string()), serde_yaml::Value::String(decoded_password));
            
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
                if k == "sni" {
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
                if k == "serviceName" {
                    service_name = v.to_string();
                }
                if k == "skipCertVerify" || k == "allowInsecure" {
                    skip_cert_verify = v.parse::<bool>().unwrap_or(false);
                }
            }
            if tls_enabled {
                map.insert(serde_yaml::Value::String("tls".to_string()), serde_yaml::Value::Bool(true));
                if !sni.is_empty() {
                    map.insert(serde_yaml::Value::String("sni".to_string()), serde_yaml::Value::String(sni));
                }
            }
            if !network.is_empty() {
                map.insert(serde_yaml::Value::String("network".to_string()), serde_yaml::Value::String(network));
            }
            if !path.is_empty() {
                map.insert(serde_yaml::Value::String("path".to_string()), serde_yaml::Value::String(path));
            }
            if !host.is_empty() {
                map.insert(serde_yaml::Value::String("host".to_string()), serde_yaml::Value::String(host));
            }
            if !service_name.is_empty() {
                map.insert(serde_yaml::Value::String("serviceName".to_string()), serde_yaml::Value::String(service_name));
            }
            if skip_cert_verify {
                map.insert(serde_yaml::Value::String("skip-cert-verify".to_string()), serde_yaml::Value::Bool(true));
            }
        }
        "hysteria" => {
            map.insert(serde_yaml::Value::String("type".to_string()), serde_yaml::Value::String("hysteria".to_string()));
            let password = url.username().to_string();
            let decoded_password = urlencoding::decode(&password).unwrap_or(std::borrow::Cow::Borrowed(&password)).into_owned();
            map.insert(serde_yaml::Value::String("auth-str".to_string()), serde_yaml::Value::String(decoded_password));
            
            let mut sni = String::new();
            for (k, v) in url.query_pairs() {
                if k == "sni" {
                    sni = v.to_string();
                }
            }
            if !sni.is_empty() {
                map.insert(serde_yaml::Value::String("sni".to_string()), serde_yaml::Value::String(sni));
            }
        }
        "hysteria2" | "hy2" => {
            map.insert(serde_yaml::Value::String("type".to_string()), serde_yaml::Value::String("hysteria2".to_string()));
            let password = url.username().to_string();
            let decoded_password = urlencoding::decode(&password).unwrap_or(std::borrow::Cow::Borrowed(&password)).into_owned();
            map.insert(serde_yaml::Value::String("password".to_string()), serde_yaml::Value::String(decoded_password));
            
            let mut sni = String::new();
            for (k, v) in url.query_pairs() {
                if k == "sni" {
                    sni = v.to_string();
                }
            }
            if !sni.is_empty() {
                map.insert(serde_yaml::Value::String("sni".to_string()), serde_yaml::Value::String(sni));
            }
        }
        "tuic" => {
            map.insert(serde_yaml::Value::String("type".to_string()), serde_yaml::Value::String("tuic".to_string()));
            let uuid = url.username().to_string();
            let decoded_uuid = urlencoding::decode(&uuid).unwrap_or(std::borrow::Cow::Borrowed(&uuid)).into_owned();
            let password = url.password().unwrap_or("").to_string();
            let decoded_password = urlencoding::decode(&password).unwrap_or(std::borrow::Cow::Borrowed(&password)).into_owned();
            
            map.insert(serde_yaml::Value::String("uuid".to_string()), serde_yaml::Value::String(decoded_uuid));
            if !decoded_password.is_empty() {
                map.insert(serde_yaml::Value::String("password".to_string()), serde_yaml::Value::String(decoded_password));
            }
            
            let mut sni = String::new();
            let mut congestion = String::new();
            for (k, v) in url.query_pairs() {
                if k == "sni" {
                    sni = v.to_string();
                }
                if k == "congestion_control" {
                    congestion = v.to_string();
                }
            }
            if !sni.is_empty() {
                map.insert(serde_yaml::Value::String("sni".to_string()), serde_yaml::Value::String(sni));
            }
            if !congestion.is_empty() {
                map.insert(serde_yaml::Value::String("congestion_control".to_string()), serde_yaml::Value::String(congestion));
            }
        }
        _ => return None,
    }
    
    Some(map)
}

pub fn normalize_profile_content(content: &str) -> String {
    let content = content.trim();
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
            serde_yaml::Value::Sequence(proxies.into_iter().map(serde_yaml::Value::Mapping).collect())
        );
        if let Ok(yaml_str) = serde_yaml::to_string(&root) {
            return yaml_str;
        }
    }
    
    content.to_string()
}

pub fn get_profile_path(profile_id: &str) -> PathBuf {
    let sanitized: String = profile_id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    get_app_dir().join("profiles").join(format!("{}.json", sanitized))
}

pub fn parse_clash_yaml_nodes(content: &str) -> Result<Vec<ProxyNode>, String> {
    let key_name = serde_yaml::Value::String("name".into());
    let key_port = serde_yaml::Value::String("port".into());
    let key_server = serde_yaml::Value::String("server".into());
    let key_type = serde_yaml::Value::String("type".into());
    let normalized = normalize_profile_content(content);
    let val: serde_yaml::Value = serde_yaml::from_str(&normalized)
        .map_err(|e| format!("YAML parsing failed: {}", e))?;
        
    let proxies = val.get("proxies")
        .and_then(|v| v.as_sequence())
        .ok_or_else(|| "No 'proxies' key found in config file".to_string())?;
        
    let mut nodes = Vec::new();
    for item in proxies {
        if let Some(map) = item.as_mapping() {
            let name = map.get(&key_name)
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed")
                .to_string();
                
            let node_type = map.get(&key_type)
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
                
            let server = map.get(&key_server)
                .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|i| i.to_string())))
                .unwrap_or_else(|| "127.0.0.1".to_string());
                
            let port_u64 = map.get(&key_port)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            
            if port_u64 == 0 || port_u64 > 65535 {
                continue;
            }
            let port = port_u64 as u16;
                
            nodes.push(ProxyNode {
                name,
                node_type,
                server,
                port,
                latency: None,
            });
        }
    }
    Ok(nodes)
}

pub fn parse_native_json_nodes(json_content: &str) -> Result<Vec<ProxyNode>, String> {
    let val: serde_json::Value = serde_json::from_str(json_content)
        .map_err(|e| format!("JSON parsing failed: {}", e))?;
        
    let outbounds = val.get("outbounds")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "No 'outbounds' array found".to_string())?;
        
    let mut nodes = Vec::new();
    for item in outbounds {
        if let Some(obj) = item.as_object() {
            let node_type = obj.get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
                
            if node_type == "selector" || node_type == "direct" || node_type == "dns" || node_type == "block" || node_type == "urltest" {
                continue;
            }
            
            let name = obj.get("tag")
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed")
                .to_string();
                
            let server = obj.get("server")
                .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|i| i.to_string())))
                .unwrap_or_else(|| "127.0.0.1".to_string());
                
            let port_u64 = obj.get("server_port")
                .and_then(|v| v.as_u64())
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
                });
            }
        }
    }
    Ok(nodes)
}

pub fn validate_profile_content(content: &str) -> Result<(), String> {
    let content = content.trim();
    if content.starts_with('{') || content.starts_with('[') {
        let _: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| format!("Invalid JSON structure: {}", e))?;
        Ok(())
    } else {
        let normalized = normalize_profile_content(content);
        // First validate the YAML itself — catches malformed indentation etc.
        let _: serde_yaml::Value = serde_yaml::from_str(&normalized)
            .map_err(|e| format!("Invalid YAML structure: {}", e))?;
        // Then ensure at least one proxy node was actually parsed.
        let nodes = parse_clash_yaml_nodes(content)
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

fn get_transport_block(map: &serde_yaml::Mapping) -> Option<serde_json::Value> {
    let key_network = serde_yaml::Value::String("network".into());
    let key_type = serde_yaml::Value::String("type".into());
    let key_ws_opts = serde_yaml::Value::String("ws-opts".into());
    let key_path = serde_yaml::Value::String("path".into());
    let key_headers = serde_yaml::Value::String("headers".into());
    let key_host = serde_yaml::Value::String("host".into());
    let key_grpc_opts = serde_yaml::Value::String("grpc-opts".into());
    let key_grpc_service_name = serde_yaml::Value::String("grpc-service-name".into());
    let key_service_name = serde_yaml::Value::String("serviceName".into());

    let network = map.get(&key_network)
        .and_then(|v| v.as_str())
        .or_else(|| map.get(&key_type).and_then(|v| v.as_str()))
        .unwrap_or("tcp");
        
    if network == "ws" {
        let mut path = "/".to_string();
        let mut headers_map = serde_json::Map::new();
        
        if let Some(ws_opts) = map.get(&key_ws_opts).and_then(|v| v.as_mapping()) {
            if let Some(p) = ws_opts.get(&key_path).and_then(|v| v.as_str()) {
                path = p.to_string();
            }
            if let Some(headers) = ws_opts.get(&key_headers).and_then(|v| v.as_mapping()) {
                for (k, v) in headers {
                    if let (Some(ks), Some(vs)) = (k.as_str(), v.as_str()) {
                        headers_map.insert(ks.to_string(), json!(vs));
                    }
                }
            }
        } else {
            if let Some(p) = map.get(&key_path).and_then(|v| v.as_str()) {
                path = p.to_string();
            }
            if let Some(h) = map.get(&key_host).and_then(|v| v.as_str()) {
                headers_map.insert("Host".to_string(), json!(h));
            }
        }
        
        Some(json!({
            "type": "ws",
            "path": path,
            "headers": headers_map
        }))
    } else if network == "grpc" {
        let mut service_name = "".to_string();
        if let Some(grpc_opts) = map.get(&key_grpc_opts).and_then(|v| v.as_mapping()) {
            if let Some(s) = grpc_opts.get(&key_grpc_service_name).and_then(|v| v.as_str()) {
                service_name = s.to_string();
            }
        } else if let Some(s) = map.get(&key_service_name).and_then(|v| v.as_str()) {
            service_name = s.to_string();
        }
        
        Some(json!({
            "type": "grpc",
            "service_name": service_name
        }))
    } else {
        None
    }
}

pub fn convert_clash_to_singbox(
    yaml_content: &str,
    gui_config: &GuiConfig,
) -> Result<serde_json::Value, String> {
    let key_alter_id = serde_yaml::Value::String("alterId".into());
    let key_auth_str = serde_yaml::Value::String("auth_str".into());
    let key_cipher = serde_yaml::Value::String("cipher".into());
    let key_congestion_control = serde_yaml::Value::String("congestion_control".into());
    let key_fingerprint = serde_yaml::Value::String("fingerprint".into());
    let key_flow = serde_yaml::Value::String("flow".into());
    let key_name = serde_yaml::Value::String("name".into());
    let key_password = serde_yaml::Value::String("password".into());
    let key_port = serde_yaml::Value::String("port".into());
    let key_public_key = serde_yaml::Value::String("public-key".into());
    let key_reality = serde_yaml::Value::String("reality".into());
    let key_reality_opts = serde_yaml::Value::String("reality-opts".into());
    let key_server = serde_yaml::Value::String("server".into());
    let key_servername = serde_yaml::Value::String("servername".into());
    let key_short_id = serde_yaml::Value::String("short-id".into());
    let key_skip_cert_verify = serde_yaml::Value::String("skip-cert-verify".into());
    let key_sni = serde_yaml::Value::String("sni".into());
    let key_tls = serde_yaml::Value::String("tls".into());
    let key_type = serde_yaml::Value::String("type".into());
    let key_username = serde_yaml::Value::String("username".into());
    let key_uuid = serde_yaml::Value::String("uuid".into());
    let normalized = normalize_profile_content(yaml_content);
    let yaml: serde_yaml::Value = serde_yaml::from_str(&normalized)
        .map_err(|e| format!("YAML parsing failed: {}", e))?;
        
    let proxies_val = yaml.get("proxies")
        .ok_or_else(|| "No 'proxies' key found in Clash config".to_string())?;
        
    let proxies_arr = proxies_val.as_sequence()
        .ok_or_else(|| "'proxies' must be a sequence/list".to_string())?;
        
    let mut outbounds = Vec::new();
    let mut node_tags = Vec::new();
    
    for item in proxies_arr {
        let mut outbound = serde_json::Map::new();
        
        let map = match item.as_mapping() {
            Some(m) => m,
            None => continue,
        };
        
        let name = map.get(&key_name)
            .and_then(|v| v.as_str())
            .unwrap_or("Unnamed")
            .to_string();
            
        let server = map.get(&key_server)
            .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|i| i.to_string())))
            .unwrap_or_else(|| "127.0.0.1".to_string());
            
        let port_u64 = map.get(&key_port)
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
            
        if port_u64 == 0 || port_u64 > 65535 {
            continue;
        }
        let port = port_u64 as u16;
            
        outbound.insert("tag".to_string(), json!(name));
        node_tags.push(name.clone());
        
        let node_type = map.get(&key_type)
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
            
        let skip_cert_verify = map.get(&key_skip_cert_verify)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
            
        match node_type {
            "ss" => {
                outbound.insert("type".to_string(), json!("shadowsocks"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let cipher = map.get(&key_cipher)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let password = map.get(&key_password)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                    
                outbound.insert("method".to_string(), json!(cipher));
                outbound.insert("password".to_string(), json!(password));
            }
            "vmess" => {
                outbound.insert("type".to_string(), json!("vmess"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let uuid = map.get(&key_uuid)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let security = map.get(&key_cipher)
                    .and_then(|v| v.as_str())
                    .unwrap_or("auto");
                let alter_id = map.get(&key_alter_id)
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                    
                outbound.insert("uuid".to_string(), json!(uuid));
                outbound.insert("security".to_string(), json!(security));
                if alter_id > 0 {
                    outbound.insert("alter_id".to_string(), json!(alter_id));
                }
                
                let tls_enabled = map.get(&key_tls)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let sni = map.get(&key_sni)
                    .and_then(|v| v.as_str())
                    .or_else(|| map.get(&key_servername).and_then(|v| v.as_str()))
                    .unwrap_or(&server);
                    
                if tls_enabled {
                    outbound.insert("tls".to_string(), json!({
                        "enabled": true,
                        "server_name": sni,
                        "insecure": skip_cert_verify
                    }));
                }
                
                if let Some(trans) = get_transport_block(map) {
                    outbound.insert("transport".to_string(), trans);
                }
            }
            "vless" => {
                outbound.insert("type".to_string(), json!("vless"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let uuid = map.get(&key_uuid)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let flow = map.get(&key_flow)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                    
                outbound.insert("uuid".to_string(), json!(uuid));
                if !flow.is_empty() {
                    outbound.insert("flow".to_string(), json!(flow));
                }
                
                let tls_enabled = map.get(&key_tls)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let sni = map.get(&key_sni)
                    .and_then(|v| v.as_str())
                    .unwrap_or(&server);
                    
                let reality_enabled = map.get(&key_reality)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                    
                if tls_enabled {
                    let mut tls_opts = serde_json::Map::new();
                    tls_opts.insert("enabled".to_string(), json!(true));
                    tls_opts.insert("server_name".to_string(), json!(sni));
                    tls_opts.insert("insecure".to_string(), json!(skip_cert_verify));
                    
                    if reality_enabled {
                        let public_key = map.get(&key_public_key)
                            .and_then(|v| v.as_str())
                            .or_else(|| {
                                map.get(&key_reality_opts)
                                    .and_then(|v| v.as_mapping())
                                    .and_then(|m| m.get(&key_public_key))
                                    .and_then(|v| v.as_str())
                            })
                            .unwrap_or("");
                            
                        let short_id = map.get(&key_short_id)
                            .and_then(|v| v.as_str())
                            .or_else(|| {
                                map.get(&key_reality_opts)
                                    .and_then(|v| v.as_mapping())
                                    .and_then(|m| m.get(&key_short_id))
                                    .and_then(|v| v.as_str())
                            })
                            .unwrap_or("");
                            
                        tls_opts.insert("reality".to_string(), json!({
                            "enabled": true,
                            "public_key": public_key,
                            "short_id": short_id
                        }));
                        
                        let fingerprint = map.get(&key_fingerprint)
                            .and_then(|v| v.as_str())
                            .unwrap_or("chrome");
                            
                        tls_opts.insert("utls".to_string(), json!({
                            "enabled": true,
                            "fingerprint": fingerprint
                        }));
                    }
                    
                    outbound.insert("tls".to_string(), serde_json::Value::Object(tls_opts));
                }
                
                if let Some(trans) = get_transport_block(map) {
                    outbound.insert("transport".to_string(), trans);
                }
            }
            "trojan" => {
                outbound.insert("type".to_string(), json!("trojan"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let password = map.get(&key_password)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                outbound.insert("password".to_string(), json!(password));
                
                let tls_enabled = map.get(&key_tls)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let sni = map.get(&key_sni)
                    .and_then(|v| v.as_str())
                    .unwrap_or(&server);
                    
                if tls_enabled {
                    outbound.insert("tls".to_string(), json!({
                        "enabled": true,
                        "server_name": sni,
                        "insecure": skip_cert_verify
                    }));
                }
                
                if let Some(trans) = get_transport_block(map) {
                    outbound.insert("transport".to_string(), trans);
                }
            }
            "hysteria" => {
                outbound.insert("type".to_string(), json!("hysteria"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let auth = map.get(&key_auth_str)
                    .and_then(|v| v.as_str())
                    .or_else(|| map.get(&key_auth_str).and_then(|v| v.as_str()))
                    .or_else(|| map.get(&key_password).and_then(|v| v.as_str()))
                    .unwrap_or("");
                outbound.insert("auth_str".to_string(), json!(auth));
                
                let sni = map.get(&key_sni)
                    .and_then(|v| v.as_str())
                    .unwrap_or(&server);
                outbound.insert("tls".to_string(), json!({
                    "enabled": true,
                    "server_name": sni,
                    "insecure": skip_cert_verify
                }));
            }
            "hysteria2" => {
                outbound.insert("type".to_string(), json!("hysteria2"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let password = map.get(&key_password)
                    .and_then(|v| v.as_str())
                    .or_else(|| map.get(&key_auth_str).and_then(|v| v.as_str()))
                    .unwrap_or("");
                outbound.insert("password".to_string(), json!(password));
                
                let sni = map.get(&key_sni)
                    .and_then(|v| v.as_str())
                    .unwrap_or(&server);
                outbound.insert("tls".to_string(), json!({
                    "enabled": true,
                    "server_name": sni,
                    "insecure": skip_cert_verify
                }));
            }
            "socks" | "socks5" => {
                outbound.insert("type".to_string(), json!("socks"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let username = map.get(&key_username)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let password = map.get(&key_password)
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
                
                let username = map.get(&key_username)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let password = map.get(&key_password)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                    
                if !username.is_empty() {
                    outbound.insert("username".to_string(), json!(username));
                }
                if !password.is_empty() {
                    outbound.insert("password".to_string(), json!(password));
                }
                
                let tls_enabled = map.get(&key_tls)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let sni = map.get(&key_sni)
                    .and_then(|v| v.as_str())
                    .unwrap_or(&server);
                    
                if tls_enabled {
                    outbound.insert("tls".to_string(), json!({
                        "enabled": true,
                        "server_name": sni,
                        "insecure": skip_cert_verify
                    }));
                }
            }
            "tuic" => {
                outbound.insert("type".to_string(), json!("tuic"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let uuid = map.get(&key_uuid)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let password = map.get(&key_password)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                    
                outbound.insert("uuid".to_string(), json!(uuid));
                if !password.is_empty() {
                    outbound.insert("password".to_string(), json!(password));
                }
                
                let congestion = map.get(&key_congestion_control)
                    .and_then(|v| v.as_str())
                    .unwrap_or("cubic");
                outbound.insert("congestion_control".to_string(), json!(congestion));
                
                let sni = map.get(&key_sni)
                    .and_then(|v| v.as_str())
                    .unwrap_or(&server);
                outbound.insert("tls".to_string(), json!({
                    "enabled": true,
                    "server_name": sni,
                    "insecure": skip_cert_verify
                }));
            }
            _ => {
                continue;
            }
        }
        
        if gui_config.tcp_fast_open {
            outbound.insert("tcp_fast_open".to_string(), json!(true));
        }
        if gui_config.tcp_multipath {
            outbound.insert("tcp_multipath".to_string(), json!(true));
        }
        
        outbounds.push(serde_json::Value::Object(outbound));
    }
    
    // Master Selector "Proxy"
    let mut proxy_selector = serde_json::Map::new();
    proxy_selector.insert("type".to_string(), json!("selector"));
    proxy_selector.insert("tag".to_string(), json!("Proxy"));
    let mut selector_outbounds = vec!["Auto".to_string()];
    selector_outbounds.extend(node_tags.clone());
    proxy_selector.insert("outbounds".to_string(), json!(selector_outbounds));
    
    // Remember selected node tag (Store Cache)
    let default_node = gui_config.selected_node_tag.as_ref()
        .filter(|tag| node_tags.contains(tag) || *tag == "Auto")
        .cloned()
        .unwrap_or_else(|| "Auto".to_string());
    proxy_selector.insert("default".to_string(), json!(default_node));
    
    // Master URLTest "Auto"
    let mut auto_urltest = serde_json::Map::new();
    auto_urltest.insert("type".to_string(), json!("urltest"));
    auto_urltest.insert("tag".to_string(), json!("Auto"));
    auto_urltest.insert("outbounds".to_string(), json!(node_tags));
    auto_urltest.insert("url".to_string(), json!("http://cp.cloudflare.com/generate_204"));
    auto_urltest.insert("interval".to_string(), json!("3m"));
    
    let mut final_outbounds = vec![
        serde_json::Value::Object(proxy_selector),
        serde_json::Value::Object(auto_urltest),
    ];
    final_outbounds.extend(outbounds);
    
    // Standard direct/block/dns outbounds
    final_outbounds.push(json!({ "type": "direct", "tag": "direct" }));
    final_outbounds.push(json!({ "type": "block", "tag": "block" }));
    
    // Build DNS config
    let mut dns_servers = vec![
        json!({
            "tag": "dns_remote",
            "address": gui_config.dns_server_remote,
            "detour": "Proxy"
        }),
        json!({
            "tag": "dns_local",
            "address": gui_config.dns_server_local,
            "detour": "direct"
        })
    ];
    
    let mut dns_rules = vec![
        json!({ "clash_mode": "Direct", "server": "dns_local" }),
    ];
    
    // Resolve custom bypass domains via local DNS (prevents leaks and improves CDN speed)
    if !gui_config.custom_bypass_domains.is_empty() {
        dns_rules.push(json!({
            "domain_suffix": gui_config.custom_bypass_domains,
            "server": "dns_local"
        }));
    }

    // Resolve custom proxy domains via remote DNS or Fake-IP
    if !gui_config.custom_proxy_domains.is_empty() {
        let target_server = if gui_config.fake_ip { "dns_fakeip" } else { "dns_remote" };
        dns_rules.push(json!({
            "domain_suffix": gui_config.custom_proxy_domains,
            "server": target_server
        }));
    }
    
    if gui_config.fake_ip {
        dns_servers.push(json!({
            "tag": "dns_fakeip",
            "address": "fakeip",
            "inet4_range": "198.18.0.0/15"
        }));
        dns_rules.push(json!({ "rule_set": ["geosite-cn"], "server": "dns_local" }));
        dns_rules.push(json!({ "query_type": ["A", "AAAA"], "server": "dns_fakeip" }));
        dns_rules.push(json!({ "clash_mode": "Global", "server": "dns_fakeip" }));
    } else {
        dns_rules.push(json!({ "clash_mode": "Global", "server": "dns_remote" }));
        dns_rules.push(json!({ "rule_set": ["geosite-cn"], "server": "dns_local" }));
    }
    
    let dns = json!({
        "servers": dns_servers,
        "rules": dns_rules
    });
    
    // Build Inbounds
    let mut inbounds = vec![
        json!({
            "type": "mixed",
            "tag": "mixed-in",
            "listen": "127.0.0.1",
            "listen_port": gui_config.mixed_port
        })
    ];
    
    if gui_config.tun_mode {
        inbounds.push(json!({
            "type": "tun",
            "tag": "tun-in",
            "interface_name": "singbox-tun",
            "address": ["172.19.0.1/30"],
            "auto_route": true,
            "strict_route": true,
            "stack": "system"
        }));
    }
    
    // Build Rules
    let mut rules_list = vec![
        json!({ "action": "sniff", "sniffer": ["http", "tls", "quic", "dns"] }),
        json!({ "protocol": "dns", "action": "hijack-dns" }),
        json!({ "port": 53, "action": "hijack-dns" }),
        json!({ "clash_mode": "Direct", "outbound": "direct" }),
        json!({ "clash_mode": "Global", "outbound": "Proxy" }),
    ];

    // Inject custom rules
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

    // Default fallbacks
    rules_list.push(json!({ "ip_is_private": true, "outbound": "direct" }));
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
        "default_domain_resolver": "dns_local"
    });
    
    let config = json!({
        "log": {
            "level": "info",
            "timestamp": true
        },
        "dns": dns,
        "inbounds": inbounds,
        "outbounds": final_outbounds,
        "route": rules,
        "experimental": {
            "clash_api": {
                "external_controller": format!("127.0.0.1:{}", gui_config.api_port),
                "secret": "",
                "default_mode": gui_config.routing_mode.as_clash_mode()
            }
        }
    });
    
    Ok(config)
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
                && (rest.starts_with("http://") || rest.starts_with("https://")) {
                    current = rest.to_string();
                    stripped = true;
                    break;
                }
        }
        if !stripped {
            break;
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
    const FALLBACK_TAG: &str = "direct";
    let outbound = json!({ "type": "direct", "tag": FALLBACK_TAG });
    match config.get_mut("outbounds") {
        Some(serde_json::Value::Array(arr)) => arr.push(outbound),
        _ => {
            if let Some(root) = config.as_object_mut() {
                root.insert("outbounds".to_string(), json!([outbound]));
            }
        }
    }
    FALLBACK_TAG.to_string()
}

/// Apply in-app startup mitigations to a generated sing-box config:
/// - rewrite broken remote rule-set proxy URLs
/// - set `download_detour` to a **real** direct outbound tag (profiles often use `DIRECT`)
/// - pin `cache_file.path` under the app data directory
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

    // 2) Absolute cache_file path under app data dir
    let cache_path = get_app_dir()
        .join("cache.db")
        .to_string_lossy()
        .to_string();
    if let Some(root) = config.as_object_mut()
        && let Some(exp) = root.entry("experimental")
            .or_insert_with(|| json!({}))
            .as_object_mut()
    {
        let cache = exp
            .entry("cache_file")
            .or_insert_with(|| json!({ "enabled": true }));
        if let Some(cache_obj) = cache.as_object_mut() {
            cache_obj
                .entry("enabled")
                .or_insert_with(|| json!(true));
            cache_obj.insert("path".to_string(), json!(cache_path));
        }
    }
}

pub fn merge_native_json_profile(
    json_content: &str,
    gui_config: &GuiConfig,
) -> Result<serde_json::Value, String> {
    let mut config: serde_json::Value = serde_json::from_str(json_content)
        .map_err(|e| format!("Invalid native Sing-Box JSON: {}", e))?;
        
    let config_obj = config.as_object_mut()
        .ok_or_else(|| "Native Sing-Box JSON must be a JSON object".to_string())?;
        
    // 1. Inbounds setup
    let inbounds = config_obj.get_mut("inbounds")
        .and_then(|v| v.as_array_mut());
        
    let mut mixed_found = false;
    let mut tun_found = false;
    
    if let Some(arr) = inbounds {
        for val in arr.iter_mut() {
            if let Some(obj) = val.as_object_mut()
                && let Some(t) = obj.get("type").and_then(|t| t.as_str()) {
                    if t == "mixed" {
                        obj.insert("listen_port".to_string(), json!(gui_config.mixed_port));
                        mixed_found = true;
                    } else if t == "tun" {
                        obj.insert("interface_name".to_string(), json!("singbox-tun"));
                        obj.insert("auto_route".to_string(), json!(true));
                        obj.insert("strict_route".to_string(), json!(true));
                        obj.insert("stack".to_string(), json!("system"));
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
            arr.push(json!({
                "type": "tun",
                "tag": "tun-in",
                "interface_name": "singbox-tun",
                "address": ["172.19.0.1/30"],
                "auto_route": true,
                "strict_route": true,
                "stack": "system"
            }));
        } else if !gui_config.tun_mode && tun_found {
            arr.retain(|val| {
                val.get("type").and_then(|t| t.as_str()) != Some("tun")
            });
        }
    } else {
        let mut arr = vec![
            json!({
                "type": "mixed",
                "tag": "mixed-in",
                "listen": "127.0.0.1",
                "listen_port": gui_config.mixed_port
            })
        ];
        if gui_config.tun_mode {
            arr.push(json!({
                "type": "tun",
                "tag": "tun-in",
                "interface_name": "singbox-tun",
                "address": ["172.19.0.1/30"],
                "auto_route": true,
                "strict_route": true,
                "stack": "system"
            }));
        }
        config_obj.insert("inbounds".to_string(), serde_json::Value::Array(arr));
    }
    
    // 2. Clash API / experimental settings setup
    let default_mode = gui_config.routing_mode.as_clash_mode();
    let experimental = config_obj.get_mut("experimental")
        .and_then(|v| v.as_object_mut());
        
    if let Some(exp_obj) = experimental {
        if let Some(clash_api_val) = exp_obj.get_mut("clash_api") {
            if let Some(clash_api_obj) = clash_api_val.as_object_mut() {
                clash_api_obj.insert("external_controller".to_string(), json!(format!("127.0.0.1:{}", gui_config.api_port)));
                clash_api_obj.insert("default_mode".to_string(), json!(default_mode));
            }
        } else {
            exp_obj.insert("clash_api".to_string(), json!({
                "external_controller": format!("127.0.0.1:{}", gui_config.api_port),
                "secret": "",
                "default_mode": default_mode
            }));
        }
    } else {
        config_obj.insert("experimental".to_string(), json!({
            "clash_api": {
                "external_controller": format!("127.0.0.1:{}", gui_config.api_port),
                "secret": "",
                "default_mode": default_mode
            }
        }));
    }
    
    Ok(config)
}

/// Read the `experimental.clash_api.secret` value (if any) from the on-disk
/// `run_config.json` produced by `prepare_run_config`. Lets `api.rs` request
/// handlers (proxies / connections / mode) attach the `Authorization: Bearer
/// <secret>` header when a native profile was authored with a custom secret.
pub fn get_clash_api_secret() -> String {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<(String, Option<String>)>> =
        std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new((String::new(), None)));
    let path = get_app_dir().join("run_config.json");
    let mtime_key = match std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| format!("{}", d.as_nanos()))
    {
        Some(k) => k,
        None => return String::new(),
    };
    if let Ok(mut g) = cache.lock() {
        if g.0 == mtime_key {
            return g.1.clone().unwrap_or_default();
        }
        let secret = std::fs::read_to_string(&path)
            .ok()
            .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
            .and_then(|v| {
                v.pointer("/experimental/clash_api/secret")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
            });
        *g = (mtime_key, secret.clone());
        secret.unwrap_or_default()
    } else {
        String::new()
    }
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
    let trimmed = content.trim();
    let res = if trimmed.starts_with('{') || trimmed.starts_with('[') {
        merge_native_json_profile(&content, gui_config)
    } else {
        convert_clash_to_singbox(&content, gui_config)
    };
    match res {
        Ok(val) => match serde_json::to_string_pretty(&val) {
            Ok(json_str) => json_str,
            Err(e) => format!("Failed to serialize config: {}", e),
        },
        Err(e) => format!("Failed to generate preview: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::RoutingMode;

    #[test]
    fn test_parse_subscription_userinfo() {
        let (u, d, t, e) = parse_subscription_userinfo(
            "upload=100; download=200; total=1000; expire=1710000000",
        );
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
            config["outbounds"]
                .as_array()
                .unwrap()
                .iter()
                .any(|o| o.get("tag").and_then(|t| t.as_str()) == Some("direct")
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
        // Drive shipped mitigate_run_config on a config shaped like the user's
        // active native profile (DIRECT outbound + gh-proxy rule-sets).
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let src = std::path::PathBuf::from(&appdata)
            .join("sing-box-gui")
            .join("run_config.json");
        if !src.exists() {
            // CI / machines without the user profile — unit shape tests above still cover logic.
            return;
        }
        let raw = std::fs::read_to_string(&src).expect("read run_config");
        let mut config: serde_json::Value =
            serde_json::from_str(&raw).expect("parse run_config");
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
        let scratch = std::env::temp_dir().join("grok-goal-b63a7b864ef0").join("implementer");
        let _ = std::fs::create_dir_all(&scratch);
        let out_path = scratch.join("run_config_mitigated_full.json");
        let pretty = serde_json::to_string_pretty(&config).unwrap();
        std::fs::write(&out_path, &pretty).expect("write mitigated config");

        // If managed core is present, require `sing-box check` + short `run` survival.
        let core = std::path::PathBuf::from(&appdata)
            .join("sing-box-gui")
            .join("bin")
            .join(if cfg!(windows) {
                "sing-box.exe"
            } else {
                "sing-box"
            });
        if !core.exists() {
            return;
        }
        let check = std::process::Command::new(&core)
            .args(["check", "-c", &out_path.to_string_lossy()])
            .output()
            .expect("run check");
        assert!(
            check.status.success(),
            "sing-box check failed: {}",
            String::from_utf8_lossy(&check.stderr)
        );

        let workdir = std::path::PathBuf::from(&appdata).join("sing-box-gui");
        let mut child = std::process::Command::new(&core)
            .args(["run", "-c", &out_path.to_string_lossy()])
            .current_dir(&workdir)
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
        let mut gui_config = GuiConfig::default();
        gui_config.routing_mode = RoutingMode::Global;
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
    fn test_custom_domains_dns_injection() {
        let mut gui_config = GuiConfig::default();
        gui_config.custom_bypass_domains = vec!["bypass.me".to_string()];
        gui_config.custom_proxy_domains = vec!["proxy.me".to_string()];
        
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
        let dns_rules = res.get("dns").unwrap().get("rules").unwrap().as_array().unwrap();
        
        // Find custom bypass domains rule
        let bypass_rule = dns_rules.iter().find(|r| {
            r.get("domain_suffix")
                .and_then(|ds| ds.as_array())
                .map(|arr| arr.iter().any(|v| v.as_str() == Some("bypass.me")))
                .unwrap_or(false)
        }).unwrap();
        assert_eq!(bypass_rule.get("server").unwrap().as_str(), Some("dns_local"));
        
        // Find custom proxy domains rule
        let proxy_rule = dns_rules.iter().find(|r| {
            r.get("domain_suffix")
                .and_then(|ds| ds.as_array())
                .map(|arr| arr.iter().any(|v| v.as_str() == Some("proxy.me")))
                .unwrap_or(false)
        }).unwrap();
        assert_eq!(proxy_rule.get("server").unwrap().as_str(), Some("dns_remote"));
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
}
