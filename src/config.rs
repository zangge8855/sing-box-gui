use std::fs;
use std::path::PathBuf;
use serde_json::json;
use crate::state::{GuiConfig, ProxyNode};

pub fn get_app_dir() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        });
    let dir = base.join("sing-box-gui");
    let _ = fs::create_dir_all(&dir);
    let _ = fs::create_dir_all(dir.join("profiles"));
    let _ = fs::create_dir_all(dir.join("bin"));
    dir
}

pub fn get_config_path() -> PathBuf {
    get_app_dir().join("gui_config.json")
}

pub fn load_gui_config() -> GuiConfig {
    let path = get_config_path();
    let mut config = if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<GuiConfig>(&content) {
                cfg
            } else {
                GuiConfig::default()
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
            if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Control Panel\\International") {
                if let Ok(locale) = hkcu.get_value::<String, _>("LocaleName") {
                    if locale.to_lowercase().starts_with("zh") {
                        config.language = crate::state::Language::Zh;
                        let _ = save_gui_config(&config);
                    }
                }
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
    fs::write(path, content)
        .map_err(|e| format!("Failed to write config file: {}", e))?;
    Ok(())
}

fn decode_base64_padded(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    
    // Attempt standard decode
    if let Ok(bytes) = STANDARD.decode(input) {
        if let Ok(s) = String::from_utf8(bytes) {
            return Some(s);
        }
    }
    
    // Attempt with padding
    let mut padded = input.to_string();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }
    if let Ok(bytes) = STANDARD.decode(&padded) {
        if let Ok(s) = String::from_utf8(bytes) {
            return Some(s);
        }
    }
    
    // Attempt URL-safe decode
    use base64::engine::general_purpose::URL_SAFE;
    if let Ok(bytes) = URL_SAFE.decode(input) {
        if let Ok(s) = String::from_utf8(bytes) {
            return Some(s);
        }
    }
    
    let mut padded_url = input.to_string();
    while padded_url.len() % 4 != 0 {
        padded_url.push('=');
    }
    if let Ok(bytes) = URL_SAFE.decode(&padded_url) {
        if let Ok(s) = String::from_utf8(bytes) {
            return Some(s);
        }
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
                            map.insert(serde_yaml::Value::String("cipher".to_string()), serde_yaml::Value::String(parts[0].to_string()));
                            map.insert(serde_yaml::Value::String("password".to_string()), serde_yaml::Value::String(parts[1].to_string()));
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
                        map.insert(serde_yaml::Value::String("cipher".to_string()), serde_yaml::Value::String(parts[0].to_string()));
                        map.insert(serde_yaml::Value::String("password".to_string()), serde_yaml::Value::String(parts[1].to_string()));
                    }
                }
            }
        }
        "vmess" => {
            map.insert(serde_yaml::Value::String("type".to_string()), serde_yaml::Value::String("vmess".to_string()));
            
            let body_b64 = link.trim_start_matches("vmess://").split('?').next().unwrap().split('#').next().unwrap();
            if let Some(decoded_json) = decode_base64_padded(body_b64) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&decoded_json) {
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
        }
        "vless" => {
            map.insert(serde_yaml::Value::String("type".to_string()), serde_yaml::Value::String("vless".to_string()));
            
            let uuid = url.username().to_string();
            map.insert(serde_yaml::Value::String("uuid".to_string()), serde_yaml::Value::String(uuid));
            
            let mut tls_enabled = false;
            let mut sni = String::new();
            let mut flow = String::new();
            for (k, v) in url.query_pairs() {
                if k == "security" && v == "tls" {
                    tls_enabled = true;
                }
                if k == "sni" {
                    sni = v.to_string();
                }
                if k == "flow" {
                    flow = v.to_string();
                }
            }
            if tls_enabled {
                map.insert(serde_yaml::Value::String("tls".to_string()), serde_yaml::Value::Bool(true));
                if !sni.is_empty() {
                    map.insert(serde_yaml::Value::String("sni".to_string()), serde_yaml::Value::String(sni));
                }
            }
            if !flow.is_empty() {
                map.insert(serde_yaml::Value::String("flow".to_string()), serde_yaml::Value::String(flow));
            }
        }
        "trojan" => {
            map.insert(serde_yaml::Value::String("type".to_string()), serde_yaml::Value::String("trojan".to_string()));
            
            let password = url.username().to_string();
            map.insert(serde_yaml::Value::String("password".to_string()), serde_yaml::Value::String(password));
            
            let mut tls_enabled = true;
            let mut sni = String::new();
            for (k, v) in url.query_pairs() {
                if k == "security" && v == "none" {
                    tls_enabled = false;
                }
                if k == "sni" {
                    sni = v.to_string();
                }
            }
            if tls_enabled {
                map.insert(serde_yaml::Value::String("tls".to_string()), serde_yaml::Value::Bool(true));
                if !sni.is_empty() {
                    map.insert(serde_yaml::Value::String("sni".to_string()), serde_yaml::Value::String(sni));
                }
            }
        }
        "hysteria" | "hysteria2" | "hy2" => {
            map.insert(serde_yaml::Value::String("type".to_string()), serde_yaml::Value::String("hysteria2".to_string()));
            let password = url.username().to_string();
            map.insert(serde_yaml::Value::String("password".to_string()), serde_yaml::Value::String(password));
            
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
            let password = url.password().unwrap_or("").to_string();
            
            map.insert(serde_yaml::Value::String("uuid".to_string()), serde_yaml::Value::String(uuid));
            if !password.is_empty() {
                map.insert(serde_yaml::Value::String("password".to_string()), serde_yaml::Value::String(password));
            }
            
            let mut sni = String::new();
            let mut congestion = String::new();
            for (k, v) in url.query_pairs() {
                if k == "sni" {
                    sni = v.to_string();
                }
                if k == "congestion_control" || k == "congestion-controller" {
                    congestion = v.to_string();
                }
            }
            if !sni.is_empty() {
                map.insert(serde_yaml::Value::String("sni".to_string()), serde_yaml::Value::String(sni));
            }
            if !congestion.is_empty() {
                map.insert(serde_yaml::Value::String("congestion-controller".to_string()), serde_yaml::Value::String(congestion));
            }
        }
        _ => return None,
    }
    
    Some(map)
}

