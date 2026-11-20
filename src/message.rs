use crate::state::{
    AppTheme, GuiConfig, IpStack, Language, LogFilter, RoutingMode, RuleField, RuleTarget, Tab,
};

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
    StartupRecovery,
    Exit,
    Update,
}

#[derive(Debug, Clone)]
pub enum Message {
    #[cfg(feature = "ui-smoke")]
    Smoke(crate::smoke::Action),
    NoOp,
    TabChanged(Tab),
    NodeSearchChanged(String),
    ConnectionsSearchChanged(String),
    SelectGroup(String),
    SelectGroupNode {
        group: String,
        node: String,
    },
    GroupNodeSelected {
        generation: u64,
        profile_id: Option<String>,
        group: String,
        node: String,
        error: Option<String>,
    },
    RulesInputChanged {
        target: RuleTarget,
        value: String,
    },
    AddRule {
        target: RuleTarget,
    },
    RemoveRule {
        field: RuleField,
        index: usize,
    },
    ProxiesFetched {
        generation: u64,
        result: Result<std::collections::HashMap<String, crate::api::ProxyInfo>, String>,
    },
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
        is_subscription: bool,
        ignored_settings: Vec<&'static str>,
        from_auto_update: bool,
        is_update: bool,
    },
    SelectProfile(String),
    ActiveNodesLoaded {
        generation: u64,
        profile_id: String,
        is_native_json: bool,
        ignored_settings: Vec<&'static str>,
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
        generation: u64,
        profile_id: Option<String>,
        tag: String,
        error: Option<String>,
    },
    StartLatencyTest,
    NodeLatencyBatch {
        generation: u64,
        results: Vec<(String, Option<u64>)>,
    },
    Tick,
    ConfigSaved {
        saved_config: GuiConfig,
        result: Result<(), String>,
    },
    #[allow(dead_code)]
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
    ModeSet {
        generation: u64,
        mode: RoutingMode,
        result: Result<(), String>,
    },
    SaveSettings,
    DiscardSettings,
    FetchConnections,
    ConnectionsFetched {
        generation: u64,
        result: Result<crate::api::ConnectionsResponse, String>,
    },
    CloseConnection(String),
    ConnectionClosed {
        generation: u64,
        result: Result<String, String>,
    },
    CloseAllConnections,
    AllConnectionsClosed {
        generation: u64,
        result: Result<(), String>,
    },
    TrayIconClicked,
    #[cfg(target_os = "linux")]
    TrayUnavailable,
    TrayMenuClicked(String),
    WindowOpened(iced::window::Id),
    WindowResized(iced::window::Id, iced::Size),
    WindowCloseRequested(iced::window::Id),
    KeyboardEvent(iced::keyboard::Event),

    ClearLogs,
    LogFilterChanged(LogFilter),
    LogSearchChanged(String),
    ResetLogFilters,
    CopyLogs,
    /// Pause/resume tailing the log view (auto-scroll to newest lines).
    ToggleLogsFollow,
    ExportLogs,
    LogsExported(Result<String, String>),
    TriggerCoreDownload,
    /// Force re-download even if core already exists.
    ForceCoreDownload,
    CoreDownloaded(Result<(), String>),
    LatencyTestUrlChanged(String),
    LatencyTestTimeoutChanged(String),
    LatencyTestComplete(u64),
    CoreVersionFetched(Result<String, String>),

    MixedPortChanged(String),
    ApiPortChanged(String),
    DnsLocalChanged(String),
    DnsRemoteChanged(String),
    CorePathChanged(String),
    BrowseCorePath,
    CorePathPicked(Option<String>),
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
    SetIpStack(IpStack),
    ToggleTcpFastOpen,
    ToggleTcpMultipath,
    ToggleAutoStartCore,
    ToggleAutoSysProxy,
    CheckUpdate,
    /// Result of GitHub latest-release check (tag + optional platform asset URL).
    UpdateChecked {
        generation: u64,
        result: Result<AppUpdateInfo, String>,
    },
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
    DismissToastAt(usize),
    ImportFromClipboard,
    ClipboardContent(Option<String>),
    ImportLocalFile,
    LocalFilePicked(Option<String>),
    ToggleConfigPreview,
    CopyConfigPreview,
    /// Async result of `generate_preview_config` (runs off the UI thread).
    ConfigPreviewGenerated {
        generation: u64,
        text: String,
    },
    ToggleProfileMore(String),
    ToggleShowAllProxies,
    CoreLivenessChecked(bool),
    /// Async result of `core::start_core` (runs off the UI thread).
    CoreStartFinished {
        generation: u64,
        result: Result<(), String>,
    },
    /// Async result of `core::stop_core`.
    CoreStopFinished,
    /// Structured download stage update from `core::subscribe_core_download_progress`.
    CoreDownloadStageChanged(crate::core::CoreDownloadStage),
    /// Deferred tray initialization after winit event loop resumes in `--minimized` mode.
    InitMinimizedTray,
    /// Checks whether tray initialized in normal startup when `--no-tray` or tray failure applies.
    StartupTrayCheck,
    /// Startup check for a detached `sing-box` instance left alive by `close_core_on_exit == false`.
    #[allow(dead_code)]
    StartupDetachedCoreChecked {
        detached_running: bool,
        stopped_result: Option<Result<bool, String>>,
    },
    /// Async result of stopping a detached `sing-box` process.
    DetachedCoreStopped {
        for_exit: bool,
        for_update: bool,
        result: Result<bool, String>,
    },
    /// Off-thread check of active native JSON profile for ignored GUI settings.
    NativeJsonIgnoredChecked(Vec<&'static str>),
    /// Off-thread validation result for `RuntimeSettingsDraft` (path/TUN checks).
    SaveSettingsValidated {
        draft: crate::state::RuntimeSettingsDraft,
        core_path_valid: bool,
        core_installed: bool,
        tun_capable: bool,
        for_exit: bool,
    },
    /// Off-thread Linux TUN capability check before starting core in TUN mode.
    CoreStartCapabilityChecked {
        generation: u64,
        tun_capable: bool,
    },
    /// Off-thread local file existence validation when saving profile edits.
    ProfileEditValidated {
        id: String,
        name: String,
        url: String,
        valid: bool,
    },
    /// Off-thread writability check of install directory before app update download.
    UpdateDirChecked {
        tag: String,
        url: String,
        sha256: String,
        size: u64,
        writable: bool,
    },
    SortConnections(crate::state::ConnectionSort),
    SetProxySort(crate::state::ProxySort),
    /// Unified off-thread startup probe checking OS proxy state, crash-recovery
    /// backup presence, and detached core liveness together.
    StartupEnvironmentChecked {
        proxy_check: Result<bool, String>,
        detached_running: bool,
        stopped_result: Option<Result<bool, String>>,
        has_backup: bool,
    },
}
