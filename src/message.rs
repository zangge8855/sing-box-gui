use crate::state::{AppTheme, GuiConfig, Language, LogFilter, RoutingMode, RuleField, Tab};

/// Latest GitHub release info for in-app updates.
#[derive(Debug, Clone)]
pub struct AppUpdateInfo {
    pub tag_name: String,
    /// Direct download URL for the current platform binary, if present on the release.
    pub download_url: Option<String>,
    /// GitHub-provided `sha256:<hex>` digest for the exact asset.
    pub sha256: Option<String>,
    /// Expected asset size from GitHub release metadata.
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemProxyContext {
    Manual,
    CoreStart,
    CoreStop,
    UnexpectedCoreStop,
    SettingsReapply,
    Exit,
    Update,
}

#[derive(Debug, Clone)]
pub enum Message {
    TabChanged(Tab),
    NodeSearchChanged(String),
    ConnectionsSearchChanged(String),
    SelectGroup(String),
    SelectGroupNode {
        group: String,
        node: String,
    },
    GroupNodeSelected {
        group: String,
        node: String,
        error: Option<String>,
    },
    RulesInputChanged {
        field: RuleField,
        value: String,
    },
    AddRule {
        field: RuleField,
    },
    RemoveRule {
        field: RuleField,
        index: usize,
    },
    ProxiesFetched(Result<std::collections::HashMap<String, crate::api::ProxyInfo>, String>),
    ToggleCore,
    NewLogBatch(Vec<String>),
    TrafficUpdated {
        up: u64,
        down: u64,
    },
    ToggleSystemProxy,
    SubscriptionInputChanged(String),
    DownloadSubscription,
    SubscriptionDownloaded {
        id: String,
        error: Option<String>,
        traffic_upload: Option<u64>,
        traffic_download: Option<u64>,
        traffic_total: Option<u64>,
        expire_at: Option<i64>,
        display_name: Option<String>,
        /// Source URL/path used for import (needed when registering a new profile).
        source_url: Option<String>,
    },
    SelectProfile(String),
    ActiveNodesLoaded {
        profile_id: String,
        result: Result<Vec<crate::state::ProxyNode>, String>,
    },
    RequestDeleteProfile(String),
    ConfirmDeleteProfile,
    CancelDeleteProfile,
    ProfileDeleteFinished {
        id: String,
        was_active: bool,
        result: Result<(), String>,
    },
    UpdateSubscription(String),
    AutoUpdateDue(Vec<String>),
    SelectNode(String),
    NodeSelected {
        tag: String,
        error: Option<String>,
    },
    StartLatencyTest,
    #[allow(dead_code)]
    NodeLatencyTested {
        tag: String,
        latency: Option<u64>,
    },
    NodeLatencyBatch(Vec<(String, Option<u64>)>),
    Tick,
    ConfigSaved {
        saved_config: GuiConfig,
        result: Result<(), String>,
    },
    StartupSystemProxyChecked(Result<bool, String>),
    SystemProxySetFinished {
        target: bool,
        context: SystemProxyContext,
        result: Result<(), String>,
    },
    AutostartSetFinished {
        target: bool,
        result: Result<(), String>,
    },
    SystemThemeDetected(bool),
    RoutingModeChanged(RoutingMode),
    ModeSet(Result<String, String>),
    SaveSettings,
    FetchConnections,
    ConnectionsFetched(Result<crate::api::ConnectionsResponse, String>),
    CloseConnection(String),
    ConnectionClosed(Result<String, String>),
    CloseAllConnections,
    AllConnectionsClosed(Result<(), String>),
    TrayIconClicked,
    TrayMenuClicked(String),
    WindowOpened(iced::window::Id),
    WindowCloseRequested(iced::window::Id),
    KeyboardEvent(iced::keyboard::Event),

    ClearLogs,
    LogFilterChanged(LogFilter),
    LogSearchChanged(String),
    ExportLogs,
    LogsExported(Result<String, String>),
    TriggerCoreDownload,
    /// Force re-download even if core already exists.
    ForceCoreDownload,
    CoreDownloaded(Result<(), String>),
    LatencyTestUrlChanged(String),
    LatencyTestTimeoutChanged(String),
    LatencyTestComplete,
    CoreVersionFetched(Result<String, String>),

    MixedPortChanged(String),
    ApiPortChanged(String),
    DnsLocalChanged(String),
    DnsRemoteChanged(String),
    CorePathChanged(String),
    ClearCorePath,
    AutoUpdateIntervalChanged(u32),
    ToggleDisableProxyOnCoreStop,
    ToggleTun,
    ToggleAutostart,
    SetLanguage(Language),
    SetTheme(AppTheme),
    OpenDataDir,
    OpenProfilesFolder,
    EditProfile(String),
    ToggleCloseCoreOnExit,
    ToggleFakeIp,
    ToggleTcpFastOpen,
    ToggleTcpMultipath,
    ToggleAutoStartCore,
    ToggleAutoSysProxy,
    CheckUpdate,
    /// Result of GitHub latest-release check (tag + optional platform asset URL).
    UpdateChecked(Result<AppUpdateInfo, String>),
    /// User (or auto-flow) requested download + install of a release asset.
    DownloadAppUpdate {
        tag: String,
        url: String,
        sha256: String,
        size: u64,
    },
    /// Download finished (path to temp binary) or failed.
    AppUpdateDownloaded {
        tag: String,
        /// Original asset URL (kept so the user can retry after a failure).
        url: String,
        sha256: String,
        size: u64,
        result: Result<std::path::PathBuf, String>,
    },
    OpenUrl(String),
    StartEditProfile(String),
    EditProfileNameChanged(String),
    EditProfileUrlChanged(String),
    SaveProfileEdit,
    CancelProfileEdit,
    DismissToast,
    ImportFromClipboard,
    ClipboardContent(Option<String>),
    ImportLocalFile,
    LocalFilePicked(Option<String>),
    ToggleConfigPreview,
    /// Async result of `generate_preview_config` (runs off the UI thread).
    ConfigPreviewGenerated(String),
    ToggleProfileMore(String),
    CoreLivenessChecked(bool),
    /// Async result of `core::start_core` (runs off the UI thread).
    CoreStartFinished(Result<(), String>),
    /// Async result of `core::stop_core`.
    CoreStopFinished,
    SortConnections(crate::state::ConnectionSort),
    SetProxySort(crate::state::ProxySort),
}
