use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde_json::json;
use crate::state::{GuiConfig, Profile, ProxyNode, RoutingMode};

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
    if path.exists() {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str::<GuiConfig>(&content) {
                return config;
            }
        }
    }
    let default = GuiConfig::default();
    let _ = save_gui_config(&default);
    default
}

pub fn save_gui_config(config: &GuiConfig) -> Result<(), String> {
    let path = get_config_path();
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(path, content)
        .map_err(|e| format!("Failed to write config file: {}", e))?;
    Ok(())
}

pub fn get_profile_path(profile_id: &str) -> PathBuf {
    get_app_dir().join("profiles").join(format!("{}.json", profile_id))
}

pub fn parse_clash_yaml_nodes(yaml_content: &str) -> Result<Vec<ProxyNode>, String> {
    let yaml: serde_yaml::Value = serde_yaml::from_str(yaml_content)
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
    let yaml: serde_yaml::Value = serde_yaml::from_str(yaml_content)
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
            "hysteria2" | "hysteria" => {
                outbound.insert("type".to_string(), json!("hysteria2"));
                outbound.insert("server".to_string(), json!(server));
                outbound.insert("server_port".to_string(), json!(port));
                
                let password = map.get(&serde_yaml::Value::String("password".to_string()))
                    .and_then(|v| v.as_str())
                    .or_else(|| map.get(&serde_yaml::Value::String("auth-str".to_string())).and_then(|v| v.as_str()))
                    .unwrap_or("");
                outbound.insert("password".to_string(), json!(password));
            }
            _ => {
                // Skip unsupported node types so we don't break
                continue;
            }
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
    proxy_selector.insert("default".to_string(), json!("Auto"));
    
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
    final_outbounds.push(json!({ "type": "dns", "tag": "dns-out" }));
    
    // Build DNS config
    let dns = json!({
        "servers": [
            {
                "tag": "dns_local",
                "address": gui_config.dns_server_local,
                "detour": "direct"
            },
            {
                "tag": "dns_remote",
                "address": gui_config.dns_server_remote,
                "detour": "Proxy"
            }
        ],
        "rules": [
            { "outbound": "any", "server": "dns_local" },
            { "clash_mode": "Direct", "server": "dns_local" },
            { "clash_mode": "Global", "server": "dns_remote" },
            { "rule_set": ["geosite-cn"], "server": "dns_local" }
        ]
    });
    
    // Build Inbounds
    let mut inbounds = vec![
        json!({
            "type": "mixed",
            "tag": "mixed-in",
            "listen": "127.0.0.1",
            "listen_port": gui_config.mixed_port,
            "sniff": true
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
            "stack": "system",
            "sniff": true
        }));
    }
    
    // Build Rules
    let rules = json!({
        "rules": [
            { "protocol": "dns", "outbound": "dns-out" },
            { "port": 53, "outbound": "dns-out" },
            { "clash_mode": "Direct", "outbound": "direct" },
            { "clash_mode": "Global", "outbound": "Proxy" },
            { "ip_is_private": true, "outbound": "direct" },
            { "rule_set": ["geosite-cn", "geoip-cn"], "outbound": "direct" }
        ],
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
        "auto_detect_interface": true
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
}

