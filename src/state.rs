use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Tab {
    #[default]
    Dashboard,
    Proxies,
    Profiles,
    Logs,
    Settings,
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

impl RoutingMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            RoutingMode::Rule => "Rule",
            RoutingMode::Global => "Global",
            RoutingMode::Direct => "Direct",
        }
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
}

impl Default for GuiConfig {
    fn default() -> Self {
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
