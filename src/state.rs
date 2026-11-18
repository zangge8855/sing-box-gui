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
    #[serde(alias = "rule", alias = "RULE")]
    Rule,
    #[serde(alias = "global", alias = "GLOBAL")]
    Global,
    #[serde(alias = "direct", alias = "DIRECT")]
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

    /// i18n key of the label shown on the routing-mode switch.
    pub fn label_key(&self) -> &'static str {
        match self {
            RoutingMode::Rule => "mode_rules",
            RoutingMode::Global => "mode_global",
            RoutingMode::Direct => "mode_direct",
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
        let extra = (char_count / 25).min(10) as u8;
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    #[serde(deserialize_with = "deserialize_null_default")]
    pub id: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub name: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub url: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub file_path: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub is_subscription: bool,
    #[serde(deserialize_with = "deserialize_null_default")]
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

/// Where a custom rule sends matching traffic. The Rules page takes one input
/// per target and files each entry as a domain or IP rule by its shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuleTarget {
    Direct,
    Proxy,
}

impl RuleTarget {
    pub fn field(self, is_ip: bool) -> RuleField {
        match (self, is_ip) {
            (RuleTarget::Direct, false) => RuleField::BypassDomains,
            (RuleTarget::Direct, true) => RuleField::BypassIps,
            (RuleTarget::Proxy, false) => RuleField::ProxyDomains,
            (RuleTarget::Proxy, true) => RuleField::ProxyIps,
        }
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
    #[serde(alias = "en", alias = "EN", alias = "en-US", alias = "en_US")]
    En,
    #[serde(
        alias = "zh",
        alias = "ZH",
        alias = "zh-CN",
        alias = "zh_CN",
        alias = "zh-Hans",
        alias = "zh-Hant"
    )]
    Zh,
}

impl Language {
    pub fn from_locale(locale: &str) -> Self {
        if locale
            .split(['-', '_', '.'])
            .next()
            .is_some_and(|code| code.eq_ignore_ascii_case("zh"))
        {
            Self::Zh
        } else {
            Self::En
        }
    }
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

fn deserialize_nonzero_u16_or<'de, D>(deserializer: D, default: u16) -> Result<u16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<u16>::deserialize(deserializer)?
        .filter(|port| *port > 0)
        .unwrap_or(default))
}

fn deserialize_null_mixed_port<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_nonzero_u16_or(deserializer, 2080)
}

fn deserialize_null_api_port<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_nonzero_u16_or(deserializer, 9090)
}

fn deserialize_null_true<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<bool>::deserialize(deserializer)?.unwrap_or(true))
}

fn deserialize_null_language<'de, D>(deserializer: D) -> Result<Language, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(
        Option::<Language>::deserialize(deserializer)?.unwrap_or_else(|| {
            Language::from_locale(sys_locale::get_locale().as_deref().unwrap_or("en"))
        }),
    )
}

fn deserialize_non_empty_string_or<'de, D>(
    deserializer: D,
    default: fn() -> String,
) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(default))
}

fn deserialize_non_empty_dns_local<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_non_empty_string_or(deserializer, default_dns_local)
}

fn deserialize_non_empty_dns_remote<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_non_empty_string_or(deserializer, default_dns_remote)
}

fn deserialize_non_empty_latency_url<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_non_empty_string_or(deserializer, default_latency_url)
}

fn deserialize_latency_timeout_ms<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<u32>::deserialize(deserializer)?
        .filter(|ms| *ms > 0)
        .unwrap_or_else(default_latency_timeout_ms))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GuiConfig {
    #[serde(deserialize_with = "deserialize_null_default")]
    pub subscriptions: Vec<Profile>,
    pub active_profile_id: Option<String>,
    #[serde(deserialize_with = "deserialize_null_mixed_port")]
    pub mixed_port: u16,
    #[serde(deserialize_with = "deserialize_null_api_port")]
    pub api_port: u16,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub routing_mode: RoutingMode,
    pub core_path: Option<String>,
    #[serde(
        default = "default_dns_local",
        deserialize_with = "deserialize_non_empty_dns_local"
    )]
    pub dns_server_local: String,
    #[serde(
        default = "default_dns_remote",
        deserialize_with = "deserialize_non_empty_dns_remote"
    )]
    pub dns_server_remote: String,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub start_on_boot: bool,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub tun_mode: bool,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub system_proxy_enabled: bool,
    /// True only when this application enabled the currently configured
    /// system proxy. Persisting ownership lets a restarted GUI safely clean up
    /// a proxy it previously installed without touching an unrelated proxy.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub system_proxy_owned: bool,
    #[serde(deserialize_with = "deserialize_null_language")]
    pub language: Language,
    pub selected_node_tag: Option<String>,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub fake_ip: bool,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub tcp_fast_open: bool,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub tcp_multipath: bool,
    #[serde(default = "default_true", deserialize_with = "deserialize_null_true")]
    pub close_core_on_exit: bool,
    #[serde(deserialize_with = "deserialize_null_default")]
    pub theme: AppTheme,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub auto_start_core: bool,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub auto_sys_proxy: bool,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub custom_bypass_domains: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub custom_proxy_domains: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub custom_bypass_ips: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub custom_proxy_ips: Vec<String>,
    /// Auto-update subscriptions every N hours (0 = disabled).
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub auto_update_interval_hours: u32,
    /// When stopping the core, also disable system proxy (default true).
    #[serde(default = "default_true", deserialize_with = "deserialize_null_true")]
    pub disable_proxy_on_core_stop: bool,
    /// URL used by Clash API latency tests (default Cloudflare 204).
    #[serde(
        default = "default_latency_url",
        deserialize_with = "deserialize_non_empty_latency_url"
    )]
    pub latency_test_url: String,
    /// Latency test timeout in milliseconds.
    #[serde(
        default = "default_latency_timeout_ms",
        deserialize_with = "deserialize_latency_timeout_ms"
    )]
    pub latency_test_timeout_ms: u32,
}

