use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Tab {
    #[default]
    Dashboard,
    Proxies,
    Profiles,
    Rules,
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

impl RoutingMode {
    /// Clash-compatible mode string for sing-box clash_api /configs.
    pub fn as_clash_mode(&self) -> &'static str {
        match self {
            RoutingMode::Rule => "Rule",
            RoutingMode::Global => "Global",
            RoutingMode::Direct => "Direct",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
    /// Seconds remaining before auto-dismiss (Tick decrements).
    pub remaining_secs: u8,
}

impl Toast {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToastKind::Success,
            remaining_secs: 3,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToastKind::Error,
            remaining_secs: 5,
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: ToastKind::Info,
            remaining_secs: 3,
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
    /// Bytes uploaded (from subscription-userinfo), if known.
    #[serde(default)]
    pub traffic_upload: Option<u64>,
    #[serde(default)]
    pub traffic_download: Option<u64>,
    #[serde(default)]
    pub traffic_total: Option<u64>,
    /// Unix timestamp expire, if known.
    #[serde(default)]
    pub expire_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuleField {
    BypassDomains,
    ProxyDomains,
    BypassIps,
    ProxyIps,
}

impl RuleField {
    pub fn as_str(self) -> &'static str {
        match self {
            RuleField::BypassDomains => "bypass_domains",
            RuleField::ProxyDomains => "proxy_domains",
            RuleField::BypassIps => "bypass_ips",
            RuleField::ProxyIps => "proxy_ips",
        }
    }

    pub fn is_ip(self) -> bool {
        matches!(self, RuleField::BypassIps | RuleField::ProxyIps)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFilter {
    #[default]
    All,
    Info,
    Warn,
    Error,
}

impl LogFilter {
    pub fn matches(self, line: &str) -> bool {
        let u = line.to_uppercase();
        match self {
            LogFilter::All => true,
            LogFilter::Error => {
                u.contains("ERROR") || u.contains("FATAL") || u.contains("FAILED")
            }
            LogFilter::Warn => {
                u.contains("WARN")
                    || u.contains("WARNING")
                    || u.contains("ERROR")
                    || u.contains("FATAL")
                    || u.contains("FAILED")
            }
            LogFilter::Info => {
                u.contains("INFO")
                    || u.contains("WARN")
                    || u.contains("WARNING")
                    || u.contains("ERROR")
                    || u.contains("FATAL")
                    || u.contains("FAILED")
            }
        }
    }
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
    pub theme: AppTheme,
    #[serde(default)]
    pub auto_start_core: bool,
    #[serde(default)]
    pub auto_sys_proxy: bool,
    #[serde(default)]
    pub custom_bypass_domains: Vec<String>,
    #[serde(default)]
    pub custom_proxy_domains: Vec<String>,
    #[serde(default)]
    pub custom_bypass_ips: Vec<String>,
    #[serde(default)]
    pub custom_proxy_ips: Vec<String>,
    /// Auto-update subscriptions every N hours (0 = disabled).
    #[serde(default)]
    pub auto_update_interval_hours: u32,
    /// When stopping the core, also disable system proxy (default true).
    #[serde(default = "default_true")]
    pub disable_proxy_on_core_stop: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AppTheme {
    #[default]
    Auto,
    Dark,
    Light,
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
            theme: AppTheme::Auto,
            auto_start_core: false,
            auto_sys_proxy: false,
            custom_bypass_domains: Vec::new(),
            custom_proxy_domains: Vec::new(),
            custom_bypass_ips: Vec::new(),
            custom_proxy_ips: Vec::new(),
            auto_update_interval_hours: 0,
            disable_proxy_on_core_stop: true,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    NotChecked,
    Checking,
    UpToDate,
    NewVersion(String),
    Error(String),
}
