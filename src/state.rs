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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RoutingMode {
    #[default]
    Rule,
    Global,
    Direct,
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
    /// Seconds to show a toast: scale with message length so long errors stay
    /// on screen long enough to read; clamped to [3, 20].
    fn duration_for(msg: &str) -> u8 {
        let char_count = msg.chars().count();
        let base: u8 = if char_count < 80 {
            3
        } else if char_count < 240 {
            6
        } else {
            10
        };
        // Extra second per ~25 chars in long messages, capped at 20
        let extra = (char_count as u32 / 25).min(10) as u8;
        base.saturating_add(extra).clamp(3, 20)
    }

    pub fn success(message: impl Into<String>) -> Self {
        let message = message.into();
        let remaining_secs = Self::duration_for(&message);
        Self {
            message,
            kind: ToastKind::Success,
            remaining_secs,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        let message = message.into();
        let remaining_secs = Self::duration_for(&message).max(5);
        Self {
            message,
            kind: ToastKind::Error,
            remaining_secs,
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        let message = message.into();
        let remaining_secs = Self::duration_for(&message);
        Self {
            message,
            kind: ToastKind::Info,
            remaining_secs,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    #[allow(dead_code)]
    pub fn matches(self, line: &str) -> bool {
        self.matches_upper(&line.to_uppercase())
    }

    pub fn matches_upper(self, uppercase_line: &str) -> bool {
        let u = uppercase_line;
        match self {
            LogFilter::All => true,
            LogFilter::Error => u.contains("ERROR") || u.contains("FATAL") || u.contains("FAILED"),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// True only when this application enabled the currently configured
    /// system proxy. Persisting ownership lets a restarted GUI safely clean up
    /// a proxy it previously installed without touching an unrelated proxy.
    #[serde(default)]
    pub system_proxy_owned: bool,
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
    /// URL used by Clash API latency tests (default Cloudflare 204).
    #[serde(default = "default_latency_url")]
    pub latency_test_url: String,
    /// Latency test timeout in milliseconds.
    #[serde(default = "default_latency_timeout_ms")]
    pub latency_test_timeout_ms: u32,
}

fn default_latency_url() -> String {
    "http://cp.cloudflare.com/generate_204".to_string()
}

fn default_latency_timeout_ms() -> u32 {
    2000
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
        #[allow(unused_mut)]
        let mut lang = Language::En;
        #[cfg(target_os = "windows")]
        {
            use winreg::RegKey;
            use winreg::enums::HKEY_CURRENT_USER;
            if let Ok(hkcu) =
                RegKey::predef(HKEY_CURRENT_USER).open_subkey("Control Panel\\International")
                && let Ok(locale) = hkcu.get_value::<String, _>("LocaleName")
                && locale.to_lowercase().starts_with("zh")
            {
                lang = Language::Zh;
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
            system_proxy_owned: false,
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
            latency_test_url: default_latency_url(),
            latency_test_timeout_ms: default_latency_timeout_ms(),
        }
    }
}

/// Editable runtime-affecting settings. Language and theme intentionally stay
/// on [`GuiConfig`] because they are applied and persisted immediately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSettingsDraft {
    pub mixed_port: String,
    pub api_port: String,
    pub dns_server_local: String,
    pub dns_server_remote: String,
    pub core_path: String,
    pub latency_test_url: String,
    pub latency_test_timeout_ms: String,
    pub start_on_boot: bool,
    pub tun_mode: bool,
    pub fake_ip: bool,
    pub tcp_fast_open: bool,
    pub tcp_multipath: bool,
    pub close_core_on_exit: bool,
    pub auto_start_core: bool,
    pub auto_sys_proxy: bool,
    pub auto_update_interval_hours: u32,
    pub disable_proxy_on_core_stop: bool,
}

impl RuntimeSettingsDraft {
    pub fn from_config(config: &GuiConfig) -> Self {
        Self {
            mixed_port: config.mixed_port.to_string(),
            api_port: config.api_port.to_string(),
            dns_server_local: config.dns_server_local.clone(),
            dns_server_remote: config.dns_server_remote.clone(),
            core_path: config.core_path.clone().unwrap_or_default(),
            latency_test_url: config.latency_test_url.clone(),
            latency_test_timeout_ms: config.latency_test_timeout_ms.to_string(),
            start_on_boot: config.start_on_boot,
            tun_mode: config.tun_mode,
            fake_ip: config.fake_ip,
            tcp_fast_open: config.tcp_fast_open,
            tcp_multipath: config.tcp_multipath,
            close_core_on_exit: config.close_core_on_exit,
            auto_start_core: config.auto_start_core,
            auto_sys_proxy: config.auto_sys_proxy,
            auto_update_interval_hours: config.auto_update_interval_hours,
            disable_proxy_on_core_stop: config.disable_proxy_on_core_stop,
        }
    }

    pub fn has_pending_changes(&self, config: &GuiConfig) -> bool {
        self.mixed_port.trim() != config.mixed_port.to_string()
            || self.api_port.trim() != config.api_port.to_string()
            || self.dns_server_local.trim() != config.dns_server_local
            || self.dns_server_remote.trim() != config.dns_server_remote
            || self.core_path.trim() != config.core_path.as_deref().unwrap_or_default()
            || self.latency_test_url.trim() != config.latency_test_url
            || self.latency_test_timeout_ms.trim() != config.latency_test_timeout_ms.to_string()
            || self.start_on_boot != config.start_on_boot
            || self.tun_mode != config.tun_mode
            || self.fake_ip != config.fake_ip
            || self.tcp_fast_open != config.tcp_fast_open
            || self.tcp_multipath != config.tcp_multipath
            || self.close_core_on_exit != config.close_core_on_exit
            || self.auto_start_core != config.auto_start_core
            || self.auto_sys_proxy != config.auto_sys_proxy
            || self.auto_update_interval_hours != config.auto_update_interval_hours
            || self.disable_proxy_on_core_stop != config.disable_proxy_on_core_stop
    }
}

#[derive(Debug, Clone)]
pub struct ProxyNode {
    pub name: String,
    pub node_type: String,
    pub server: String,
    pub port: u16,
    pub latency: Option<u64>,
    /// False for native sing-box endpoints that are informational unless a
    /// selector exposes them through the Clash API.
    pub selectable: bool,
}

#[derive(Debug, Default, Clone)]
pub struct Bandwidth {
    pub up: u64,
    pub down: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CoreInstallState {
    #[default]
    Idle,
    Downloading,
    Verifying,
    Extracting,
    Installing,
    Installed,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    NotChecked,
    Checking,
    UpToDate,
    /// Remote tag and optional direct asset URL for the current platform.
    NewVersion {
        tag: String,
        download_url: Option<String>,
        sha256: Option<String>,
        size: Option<u64>,
    },
    /// In-app download of the release binary is in progress.
    Downloading {
        tag: String,
    },
    /// The verified binary is staged and the core/proxy shutdown sequence is
    /// preparing the atomic replacement.
    Installing {
        tag: String,
    },
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ConnectionSort {
    #[default]
    None,
    Host,
    Process,
    Network,
    Chains,
    Rule,
    Download,
    Upload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ProxySort {
    #[default]
    Latency,
    Name,
    Original,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toast_duration_scales_with_length_and_clamps() {
        // Short message resolves to the minimum (3 seconds).
        assert_eq!(Toast::success("ok").remaining_secs, 3);
        assert_eq!(Toast::info("ok").remaining_secs, 3);
        // Errors clamp to at least 5 seconds.
        assert!(Toast::error("err").remaining_secs >= 5);
        // A very long error stays clamped to the 20-second ceiling even for huge strings.
        let long = "FATAL ".repeat(50);
        let long_toast = Toast::error(long.clone());
        assert!(long_toast.remaining_secs <= 20);
        assert!(long_toast.remaining_secs >= 5);
        // Success / info never exceed the clamp either.
        let long_success = "ok ".repeat(200);
        assert!(Toast::success(long_success).remaining_secs <= 20);
    }

    #[test]
    fn toast_duration_uses_unicode_characters_not_utf8_bytes() {
        let ascii = Toast::info("a".repeat(80));
        let chinese = Toast::info("界".repeat(80));
        assert_eq!(ascii.remaining_secs, chinese.remaining_secs);
    }

    #[test]
    fn routing_modes_have_clash_strings() {
        assert_eq!(RoutingMode::Rule.as_clash_mode(), "Rule");
        assert_eq!(RoutingMode::Global.as_clash_mode(), "Global");
        assert_eq!(RoutingMode::Direct.as_clash_mode(), "Direct");
    }

    #[test]
    fn runtime_draft_only_reports_unapplied_changes() {
        let config = GuiConfig::default();
        let mut draft = RuntimeSettingsDraft::from_config(&config);
        assert!(!draft.has_pending_changes(&config));
        draft.latency_test_timeout_ms.clear();
        assert!(draft.has_pending_changes(&config));
        draft = RuntimeSettingsDraft::from_config(&config);
        draft.tun_mode = !draft.tun_mode;
        assert!(draft.has_pending_changes(&config));
    }
}