pub fn normalize_profile_content(content: &str) -> String {
    let content_trimmed = content.trim();
    
    if content_trimmed.contains("proxies:") {
        return content_trimmed.to_string();
    }
    
    let decoded_content = if let Some(decoded) = decode_base64_padded(content_trimmed) {
        decoded
    } else {
        content_trimmed.to_string()
    };
    
    let mut proxies = Vec::new();
    for line in decoded_content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(map) = parse_share_link(line) {
            proxies.push(serde_yaml::Value::Mapping(map));
        }
    }
    
    if !proxies.is_empty() {
        let mut yaml_map = serde_yaml::Mapping::new();
        yaml_map.insert(serde_yaml::Value::String("proxies".to_string()), serde_yaml::Value::Sequence(proxies));
        if let Ok(yaml_str) = serde_yaml::to_string(&serde_yaml::Value::Mapping(yaml_map)) {
            return yaml_str;
        }
    }
    
    content.to_string()
}

pub fn get_profile_path(profile_id: &str) -> PathBuf {
    get_app_dir().join("profiles").join(format!("{}.json", profile_id))
}

pub fn parse_clash_yaml_nodes(yaml_content: &str) -> Result<Vec<ProxyNode>, String> {
    let normalized = normalize_profile_content(yaml_content);
    let yaml: serde_yaml::Value = serde_yaml::from_str(&normalized)
        .map_err(|e| format!("YAML parsing failed: {}", e))?;
    
    let proxies_val = yaml.get("proxies")
        .ok_or_else(|| "No 'proxies' key found in Clash config".to_string())?;
    
    let proxies_arr = proxies_val.as_sequence()
        .ok_or_else(|| "'proxies' must be a sequence/list".to_string())?;
    
    let mut nodes = Vec::new();
    for item in proxies_arr {
        if let Some(map) = item.as_mapping() {
            let name = map.get(&serde_yaml::Value::String("name".to_string()))
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed")
                .to_string();
                
            let node_type = map.get(&serde_yaml::Value::String("type".to_string()))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
                
            let server = map.get(&serde_yaml::Value::String("server".to_string()))
                .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|i| i.to_string())))
                .unwrap_or_else(|| "127.0.0.1".to_string());
                
            let port = map.get(&serde_yaml::Value::String("port".to_string()))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u16;
                
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