fn default_dns_local() -> String {
    "223.5.5.5".to_string()
}

fn default_dns_remote() -> String {
    "8.8.8.8".to_string()
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
    #[serde(alias = "auto", alias = "AUTO", alias = "system", alias = "System")]
    Auto,
    #[serde(alias = "dark", alias = "DARK")]
    Dark,
    #[serde(alias = "light", alias = "LIGHT")]
    Light,
}

impl Default for GuiConfig {
    fn default() -> Self {
        let lang = Language::from_locale(sys_locale::get_locale().as_deref().unwrap_or("en"));
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
            || self.dns_server_local.trim() != config.dns_server_local.trim()
            || self.dns_server_remote.trim() != config.dns_server_remote.trim()
            || self.core_path.trim() != config.core_path.as_deref().unwrap_or_default().trim()
            || self.latency_test_url.trim() != config.latency_test_url.trim()
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
    fn locale_detection_handles_desktop_locale_formats() {
        for locale in ["zh_CN.UTF-8", "zh-Hans-CN", "zh-Hant-TW", "ZH_hk"] {
            assert_eq!(Language::from_locale(locale), Language::Zh);
        }
        for locale in ["en_US.UTF-8", "de-DE", "C", "", "zhongwen"] {
            assert_eq!(Language::from_locale(locale), Language::En);
        }
    }

    #[test]
    fn older_settings_keep_profiles_and_fill_new_fields() {
        let config: GuiConfig = serde_json::from_value(serde_json::json!({
            "language": "En", "mixed_port": 8080, "active_profile_id": "work",
            "subscriptions": [{ "id": "work", "name": "Work", "url": "https://example.com/sub",
                "file_path": "work.json", "is_subscription": true, "updated_at": "2026-09-06" }]
        }))
        .unwrap();
        assert_eq!(config.subscriptions[0].name, "Work");
        assert_eq!(config.active_profile_id.as_deref(), Some("work"));
        assert_eq!(config.mixed_port, 8080);
        assert_eq!(config.api_port, 9090);
        assert_eq!(config.language, Language::En);
        assert!(config.close_core_on_exit);
        assert!(!config.system_proxy_owned);
    }

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
        let mut config = GuiConfig {
            core_path: Some("  /opt/sing-box  ".into()),
            dns_server_local: " 223.5.5.5 ".into(),
            dns_server_remote: " 8.8.8.8 ".into(),
            latency_test_url: " http://cp.cloudflare.com/generate_204 ".into(),
            ..GuiConfig::default()
        };
        let mut draft = RuntimeSettingsDraft::from_config(&config);
        assert!(!draft.has_pending_changes(&config));
        draft.latency_test_timeout_ms.clear();
        assert!(draft.has_pending_changes(&config));
        config = GuiConfig::default();
        draft = RuntimeSettingsDraft::from_config(&config);
        draft.tun_mode = !draft.tun_mode;
        assert!(draft.has_pending_changes(&config));
    }

    #[test]
    fn legacy_nulls_empty_defaults_and_enum_aliases_migrate_cleanly() {
        let config: GuiConfig = serde_json::from_value(serde_json::json!({
            "subscriptions": [{
                "id": "legacy",
                "name": null,
                "url": null,
                "file_path": "legacy.json"
            }],
            "active_profile_id": "legacy",
            "mixed_port": null,
            "api_port": null,
            "routing_mode": "global",
            "language": "zh_CN",
            "theme": "system",
            "dns_server_local": "   ",
            "dns_server_remote": null,
            "latency_test_url": "",
            "latency_test_timeout_ms": 0,
            "close_core_on_exit": null,
            "disable_proxy_on_core_stop": null,
            "custom_bypass_domains": null,
            "custom_proxy_ips": null
        }))
        .unwrap();

        assert_eq!(config.subscriptions.len(), 1);
        assert_eq!(config.subscriptions[0].id, "legacy");
        assert_eq!(config.subscriptions[0].file_path, "legacy.json");
        assert_eq!(config.mixed_port, 2080);
        assert_eq!(config.api_port, 9090);
        assert_eq!(config.routing_mode, RoutingMode::Global);
        assert_eq!(config.language, Language::Zh);
        assert_eq!(config.theme, AppTheme::Auto);
        assert_eq!(config.dns_server_local, "223.5.5.5");
        assert_eq!(config.dns_server_remote, "8.8.8.8");
        assert_eq!(
            config.latency_test_url,
            "http://cp.cloudflare.com/generate_204"
        );
        assert_eq!(config.latency_test_timeout_ms, 2000);
        assert!(config.close_core_on_exit);
        assert!(config.disable_proxy_on_core_stop);
        assert!(config.custom_bypass_domains.is_empty());
        assert!(config.custom_proxy_ips.is_empty());

        let null_subs: GuiConfig = serde_json::from_value(serde_json::json!({
            "subscriptions": null,
            "routing_mode": null,
            "theme": null,
            "mixed_port": 0,
            "api_port": 0
        }))
        .unwrap();
        assert!(null_subs.subscriptions.is_empty());
        assert_eq!(null_subs.routing_mode, RoutingMode::Rule);
        assert_eq!(null_subs.theme, AppTheme::Auto);
        assert_eq!(null_subs.mixed_port, 2080);
        assert_eq!(null_subs.api_port, 9090);
    }
}
