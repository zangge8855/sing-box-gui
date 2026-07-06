use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Tab {
    #[default]
    Dashboard,
    Proxies,
    Profiles,
    Logs,
    Settings,
    Connections,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingMode {
    Rule,
    Global,
    Direct,
}

impl Default for RoutingMode {
    fn default() -> Self {
        RoutingMode::Rule
    }
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub url: String,
    pub file_path: String,
    pub is_subscription: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Language {
    #[default]
    En,
    Zh,
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiConfig {
    pub subscriptions: Vec<Profile>,
    pub active_profile_id: Option<String>,
    pub mixed_port: u16,
    pub api_port: u16,
    pub routing_mode: RoutingMode,
    pub core_path: Option<String>,
    pub dns_server_local: String,
    pub dns_server_remote: String,
    pub start_on_boot: bool,
    pub tun_mode: bool,
    pub system_proxy_enabled: bool,
    pub language: Language,
    pub selected_node_tag: Option<String>,
    pub fake_ip: bool,
    pub tcp_fast_open: bool,
    pub tcp_multipath: bool,
    pub close_core_on_exit: bool,
}

impl Default for GuiConfig {
    fn default() -> Self {
        let mut lang = Language::En;
        #[cfg(target_os = "windows")]
        {
            use winreg::RegKey;
            use winreg::enums::HKEY_CURRENT_USER;
            if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Control Panel\\International") {
                if let Ok(locale) = hkcu.get_value::<String, _>("LocaleName") {
                    if locale.to_lowercase().starts_with("zh") {
                        lang = Language::Zh;
                    }
                }
            }
        }
        Self {
            subscriptions: Vec::new(),
            active_profile_id: None,
            mixed_port: 2080,
            api_port: 9090,
            routing_mode: RoutingMode::Rule,
            core_path: None,
            dns_server_local: "223.5.5.5".to_string(),
            dns_server_remote: "8.8.8.8".to_string(),
            start_on_boot: false,
            tun_mode: false,
            system_proxy_enabled: false,
            language: lang,
            selected_node_tag: None,
            fake_ip: false,
            tcp_fast_open: false,
            tcp_multipath: false,
            close_core_on_exit: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProxyNode {
    pub name: String,
    pub node_type: String,
    pub server: String,
    pub port: u16,
    pub latency: Option<u64>,
}

#[derive(Debug, Default, Clone)]
pub struct Bandwidth {
    pub up: u64,
    pub down: u64,
}