pub fn convert_clash_to_singbox(
    yaml_content: &str,
    gui_config: &GuiConfig,
) -> Result<serde_json::Value, String> {
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
        
        let name = map.get(&serde_yaml::Value::String("name".to_string()))
            .and_then(|v| v.as_str())
            .unwrap_or("Unnamed")
            .to_string();
            
        let node_type = map.get(&serde_yaml::Value::String("type".to_string()))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
            
        let server = map.get(&serde_yaml::Value::String("server".to_string()))
            .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|i| i.to_string())))
            .unwrap_or_else(|| "127.0.0.1".to_string());
            
        let port = map.get(&serde_yaml::Value::String("port".to_string()))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u16;
            
        outbound.insert("tag".to_string(), json!(name));
        node_tags.push(name.clone());
        
        match node_type {
            "ss" => {
                outbound.insert("type".to_string(), json!("shadowsocks"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let cipher = map.get(&serde_yaml::Value::String("cipher".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("aes-256-gcm");
                let password = map.get(&serde_yaml::Value::String("password".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                    
                outbound.insert("method".to_string(), json!(cipher));
                outbound.insert("password".to_string(), json!(password));
            }
            "vmess" => {
                outbound.insert("type".to_string(), json!("vmess"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let uuid = map.get(&serde_yaml::Value::String("uuid".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let alter_id = map.get(&serde_yaml::Value::String("alterId".to_string()))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let cipher = map.get(&serde_yaml::Value::String("cipher".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("auto");
                    
                outbound.insert("uuid".to_string(), json!(uuid));
                outbound.insert("security".to_string(), json!(cipher));
                outbound.insert("alter_id".to_string(), json!(alter_id));
                
                let tls_enabled = map.get(&serde_yaml::Value::String("tls".to_string()))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let sni = map.get(&serde_yaml::Value::String("sni".to_string()))
                    .and_then(|v| v.as_str())
                    .or_else(|| map.get(&serde_yaml::Value::String("servername".to_string())).and_then(|v| v.as_str()))
                    .unwrap_or(&server);
                    
                if tls_enabled {
                    outbound.insert("tls".to_string(), json!({
                        "enabled": true,
                        "server_name": sni,
                        "insecure": true
                    }));
                }
                
                let network = map.get(&serde_yaml::Value::String("network".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("tcp");
                    
                if network == "ws" {
                    let ws_opts = map.get(&serde_yaml::Value::String("ws-opts".to_string()));
                    let mut path = "/".to_string();
                    let mut headers_map = serde_json::Map::new();
                    
                    if let Some(ws_val) = ws_opts {
                        if let Some(ws_map) = ws_val.as_mapping() {
                            path = ws_map.get(&serde_yaml::Value::String("path".to_string()))
                                .and_then(|v| v.as_str())
                                .unwrap_or("/")
                                .to_string();
                            if let Some(headers) = ws_map.get(&serde_yaml::Value::String("headers".to_string())).and_then(|v| v.as_mapping()) {
                                for (k, v) in headers {
                                    if let (Some(ks), Some(vs)) = (k.as_str(), v.as_str()) {
                                        headers_map.insert(ks.to_string(), json!(vs));
                                    }
                                }
                            }
                        }
                    }
                    
                    outbound.insert("transport".to_string(), json!({
                        "type": "ws",
                        "path": path,
                        "headers": headers_map
                    }));
                }
            }
            "vless" => {
                outbound.insert("type".to_string(), json!("vless"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let uuid = map.get(&serde_yaml::Value::String("uuid".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let flow = map.get(&serde_yaml::Value::String("flow".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                    
                outbound.insert("uuid".to_string(), json!(uuid));
                if !flow.is_empty() {
                    outbound.insert("flow".to_string(), json!(flow));
                }
                
                let tls_enabled = map.get(&serde_yaml::Value::String("tls".to_string()))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let sni = map.get(&serde_yaml::Value::String("sni".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&server);
                    
                if tls_enabled {
                    outbound.insert("tls".to_string(), json!({
                        "enabled": true,
                        "server_name": sni
                    }));
                }
            }
            "trojan" => {
                outbound.insert("type".to_string(), json!("trojan"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let password = map.get(&serde_yaml::Value::String("password".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                outbound.insert("password".to_string(), json!(password));
                
                let tls_enabled = map.get(&serde_yaml::Value::String("tls".to_string()))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let sni = map.get(&serde_yaml::Value::String("sni".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&server);
                    
                if tls_enabled {
                    outbound.insert("tls".to_string(), json!({
                        "enabled": true,
                        "server_name": sni
                    }));
                }
            }
            "hysteria" => {
                outbound.insert("type".to_string(), json!("hysteria"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let auth = map.get(&serde_yaml::Value::String("auth_str".to_string()))
                    .and_then(|v| v.as_str())
                    .or_else(|| map.get(&serde_yaml::Value::String("auth-str".to_string())).and_then(|v| v.as_str()))
                    .or_else(|| map.get(&serde_yaml::Value::String("password".to_string())).and_then(|v| v.as_str()))
                    .unwrap_or("");
                outbound.insert("auth_str".to_string(), json!(auth));
                
                let sni = map.get(&serde_yaml::Value::String("sni".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&server);
                outbound.insert("tls".to_string(), json!({
                    "enabled": true,
                    "server_name": sni,
                    "insecure": true
                }));
            }
            "hysteria2" => {
                outbound.insert("type".to_string(), json!("hysteria2"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let password = map.get(&serde_yaml::Value::String("password".to_string()))
                    .and_then(|v| v.as_str())
                    .or_else(|| map.get(&serde_yaml::Value::String("auth-str".to_string())).and_then(|v| v.as_str()))
                    .unwrap_or("");
                outbound.insert("password".to_string(), json!(password));
                
                let sni = map.get(&serde_yaml::Value::String("sni".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&server);
                outbound.insert("tls".to_string(), json!({
                    "enabled": true,
                    "server_name": sni,
                    "insecure": true
                }));
            }
            "socks" | "socks5" => {
                outbound.insert("type".to_string(), json!("socks"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let username = map.get(&serde_yaml::Value::String("username".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let password = map.get(&serde_yaml::Value::String("password".to_string()))
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
                
                let username = map.get(&serde_yaml::Value::String("username".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let password = map.get(&serde_yaml::Value::String("password".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !username.is_empty() {
                    outbound.insert("username".to_string(), json!(username));
                }
                if !password.is_empty() {
                    outbound.insert("password".to_string(), json!(password));
                }
            }
            "tuic" => {
                outbound.insert("type".to_string(), json!("tuic"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let uuid = map.get(&serde_yaml::Value::String("uuid".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let password = map.get(&serde_yaml::Value::String("password".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let congestion = map.get(&serde_yaml::Value::String("congestion-controller".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("bbr");
                    
                outbound.insert("uuid".to_string(), json!(uuid));
                if !password.is_empty() {
                    outbound.insert("password".to_string(), json!(password));
                }
                outbound.insert("congestion_control".to_string(), json!(congestion));
                
                let sni = map.get(&serde_yaml::Value::String("sni".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&server);
                outbound.insert("tls".to_string(), json!({
                    "enabled": true,
                    "server_name": sni,
                    "insecure": true
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
                "secret": ""
            }
        }
    });
    
    Ok(config)
}

pub fn merge_native_json_profile(
    json_content: &str,
    gui_config: &GuiConfig,
) -> Result<serde_json::Value, String> {
    let mut config: serde_json::Value = serde_json::from_str(json_content)
        .map_err(|e| format!("Invalid native Sing-Box JSON: {}", e))?;
        
    // 1. Inbounds setup
    let inbounds = config.get_mut("inbounds")
        .and_then(|v| v.as_array_mut());
        
    let mut mixed_found = false;
    let mut tun_found = false;
    
    if let Some(arr) = inbounds {
        for val in arr.iter_mut() {
            if let Some(obj) = val.as_object_mut() {
                if let Some(t) = obj.get("type").and_then(|t| t.as_str()) {
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
            // Remove tun inbound if user disabled tun in GUI
            arr.retain(|val| {
                val.get("type").and_then(|t| t.as_str()) != Some("tun")
            });
        }
    } else {
        // No inbounds array at all, create it
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
        config.as_object_mut().unwrap().insert("inbounds".to_string(), serde_json::Value::Array(arr));
    }
    
    // 2. Clash API / experimental settings setup
    let experimental = config.get_mut("experimental")
        .and_then(|v| v.as_object_mut());
        
    if let Some(exp_obj) = experimental {
        if let Some(clash_api_val) = exp_obj.get_mut("clash_api") {
            if let Some(clash_api_obj) = clash_api_val.as_object_mut() {
                clash_api_obj.insert("external_controller".to_string(), json!(format!("127.0.0.1:{}", gui_config.api_port)));
            }
        } else {
            exp_obj.insert("clash_api".to_string(), json!({
                "external_controller": format!("127.0.0.1:{}", gui_config.api_port),
                "secret": ""
            }));
        }
    } else if let Some(config_obj) = config.as_object_mut() {
        config_obj.insert("experimental".to_string(), json!({
            "clash_api": {
                "external_controller": format!("127.0.0.1:{}", gui_config.api_port),
                "secret": ""
            }
        }));
    }
    
    Ok(config)
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
    fn test_user_actual_config() {
        let app_dir = get_app_dir();
        let active_id = "1783319263498";
        let path = app_dir.join("profiles").join(format!("{}.json", active_id));
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap();
            let mut config = GuiConfig::default();
            config.api_port = 9090;
            config.mixed_port = 2080;
            let final_config = convert_clash_to_singbox(&content, &config).unwrap();
            let out_str = serde_json::to_string_pretty(&final_config).unwrap();
            let temp_path = std::env::temp_dir().join("test_run_config.json");
            std::fs::write(&temp_path, out_str).unwrap();
            
            let core_path = app_dir.join("bin").join("sing-box.exe");
            if core_path.exists() {
                let mut cmd = std::process::Command::new(&core_path);
                cmd.args(&["check", "-c", &temp_path.to_string_lossy()]);
                cmd.env("ENABLE_DEPRECATED_LEGACY_DNS_SERVERS", "true");
                cmd.env("ENABLE_DEPRECATED_MISSING_DOMAIN_RESOLVER", "true");
                let status = cmd.status().unwrap();
                assert!(status.success(), "sing-box check failed!");
            }
        }
    }
}

