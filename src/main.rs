// Copyright (C) 2026 zangge8855
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
// Only hide the console window on Windows; leave stdio available on mac/linux.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod api;
mod config;
mod core;
mod message;
mod platform;
#[cfg(feature = "ui-smoke")]
mod smoke;
mod state;
mod sysproxy;
mod tray;
mod ui;
mod update;

use futures::{SinkExt, StreamExt};
use iced::widget::{button, column, container, responsive, row, text, tooltip};
use iced::{Alignment, Element, Font, Length, Subscription, Task};
use message::Message;
use state::{Bandwidth, GuiConfig, Profile, ProxyNode, RuntimeSettingsDraft, Tab, Toast};
use std::sync::{Mutex, OnceLock};
use tokio::sync::mpsc;

// OnceLocks for streaming logs and traffic stats asynchronously
static LOG_RX: OnceLock<Mutex<Option<mpsc::Receiver<String>>>> = OnceLock::new();
static LOG_TX: OnceLock<mpsc::Sender<String>> = OnceLock::new();

static TRAFFIC_RX: OnceLock<Mutex<Option<mpsc::Receiver<api::TrafficInfo>>>> = OnceLock::new();
static TRAFFIC_TX: OnceLock<mpsc::Sender<api::TrafficInfo>> = OnceLock::new();

const MAX_LOG_LINES: usize = 500;
const MAX_LOG_LINE_CHARS: usize = 4096;
const MAX_CONNECTION_SNAPSHOT: usize = 1_000;

pub fn get_log_tx() -> mpsc::Sender<String> {
    LOG_TX
        .get_or_init(|| {
            let (tx, rx) = mpsc::channel(512);
            let _ = LOG_RX.set(Mutex::new(Some(rx)));
            tx
        })
        .clone()
}

pub fn get_traffic_tx() -> mpsc::Sender<api::TrafficInfo> {
    TRAFFIC_TX
        .get_or_init(|| {
            let (tx, rx) = mpsc::channel(8);
            let _ = TRAFFIC_RX.set(Mutex::new(Some(rx)));
            tx
        })
        .clone()
}

struct App {
    #[cfg(feature = "ui-smoke")]
    smoke: Option<smoke::Runner>,
    current_tab: Tab,
    gui_config: GuiConfig,
    core_running: bool,
    sys_proxy_enabled: bool,
    log_lines: std::collections::VecDeque<String>,
    current_speed: Bandwidth,
    speed_history: Vec<(u64, u64)>, // (up, down)
    total_uploaded: u64,
    total_downloaded: u64,
    active_connections: Vec<crate::api::Connection>,
    active_profile_nodes: Vec<ProxyNode>,
    selected_node_tag: Option<String>,
    latency_testing: bool,
    downloading: bool,
    /// Which subscription id an in-flight update belongs to (None for a new
    /// import) — lets the Profiles page mark only that card as downloading.
    updating_profile_id: Option<String>,
    url_input: String,
    core_installed: bool,
    core_install_state: state::CoreInstallState,
    update_status: state::UpdateStatus,
    node_search: String,
    profile_error: Option<String>,
    selected_group: String,
    pending_routing_mode: Option<state::RoutingMode>,
    proxy_groups: std::collections::HashMap<String, crate::api::ProxyInfo>,
    bypass_domain_input: String,
    proxy_domain_input: String,
    bypass_ip_input: String,
    proxy_ip_input: String,
    settings_draft: RuntimeSettingsDraft,
    settings_errors: std::collections::BTreeMap<&'static str, &'static str>,
    connections_search: String,
    tray: Option<tray::Tray>,
    window_id: Option<iced::window::Id>,
    window_size: iced::Size,

    // Performance and UX optimizations
    traffic_cancel_tx: Option<tokio::sync::oneshot::Sender<()>>,
    confirm_delete_profile_id: Option<String>,
    editing_profile_id: Option<String>,
    editing_profile_name: String,
    editing_profile_url: String,

    // Redundant updates prevention
    last_core_running: bool,
    last_sys_proxy_enabled: bool,
    last_routing_mode: state::RoutingMode,

    toast: Option<Toast>,
    log_filter: state::LogFilter,
    log_search: String,
    core_version: Option<String>,
    /// Seconds since last auto-update scan (Tick increments).
    auto_update_tick_counter: u32,
    /// Counter used to throttle per-Tick authoritative liveness checks.
    tick_authority_counter: u32,
    /// Settings: expand generated config preview.
    config_preview_expanded: bool,
    /// Cached result of `generate_preview_config`, refreshed asynchronously.
    config_preview: Option<String>,
    /// Profiles: which card shows secondary actions.
    profile_more_id: Option<String>,
    /// Queued subscription IDs for sequential auto-update.
    pending_auto_updates: std::collections::VecDeque<String>,
    /// Follow new log lines (snap scroll to end).
    logs_follow: bool,
    /// Core start in progress (async; UI stays responsive).
    core_starting: bool,
    /// Core stop in progress (async).
    core_stopping: bool,
    /// After async stop completes, start again (settings/profile restart).
    pending_core_restart: bool,
    /// After an in-flight start finishes, stop immediately (e.g. active profile deleted).
    force_stop_after_start: bool,
    /// Tick counter for tab-aware API poll throttling.
    poll_tick_counter: u32,
    proxies_fetch_in_flight: bool,
    connections_fetch_in_flight: bool,
    last_proxies_fetch: Option<std::time::Instant>,
    last_connections_fetch: Option<std::time::Instant>,
    config_save_in_flight: bool,
    config_save_failures: u32,
    config_save_retry_at: Option<std::time::Instant>,
    last_config_save_log_at: Option<std::time::Instant>,
    pending_restart_after_save: bool,
    pending_proxy_reapply_after_save: bool,
    pending_settings_save_feedback: bool,
    system_proxy_busy: bool,
    autostart_busy: bool,

    connections_sort: state::ConnectionSort,
    connections_sort_desc: bool,
    proxy_sort: state::ProxySort,

    cached_system_is_light: bool,
    config_dirty: bool,
    pending_exit: bool,
    pending_update: Option<PendingAppUpdate>,
}

struct RuntimeSettingsCommit {
    config: GuiConfig,
    restart_required: bool,
    proxy_reapply_required: bool,
    autostart_target: Option<bool>,
}

#[derive(Debug, Clone)]
struct PendingAppUpdate {
    tag: String,
    url: String,
    sha256: String,
    size: u64,
    path: std::path::PathBuf,
}

impl App {
    /// GUI notices share the core log's cap. Only the batch handler used to
    /// trim, so with a quiet core these could accumulate without bound.
    fn push_log(&mut self, line: String) {
        self.log_lines.push_back(line);
        while self.log_lines.len() > MAX_LOG_LINES {
            self.log_lines.pop_front();
        }
    }

    fn theme(&self) -> iced::Theme {
        match self.gui_config.theme {
            state::AppTheme::Dark => iced::Theme::Dark,
            state::AppTheme::Light => iced::Theme::Light,
            state::AppTheme::Auto => {
                if self.cached_system_is_light {
                    iced::Theme::Light
                } else {
                    iced::Theme::Dark
                }
            }
        }
    }

    fn theme_ref(&self) -> &'static iced::Theme {
        static DARK: iced::Theme = iced::Theme::Dark;
        static LIGHT: iced::Theme = iced::Theme::Light;
        match self.gui_config.theme {
            state::AppTheme::Dark => &DARK,
            state::AppTheme::Light => &LIGHT,
            state::AppTheme::Auto => {
                if self.cached_system_is_light {
                    &LIGHT
                } else {
                    &DARK
                }
            }
        }
    }

    /// Regenerate the config preview off the UI thread whenever it is expanded,
    /// and clear the cache when collapsed. Safe to call on every relevant state change.
    fn refresh_config_preview(&mut self) -> Task<Message> {
        if self.config_preview_expanded {
            self.config_preview = Some("...".to_string());
            let gui_config = self.gui_config.clone();
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        crate::config::generate_preview_config(&gui_config)
                    })
                    .await
                    .unwrap_or_else(|e| format!("Failed to run preview task: {e}"))
                },
                Message::ConfigPreviewGenerated,
            )
        } else {
            self.config_preview = None;
            Task::none()
        }
    }

    fn new() -> (Self, Task<Message>) {
        let gui_config = if is_ui_smoke() {
            GuiConfig::default()
        } else {
            config::load_gui_config()
        };
        Self::new_with_config(gui_config)
    }

    fn new_with_config(gui_config: GuiConfig) -> (Self, Task<Message>) {
        // The native theme task and subscription work on GNOME, KDE, Windows
        // and macOS without polling shell commands or assuming a desktop.
        let cached_system_is_light = false;
        let core_installed = core::is_core_installed(&gui_config);
        let selected_node_tag = gui_config.selected_node_tag.clone();

        let settings_draft = RuntimeSettingsDraft::from_config(&gui_config);

        let initial_routing_mode = gui_config.routing_mode;
        let mut app = Self {
            #[cfg(feature = "ui-smoke")]
            smoke: smoke::Runner::from_env(),
            current_tab: Tab::Dashboard,
            gui_config,
            core_running: false,
            sys_proxy_enabled: false,
            log_lines: std::collections::VecDeque::new(),
            current_speed: Bandwidth::default(),
            speed_history: vec![(0, 0); 30],
            total_uploaded: 0,
            total_downloaded: 0,
            active_connections: Vec::new(),
            active_profile_nodes: Vec::new(),
            selected_node_tag,
            latency_testing: false,
            downloading: false,
            updating_profile_id: None,
            url_input: String::new(),
            core_installed,
            core_install_state: state::CoreInstallState::Idle,
            update_status: state::UpdateStatus::NotChecked,
            node_search: String::new(),
            profile_error: None,
            selected_group: String::new(),
            pending_routing_mode: None,
            proxy_groups: std::collections::HashMap::new(),
            bypass_domain_input: String::new(),
            proxy_domain_input: String::new(),
            bypass_ip_input: String::new(),
            proxy_ip_input: String::new(),
            settings_draft,
            settings_errors: std::collections::BTreeMap::new(),
            connections_search: String::new(),
            tray: None,
            window_id: None,
            window_size: iced::Size::new(1080.0, 750.0),

            traffic_cancel_tx: None,
            confirm_delete_profile_id: None,
            editing_profile_id: None,
            editing_profile_name: String::new(),
            editing_profile_url: String::new(),

            last_core_running: false,
            last_sys_proxy_enabled: false,
            last_routing_mode: initial_routing_mode,
            toast: None,
            log_filter: state::LogFilter::All,
            log_search: String::new(),
            core_version: None,
            auto_update_tick_counter: 0,
            tick_authority_counter: 0,
            config_preview_expanded: false,
            config_preview: None,
            profile_more_id: None,
            pending_auto_updates: std::collections::VecDeque::new(),
            logs_follow: true,
            core_starting: false,
            core_stopping: false,
            pending_core_restart: false,
            force_stop_after_start: false,
            poll_tick_counter: 0,
            proxies_fetch_in_flight: false,
            connections_fetch_in_flight: false,
            last_proxies_fetch: None,
            last_connections_fetch: None,
            config_save_in_flight: false,
            config_save_failures: 0,
            config_save_retry_at: None,
            last_config_save_log_at: None,
            pending_restart_after_save: false,
            pending_proxy_reapply_after_save: false,
            pending_settings_save_feedback: false,
            system_proxy_busy: false,
            autostart_busy: false,
            connections_sort: state::ConnectionSort::None,
            connections_sort_desc: false,
            proxy_sort: state::ProxySort::Latency,

            cached_system_is_light,
            config_dirty: false,
            pending_exit: false,
            pending_update: None,
        };

        #[cfg(feature = "ui-smoke")]
        if app.smoke.is_some() {
            let open = app.open_main_window();
            return (app, open);
        }

        // Force initialization of log and traffic streams on startup
        let _ = get_log_tx();
        let _ = get_traffic_tx();

        // Restore persisted ownership in memory, then verify the actual system
        // state asynchronously so registry / networksetup / gsettings never
        // stalls the UI thread during startup.
        sysproxy::restore_system_proxy_owned(app.gui_config.system_proxy_owned);

        // Initial tray menu synchronization
        app.update_tray_menu();

        let mut tasks = Vec::new();
        tasks
            .push(iced::system::theme().map(|mode| {
                Message::SystemThemeDetected(matches!(mode, iced::theme::Mode::Light))
            }));
        let startup_port = app.gui_config.mixed_port;
        tasks.push(Task::perform(
            async move {
                tokio::task::spawn_blocking(move || sysproxy::check_system_proxy(startup_port))
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|result| result)
            },
            Message::StartupSystemProxyChecked,
        ));
        if app.gui_config.active_profile_id.is_some() {
            tasks.push(app.load_active_nodes_task());
        }
        if app.gui_config.auto_start_core
            && app.gui_config.active_profile_id.is_some()
            && app.core_installed
        {
            tasks.push(Task::done(Message::ToggleCore));
        }
        if app.core_installed {
            let cfg = app.gui_config.clone();
            tasks.push(Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || core::get_core_version(&cfg))
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                },
                Message::CoreVersionFetched,
            ));
        }

        tasks.push(app.open_main_window());
        (app, Task::batch(tasks))
    }

    fn load_active_nodes_task(&self) -> Task<Message> {
        let Some(profile_id) = self.gui_config.active_profile_id.clone() else {
            return Task::none();
        };
        let result_profile_id = profile_id.clone();
        let path = config::get_profile_path(&profile_id);
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let content = std::fs::read_to_string(&path)
                        .map_err(|error| format!("Failed to read active profile: {error}"))?;
                    let trimmed = content.trim();
                    if trimmed.starts_with('{') || trimmed.starts_with('[') {
                        config::parse_native_json_nodes(&content)
                    } else {
                        config::parse_clash_yaml_nodes(&content)
                    }
                })
                .await
                .map_err(|error| format!("Profile parse task failed: {error}"))?
            },
            move |result| Message::ActiveNodesLoaded {
                profile_id: result_profile_id.clone(),
                result,
            },
        )
    }

    fn tray_state(&self) -> tray::TrayState {
        tray::TrayState {
            language: self.gui_config.language,
            core_running: self.core_running,
            system_proxy_enabled: self.sys_proxy_enabled,
            routing_mode: self.gui_config.routing_mode,
        }
    }

    fn update_tray_menu(&self) {
        if let Some(tray) = &self.tray {
            tray.update(self.tray_state());
        }
    }

    fn core_busy(&self) -> bool {
        self.core_starting || self.core_stopping
    }

    fn core_install_busy(&self) -> bool {
        matches!(
            &self.core_install_state,
            state::CoreInstallState::Downloading
                | state::CoreInstallState::Verifying
                | state::CoreInstallState::Extracting
                | state::CoreInstallState::Installing
        )
    }

    fn app_update_busy(&self) -> bool {
        self.pending_update.is_some()
            || matches!(
                &self.update_status,
                state::UpdateStatus::Downloading { .. } | state::UpdateStatus::Installing { .. }
            )
    }

    fn maintenance_busy(&self) -> bool {
        self.core_install_busy() || self.app_update_busy()
    }

    fn open_main_window(&mut self) -> Task<Message> {
        let (id, open) = iced::window::open(main_window_settings(self.window_size));
        // Record the pending window too, so double tray clicks cannot open two.
        self.window_id = Some(id);
        open.map(Message::WindowOpened)
    }

    /// Wayland cannot hide/show an existing surface. Close the window while
    /// keeping the daemon alive, then create a fresh surface on tray activation.
    fn show_main_window(&mut self) -> Task<Message> {
        if let Some(id) = self.window_id {
            Task::batch(vec![
                iced::window::minimize(id, false),
                iced::window::gain_focus(id),
            ])
        } else {
            self.open_main_window()
        }
    }

    fn request_exit(&mut self) -> Task<Message> {
        #[cfg(feature = "ui-smoke")]
        if self.smoke.is_some() {
            return iced::exit();
        }
        if self.pending_exit {
            return Task::none();
        }
        if self.maintenance_busy() {
            self.toast_info(self.tr("operation_in_progress"));
            // Toasts render inside the window; tray Exit with a hidden window
            // would otherwise appear to do nothing at all.
            return self.show_main_window();
        }
        self.pending_exit = true;
        let settings_task = match self.commit_runtime_settings(true) {
            Ok(task) => task,
            Err(()) => {
                // Exiting abandons an invalid settings draft instead of
                // silently refusing to quit behind a hidden validation toast.
                self.settings_draft = RuntimeSettingsDraft::from_config(&self.gui_config);
                self.settings_errors.clear();
                match self.commit_runtime_settings(true) {
                    Ok(task) => task,
                    Err(()) => {
                        self.pending_exit = false;
                        return Task::none();
                    }
                }
            }
        };
        Task::batch(vec![settings_task, self.continue_exit()])
    }

    fn continue_exit(&mut self) -> Task<Message> {
        if self.autostart_busy || self.config_save_in_flight || self.system_proxy_busy {
            return Task::none();
        }
        if self.config_dirty {
            return self.start_config_save();
        }
        if self.gui_config.close_core_on_exit {
            if self.core_starting {
                self.force_stop_after_start = true;
                return Task::none();
            }
            if self.core_stopping {
                return Task::none();
            }
            if self.core_running || core::is_core_running_fast() {
                return self.task_stop_core();
            }
        }
        if self.gui_config.system_proxy_owned || sysproxy::is_system_proxy_owned() {
            return self.task_set_system_proxy(false, message::SystemProxyContext::Exit);
        }
        iced::exit()
    }

    fn fail_pending_update(&mut self, pending: PendingAppUpdate, error: String) -> Task<Message> {
        if let Err(cleanup_error) = std::fs::remove_file(&pending.path)
            && cleanup_error.kind() != std::io::ErrorKind::NotFound
        {
            self.push_log(format!(
                "[GUI] Failed to remove staged update {}: {cleanup_error}",
                pending.path.display()
            ));
        }
        self.update_status = state::UpdateStatus::NewVersion {
            tag: pending.tag,
            download_url: Some(pending.url),
            sha256: Some(pending.sha256),
            size: Some(pending.size),
        };
        self.push_log(format!("[GUI] Failed to apply update: {error}"));
        self.toast_error(format!("{}: {error}", self.tr("update_apply_failed")));
        Task::none()
    }

    fn continue_update_install(&mut self) -> Task<Message> {
        if self.pending_update.is_none() {
            return Task::none();
        }
        if self.core_starting {
            self.force_stop_after_start = true;
            return Task::none();
        }
        if self.core_stopping || self.system_proxy_busy || self.config_save_in_flight {
            return Task::none();
        }
        if self.core_running || core::is_core_running_fast() {
            return self.task_stop_core();
        }
        if self.gui_config.system_proxy_owned || sysproxy::is_system_proxy_owned() {
            return self.task_set_system_proxy(false, message::SystemProxyContext::Update);
        }
        if self.config_dirty {
            return self.start_config_save();
        }

        let pending = self.pending_update.take().expect("checked pending update");
        match apply_update_and_restart(&pending.path) {
            Ok(()) => {
                self.push_log("[GUI] Update scheduled; exiting to apply.".to_string());
                iced::exit()
            }
            Err(error) => self.fail_pending_update(pending, error),
        }
    }

    fn start_config_save(&mut self) -> Task<Message> {
        if self.config_save_in_flight || self.autostart_busy || !self.config_dirty {
            return Task::none();
        }
        let cfg = self.gui_config.clone();
        let saved_config = cfg.clone();
        self.config_save_in_flight = true;
        self.config_save_retry_at = None;
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || config::save_gui_config(&cfg))
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|result| result)
            },
            move |result| Message::ConfigSaved {
                saved_config: saved_config.clone(),
                result,
            },
        )
    }

    fn task_set_system_proxy(
        &mut self,
        target: bool,
        context: message::SystemProxyContext,
    ) -> Task<Message> {
        if self.system_proxy_busy {
            return Task::none();
        }
        self.system_proxy_busy = true;
        let port = self.gui_config.mixed_port;
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || sysproxy::set_system_proxy(target, port))
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|result| result)
            },
            move |result| Message::SystemProxySetFinished {
                target,
                context,
                result,
            },
        )
    }

    fn validate_runtime_settings(
        &self,
    ) -> Result<RuntimeSettingsCommit, std::collections::BTreeMap<&'static str, &'static str>> {
        let draft = &self.settings_draft;
        let mut errors = std::collections::BTreeMap::new();
        let mixed_port = match parse_runtime_port(&draft.mixed_port) {
            Ok(value) => Some(value),
            Err(key) => {
                errors.insert("mixed_port", key);
                None
            }
        };
        let api_port = match parse_runtime_port(&draft.api_port) {
            Ok(value) => Some(value),
            Err(key) => {
                errors.insert("api_port", key);
                None
            }
        };
        if mixed_port.is_some() && mixed_port == api_port {
            errors.insert("mixed_port", "port_conflict_error");
            errors.insert("api_port", "port_conflict_error");
        }
        if !is_valid_dns_server_address(draft.dns_server_local.trim()) {
            errors.insert("dns_local", "invalid_dns_server");
        }
        if !is_valid_dns_server_address(draft.dns_server_remote.trim()) {
            errors.insert("dns_remote", "invalid_dns_server");
        }
        let latency_url = draft.latency_test_url.trim();
        if !url::Url::parse(latency_url)
            .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
        {
            errors.insert("latency_url", "invalid_latency_url");
        }
        let latency_timeout = match parse_latency_timeout(&draft.latency_test_timeout_ms) {
            Ok(value) => Some(value),
            Err(()) => {
                errors.insert("latency_timeout", "invalid_latency_timeout");
                None
            }
        };
        let core_path = draft.core_path.trim();
        if !core_path.is_empty() && !std::path::Path::new(core_path).is_file() {
            errors.insert("core_path", "invalid_core_path");
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        let mut next = self.gui_config.clone();
        let mixed_port = mixed_port.expect("validated mixed port");
        let api_port = api_port.expect("validated API port");
        let latency_timeout = latency_timeout.expect("validated latency timeout");
        let proxy_reapply_required = next.mixed_port != mixed_port
            && self.sys_proxy_enabled
            && (next.system_proxy_owned || sysproxy::is_system_proxy_owned());
        let restart_required = next.mixed_port != mixed_port
            || next.api_port != api_port
            || next.dns_server_local != draft.dns_server_local.trim()
            || next.dns_server_remote != draft.dns_server_remote.trim()
            || next.core_path.as_deref().unwrap_or_default() != core_path
            || next.tun_mode != draft.tun_mode
            || next.fake_ip != draft.fake_ip
            || next.tcp_fast_open != draft.tcp_fast_open
            || next.tcp_multipath != draft.tcp_multipath;
        let autostart_target =
            (next.start_on_boot != draft.start_on_boot).then_some(draft.start_on_boot);

        next.mixed_port = mixed_port;
        next.api_port = api_port;
        next.dns_server_local = draft.dns_server_local.trim().to_string();
        next.dns_server_remote = draft.dns_server_remote.trim().to_string();
        next.core_path = (!core_path.is_empty()).then(|| core_path.to_string());
        next.latency_test_url = latency_url.to_string();
        next.latency_test_timeout_ms = latency_timeout;
        next.start_on_boot = draft.start_on_boot;
        next.tun_mode = draft.tun_mode;
        next.fake_ip = draft.fake_ip;
        next.tcp_fast_open = draft.tcp_fast_open;
        next.tcp_multipath = draft.tcp_multipath;
        next.close_core_on_exit = draft.close_core_on_exit;
        next.auto_start_core = draft.auto_start_core;
        next.auto_sys_proxy = draft.auto_sys_proxy;
        next.auto_update_interval_hours = draft.auto_update_interval_hours;
        next.disable_proxy_on_core_stop = draft.disable_proxy_on_core_stop;

        Ok(RuntimeSettingsCommit {
            config: next,
            restart_required,
            proxy_reapply_required,
            autostart_target,
        })
    }

    fn commit_runtime_settings(&mut self, for_exit: bool) -> Result<Task<Message>, ()> {
        if !self.settings_draft.has_pending_changes(&self.gui_config) {
            self.settings_errors.clear();
            return Ok(self.start_config_save());
        }
        let commit = match self.validate_runtime_settings() {
            Ok(commit) => commit,
            Err(errors) => {
                self.settings_errors = errors;
                let key = self
                    .settings_errors
                    .values()
                    .next()
                    .copied()
                    .unwrap_or("settings_invalid");
                self.toast_error(self.tr(key));
                return Err(());
            }
        };

        self.settings_errors.clear();
        self.gui_config = commit.config;
        self.core_installed = core::is_core_installed(&self.gui_config);
        self.config_dirty = true;
        self.pending_restart_after_save =
            !for_exit && commit.restart_required && (self.core_running || self.core_starting);
        self.pending_proxy_reapply_after_save = !for_exit && commit.proxy_reapply_required;
        if self.gui_config.tun_mode && !platform::is_running_elevated() {
            self.toast_info(self.tr("tun_admin_banner"));
        }

        if let Some(target) = commit.autostart_target {
            self.autostart_busy = true;
            return Ok(Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || platform::set_autostart(target))
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|result| result)
                },
                move |result| Message::AutostartSetFinished { target, result },
            ));
        }

        self.settings_draft = RuntimeSettingsDraft::from_config(&self.gui_config);
        Ok(self.start_config_save())
    }

    /// Schedule an async core stop (runs off the UI thread).
    fn task_stop_core(&mut self) -> Task<Message> {
        self.core_stopping = true;
        Task::perform(
            async {
                let _ = tokio::task::spawn_blocking(core::stop_core).await;
            },
            |_| Message::CoreStopFinished,
        )
    }

    /// Schedule an async core start with a snapshot of current config.
    fn task_start_core(&mut self) -> Task<Message> {
        self.core_starting = true;
        let cfg = self.gui_config.clone();
        let log_tx = get_log_tx();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || core::start_core(&cfg, log_tx))
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|r| r)
            },
            Message::CoreStartFinished,
        )
    }

    /// Apply post-start side effects (traffic monitor, optional system proxy).
    fn on_core_started_ok(&mut self) -> Task<Message> {
        self.core_running = true;
        self.push_log("[GUI] sing-box core started successfully.".to_string());

        if let Some(cancel_tx) = self.traffic_cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.traffic_cancel_tx = Some(tx);
        let traffic_tx = get_traffic_tx();
        api::spawn_traffic_monitor(self.gui_config.api_port, traffic_tx, rx);

        let wake_task = Task::perform(
            async {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            },
            |_| Message::Tick,
        );
        if self.gui_config.auto_sys_proxy && !self.sys_proxy_enabled {
            Task::batch(vec![
                wake_task,
                self.task_set_system_proxy(true, message::SystemProxyContext::CoreStart),
            ])
        } else {
            wake_task
        }
    }

    /// Drop API-derived snapshots that only make sense while the core runs.
    /// Leaving them in place shows live-looking rows (connections, group node
    /// cards) whose actions silently no-op against a stopped core.
    fn clear_api_snapshots(&mut self) {
        self.pending_routing_mode = None;
        self.active_connections.clear();
        self.proxy_groups.clear();
        // Keep `selected_group` so the previous group restores on restart.
    }

    /// The core died without a stop request: surface why, tear down live
    /// state, and hand the system proxy back so traffic isn't black-holed.
    fn handle_unexpected_core_exit(&mut self) -> Task<Message> {
        self.core_running = false;
        if let Some(msg) = core::take_unexpected_core_exit() {
            let detail = redact_log_line(&msg);
            self.push_log(format!("[GUI] {detail}"));
            self.toast_error(format!("{}: {detail}", self.tr("core_unexpected_exit")));
        }
        if let Some(cancel_tx) = self.traffic_cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
        self.current_speed = Bandwidth::default();
        self.total_uploaded = 0;
        self.total_downloaded = 0;
        self.clear_api_snapshots();

        if self.gui_config.disable_proxy_on_core_stop
            && self.sys_proxy_enabled
            && (self.gui_config.system_proxy_owned || sysproxy::is_system_proxy_owned())
        {
            return self
                .task_set_system_proxy(false, message::SystemProxyContext::UnexpectedCoreStop);
        }
        Task::none()
    }

    /// Tear down traffic monitor and optionally disable system proxy after stop.
    fn on_core_stopped_cleanup(&mut self) {
        self.core_running = false;
        if let Some(cancel_tx) = self.traffic_cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
        self.current_speed = Bandwidth::default();
        self.total_uploaded = 0;
        self.total_downloaded = 0;
        self.clear_api_snapshots();
        self.push_log("[GUI] sing-box core stopped.".to_string());
    }

    fn restart_core(&mut self) -> Task<Message> {
        if self.core_busy() {
            // Coalesce: ensure we restart once the in-flight transition ends.
            self.pending_core_restart = true;
            return Task::none();
        }
        if self.core_running {
            self.pending_core_restart = true;
            self.push_log("[GUI] Restarting core to apply new settings...".to_string());
            self.task_stop_core()
        } else {
            Task::none()
        }
    }

    fn show_toast(&mut self, toast: Toast) {
        self.toast = Some(toast);
    }

    fn toast_success(&mut self, msg: impl Into<String>) {
        self.show_toast(Toast::success(msg));
    }

    fn toast_error(&mut self, msg: impl Into<String>) {
        self.show_toast(Toast::error(msg));
    }

    fn toast_info(&mut self, msg: impl Into<String>) {
        self.show_toast(Toast::info(msg));
    }

    fn tr(&self, key: &'static str) -> &'static str {
        ui::i18n::tr(self.gui_config.language, key)
    }

    /// Start next queued auto-update if idle.
    fn kick_pending_auto_update(&mut self) -> Task<Message> {
        if self.downloading {
            return Task::none();
        }
        if let Some(id) = self.pending_auto_updates.pop_front() {
            Task::done(Message::UpdateSubscription(id))
        } else {
            Task::none()
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let task = self.handle_update(message);
        if self.core_running != self.last_core_running
            || self.sys_proxy_enabled != self.last_sys_proxy_enabled
            || self.gui_config.routing_mode != self.last_routing_mode
        {
            self.update_tray_menu();
            self.last_core_running = self.core_running;
            self.last_sys_proxy_enabled = self.sys_proxy_enabled;
            self.last_routing_mode = self.gui_config.routing_mode;
        }
        task
    }

    fn handle_update(&mut self, message: Message) -> Task<Message> {
        match message {
            #[cfg(feature = "ui-smoke")]
            Message::Smoke(action) => smoke::update(self, action),
            Message::NoOp => Task::none(),
            Message::TrayIconClicked => self.show_main_window(),
            Message::TrayMenuClicked(id_str) => match id_str.as_str() {
                "show_window" => self.show_main_window(),
                "toggle_core" => Task::done(Message::ToggleCore),
                "toggle_system_proxy" => Task::done(Message::ToggleSystemProxy),
                "mode_rule" => Task::done(Message::RoutingModeChanged(state::RoutingMode::Rule)),
                "mode_global" => {
                    Task::done(Message::RoutingModeChanged(state::RoutingMode::Global))
                }
                "mode_direct" => {
                    Task::done(Message::RoutingModeChanged(state::RoutingMode::Direct))
                }
                "exit_app" => self.request_exit(),
                _ => Task::none(),
            },
            Message::WindowOpened(id) => {
                self.window_id = Some(id);
                if self.tray.is_none() {
                    self.tray = Some(tray::Tray::new(self.tray_state()));
                }
                #[cfg(feature = "ui-smoke")]
                if self.smoke.is_some() {
                    return smoke::window_opened(self);
                }
                Task::none()
            }
            Message::WindowResized(id, size) => {
                if self.window_id == Some(id) {
                    self.window_size = size;
                }
                Task::none()
            }
            #[cfg(target_os = "linux")]
            Message::TrayUnavailable => {
                if self.window_id.is_none() {
                    self.show_main_window()
                } else {
                    Task::none()
                }
            }
            Message::WindowCloseRequested(id) => {
                if self.window_id != Some(id) {
                    return Task::none();
                }
                if self.tray.as_ref().is_some_and(tray::Tray::available) {
                    self.window_id = None;
                    iced::window::close(id)
                } else {
                    self.request_exit()
                }
            }
            Message::KeyboardEvent(iced::keyboard::Event::KeyPressed {
                key,
                modifiers,
                repeat,
                ..
            }) if !repeat => {
                match key.as_ref() {
                    iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) => {
                        return if modifiers.shift() {
                            iced::widget::operation::focus_previous()
                        } else {
                            iced::widget::operation::focus_next()
                        };
                    }
                    iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
                        if self.editing_profile_id.is_some() {
                            return Task::done(Message::CancelProfileEdit);
                        }
                        if self.confirm_delete_profile_id.is_some() {
                            return Task::done(Message::CancelDeleteProfile);
                        }
                        self.toast = None;
                    }
                    iced::keyboard::Key::Character(character)
                        if modifiers.command() && character.eq_ignore_ascii_case("s") =>
                    {
                        if self.editing_profile_id.is_some() {
                            return Task::done(Message::SaveProfileEdit);
                        }
                        // Only save settings from the Settings tab — firing a
                        // "Settings saved" toast from Logs/Dashboard is noise.
                        if self.current_tab == state::Tab::Settings {
                            return Task::done(Message::SaveSettings);
                        }
                    }
                    iced::keyboard::Key::Character(character) if modifiers.command() => {
                        let tab = match character {
                            "1" => Some(Tab::Dashboard),
                            "2" => Some(Tab::Proxies),
                            "3" => Some(Tab::Profiles),
                            "4" => Some(Tab::Rules),
                            "5" => Some(Tab::Connections),
                            "6" => Some(Tab::Logs),
                            "7" => Some(Tab::Settings),
                            _ => None,
                        };
                        if let Some(tab) = tab {
                            return Task::done(Message::TabChanged(tab));
                        }
                    }
                    _ => {}
                }
                Task::none()
            }
            Message::KeyboardEvent(_) => Task::none(),
            Message::TabChanged(tab) => {
                self.current_tab = tab;
                let mut tasks = Vec::new();
                if self.core_running && !self.core_busy() {
                    let api_port = self.gui_config.api_port;
                    let stale = |last: Option<std::time::Instant>| {
                        last.is_none_or(|at| at.elapsed() >= std::time::Duration::from_secs(2))
                    };
                    match tab {
                        Tab::Proxies
                            if !self.proxies_fetch_in_flight && stale(self.last_proxies_fetch) =>
                        {
                            self.proxies_fetch_in_flight = true;
                            tasks.push(Task::perform(
                                async move { api::fetch_proxies(api_port).await },
                                |res| Message::ProxiesFetched(res.map(|r| r.proxies)),
                            ));
                        }
                        Tab::Connections if stale(self.last_connections_fetch) => {
                            tasks.push(Task::done(Message::FetchConnections));
                        }
                        Tab::Dashboard => {
                            if !self.proxies_fetch_in_flight && stale(self.last_proxies_fetch) {
                                self.proxies_fetch_in_flight = true;
                                tasks.push(Task::perform(
                                    async move { api::fetch_proxies(api_port).await },
                                    |res| Message::ProxiesFetched(res.map(|r| r.proxies)),
                                ));
                            }
                            if stale(self.last_connections_fetch) {
                                tasks.push(Task::done(Message::FetchConnections));
                            }
                        }
                        _ => {}
                    }
                }
                Task::batch(tasks)
            }
            Message::NodeSearchChanged(query) => {
                self.node_search = query;
                Task::none()
            }
            Message::ConnectionsSearchChanged(query) => {
                self.connections_search = query;
                Task::none()
            }
            Message::SelectGroup(group) => {
                self.selected_group = group;
                Task::none()
            }
            Message::SelectGroupNode { group, node } => {
                if !self.core_running {
                    return Task::none();
                }
                let api_port = self.gui_config.api_port;
                let group_clone = group.clone();
                let node_clone = node.clone();

                Task::perform(
                    async move { api::select_proxy(api_port, &group_clone, &node_clone).await },
                    move |res| Message::GroupNodeSelected {
                        group: group.clone(),
                        node: node.clone(),
                        error: res.err(),
                    },
                )
            }
            Message::GroupNodeSelected { group, node, error } => {
                if let Some(err) = error {
                    let detail = redact_log_line(&err);
                    self.push_log(format!(
                        "[GUI] Failed to select node {} for group {}: {}",
                        node, group, detail
                    ));
                    self.toast_error(format!("{}: {detail}", self.tr("node_select_failed")));
                } else {
                    if let Some(g_info) = self.proxy_groups.get_mut(&group) {
                        g_info.now = Some(node.clone());
                    }
                    if group == "Proxy" {
                        self.selected_node_tag = Some(node.clone());
                        self.gui_config.selected_node_tag = Some(node.clone());
                        self.config_dirty = true;
                    }
                    self.push_log(format!("[GUI] Selected node: {} for group {}", node, group));
                }
                Task::none()
            }
            Message::RulesInputChanged { field, value } => {
                match field {
                    state::RuleField::BypassDomains => self.bypass_domain_input = value,
                    state::RuleField::ProxyDomains => self.proxy_domain_input = value,
                    state::RuleField::BypassIps => self.bypass_ip_input = value,
                    state::RuleField::ProxyIps => self.proxy_ip_input = value,
                }
                Task::none()
            }
            Message::AddRule { field } => {
                let raw = match field {
                    state::RuleField::BypassDomains => &self.bypass_domain_input,
                    state::RuleField::ProxyDomains => &self.proxy_domain_input,
                    state::RuleField::BypassIps => &self.bypass_ip_input,
                    state::RuleField::ProxyIps => &self.proxy_ip_input,
                };
                let normalized = match normalize_custom_rule(field, raw) {
                    Ok(value) => value,
                    Err(key) => {
                        self.toast_error(self.tr(key));
                        return Task::none();
                    }
                };
                let (val, list) = match field {
                    state::RuleField::BypassDomains => (
                        &mut self.bypass_domain_input,
                        &mut self.gui_config.custom_bypass_domains,
                    ),
                    state::RuleField::ProxyDomains => (
                        &mut self.proxy_domain_input,
                        &mut self.gui_config.custom_proxy_domains,
                    ),
                    state::RuleField::BypassIps => (
                        &mut self.bypass_ip_input,
                        &mut self.gui_config.custom_bypass_ips,
                    ),
                    state::RuleField::ProxyIps => (
                        &mut self.proxy_ip_input,
                        &mut self.gui_config.custom_proxy_ips,
                    ),
                };
                if !list
                    .iter()
                    .any(|item| item.eq_ignore_ascii_case(&normalized))
                {
                    list.push(normalized.clone());
                    val.clear();
                    self.config_dirty = true;
                    self.push_log(format!(
                        "[GUI] Added custom rule to {}: {}",
                        field.as_str(),
                        normalized
                    ));
                    return self.restart_core();
                }
                self.toast_info(self.tr("duplicate_rule"));
                Task::none()
            }
            Message::RemoveRule { field, index } => {
                let list = match field {
                    state::RuleField::BypassDomains => &mut self.gui_config.custom_bypass_domains,
                    state::RuleField::ProxyDomains => &mut self.gui_config.custom_proxy_domains,
                    state::RuleField::BypassIps => &mut self.gui_config.custom_bypass_ips,
                    state::RuleField::ProxyIps => &mut self.gui_config.custom_proxy_ips,
                };
                if index < list.len() {
                    let removed = list.remove(index);
                    self.config_dirty = true;
                    self.push_log(format!(
                        "[GUI] Removed custom rule from {}: {}",
                        field.as_str(),
                        removed
                    ));
                    return self.restart_core();
                }
                Task::none()
            }
            Message::ProxiesFetched(res) => {
                self.proxies_fetch_in_flight = false;
                if !self.core_running || self.core_stopping {
                    return Task::none();
                }
                match res {
                    Ok(mut groups_map) => {
                        compact_proxy_history(&mut groups_map);
                        self.last_proxies_fetch = Some(std::time::Instant::now());
                        if self.proxy_groups != groups_map {
                            self.proxy_groups = groups_map;
                        }
                        if self.selected_group.is_empty() && !self.proxy_groups.is_empty() {
                            if self.proxy_groups.contains_key("Proxy") {
                                self.selected_group = "Proxy".to_string();
                            } else if let Some(k) = self.proxy_groups.keys().next() {
                                self.selected_group = k.clone();
                            }
                        }
                    }
                    Err(_e) => {
                        // Suppress background polling HTTP errors
                    }
                }
                Task::none()
            }
            Message::ToggleCore => {
                if self.maintenance_busy() {
                    self.toast_info(self.tr("operation_in_progress"));
                    return Task::none();
                }
                if self.core_busy() {
                    return Task::none();
                }
                if self.core_running {
                    self.pending_core_restart = false;
                    self.task_stop_core()
                } else {
                    self.task_start_core()
                }
            }
            Message::CoreStartFinished(res) => {
                self.core_starting = false;
                match res {
                    Ok(()) => {
                        if self.force_stop_after_start {
                            self.force_stop_after_start = false;
                            self.pending_core_restart = false;
                            self.core_running = true;
                            return self.task_stop_core();
                        }
                        if self.pending_core_restart {
                            // Settings/profile changed while starting — recycle with latest config.
                            self.core_running = true;
                            self.pending_core_restart = true;
                            return self.task_stop_core();
                        }
                        self.on_core_started_ok()
                    }
                    Err(e) => {
                        let detail = redact_log_line(&e);
                        self.core_running = false;
                        self.force_stop_after_start = false;
                        self.push_log(format!("[GUI] Error starting core: {detail}"));
                        self.toast_error(self.tr("core_start_failed").to_string() + ": " + &detail);
                        // Drop pending restart so we don't loop on a broken config.
                        self.pending_core_restart = false;
                        if self.pending_exit {
                            return self.continue_exit();
                        }
                        Task::none()
                    }
                }
            }
            Message::CoreStopFinished => {
                self.core_stopping = false;
                self.on_core_stopped_cleanup();
                if self.pending_update.is_some() {
                    return self.continue_update_install();
                }
                if self.pending_exit {
                    return self.continue_exit();
                }

                if self.pending_core_restart {
                    self.pending_core_restart = false;
                    return self.task_start_core();
                }
                if self.gui_config.disable_proxy_on_core_stop
                    && self.sys_proxy_enabled
                    && (self.gui_config.system_proxy_owned || sysproxy::is_system_proxy_owned())
                {
                    return self
                        .task_set_system_proxy(false, message::SystemProxyContext::CoreStop);
                }
                Task::none()
            }
            Message::NewLogBatch(lines) => {
                for line in lines {
                    if line.contains("Downloading sing-box core") {
                        self.core_install_state = state::CoreInstallState::Downloading;
                    } else if line.contains("Verifying core") {
                        self.core_install_state = state::CoreInstallState::Verifying;
                    } else if line.contains("Extracting core") {
                        self.core_install_state = state::CoreInstallState::Extracting;
                    } else if line.contains("Installing core") {
                        self.core_install_state = state::CoreInstallState::Installing;
                    }
                    // Trimmed once after the batch below.
                    self.log_lines
                        .push_back(truncate_log_line(&redact_log_line(&line)));
                }
                while self.log_lines.len() > MAX_LOG_LINES {
                    self.log_lines.pop_front();
                }
                // Tail scrolling is throttled by Tick so a log burst does not
                // trigger a full layout operation for every individual line.
                Task::none()
            }
            Message::ClearLogs => {
                self.log_lines.clear();
                Task::none()
            }
            Message::LogFilterChanged(filter) => {
                self.log_filter = filter;
                Task::none()
            }
            Message::LogSearchChanged(q) => {
                self.log_search = q;
                Task::none()
            }
            Message::ToggleLogsFollow => {
                self.logs_follow = !self.logs_follow;
                if self.logs_follow && !self.log_lines.is_empty() {
                    return iced::widget::operation::snap_to(
                        ui::logs::get_logs_scrollable_id().clone(),
                        iced::widget::scrollable::RelativeOffset::END,
                    );
                }
                Task::none()
            }
            Message::ExportLogs => {
                let lines = self.log_lines.clone();
                Task::perform(
                    async move {
                        let path = config::get_app_dir().join(format!(
                            "logs_export_{}.txt",
                            chrono::Local::now().format("%Y%m%d_%H%M%S")
                        ));
                        tokio::fs::write(&path, lines.into_iter().collect::<Vec<_>>().join("\n"))
                            .await
                            .map(|_| path.to_string_lossy().to_string())
                            .map_err(|e| e.to_string())
                    },
                    Message::LogsExported,
                )
            }
            Message::LogsExported(Ok(path)) => {
                self.push_log(format!("[GUI] Logs exported to {}", path));
                self.toast_success(self.tr("logs_exported").replace("{}", &path));
                task_open_path(std::path::PathBuf::from(path))
            }
            Message::LogsExported(Err(e)) => {
                self.toast_error(format!(
                    "{}: {}",
                    self.tr("logs_export_failed"),
                    redact_log_line(&e)
                ));
                Task::none()
            }
            Message::CoreVersionFetched(Ok(ver)) => {
                self.core_version = Some(ver);
                Task::none()
            }
            Message::CoreVersionFetched(Err(_)) => {
                self.core_version = None;
                Task::none()
            }
            Message::AutoUpdateIntervalChanged(hours) => {
                self.settings_draft.auto_update_interval_hours = hours;
                Task::none()
            }
            Message::ToggleDisableProxyOnCoreStop => {
                self.settings_draft.disable_proxy_on_core_stop =
                    !self.settings_draft.disable_proxy_on_core_stop;
                Task::none()
            }
            Message::ImportFromClipboard => iced::clipboard::read().map(Message::ClipboardContent),
            Message::ClipboardContent(Some(text)) => {
                let text = text.trim().to_string();
                if text.is_empty() {
                    self.toast_info(self.tr("toast_clipboard_empty"));
                    return Task::none();
                }
                self.url_input = text;
                Task::done(Message::DownloadSubscription)
            }
            Message::ClipboardContent(None) => {
                self.toast_error(self.tr("clipboard_read_failed"));
                Task::none()
            }
            Message::ImportLocalFile => Task::perform(
                async move {
                    rfd::AsyncFileDialog::new()
                        .add_filter("Config", &["yaml", "yml", "json", "txt"])
                        .pick_file()
                        .await
                        .map(|f| f.path().to_string_lossy().to_string())
                },
                Message::LocalFilePicked,
            ),
            Message::LocalFilePicked(Some(path)) => {
                self.url_input = path;
                Task::done(Message::DownloadSubscription)
            }
            Message::LocalFilePicked(None) => Task::none(),
            Message::TriggerCoreDownload => {
                if self.app_update_busy() {
                    self.toast_info(self.tr("operation_in_progress"));
                    return Task::none();
                }
                if self.core_running || self.core_busy() {
                    self.toast_info(self.tr("core_download_requires_stop"));
                    return Task::none();
                }
                if !matches!(
                    &self.core_install_state,
                    state::CoreInstallState::Idle
                        | state::CoreInstallState::Installed
                        | state::CoreInstallState::Error(_)
                ) {
                    return Task::none();
                }
                let log_tx = get_log_tx();
                self.push_log("[GUI] Starting sing-box core download...".to_string());
                self.core_install_state = state::CoreInstallState::Downloading;
                Task::perform(
                    async move { core::download_core(log_tx, false).await },
                    Message::CoreDownloaded,
                )
            }
            Message::ForceCoreDownload => {
                if self.app_update_busy() {
                    self.toast_info(self.tr("operation_in_progress"));
                    return Task::none();
                }
                if self.core_running || self.core_busy() {
                    self.toast_info(self.tr("core_download_requires_stop"));
                    return Task::none();
                }
                if !matches!(
                    &self.core_install_state,
                    state::CoreInstallState::Idle
                        | state::CoreInstallState::Installed
                        | state::CoreInstallState::Error(_)
                ) {
                    return Task::none();
                }
                let log_tx = get_log_tx();
                self.push_log("[GUI] Force reinstalling sing-box core...".to_string());
                self.core_install_state = state::CoreInstallState::Downloading;
                Task::perform(
                    async move { core::download_core(log_tx, true).await },
                    Message::CoreDownloaded,
                )
            }
            Message::LatencyTestUrlChanged(url) => {
                self.settings_draft.latency_test_url = url;
                self.settings_errors.remove("latency_url");
                Task::none()
            }
            Message::LatencyTestTimeoutChanged(s) => {
                self.settings_draft.latency_test_timeout_ms = s;
                self.settings_errors.remove("latency_timeout");
                Task::none()
            }
            Message::CoreDownloaded(res) => {
                match res {
                    Ok(_) => {
                        self.core_installed = true;
                        self.core_install_state = state::CoreInstallState::Installed;
                        self.push_log(
                            "[GUI] sing-box core downloaded and installed successfully."
                                .to_string(),
                        );
                        self.toast_success(self.tr("core_download_success"));
                        let cfg = self.gui_config.clone();
                        return Task::perform(
                            async move {
                                tokio::task::spawn_blocking(move || core::get_core_version(&cfg))
                                    .await
                                    .map_err(|e| e.to_string())
                                    .and_then(|r| r)
                            },
                            Message::CoreVersionFetched,
                        );
                    }
                    Err(e) => {
                        let detail = redact_log_line(&e);
                        self.core_install_state = state::CoreInstallState::Error(detail.clone());
                        self.push_log(format!("[GUI ERROR] Failed to download core: {detail}"));
                        self.toast_error(
                            self.tr("core_install_failed").to_string() + ": " + &detail,
                        );
                    }
                }
                Task::none()
            }
            Message::TrafficUpdated { up, down } => {
                self.current_speed = Bandwidth { up, down };
                Task::none()
            }
            Message::ToggleSystemProxy => {
                if self.system_proxy_busy {
                    return Task::none();
                }
                let target = !self.sys_proxy_enabled;
                self.task_set_system_proxy(target, message::SystemProxyContext::Manual)
            }
            Message::SubscriptionInputChanged(url) => {
                self.url_input = url;
                self.profile_error = None;
                Task::none()
            }
            Message::DownloadSubscription => {
                if self.url_input.trim().is_empty() {
                    return Task::none();
                }
                // The Download button disables while downloading, but Enter in
                // the URL field and the clipboard/file import paths land here
                // too — guard them all against concurrent fetches.
                if self.downloading {
                    self.toast_info(self.tr("operation_in_progress"));
                    return Task::none();
                }
                self.downloading = true;
                self.profile_error = None;
                let url = self.url_input.clone();

                Task::perform(download_profile(url), |res| match res {
                    Ok(r) => Message::SubscriptionDownloaded {
                        id: r.id,
                        source_url: Some(r.source_url),
                        error: None,
                        traffic_upload: r.traffic_upload,
                        traffic_download: r.traffic_download,
                        traffic_total: r.traffic_total,
                        expire_at: r.expire_at,
                        display_name: r.display_name,
                    },
                    Err(e) => Message::SubscriptionDownloaded {
                        id: String::new(),
                        error: Some(e),
                        traffic_upload: None,
                        traffic_download: None,
                        traffic_total: None,
                        expire_at: None,
                        display_name: None,
                        source_url: None,
                    },
                })
            }
            Message::SubscriptionDownloaded {
                id,
                error,
                traffic_upload,
                traffic_download,
                traffic_total,
                expire_at,
                display_name,
                source_url,
            } => {
                self.downloading = false;
                self.updating_profile_id = None;
                let mut load_nodes = false;
                if let Some(err) = error {
                    let detail = redact_log_line(&err);
                    self.profile_error =
                        Some(format!("{}: {detail}", self.tr("profile_download_failed")));
                    self.push_log(format!("[GUI] Download failed: {detail}"));
                    self.toast_error(format!("{}: {detail}", self.tr("profile_download_failed")));
                } else {
                    self.profile_error = None;
                    self.url_input.clear();

                    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                    // Mutate in-memory config only (profile file already written by fetch task).
                    // Never re-load gui_config from disk here — that races concurrent saves.
                    if !id.is_empty() {
                        if let Some(p) = self
                            .gui_config
                            .subscriptions
                            .iter_mut()
                            .find(|p| p.id == id)
                        {
                            p.updated_at = now;
                            if traffic_upload.is_some() {
                                p.traffic_upload = traffic_upload;
                            }
                            if traffic_download.is_some() {
                                p.traffic_download = traffic_download;
                            }
                            if traffic_total.is_some() {
                                p.traffic_total = traffic_total;
                            }
                            if expire_at.is_some() {
                                p.expire_at = expire_at;
                            }
                            if let Some(ref name) = display_name
                                && !name.is_empty()
                            {
                                p.name = name.clone();
                            }
                        } else {
                            let name =
                                display_name.filter(|n| !n.is_empty()).unwrap_or_else(|| {
                                    format!("Sub_{}", &id[id.len().saturating_sub(6)..])
                                });
                            let path = config::get_profile_path(&id);
                            let url = source_url.unwrap_or_default();
                            self.gui_config.subscriptions.push(Profile {
                                id: id.clone(),
                                name,
                                url,
                                file_path: path.to_string_lossy().to_string(),
                                is_subscription: true,
                                updated_at: now,
                                traffic_upload,
                                traffic_download,
                                traffic_total,
                                expire_at,
                            });
                        }
                        self.config_dirty = true;
                    }

                    let is_active =
                        self.gui_config.active_profile_id.as_deref() == Some(id.as_str());
                    if self.gui_config.active_profile_id.is_none() && !id.is_empty() {
                        self.gui_config.active_profile_id = Some(id.clone());
                        self.config_dirty = true;
                        load_nodes = true;
                    } else if is_active {
                        load_nodes = true;
                    }
                    self.push_log("[GUI] Subscription downloaded successfully.".to_string());
                    self.toast_success(self.tr("toast_sub_ok"));
                }
                let next_update = self.kick_pending_auto_update();
                let save_task = self.start_config_save();
                let is_active = self.gui_config.active_profile_id.as_deref() == Some(id.as_str());
                let mut tasks = vec![next_update, save_task];
                if load_nodes {
                    tasks.push(self.load_active_nodes_task());
                    if is_active && self.core_running {
                        tasks.push(self.restart_core());
                    }
                }
                Task::batch(tasks)
            }
            Message::SelectProfile(id) => {
                if self.downloading {
                    // Switching mid-download would restart the core against a
                    // profile file the in-flight update is about to replace.
                    self.toast_info(self.tr("operation_in_progress"));
                    return Task::none();
                }
                self.confirm_delete_profile_id = None;
                self.gui_config.active_profile_id = Some(id);
                self.gui_config.selected_node_tag = None;
                self.active_profile_nodes.clear();
                self.proxy_groups.clear();
                self.selected_group.clear();
                self.node_search.clear();
                self.config_dirty = true;
                self.push_log("[GUI] Active profile updated.".to_string());
                self.toast_success(self.tr("profile_selected_toast"));
                let save_task = self.start_config_save();
                Task::batch(vec![
                    save_task,
                    self.load_active_nodes_task(),
                    self.restart_core(),
                ])
            }
            Message::ActiveNodesLoaded { profile_id, result } => {
                if self.gui_config.active_profile_id.as_deref() != Some(profile_id.as_str()) {
                    return Task::none();
                }
                self.active_profile_nodes = result.unwrap_or_default();
                Task::none()
            }
            Message::RequestDeleteProfile(id) => {
                self.confirm_delete_profile_id = Some(id);
                Task::none()
            }
            Message::CancelDeleteProfile => {
                self.confirm_delete_profile_id = None;
                Task::none()
            }
            Message::ConfirmDeleteProfile => {
                let id = self.confirm_delete_profile_id.take();
                if let Some(id) = id {
                    let path = config::get_profile_path(&id);
                    let was_active = self.gui_config.active_profile_id.as_deref() == Some(&id);
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || remove_profile_file(&path))
                                .await
                                .map_err(|error| error.to_string())
                                .and_then(|result| result)
                        },
                        move |result| Message::ProfileDeleteFinished {
                            id,
                            was_active,
                            result,
                        },
                    );
                }
                Task::none()
            }
            Message::ProfileDeleteFinished {
                id,
                was_active,
                result,
            } => {
                if let Err(error) = result {
                    self.toast_error(format!("{}: {error}", self.tr("profile_delete_failed")));
                    return Task::none();
                }
                self.gui_config.subscriptions.retain(|p| p.id != id);
                if was_active {
                    self.gui_config.active_profile_id = None;
                    self.gui_config.selected_node_tag = None;
                    self.active_profile_nodes.clear();
                }
                self.config_dirty = true;
                self.push_log("[GUI] Profile deleted.".to_string());
                self.toast_success(self.tr("toast_profile_deleted"));
                let save_task = self.start_config_save();

                // Active profile deleted while core runs → stop to avoid orphan config.
                if was_active {
                    self.pending_core_restart = false;
                    if self.core_starting {
                        self.force_stop_after_start = true;
                        self.toast_info(self.tr("toast_core_stopped_profile_deleted"));
                    } else if self.core_running && !self.core_stopping {
                        self.toast_info(self.tr("toast_core_stopped_profile_deleted"));
                        return Task::batch(vec![save_task, self.task_stop_core()]);
                    }
                }
                save_task
            }
            Message::SelectNode(tag) => {
                if !self.core_running {
                    self.selected_node_tag = Some(tag.clone());
                    self.gui_config.selected_node_tag = Some(tag.clone());
                    self.config_dirty = true;
                    return self.start_config_save();
                }
                let tag_clone = tag.clone();
                let api_port = self.gui_config.api_port;

                Task::perform(
                    async move { api::select_proxy(api_port, "Proxy", &tag_clone).await },
                    move |res| match res {
                        Ok(_) => Message::NodeSelected {
                            tag: tag.clone(),
                            error: None,
                        },
                        Err(e) => Message::NodeSelected {
                            tag: tag.clone(),
                            error: Some(e),
                        },
                    },
                )
            }
            Message::NodeSelected { tag, error } => {
                if let Some(err) = error {
                    let detail = redact_log_line(&err);
                    self.push_log(format!("[GUI] Failed to select node: {}", detail));
                    self.toast_error(format!("{}: {detail}", self.tr("node_select_failed")));
                } else {
                    self.selected_node_tag = Some(tag.clone());
                    self.gui_config.selected_node_tag = Some(tag.clone());
                    self.config_dirty = true;
                    self.push_log(format!("[GUI] Selected node: {}", tag));
                    return self.start_config_save();
                }
                Task::none()
            }
            Message::StartLatencyTest => {
                if self.latency_testing {
                    return Task::none();
                }
                if !self.core_running {
                    self.toast_info(self.tr("toast_start_core_first"));
                    return Task::none();
                }
                if !self.active_profile_nodes.iter().any(|node| node.selectable) {
                    self.toast_info(self.tr("toast_no_nodes"));
                    return Task::none();
                }
                self.latency_testing = true;
                let api_port = self.gui_config.api_port;
                let test_url = self.gui_config.latency_test_url.clone();
                let timeout_ms = self.gui_config.latency_test_timeout_ms;
                // Cap concurrent delay probes so Clash API / UI are not stampeded.
                let node_tags: Vec<String> = self
                    .active_profile_nodes
                    .iter()
                    .filter(|node| node.selectable)
                    .map(|node| node.name.clone())
                    .collect();

                let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(8));
                let chunks = node_tags.chunks(20).map(|c| c.to_vec()).collect::<Vec<_>>();
                let tasks = chunks
                    .into_iter()
                    .map(|chunk| {
                        let test_url = test_url.clone();
                        let sem = std::sync::Arc::clone(&sem);
                        Task::perform(
                            async move {
                                let mut results = Vec::with_capacity(chunk.len());
                                for tag in chunk {
                                    let _permit = sem.acquire().await;
                                    let latency = api::test_node_latency(
                                        api_port, &tag, &test_url, timeout_ms,
                                    )
                                    .await;
                                    results.push((tag, latency.ok()));
                                }
                                results
                            },
                            Message::NodeLatencyBatch,
                        )
                    })
                    .collect::<Vec<_>>();

                Task::batch(tasks).chain(Task::done(Message::LatencyTestComplete))
            }
            Message::NodeLatencyBatch(batch) => {
                for (tag, latency) in batch {
                    for node in &mut self.active_profile_nodes {
                        if node.name == tag {
                            node.latency = latency;
                        }
                    }
                }
                Task::none()
            }
            Message::LatencyTestComplete => {
                self.latency_testing = false;
                self.toast_success(self.tr("toast_latency_done"));
                Task::none()
            }
            Message::UpdateSubscription(id) => {
                if self.downloading {
                    self.toast_info(self.tr("operation_in_progress"));
                    return Task::none();
                }
                self.confirm_delete_profile_id = None;
                let sub = self.gui_config.subscriptions.iter().find(|p| p.id == id);
                if let Some(profile) = sub {
                    let url = profile.url.clone();
                    let id_clone = id.clone();
                    self.downloading = true;
                    self.updating_profile_id = Some(id.clone());
                    self.profile_error = None;
                    self.push_log(format!(
                        "[GUI] Updating subscription: {}",
                        ui::profiles::mask_sensitive_url(&url)
                    ));

                    Task::perform(
                        async move { fetch_and_save_subscription(url, id_clone).await },
                        |res| match res {
                            Ok(r) => Message::SubscriptionDownloaded {
                                id: r.id,
                                error: None,
                                traffic_upload: r.traffic_upload,
                                traffic_download: r.traffic_download,
                                traffic_total: r.traffic_total,
                                expire_at: r.expire_at,
                                display_name: r.display_name,
                                source_url: Some(r.source_url),
                            },
                            Err(e) => Message::SubscriptionDownloaded {
                                id: String::new(),
                                error: Some(e),
                                traffic_upload: None,
                                traffic_download: None,
                                traffic_total: None,
                                expire_at: None,
                                display_name: None,
                                source_url: None,
                            },
                        },
                    )
                } else {
                    self.push_log("[GUI] Subscription not found.".to_string());
                    Task::none()
                }
            }
            Message::AutoUpdateDue(ids) => {
                if ids.is_empty() {
                    return Task::none();
                }
                enqueue_pending_updates(&mut self.pending_auto_updates, ids);
                self.kick_pending_auto_update()
            }
            Message::Tick => {
                let mut tasks = Vec::new();
                let retry_ready = self
                    .config_save_retry_at
                    .is_none_or(|deadline| std::time::Instant::now() >= deadline);
                if self.config_dirty
                    && !self.config_save_in_flight
                    && !self.autostart_busy
                    && retry_ready
                {
                    tasks.push(self.start_config_save());
                }

                // Advance the chart exactly once per wall-clock tick. Traffic
                // events only update the latest sample, so zero or constant
                // speeds no longer freeze the time axis.
                advance_speed_history(&mut self.speed_history, &self.current_speed);

                // Lock-free fast path; during async start/stop prefer the transition flags.
                if !self.core_busy() {
                    let was_running = self.core_running;
                    self.core_running = core::is_core_running_fast();
                    // The cache can flip before `CoreLivenessChecked` lands;
                    // run the crash cleanup on whichever path sees it first.
                    if was_running && !self.core_running {
                        tasks.push(self.handle_unexpected_core_exit());
                    }
                }

                let check_authoritative = self.core_running
                    && !self.core_busy()
                    && self.tick_authority_counter.is_multiple_of(5);
                self.tick_authority_counter = self.tick_authority_counter.wrapping_add(1);

                // Auto-dismiss toast
                if let Some(ref mut toast) = self.toast {
                    if toast.remaining_secs > 0 {
                        toast.remaining_secs -= 1;
                    }
                    if toast.remaining_secs == 0 {
                        self.toast = None;
                    }
                }

                // Auto-update scan every 60 seconds
                self.auto_update_tick_counter = self.auto_update_tick_counter.saturating_add(1);
                if self.logs_follow
                    && self.current_tab == state::Tab::Logs
                    && !self.log_lines.is_empty()
                {
                    tasks.push(iced::widget::operation::snap_to(
                        ui::logs::get_logs_scrollable_id().clone(),
                        iced::widget::scrollable::RelativeOffset::END,
                    ));
                }
                if self.auto_update_tick_counter >= 60 {
                    self.auto_update_tick_counter = 0;
                    let hours = self.gui_config.auto_update_interval_hours;
                    if hours > 0 && !self.downloading {
                        let due = collect_due_subscription_ids(&self.gui_config, hours);
                        if !due.is_empty() {
                            tasks.push(Task::done(Message::AutoUpdateDue(due)));
                        }
                    }
                }

                // Tab-aware API polling — avoid 1 Hz full scrapes on idle tabs.
                if self.core_running && !self.core_busy() {
                    self.poll_tick_counter = self.poll_tick_counter.wrapping_add(1);
                    let tick = self.poll_tick_counter;
                    let api_port = self.gui_config.api_port;
                    let (want_proxies, want_connections) = should_poll_api(self.current_tab, tick);

                    if want_proxies && !self.proxies_fetch_in_flight {
                        self.proxies_fetch_in_flight = true;
                        tasks.push(Task::perform(
                            async move { api::fetch_proxies(api_port).await },
                            |res| Message::ProxiesFetched(res.map(|r| r.proxies)),
                        ));
                    }
                    if want_connections {
                        tasks.push(Task::done(Message::FetchConnections));
                    }
                }

                if check_authoritative {
                    tasks.push(Task::perform(
                        async move {
                            tokio::task::spawn_blocking(core::is_core_running)
                                .await
                                .unwrap_or(false)
                        },
                        Message::CoreLivenessChecked,
                    ));
                }

                Task::batch(tasks)
            }
            Message::ConfigSaved {
                saved_config,
                result,
            } => {
                self.config_save_in_flight = false;
                let mut tasks = vec![self.refresh_config_preview()];
                match result {
                    Ok(()) => {
                        self.config_save_failures = 0;
                        self.config_save_retry_at = None;
                        self.config_dirty = self.gui_config != saved_config;
                        if self.config_dirty {
                            tasks.push(self.start_config_save());
                        } else if self.pending_update.is_some() {
                            tasks.push(self.continue_update_install());
                        } else if self.pending_exit {
                            tasks.push(self.continue_exit());
                        } else {
                            if self.pending_settings_save_feedback
                                && !self.pending_proxy_reapply_after_save
                            {
                                self.pending_settings_save_feedback = false;
                                self.toast_success(self.tr("settings_saved_toast"));
                            }
                            if self.pending_proxy_reapply_after_save {
                                self.pending_proxy_reapply_after_save = false;
                                tasks.push(self.task_set_system_proxy(
                                    true,
                                    message::SystemProxyContext::SettingsReapply,
                                ));
                            } else if self.pending_restart_after_save {
                                self.pending_restart_after_save = false;
                                tasks.push(self.restart_core());
                            }
                        }
                    }
                    Err(error) => {
                        self.config_dirty = true;
                        self.config_save_failures = self.config_save_failures.saturating_add(1);
                        let exponent = self.config_save_failures.saturating_sub(1).min(5);
                        let delay_secs = 2u64.saturating_pow(exponent).min(60);
                        self.config_save_retry_at = Some(
                            std::time::Instant::now() + std::time::Duration::from_secs(delay_secs),
                        );
                        let should_log = self.last_config_save_log_at.is_none_or(|last| {
                            last.elapsed() >= std::time::Duration::from_secs(30)
                        });
                        if should_log {
                            self.last_config_save_log_at = Some(std::time::Instant::now());
                            self.push_log(format!("[GUI] Failed to save settings: {error}"));
                        }
                        if self.pending_exit {
                            self.pending_exit = false;
                            self.toast_error(format!("{}: {error}", self.tr("config_save_failed")));
                        } else if let Some(pending) = self.pending_update.take() {
                            tasks.push(self.fail_pending_update(
                                pending,
                                format!("{}: {error}", self.tr("config_save_failed")),
                            ));
                        }
                    }
                }
                Task::batch(tasks)
            }
            Message::StartupSystemProxyChecked(result) => {
                match result {
                    Ok(enabled) => {
                        self.sys_proxy_enabled = enabled;
                        let owned = enabled && self.gui_config.system_proxy_owned;
                        sysproxy::restore_system_proxy_owned(owned);
                        if self.gui_config.system_proxy_enabled != enabled
                            || self.gui_config.system_proxy_owned != owned
                        {
                            self.gui_config.system_proxy_enabled = enabled;
                            self.gui_config.system_proxy_owned = owned;
                            self.config_dirty = true;
                            return self.start_config_save();
                        }
                    }
                    Err(error) => {
                        self.push_log(format!("[GUI] Failed to detect system proxy: {error}"))
                    }
                }
                Task::none()
            }
            Message::AutostartSetFinished { target, result } => {
                self.autostart_busy = false;
                match result {
                    Ok(()) => {
                        self.settings_draft = RuntimeSettingsDraft::from_config(&self.gui_config);
                        let save_task = self.start_config_save();
                        if self.pending_exit {
                            Task::batch(vec![save_task, self.continue_exit()])
                        } else {
                            save_task
                        }
                    }
                    Err(error) => {
                        self.gui_config.start_on_boot = !target;
                        self.settings_draft.start_on_boot = !target;
                        self.settings_errors
                            .insert("autostart", "autostart_apply_failed");
                        self.toast_error(format!("{}: {error}", self.tr("autostart_apply_failed")));
                        self.pending_exit = false;
                        self.start_config_save()
                    }
                }
            }
            Message::SystemProxySetFinished {
                target,
                context,
                result,
            } => {
                self.system_proxy_busy = false;
                match result {
                    Ok(()) => {
                        self.sys_proxy_enabled = target;
                        let proxy_config_changed = self.gui_config.system_proxy_enabled != target
                            || self.gui_config.system_proxy_owned != target;
                        self.gui_config.system_proxy_enabled = target;
                        self.gui_config.system_proxy_owned = target;
                        sysproxy::restore_system_proxy_owned(target);
                        self.config_dirty |= proxy_config_changed;
                        self.push_log(format!(
                            "[GUI] System proxy changed to {target} ({context:?})."
                        ));
                        if context == message::SystemProxyContext::Exit {
                            return self.continue_exit();
                        }
                        if context == message::SystemProxyContext::Update {
                            return self.continue_update_install();
                        }
                        if context == message::SystemProxyContext::SettingsReapply {
                            if self.pending_settings_save_feedback {
                                self.pending_settings_save_feedback = false;
                                self.toast_success(self.tr("settings_saved_toast"));
                            }
                            if self.pending_restart_after_save {
                                self.pending_restart_after_save = false;
                                return self.restart_core();
                            }
                        }
                        self.start_config_save()
                    }
                    Err(error) => {
                        self.push_log(format!(
                            "[GUI] Failed to change system proxy ({context:?}): {error}"
                        ));
                        self.toast_error(format!(
                            "{}: {error}",
                            self.tr("system_proxy_apply_failed")
                        ));
                        if context == message::SystemProxyContext::Exit {
                            self.pending_exit = false;
                        }
                        if context == message::SystemProxyContext::SettingsReapply {
                            self.pending_settings_save_feedback = false;
                            self.sys_proxy_enabled = false;
                            self.gui_config.system_proxy_enabled = false;
                            self.gui_config.system_proxy_owned = false;
                            sysproxy::restore_system_proxy_owned(false);
                            self.config_dirty = true;
                            let save_task = self.start_config_save();
                            if self.pending_restart_after_save {
                                self.pending_restart_after_save = false;
                                return Task::batch(vec![save_task, self.restart_core()]);
                            }
                            return save_task;
                        }
                        if context == message::SystemProxyContext::Update
                            && let Some(pending) = self.pending_update.take()
                        {
                            return self.fail_pending_update(
                                pending,
                                format!("{}: {error}", self.tr("system_proxy_apply_failed")),
                            );
                        }
                        Task::none()
                    }
                }
            }
            Message::SystemThemeDetected(is_light) => {
                self.cached_system_is_light = is_light;
                Task::none()
            }
            Message::CoreLivenessChecked(running) => {
                // Only a running → exited transition matters. Ignore results
                // that are stale relative to a stop/start issued meanwhile.
                if running || self.core_busy() || !self.core_running || core::is_core_running_fast()
                {
                    return Task::none();
                }
                self.handle_unexpected_core_exit()
            }
            Message::FetchConnections => {
                if self.core_running && !self.connections_fetch_in_flight {
                    self.connections_fetch_in_flight = true;
                    let api_port = self.gui_config.api_port;
                    return Task::perform(
                        async move { api::fetch_connections(api_port).await },
                        Message::ConnectionsFetched,
                    );
                }
                Task::none()
            }
            Message::ConnectionsFetched(Ok(res)) => {
                if !self.core_running || self.core_stopping {
                    self.connections_fetch_in_flight = false;
                    return Task::none();
                }
                self.connections_fetch_in_flight = false;
                self.last_connections_fetch = Some(std::time::Instant::now());
                let mut connections = res.connections.unwrap_or_default();
                connections.truncate(MAX_CONNECTION_SNAPSHOT);
                if self.active_connections != connections {
                    self.active_connections = connections;
                }
                if self.total_downloaded != res.download_total {
                    self.total_downloaded = res.download_total;
                }
                if self.total_uploaded != res.upload_total {
                    self.total_uploaded = res.upload_total;
                }
                Task::none()
            }
            Message::ConnectionsFetched(Err(_e)) => {
                self.connections_fetch_in_flight = false;
                // Suppress background polling HTTP errors
                Task::none()
            }
            Message::CloseConnection(id) => {
                if self.core_running {
                    let api_port = self.gui_config.api_port;
                    let id_clone = id.clone();
                    return Task::perform(
                        async move {
                            match api::close_connection(api_port, &id_clone).await {
                                Ok(_) => Ok(id_clone),
                                Err(e) => Err(e),
                            }
                        },
                        Message::ConnectionClosed,
                    );
                }
                Task::none()
            }
            Message::ConnectionClosed(Ok(id)) => {
                self.push_log(format!("[GUI] Closed connection {}", id));
                self.active_connections.retain(|c| c.id != id);
                Task::none()
            }
            Message::ConnectionClosed(Err(e)) => {
                self.push_log(format!("[GUI] Failed to close connection: {}", e));
                self.toast_error(format!(
                    "{}: {}",
                    self.tr("connection_close_failed"),
                    redact_log_line(&e)
                ));
                Task::none()
            }
            Message::CloseAllConnections => {
                if !self.core_running {
                    self.toast_info(self.tr("core_not_running"));
                    return Task::none();
                }
                let api_port = self.gui_config.api_port;
                Task::perform(
                    async move { api::close_all_connections(api_port).await },
                    Message::AllConnectionsClosed,
                )
            }
            Message::AllConnectionsClosed(Ok(())) => {
                self.active_connections.clear();
                self.push_log("[GUI] Closed all connections.".to_string());
                self.toast_success(self.tr("all_connections_closed"));
                Task::none()
            }
            Message::AllConnectionsClosed(Err(e)) => {
                self.push_log(format!("[GUI] Failed to close all connections: {}", e));
                self.toast_error(format!(
                    "{}: {}",
                    self.tr("connections_close_failed"),
                    redact_log_line(&e)
                ));
                Task::none()
            }
            Message::RoutingModeChanged(mode) => {
                if self.core_busy() || self.pending_routing_mode.is_some() {
                    self.toast_info(self.tr("operation_in_progress"));
                    return Task::none();
                }
                if mode == self.gui_config.routing_mode {
                    return Task::none();
                }
                if self.core_running {
                    self.pending_routing_mode = Some(mode);
                    let api_port = self.gui_config.api_port;
                    Task::perform(
                        async move { api::set_mode(api_port, mode.as_clash_mode()).await },
                        move |result| Message::ModeSet { mode, result },
                    )
                } else {
                    self.gui_config.routing_mode = mode;
                    self.config_dirty = true;
                    self.toast_success(
                        self.tr("routing_mode_pending")
                            .replace("{}", mode.as_clash_mode()),
                    );
                    self.start_config_save()
                }
            }
            Message::ModeSet { mode, result } => {
                if self.pending_routing_mode != Some(mode) {
                    return Task::none();
                }
                self.pending_routing_mode = None;
                if !self.core_running || self.core_busy() {
                    return Task::none();
                }
                match result {
                    Ok(()) => {
                        // Persist/display only a mode acknowledged by the core.
                        self.gui_config.routing_mode = mode;
                        self.config_dirty = true;
                        self.toast_success(
                            self.tr("routing_mode_switched")
                                .replace("{}", mode.as_clash_mode()),
                        );
                        self.start_config_save()
                    }
                    Err(error) => {
                        self.toast_error(
                            self.tr("routing_mode_failed")
                                .replace("{}", &redact_log_line(&error)),
                        );
                        Task::none()
                    }
                }
            }
            Message::DismissToast => {
                self.toast = None;
                Task::none()
            }

            // New type-safe configuration messages
            Message::MixedPortChanged(val) => {
                self.settings_draft.mixed_port = val;
                self.settings_errors.remove("mixed_port");
                Task::none()
            }
            Message::ApiPortChanged(val) => {
                self.settings_draft.api_port = val;
                self.settings_errors.remove("api_port");
                Task::none()
            }
            Message::DnsLocalChanged(val) => {
                self.settings_draft.dns_server_local = val;
                self.settings_errors.remove("dns_local");
                Task::none()
            }
            Message::DnsRemoteChanged(val) => {
                self.settings_draft.dns_server_remote = val;
                self.settings_errors.remove("dns_remote");
                Task::none()
            }
            Message::CorePathChanged(val) => {
                self.settings_draft.core_path = val;
                self.settings_errors.remove("core_path");
                Task::none()
            }
            Message::ClearCorePath => {
                self.settings_draft.core_path.clear();
                self.settings_errors.remove("core_path");
                self.toast_info(self.tr("core_path_default_draft"));
                Task::none()
            }
            Message::ToggleTun => {
                self.settings_draft.tun_mode = !self.settings_draft.tun_mode;
                Task::none()
            }
            Message::ToggleAutostart => {
                self.settings_draft.start_on_boot = !self.settings_draft.start_on_boot;
                self.settings_errors.remove("autostart");
                Task::none()
            }
            Message::ToggleAutoStartCore => {
                self.settings_draft.auto_start_core = !self.settings_draft.auto_start_core;
                Task::none()
            }
            Message::ToggleAutoSysProxy => {
                self.settings_draft.auto_sys_proxy = !self.settings_draft.auto_sys_proxy;
                Task::none()
            }
            Message::SetLanguage(lang) => {
                self.gui_config.language = lang;
                self.config_dirty = true;
                self.update_tray_menu();
                self.start_config_save()
            }
            Message::SetTheme(theme) => {
                self.gui_config.theme = theme;
                self.config_dirty = true;
                self.start_config_save()
            }
            Message::OpenDataDir => task_open_path(config::get_app_dir()),
            Message::OpenProfilesFolder => task_open_path(config::get_app_dir().join("profiles")),
            Message::EditProfile(id) => {
                let path = config::get_profile_path(&id);
                task_open_path(path)
            }
            Message::StartEditProfile(id) => {
                if self.downloading {
                    self.toast_info(self.tr("operation_in_progress"));
                    return Task::none();
                }
                if let Some(profile) = self.gui_config.subscriptions.iter().find(|p| p.id == id) {
                    self.editing_profile_id = Some(id);
                    self.editing_profile_name = profile.name.clone();
                    self.editing_profile_url = profile.url.clone();
                }
                Task::none()
            }
            Message::EditProfileNameChanged(name) => {
                self.editing_profile_name = name;
                Task::none()
            }
            Message::EditProfileUrlChanged(url) => {
                self.editing_profile_url = url;
                Task::none()
            }
            Message::SaveProfileEdit => {
                if self.downloading {
                    self.toast_info(self.tr("operation_in_progress"));
                    return Task::none();
                }
                if let Some(id) = self.editing_profile_id.clone() {
                    let name = self.editing_profile_name.trim().to_string();
                    let url = self.editing_profile_url.trim().to_string();
                    if name.is_empty() {
                        self.toast_error(self.tr("profile_name_required"));
                        return Task::none();
                    }
                    if !is_valid_profile_source(&url) {
                        self.toast_error(self.tr("profile_url_invalid"));
                        return Task::none();
                    }
                    if let Some(profile) = self
                        .gui_config
                        .subscriptions
                        .iter_mut()
                        .find(|p| p.id == id)
                    {
                        let url_changed = profile.url != url;
                        profile.name = name;
                        profile.url = url;
                        self.config_dirty = true;
                        if url_changed {
                            self.toast_info(self.tr("profile_url_update_required"));
                        }
                    }
                    self.editing_profile_id = None;
                    // Profile edits are an explicit save action. Persist them
                    // immediately instead of waiting for the next one-second
                    // tick (the retry path still handles a transient failure).
                    return self.start_config_save();
                }
                Task::none()
            }
            Message::CancelProfileEdit => {
                self.editing_profile_id = None;
                Task::none()
            }
            Message::SortConnections(col) => {
                if self.connections_sort == col {
                    self.connections_sort_desc = !self.connections_sort_desc;
                } else {
                    self.connections_sort = col;
                    self.connections_sort_desc = matches!(
                        col,
                        state::ConnectionSort::Download | state::ConnectionSort::Upload
                    );
                }
                Task::none()
            }
            Message::SetProxySort(mode) => {
                self.proxy_sort = mode;
                Task::none()
            }
            Message::ToggleConfigPreview => {
                self.config_preview_expanded = !self.config_preview_expanded;
                self.refresh_config_preview()
            }
            Message::ConfigPreviewGenerated(text) => {
                if self.config_preview_expanded {
                    self.config_preview = Some(text);
                }
                Task::none()
            }
            Message::ToggleProfileMore(id) => {
                // A pending delete confirmation also holds the row open, so
                // collapsing it has to dismiss the confirmation too.
                let confirming = self.confirm_delete_profile_id.as_deref() == Some(id.as_str());
                if confirming || self.profile_more_id.as_deref() == Some(id.as_str()) {
                    self.profile_more_id = None;
                    if confirming {
                        self.confirm_delete_profile_id = None;
                    }
                } else {
                    self.profile_more_id = Some(id);
                }
                Task::none()
            }
            Message::ToggleCloseCoreOnExit => {
                self.settings_draft.close_core_on_exit = !self.settings_draft.close_core_on_exit;
                Task::none()
            }
            Message::ToggleFakeIp => {
                self.settings_draft.fake_ip = !self.settings_draft.fake_ip;
                Task::none()
            }
            Message::ToggleTcpFastOpen => {
                self.settings_draft.tcp_fast_open = !self.settings_draft.tcp_fast_open;
                Task::none()
            }
            Message::ToggleTcpMultipath => {
                self.settings_draft.tcp_multipath = !self.settings_draft.tcp_multipath;
                Task::none()
            }
            Message::SaveSettings => {
                if self.autostart_busy {
                    // A second commit while set_autostart runs would race the
                    // first writer and can leave the OS state opposite the UI.
                    self.toast_info(self.tr("operation_in_progress"));
                    return Task::none();
                }
                self.pending_settings_save_feedback = true;
                match self.commit_runtime_settings(false) {
                    Ok(task) => {
                        if !self.config_dirty && !self.autostart_busy {
                            self.pending_settings_save_feedback = false;
                            self.toast_success(self.tr("settings_saved_toast"));
                        }
                        task
                    }
                    Err(()) => {
                        self.pending_settings_save_feedback = false;
                        Task::none()
                    }
                }
            }
            Message::DiscardSettings => {
                self.settings_draft = RuntimeSettingsDraft::from_config(&self.gui_config);
                self.settings_errors.clear();
                Task::none()
            }
            Message::CheckUpdate => {
                if self.maintenance_busy() {
                    self.toast_info(self.tr("operation_in_progress"));
                    return Task::none();
                }
                self.update_status = state::UpdateStatus::Checking;
                Task::perform(check_app_update(), Message::UpdateChecked)
            }
            Message::UpdateChecked(result) => {
                match result {
                    Ok(info) => {
                        let local_version = env!("CARGO_PKG_VERSION");
                        if update::is_remote_version_newer(local_version, info.tag_name.trim()) {
                            self.update_status = state::UpdateStatus::NewVersion {
                                tag: info.tag_name,
                                download_url: info.download_url,
                                sha256: info.sha256,
                                size: info.size,
                            };
                        } else {
                            // Same or lower remote tag, or both unreadable → up-to-date.
                            self.update_status = state::UpdateStatus::UpToDate;
                        }
                    }
                    Err(e) => {
                        self.update_status = state::UpdateStatus::Error(e);
                    }
                }
                Task::none()
            }
            Message::DownloadAppUpdate {
                tag,
                url,
                sha256,
                size,
            } => {
                if self.maintenance_busy() || self.pending_exit {
                    self.toast_info(self.tr("operation_in_progress"));
                    return Task::none();
                }
                self.update_status = state::UpdateStatus::Downloading { tag: tag.clone() };
                self.toast_info(self.tr("toast_update_downloading"));
                let url_for_msg = url.clone();
                let digest_for_msg = sha256.clone();
                Task::perform(
                    download_app_update_binary(tag.clone(), url, sha256, size),
                    move |result| Message::AppUpdateDownloaded {
                        tag,
                        url: url_for_msg,
                        sha256: digest_for_msg,
                        size,
                        result,
                    },
                )
            }
            Message::AppUpdateDownloaded {
                tag,
                url,
                sha256,
                size,
                result,
            } => {
                match result {
                    Ok(path) => {
                        self.push_log(format!(
                            "[GUI] Update {} downloaded to {}",
                            tag,
                            path.display()
                        ));
                        self.toast_info(self.tr("toast_update_installing"));
                        // Stop core and clear proxy before replacing the binary.
                        self.force_stop_after_start = false;
                        self.pending_core_restart = false;
                        self.pending_update = Some(PendingAppUpdate {
                            tag: tag.clone(),
                            url,
                            sha256,
                            size,
                            path,
                        });
                        self.update_status = state::UpdateStatus::Installing { tag };
                        self.continue_update_install()
                    }
                    Err(e) => {
                        self.push_log(format!("[GUI] Update download failed: {}", e));
                        // Keep download URL so the user can retry in-app.
                        self.update_status = state::UpdateStatus::NewVersion {
                            tag,
                            download_url: Some(url),
                            sha256: Some(sha256),
                            size: Some(size),
                        };
                        self.toast_error(self.tr("update_download_failed").replace("{}", &e));
                        Task::none()
                    }
                }
            }
            Message::OpenUrl(url) => {
                // `open::that` delegates to ShellExecuteW / open / xdg-open
                // safely, avoiding cmd-shell injection on user-controlled URLs.
                Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || open::that(&url)).await;
                    },
                    |_| Message::NoOp,
                )
            }
        }
    }

    fn view(&self, _window: iced::window::Id) -> Element<'_, Message> {
        let lang = self.gui_config.language;
        let theme_ref = self.theme_ref();

        let current_tab = self.current_tab;
        let core_running = self.core_running;
        let core_starting = self.core_starting;
        let core_stopping = self.core_stopping;
        let sys_proxy_enabled = self.sys_proxy_enabled;
        let total_uploaded = self.total_uploaded;
        let total_downloaded = self.total_downloaded;
        let latency_testing = self.latency_testing;
        let downloading = self.downloading;
        let core_installed = self.core_installed;

        // References with lifetime 'a
        let gui_config_ref = &self.gui_config;
        let current_speed_ref = &self.current_speed;
        let speed_history_ref = &self.speed_history;
        let active_profile_nodes_ref = &self.active_profile_nodes;
        let selected_node_tag_ref = self.selected_node_tag.as_deref();
        let node_search_ref = &self.node_search;
        let proxy_groups_ref = &self.proxy_groups;
        let selected_group_ref = &self.selected_group;
        let url_input_ref = &self.url_input;
        let profile_error_ref = self.profile_error.as_deref();
        let confirm_delete_profile_id_ref = self.confirm_delete_profile_id.as_deref();
        let bypass_domain_input_ref = &self.bypass_domain_input;
        let proxy_domain_input_ref = &self.proxy_domain_input;
        let bypass_ip_input_ref = &self.bypass_ip_input;
        let proxy_ip_input_ref = &self.proxy_ip_input;
        let active_connections_ref = &self.active_connections;
        let connections_search_ref = &self.connections_search;
        let log_lines_ref = &self.log_lines;
        let settings_draft_ref = &self.settings_draft;
        let settings_errors_ref = &self.settings_errors;
        let core_install_state_ref = &self.core_install_state;
        let update_status_ref = &self.update_status;
        let editing_profile_id_ref = self.editing_profile_id.as_deref();
        let editing_profile_name_ref = &self.editing_profile_name;
        let editing_profile_url_ref = &self.editing_profile_url;
        let toast_ref = self.toast.as_ref();
        let log_filter = self.log_filter;
        let log_search_ref = &self.log_search;
        let core_version_ref = self.core_version.as_deref();
        let active_connections_count = self.active_connections.len();
        let selected_node_for_dash = self.selected_node_tag.as_deref();
        let profile_more_id_ref = self.profile_more_id.as_deref();
        let updating_profile_id_ref = self.updating_profile_id.as_deref();
        let config_preview_expanded = self.config_preview_expanded;
        let logs_follow = self.logs_follow;
        let system_proxy_busy = self.system_proxy_busy;

        let main_content = responsive(move |size| {
            let theme = theme_ref;
            let is_compact = size.width < ui::SHELL_COMPACT_W;
            let text_muted = ui::theme::text_muted(theme);

            // Render active tab view
            let content = match current_tab {
                Tab::Dashboard => ui::dashboard::render(
                    gui_config_ref,
                    core_running,
                    core_starting,
                    core_stopping,
                    core_installed,
                    sys_proxy_enabled,
                    system_proxy_busy,
                    current_speed_ref,
                    speed_history_ref,
                    total_uploaded,
                    total_downloaded,
                    selected_node_for_dash,
                    active_connections_count,
                    theme,
                ),
                Tab::Proxies => ui::proxies::render(
                    gui_config_ref,
                    active_profile_nodes_ref,
                    selected_node_tag_ref,
                    latency_testing,
                    node_search_ref,
                    proxy_groups_ref,
                    selected_group_ref,
                    core_running,
                    self.proxy_sort,
                    theme,
                ),
                Tab::Profiles => ui::profiles::render(
                    gui_config_ref,
                    url_input_ref,
                    downloading,
                    profile_error_ref,
                    confirm_delete_profile_id_ref,
                    editing_profile_id_ref,
                    editing_profile_name_ref,
                    editing_profile_url_ref,
                    profile_more_id_ref,
                    updating_profile_id_ref,
                    theme,
                ),
                Tab::Rules => ui::rules::render(
                    gui_config_ref,
                    bypass_domain_input_ref,
                    proxy_domain_input_ref,
                    bypass_ip_input_ref,
                    proxy_ip_input_ref,
                    theme,
                ),
                Tab::Connections => ui::connections::render(
                    gui_config_ref,
                    active_connections_ref,
                    core_running,
                    connections_search_ref,
                    self.connections_sort,
                    self.connections_sort_desc,
                    theme,
                ),
                Tab::Logs => ui::logs::render(
                    gui_config_ref,
                    log_lines_ref,
                    log_filter,
                    log_search_ref,
                    logs_follow,
                    core_running,
                    theme,
                ),
                Tab::Settings => ui::settings::render(
                    gui_config_ref,
                    settings_draft_ref,
                    settings_errors_ref,
                    core_installed,
                    core_install_state_ref,
                    core_version_ref,
                    update_status_ref,
                    config_preview_expanded,
                    self.config_preview.as_deref(),
                    theme,
                ),
            };

            let make_tab_btn =
                |tab: Tab, icon_char: char, key: &'static str| -> Element<'_, Message> {
                    let active = current_tab == tab;
                    let icon_color = if active {
                        ui::theme::accent_text(theme)
                    } else {
                        ui::theme::text_tertiary(theme)
                    };

                    let btn_content: Element<'_, Message> = if is_compact {
                        container(
                            text(icon_char.to_string())
                                .font(Font::with_name("Material Icons"))
                                .size(ui::ICON_SIZE_LG)
                                .color(icon_color),
                        )
                        .width(Length::Fill)
                        .align_x(Alignment::Center)
                        .into()
                    } else {
                        row![
                            text(icon_char.to_string())
                                .font(Font::with_name("Material Icons"))
                                .size(ui::ICON_SIZE_LG)
                                .color(icon_color),
                            text(ui::i18n::tr(lang, key))
                                .size(ui::theme::TYPE_BODY)
                                .font(ui::theme::ui_font(if active {
                                    iced::font::Weight::Semibold
                                } else {
                                    iced::font::Weight::Normal
                                }))
                                .color(if active {
                                    ui::theme::accent_text(theme)
                                } else {
                                    ui::theme::text_muted(theme)
                                })
                        ]
                        .spacing(ui::SP_12)
                        .align_y(Alignment::Center)
                        .into()
                    };

                    let btn = button(btn_content)
                        .padding(if is_compact { [10, 0] } else { [9, 12] })
                        .width(Length::Fill)
                        .style(ui::theme::nav_item(active))
                        .on_press(Message::TabChanged(tab));

                    // Compact icon rail: show tab name on hover
                    if is_compact {
                        tooltip(
                            btn,
                            container(
                                text(ui::i18n::tr(lang, key))
                                    .size(ui::theme::TYPE_BTN_SM)
                                    .color(ui::theme::text_primary(theme)),
                            )
                            .padding([6, 10])
                            .style(ui::theme::popover_bg),
                            tooltip::Position::Right,
                        )
                        .into()
                    } else {
                        btn.into()
                    }
                };

            let status_dot_color = if core_running {
                ui::theme::SUCCESS
            } else {
                ui::theme::text_tertiary(theme)
            };

            let nav = column![
                make_tab_btn(Tab::Dashboard, '\u{E871}', "tab_dashboard"),
                make_tab_btn(Tab::Proxies, ui::icons::ICON_PROXIES, "tab_proxies"),
                make_tab_btn(Tab::Profiles, '\u{E2C7}', "tab_profiles"),
                make_tab_btn(Tab::Rules, '\u{E41E}', "tab_rules"),
                make_tab_btn(Tab::Connections, '\u{E894}', "tab_connections"),
                make_tab_btn(Tab::Logs, '\u{E85D}', "tab_logs"),
                make_tab_btn(Tab::Settings, ui::icons::ICON_SETTINGS, "tab_settings"),
            ]
            .spacing(2)
            .width(Length::Fill);

            let sidebar = if is_compact {
                container(
                    column![
                        nav,
                        iced::widget::Space::new().height(Length::Fill),
                        container({
                            let dot = container(iced::widget::Space::new())
                                .width(8)
                                .height(8)
                                .style(move |_t| container::Style {
                                    background: Some(iced::Background::Color(status_dot_color)),
                                    border: iced::Border {
                                        radius: 4.0.into(),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                });
                            if core_running {
                                container(dot)
                                    .padding(4)
                                    .style(move |_t| ui::theme::status_ring(status_dot_color))
                            } else {
                                container(dot).padding(4)
                            }
                        })
                        .center_x(Length::Fill)
                    ]
                    .width(Length::Fill)
                    .align_x(Alignment::Center),
                )
                .width(Length::Fixed(64.0))
                .height(Length::Fill)
                .padding([22, 10])
                .style(ui::theme::sidebar_bg)
            } else {
                // Footer: core state and version, separated by a hairline
                // rather than boxed in another card.
                let footer = column![
                    container(iced::widget::Space::new())
                        .height(1)
                        .width(Length::Fill)
                        .style(ui::theme::hairline),
                    row![
                        ui::status_dot(
                            status_dot_color,
                            core_running,
                            if core_running {
                                ui::i18n::tr(lang, "status_running")
                            } else {
                                ui::i18n::tr(lang, "status_stopped")
                            },
                            if core_running {
                                ui::theme::success_text(theme)
                            } else {
                                text_muted
                            },
                            ui::theme::TYPE_CAPTION
                        ),
                        iced::widget::Space::new().width(Length::Fill),
                        text(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .size(ui::theme::TYPE_MICRO)
                            .font(ui::theme::metric_font())
                            .color(ui::theme::text_tertiary(theme))
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .padding([12, 6])
                    .width(Length::Fill),
                ]
                .width(Length::Fill);

                // No brand block: the native title bar already carries the
                // app icon and name, so navigation starts at the top.
                container(column![
                    nav,
                    iced::widget::Space::new().height(Length::Fill),
                    footer,
                ])
                .width(Length::Fixed(216.0))
                .height(Length::Fill)
                .padding(iced::Padding {
                    top: 22.0,
                    right: 12.0,
                    bottom: 8.0,
                    left: 12.0,
                })
                .style(ui::theme::sidebar_bg)
            };

            let content: Element<'_, Message> = container(
                container(content)
                    .width(Length::Fill)
                    .max_width(ui::PAGE_MAX_WIDTH)
                    .height(Length::Fill),
            )
            .center_x(Length::Fill)
            .height(Length::Fill)
            .into();

            let main_body = if let Some(toast) = toast_ref {
                let toast_el = ui::toast::render(toast, theme);
                iced::widget::stack![
                    container(content).width(Length::Fill).height(Length::Fill),
                    container(
                        column![
                            row![iced::widget::Space::new().width(Length::Fill), toast_el,]
                                .padding([16, 20])
                        ]
                        .width(Length::Fill)
                    )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::End)
                    .align_y(Alignment::Start)
                ]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            } else {
                content
            };

            let main_layout = container(main_body)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(ui::theme::main_bg);

            let divider = container(iced::widget::Space::new())
                .width(1)
                .height(Length::Fill)
                .style(ui::theme::hairline);

            row![sidebar, divider, main_layout]
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
        });

        main_content.into()
    }

    fn subscription(&self) -> Subscription<Message> {
        #[cfg(feature = "ui-smoke")]
        if self.smoke.is_some() {
            return Subscription::run(tray::events);
        }
        let mut subs = vec![
            iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick),
            iced::window::close_requests().map(Message::WindowCloseRequested),
            iced::window::events().filter_map(|(id, event)| {
                if let iced::window::Event::Resized(size) = event {
                    Some(Message::WindowResized(id, size))
                } else {
                    None
                }
            }),
        ];

        // Live streams for logs, traffic stats, and tray events
        subs.push(Subscription::run(log_subscription));
        subs.push(Subscription::run(traffic_subscription));
        subs.push(Subscription::run(tray::events));
        // Global shortcuts also work while a text field has focus. In
        // particular, Escape cancels an edit instead of only blurring it.
        subs.push(iced::event::listen_with(|event, _, _| match event {
            iced::Event::Keyboard(event) => Some(Message::KeyboardEvent(event)),
            _ => None,
        }));
        subs.push(
            iced::system::theme_changes()
                .map(|mode| Message::SystemThemeDetected(matches!(mode, iced::theme::Mode::Light))),
        );

        Subscription::batch(subs)
    }
}

// Subscription worker for streaming log lines
fn log_subscription() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(
        100,
        |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
            struct RxLease {
                slot: &'static Mutex<Option<mpsc::Receiver<String>>>,
                rx: Option<mpsc::Receiver<String>>,
            }
            impl Drop for RxLease {
                fn drop(&mut self) {
                    let rx = match self.rx.take() {
                        Some(rx) => rx,
                        None => return,
                    };
                    let mut slot = match self.slot.lock() {
                        Ok(s) => s,
                        Err(e) => e.into_inner(),
                    };
                    if slot.is_none() {
                        *slot = Some(rx);
                    }
                }
            }
            let Some(slot) = LOG_RX.get() else { return };
            let mut lease = RxLease {
                slot,
                rx: slot.lock().unwrap_or_else(|e| e.into_inner()).take(),
            };
            if let Some(r) = lease.rx.as_mut() {
                while let Some(line) = r.recv().await {
                    let mut batch = Vec::with_capacity(32);
                    batch.push(line);
                    while batch.len() < 64 {
                        match r.try_recv() {
                            Ok(next) => batch.push(next),
                            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
                            | Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
                        }
                    }
                    let _ = output.send(Message::NewLogBatch(batch)).await;
                    if output.is_closed() {
                        break;
                    }
                }
            }
            // On stream end, Drop returns the receiver to the slot.
        },
    )
}

// Subscription worker for streaming real-time Clash API traffic stats
fn traffic_subscription() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(
        100,
        |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
            struct RxLease {
                slot: &'static Mutex<Option<mpsc::Receiver<api::TrafficInfo>>>,
                rx: Option<mpsc::Receiver<api::TrafficInfo>>,
            }
            impl Drop for RxLease {
                fn drop(&mut self) {
                    let rx = match self.rx.take() {
                        Some(rx) => rx,
                        None => return,
                    };
                    let mut slot = match self.slot.lock() {
                        Ok(s) => s,
                        Err(e) => e.into_inner(),
                    };
                    if slot.is_none() {
                        *slot = Some(rx);
                    }
                }
            }
            let Some(slot) = TRAFFIC_RX.get() else { return };
            let mut lease = RxLease {
                slot,
                rx: slot.lock().unwrap_or_else(|e| e.into_inner()).take(),
            };
            if let Some(r) = lease.rx.as_mut() {
                while let Some(info) = r.recv().await {
                    let _ = output
                        .send(Message::TrafficUpdated {
                            up: info.up,
                            down: info.down,
                        })
                        .await;
                    if output.is_closed() {
                        break;
                    }
                }
            }
            // On stream end, Drop returns the receiver to the slot.
        },
    )
}

struct ProfileFetchResult {
    id: String,
    source_url: String,
    display_name: Option<String>,
    traffic_upload: Option<u64>,
    traffic_download: Option<u64>,
    traffic_total: Option<u64>,
    expire_at: Option<i64>,
}

/// Download or open a profile URL/path and write the profile file only.
/// The GUI thread owns `gui_config` mutations (avoids load/save races).
async fn download_profile(url: String) -> Result<ProfileFetchResult, String> {
    let url = url.trim().to_string();
    let (content, meta) = load_profile_content(&url).await?;
    config::validate_profile_content(&content)?;

    let id = chrono::Utc::now().timestamp_millis().to_string();
    let name = meta.display_name.clone().unwrap_or_else(|| {
        if std::path::Path::new(&url).exists() {
            std::path::Path::new(&url)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("Local_Config")
                .to_string()
        } else {
            format!("Sub_{}", &id[id.len().saturating_sub(6)..])
        }
    });

    let path = config::get_profile_path(&id);
    // Inline imports contain credentials. Persist their saved file as the
    // source instead of copying the entire payload into GUI metadata/labels.
    let source_url = if is_http_profile_source(&url) || std::path::Path::new(&url).is_file() {
        url
    } else {
        path.to_string_lossy().into_owned()
    };
    let write_path = path.clone();
    tokio::task::spawn_blocking(move || config::atomic_write(&write_path, content.as_bytes()))
        .await
        .map_err(|e| format!("Profile save task failed: {e}"))??;

    Ok(ProfileFetchResult {
        id,
        source_url,
        display_name: Some(name),
        traffic_upload: meta.traffic_upload,
        traffic_download: meta.traffic_download,
        traffic_total: meta.traffic_total,
        expire_at: meta.expire_at,
    })
}

/// Update an existing profile id from its URL.
async fn fetch_and_save_subscription(
    url: String,
    id: String,
) -> Result<ProfileFetchResult, String> {
    let url = url.trim().to_string();
    let (content, meta) = load_profile_content(&url).await?;
    config::validate_profile_content(&content)?;
    let path = config::get_profile_path(&id);
    let write_path = path.clone();
    tokio::task::spawn_blocking(move || config::atomic_write(&write_path, content.as_bytes()))
        .await
        .map_err(|e| format!("Profile save task failed: {e}"))??;
    Ok(ProfileFetchResult {
        id,
        source_url: url,
        display_name: meta.display_name,
        traffic_upload: meta.traffic_upload,
        traffic_download: meta.traffic_download,
        traffic_total: meta.traffic_total,
        expire_at: meta.expire_at,
    })
}

struct ProfileContentMeta {
    display_name: Option<String>,
    traffic_upload: Option<u64>,
    traffic_download: Option<u64>,
    traffic_total: Option<u64>,
    expire_at: Option<i64>,
}

const MAX_PROFILE_BYTES: usize = 16 * 1024 * 1024;

fn is_http_profile_source(source: &str) -> bool {
    url::Url::parse(source).ok().is_some_and(|url| {
        // Url::port() removes explicit default ports (80/443). Keep that
        // distinction so an HTTP node using port 80 is imported, not fetched.
        let explicit_port = source
            .split_once("://")
            .and_then(|(_, rest)| rest.split(['/', '?', '#']).next())
            .and_then(|authority| authority.rsplit('@').next())
            .and_then(|host| host.rsplit_once(':'))
            .is_some_and(|(_, port)| port.parse::<u16>().is_ok());
        matches!(url.scheme(), "http" | "https") && url.host_str().is_some()
            // HTTP proxy sharing links use user:password@host:port with no
            // resource path. An authenticated subscription URL has a path.
            && !(url.password().is_some() && explicit_port
                && url.path() == "/" && url.query().is_none())
    })
}

fn decode_profile_bytes(bytes: &[u8]) -> Result<String, String> {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok(text.to_string());
    }
    let (decoded, _, had_errors) = encoding_rs::GBK.decode(bytes);
    if !had_errors {
        return Ok(decoded.into_owned());
    }
    Err("Profile is neither valid UTF-8 nor GBK-compatible text".to_string())
}

async fn load_profile_content(url: &str) -> Result<(String, ProfileContentMeta), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("Profile source is empty".to_string());
    }
    if url.len() > MAX_PROFILE_BYTES {
        return Err("Profile is larger than the 16 MiB safety limit".to_string());
    }
    let mut meta = ProfileContentMeta {
        display_name: None,
        traffic_upload: None,
        traffic_download: None,
        traffic_total: None,
        expire_at: None,
    };

    if std::path::Path::new(url).is_file() {
        use tokio::io::AsyncReadExt;
        let file = tokio::fs::File::open(url)
            .await
            .map_err(|e| format!("Failed to read local file: {}", e))?;
        let mut bytes = Vec::new();
        file.take(MAX_PROFILE_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(|e| format!("Failed to read local file: {e}"))?;
        if bytes.len() > MAX_PROFILE_BYTES {
            return Err("Profile is larger than the 16 MiB safety limit".to_string());
        }
        let content = decode_profile_bytes(&bytes)?;
        return Ok((content, meta));
    }

    if !is_http_profile_source(url) {
        // The parser already supports sharing links, Base64 bundles, YAML and
        // native JSON. Route clipboard payloads to it instead of URL fetching.
        config::validate_profile_content(url)?;
        meta.display_name = config::parse_native_json_nodes(url)
            .or_else(|_| config::parse_clash_yaml_nodes(url))
            .ok()
            .and_then(|nodes| nodes.first().map(|node| node.name.clone()));
        return Ok((url.to_owned(), meta));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let res = client
        .get(url)
        .header("User-Agent", "clash")
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Download failed with status: {}", res.status()));
    }

    if res
        .content_length()
        .is_some_and(|n| n > MAX_PROFILE_BYTES as u64)
    {
        return Err("Subscription is larger than the 16 MiB safety limit".to_string());
    }

    if let Some(userinfo) = res.headers().get("subscription-userinfo")
        && let Ok(s) = userinfo.to_str()
    {
        let (u, d, t, e) = config::parse_subscription_userinfo(s);
        meta.traffic_upload = u;
        meta.traffic_download = d;
        meta.traffic_total = t;
        meta.expire_at = e;
    }

    if let Some(title) = res
        .headers()
        .get("profile-title")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_profile_title_header)
    {
        meta.display_name = Some(title);
    }

    // Content-Disposition filename or content-disposition profile name
    if meta.display_name.is_none()
        && let Some(cd) = res.headers().get(reqwest::header::CONTENT_DISPOSITION)
        && let Ok(s) = cd.to_str()
        && let Some(name) = parse_content_disposition_filename(s)
    {
        meta.display_name = Some(name);
    }

    let mut stream = res.bytes_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Failed to read content: {e}"))?;
        if bytes.len().saturating_add(chunk.len()) > MAX_PROFILE_BYTES {
            return Err("Subscription is larger than the 16 MiB safety limit".to_string());
        }
        bytes.extend_from_slice(&chunk);
    }
    let content = decode_profile_bytes(&bytes)?;
    Ok((content, meta))
}

fn parse_profile_title_header(header: &str) -> Option<String> {
    use base64::{Engine as _, engine::general_purpose};

    let raw = header.trim().trim_matches('"');
    let decoded = if let Some(encoded) = raw.strip_prefix("base64:") {
        general_purpose::STANDARD
            .decode(encoded.trim())
            .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(encoded.trim()))
            .or_else(|_| general_purpose::URL_SAFE.decode(encoded.trim()))
            .or_else(|_| general_purpose::URL_SAFE_NO_PAD.decode(encoded.trim()))
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())?
    } else {
        urlencoding::decode(raw).ok()?.into_owned()
    };
    let title = decoded.trim().trim_matches('"');
    if title.is_empty() {
        return None;
    }
    Some(title.chars().take(128).collect())
}

fn parse_content_disposition_filename(header: &str) -> Option<String> {
    // filename="xxx" or filename*=UTF-8''xxx
    for part in header.split(';') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix("filename*=") {
            let rest = rest.trim_matches('"');
            if let Some(encoded) = rest.split("''").nth(1) {
                return Some(urlencoding::decode(encoded).ok()?.into_owned());
            }
        }
        if let Some(rest) = part.strip_prefix("filename=") {
            return Some(rest.trim_matches('"').to_string());
        }
    }
    None
}

/// Append unique subscription ids to the auto-update queue (FIFO).
fn enqueue_pending_updates(
    queue: &mut std::collections::VecDeque<String>,
    ids: impl IntoIterator<Item = String>,
) {
    for id in ids {
        if !queue.contains(&id) {
            queue.push_back(id);
        }
    }
}

fn collect_due_subscription_ids(gui_config: &GuiConfig, hours: u32) -> Vec<String> {
    let threshold = chrono::Duration::hours(hours as i64);
    let now = chrono::Local::now();
    let mut due = Vec::new();
    for p in &gui_config.subscriptions {
        if p.url.is_empty() || std::path::Path::new(&p.url).exists() {
            continue; // skip pure local files without remote URL
        }
        if p.url.starts_with("http://") || p.url.starts_with("https://") {
            let stale = chrono::NaiveDateTime::parse_from_str(&p.updated_at, "%Y-%m-%d %H:%M:%S")
                .ok()
                .and_then(|ndt| ndt.and_local_timezone(chrono::Local).single())
                .map(|dt| now.signed_duration_since(dt) >= threshold)
                .unwrap_or(true);
            if stale {
                due.push(p.id.clone());
            }
        }
    }
    due
}

fn normalize_custom_rule(field: state::RuleField, value: &str) -> Result<String, &'static str> {
    let value = value.trim();
    if field.is_ip() {
        let (address_text, prefix_text) = value
            .split_once('/')
            .map_or((value, None), |(address, prefix)| (address, Some(prefix)));
        let address: std::net::IpAddr = address_text.parse().map_err(|_| "invalid_ip_rule")?;
        let max_prefix: u8 = if address.is_ipv4() { 32 } else { 128 };
        let prefix = match prefix_text {
            Some(prefix) => prefix
                .parse::<u8>()
                .ok()
                .filter(|prefix| *prefix <= max_prefix)
                .ok_or("invalid_ip_rule")?,
            None => max_prefix,
        };
        return Ok(format!("{address}/{prefix}"));
    }

    let domain = value
        .trim_start_matches("*.")
        .trim_start_matches('.')
        .trim_end_matches('.')
        .to_lowercase();
    let valid = !domain.is_empty()
        && domain.len() <= 253
        && !domain.chars().any(|character| {
            character.is_whitespace() || matches!(character, '/' | ':' | '?' | '#')
        })
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|character| {
                    character.is_alphanumeric() || character == '-' || character == '_'
                })
        });
    if valid {
        Ok(domain)
    } else {
        Err("invalid_domain_rule")
    }
}

fn parse_runtime_port(value: &str) -> Result<u16, &'static str> {
    if value.trim().is_empty() {
        return Err("port_empty_error");
    }
    let port = value
        .trim()
        .parse::<u16>()
        .map_err(|_| "port_invalid_error")?;
    if port < 1024 {
        return Err("port_reserved_error");
    }
    Ok(port)
}

fn parse_latency_timeout(value: &str) -> Result<u32, ()> {
    value
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|timeout| (500..=30_000).contains(timeout))
        .ok_or(())
}

fn advance_speed_history(history: &mut Vec<(u64, u64)>, speed: &Bandwidth) {
    history.push((speed.up, speed.down));
    if history.len() > 30 {
        let excess = history.len() - 30;
        history.drain(..excess);
    }
}

fn is_valid_dns_server_address(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() || value.len() > 2048 || value.chars().any(char::is_whitespace) {
        return false;
    }
    if value.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }
    if value.contains("://") {
        return url::Url::parse(value).is_ok_and(|url| {
            !url.scheme().is_empty()
                && (url.host_str().is_some() || matches!(url.scheme(), "local" | "dhcp" | "rcode"))
        });
    }
    if url::Url::parse(&format!("udp://{value}")).is_ok_and(|url| url.host_str().is_some()) {
        return true;
    }
    value.len() <= 253
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .chars()
                    .all(|character| character.is_alphanumeric() || character == '-')
        })
}

fn is_valid_profile_source(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() || value.len() > 4096 {
        return false;
    }
    if std::path::Path::new(value).is_file() {
        return true;
    }
    url::Url::parse(value)
        .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
}

fn remove_profile_file(path: &std::path::Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn truncate_log_line(line: &str) -> String {
    if line.chars().count() <= MAX_LOG_LINE_CHARS {
        return line.to_string();
    }
    let mut truncated: String = line.chars().take(MAX_LOG_LINE_CHARS).collect();
    truncated.push_str(" … [truncated]");
    truncated
}

fn redact_log_line(line: &str) -> String {
    let sensitive_key = |key: &str| {
        let key = key.to_ascii_lowercase().replace('-', "_");
        key.contains("token")
            || key.contains("secret")
            || key.contains("password")
            || key.contains("passwd")
            || key.contains("private_key")
            || key == "key"
    };
    let line = redact_json_segment(line).unwrap_or_else(|| line.to_string());
    line.split_whitespace()
        .map(|part| {
            let url_start = part.find("https://").or_else(|| part.find("http://"));
            if let Some(prefix_len) = url_start {
                let candidate = &part[prefix_len..];
                let trimmed = candidate.trim_end_matches(|c: char| {
                    matches!(c, '"' | '\'' | ',' | ';' | ')' | ']' | '}' | '.')
                });
                if let Ok(parsed) = url::Url::parse(trimmed)
                    && matches!(parsed.scheme(), "http" | "https")
                {
                    let masked = ui::profiles::mask_sensitive_url(trimmed);
                    let suffix_start = prefix_len.saturating_add(trimmed.len());
                    return format!("{}{}{}", &part[..prefix_len], masked, &part[suffix_start..]);
                }
            }
            if let Some((key, _value)) = part.split_once('=')
                && sensitive_key(key.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_'))
            {
                return format!("{key}=******");
            }
            part.to_string()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Redact structured JSON embedded in a core/API log line before the lighter
/// whitespace-based redaction below. This catches forms such as
/// `INFO {"password":"...","token":"..."}` that do not contain an `=`
/// separator and would otherwise be displayed verbatim.
fn redact_json_segment(line: &str) -> Option<String> {
    let start = line.find(['{', '['])?;
    let end = line.rfind(['}', ']'])?;
    if end < start {
        return None;
    }
    let mut value = serde_json::from_str::<serde_json::Value>(&line[start..=end]).ok()?;
    config::redact_sensitive_json(&mut value);
    let replacement = serde_json::to_string(&value).ok()?;
    Some(format!(
        "{}{}{}",
        &line[..start],
        replacement,
        &line[end + 1..]
    ))
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use state::{GuiConfig, Profile};

    #[test]
    fn old_window_resize_does_not_change_the_restored_window() {
        let (mut app, startup) = App::new_with_config(GuiConfig::default());
        drop(startup);
        let old = app.window_id.expect("pending startup window");
        let restored = iced::window::Id::unique();
        app.window_id = Some(restored);
        let expected = iced::Size::new(900.0, 650.0);
        drop(app.update(Message::WindowResized(restored, expected)));
        drop(app.update(Message::WindowResized(old, iced::Size::new(640.0, 480.0))));
        assert_eq!(app.window_size, expected);
    }

    #[test]
    fn routing_mode_changes_commit_only_after_core_acknowledgement() {
        let (mut app, startup) = App::new_with_config(GuiConfig::default());
        drop(startup);
        app.core_running = true;
        drop(app.update(Message::RoutingModeChanged(state::RoutingMode::Global)));
        assert_eq!(app.gui_config.routing_mode, state::RoutingMode::Rule);
        assert_eq!(app.pending_routing_mode, Some(state::RoutingMode::Global));
        drop(app.update(Message::ModeSet {
            mode: state::RoutingMode::Global,
            result: Err("API unavailable".into()),
        }));
        assert_eq!(app.gui_config.routing_mode, state::RoutingMode::Rule);
        assert!(app.pending_routing_mode.is_none());
        drop(app.update(Message::RoutingModeChanged(state::RoutingMode::Direct)));
        drop(app.update(Message::ModeSet {
            mode: state::RoutingMode::Direct,
            result: Ok(()),
        }));
        assert_eq!(app.gui_config.routing_mode, state::RoutingMode::Direct);
    }

    #[test]
    fn stopped_core_does_not_repopulate_stale_api_snapshots() {
        let (mut app, startup) = App::new_with_config(GuiConfig::default());
        drop(startup);
        let proxy = api::ProxyInfo {
            name: "Old".into(),
            proxy_type: "Selector".into(),
            udp: None,
            history: None,
            now: None,
            all: Some(vec![]),
        };
        drop(app.update(Message::ProxiesFetched(Ok(
            std::collections::HashMap::from([("Old".into(), proxy)]),
        ))));
        assert!(app.proxy_groups.is_empty());
    }

    #[tokio::test]
    async fn clipboard_payloads_use_the_profile_parser_without_http() {
        use base64::Engine;
        let link = "vless://11111111-1111-1111-1111-111111111111@node.example.com:443?security=tls#Clipboard";
        let bundle = base64::engine::general_purpose::STANDARD.encode(link);
        let native = r#"{"outbounds":[{"type":"vless","tag":"Native","server":"node.example.com","server_port":443,"uuid":"11111111-1111-1111-1111-111111111111"}]}"#;
        let yaml = "proxies:\n  - name: Local\n    type: trojan\n    server: node.example.com\n    port: 443\n    password: example\n";
        for payload in [link, bundle.as_str(), native, yaml] {
            let (content, _) = load_profile_content(payload).await.unwrap();
            assert_eq!(content, payload.trim());
            assert!(config::validate_profile_content(&content).is_ok());
        }
        assert!(load_profile_content("not a profile").await.is_err());
        assert!(
            load_profile_content("ftp://example.com/file")
                .await
                .is_err()
        );
    }

    #[test]
    fn authenticated_http_proxy_links_are_distinct_from_subscription_urls() {
        assert!(!is_http_profile_source(
            "http://user:pass@proxy.example:8080#Office"
        ));
        assert!(!is_http_profile_source(
            "https://user:pass@proxy.example:8443"
        ));
        assert!(!is_http_profile_source("http://user:pass@proxy.example:80"));
        assert!(!is_http_profile_source(
            "https://user:pass@proxy.example:443"
        ));
        assert!(!is_http_profile_source("http://user:pass@[::1]:80#Office"));
        assert!(is_http_profile_source("https://user:pass@example.com/"));
        assert!(is_http_profile_source(
            "https://user:pass@example.com/subscription"
        ));
        assert!(is_http_profile_source("https://example.com/sub?token=abc"));
    }

    #[test]
    fn enqueue_pending_updates_dedupes_and_preserves_order() {
        let mut q = std::collections::VecDeque::new();
        enqueue_pending_updates(&mut q, vec!["a".into(), "b".into()]);
        enqueue_pending_updates(&mut q, vec!["b".into(), "c".into(), "a".into()]);
        assert_eq!(
            q.into_iter().collect::<Vec<_>>(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn collect_due_includes_stale_http_subscriptions() {
        let mut cfg = GuiConfig::default();
        cfg.subscriptions.push(Profile {
            id: "remote-stale".into(),
            name: "Stale".into(),
            url: "https://example.com/sub".into(),
            file_path: String::new(),
            is_subscription: true,
            // Far in the past → due for any positive interval
            updated_at: "2000-01-01 00:00:00".into(),
            traffic_upload: None,
            traffic_download: None,
            traffic_total: None,
            expire_at: None,
        });
        cfg.subscriptions.push(Profile {
            id: "local-file".into(),
            name: "Local".into(),
            url: "C:\\not\\a\\real\\path\\but\\absolute".into(),
            file_path: String::new(),
            is_subscription: false,
            updated_at: "2000-01-01 00:00:00".into(),
            traffic_upload: None,
            traffic_download: None,
            traffic_total: None,
            expire_at: None,
        });
        let due = collect_due_subscription_ids(&cfg, 6);
        assert!(due.contains(&"remote-stale".to_string()));
        assert!(!due.iter().any(|id| id == "local-file"));
    }

    #[test]
    fn decode_profile_bytes_strips_utf8_bom_and_preserves_cjk() {
        let bytes = b"\xEF\xBB\xBFproxies:\n  - name: \xE8\x8A\x82\xE7\x82\xB9\n";
        let decoded = decode_profile_bytes(bytes).unwrap();
        assert!(decoded.starts_with("proxies:"));
        assert!(decoded.contains("节点"));
        assert!(!decoded.starts_with('\u{feff}'));
    }

    #[test]
    fn decode_profile_bytes_supports_gbk_subscription_names() {
        let (encoded, _, had_errors) = encoding_rs::GBK.encode("proxies:\n  - name: 香港节点\n");
        assert!(!had_errors);
        let decoded = decode_profile_bytes(&encoded).unwrap();
        assert!(decoded.contains("香港节点"));
    }

    #[test]
    fn profile_title_header_supports_url_and_base64_encodings() {
        use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};

        assert_eq!(
            parse_profile_title_header("%E9%A6%99%E6%B8%AF%E8%8A%82%E7%82%B9"),
            Some("香港节点".to_string())
        );
        let encoded = STANDARD_NO_PAD.encode("高级订阅");
        assert_eq!(
            parse_profile_title_header(&format!("base64:{encoded}")),
            Some("高级订阅".to_string())
        );
        assert_eq!(parse_profile_title_header("   "), None);
    }

    #[test]
    fn custom_rule_normalization_handles_domains_and_ip_cidr() {
        assert_eq!(
            normalize_custom_rule(state::RuleField::BypassDomains, " *.Example.COM. "),
            Ok("example.com".to_string())
        );
        assert_eq!(
            normalize_custom_rule(state::RuleField::ProxyIps, "192.0.2.1"),
            Ok("192.0.2.1/32".to_string())
        );
        assert_eq!(
            normalize_custom_rule(state::RuleField::ProxyIps, "2001:db8::/48"),
            Ok("2001:db8::/48".to_string())
        );
        assert_eq!(
            normalize_custom_rule(state::RuleField::ProxyIps, "192.0.2.1/64"),
            Err("invalid_ip_rule")
        );
        assert_eq!(
            normalize_custom_rule(state::RuleField::BypassDomains, "https://example.com"),
            Err("invalid_domain_rule")
        );
    }

    #[test]
    fn dns_server_validation_accepts_native_forms_and_rejects_bad_input() {
        for valid in [
            "223.5.5.5",
            "8.8.8.8:53",
            "dns.google",
            "https://cloudflare-dns.com/dns-query",
            "tls://1.1.1.1",
            "dhcp://auto",
            "rcode://success",
        ] {
            assert!(is_valid_dns_server_address(valid), "valid={valid}");
        }
        for invalid in ["", "https://", "bad host", "http://"] {
            assert!(!is_valid_dns_server_address(invalid), "invalid={invalid}");
        }
    }

    #[test]
    fn runtime_setting_parsers_enforce_safe_ranges() {
        assert_eq!(parse_runtime_port("2080"), Ok(2080));
        assert_eq!(parse_runtime_port(""), Err("port_empty_error"));
        assert_eq!(parse_runtime_port("80"), Err("port_reserved_error"));
        assert_eq!(parse_runtime_port("70000"), Err("port_invalid_error"));
        assert_eq!(parse_latency_timeout("500"), Ok(500));
        assert_eq!(parse_latency_timeout("30000"), Ok(30_000));
        assert!(parse_latency_timeout("499").is_err());
        assert!(parse_latency_timeout("30001").is_err());
        assert!(parse_latency_timeout("").is_err());
    }

    #[test]
    fn zero_and_constant_traffic_advance_the_fixed_history_window() {
        let mut history = vec![(9, 9); 30];
        advance_speed_history(&mut history, &Bandwidth { up: 0, down: 0 });
        assert_eq!(history.len(), 30);
        assert_eq!(history.last(), Some(&(0, 0)));
        advance_speed_history(&mut history, &Bandwidth { up: 42, down: 84 });
        advance_speed_history(&mut history, &Bandwidth { up: 42, down: 84 });
        assert_eq!(history.len(), 30);
        assert_eq!(&history[28..], &[(42, 84), (42, 84)]);
    }

    #[test]
    fn profile_source_validation_accepts_existing_files_and_http_urls() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profile.yaml");
        std::fs::write(&path, "proxies: []").unwrap();
        assert!(is_valid_profile_source(&path.to_string_lossy()));
        assert!(is_valid_profile_source("https://example.com/sub?token=abc"));
        assert!(!is_valid_profile_source("ftp://example.com/sub"));
        assert!(!is_valid_profile_source("not a path or url"));
    }

    #[test]
    fn profile_source_validation_rejects_non_http_urls() {
        assert!(!is_valid_profile_source("ftp://example.com/profile.yaml"));
        assert!(!is_valid_profile_source("file:///tmp/profile.yaml"));
    }

    #[test]
    fn profile_delete_failure_is_reported_without_treating_it_as_success() {
        let dir = tempfile::tempdir().unwrap();
        assert!(remove_profile_file(&dir.path().join("missing.yaml")).is_ok());
        assert!(remove_profile_file(dir.path()).is_err());
    }

    #[test]
    fn log_redaction_handles_url_punctuation_and_sensitive_keys() {
        let redacted = redact_log_line(
            "request=(https://user:pass@example.com/sub?access-token=a%2Fb), private-key=secret",
        );
        assert!(!redacted.contains("pass"), "redacted={redacted}");
        assert!(!redacted.contains("a%2Fb"), "redacted={redacted}");
        assert!(
            !redacted.contains("private-key=secret"),
            "redacted={redacted}"
        );
        assert!(redacted.contains("******"));
    }

    #[test]
    fn log_redaction_masks_embedded_json_credentials() {
        let redacted = redact_log_line(
            r#"INFO {"password":"secret","token":"abc","private-key":"pem","name":"safe"}"#,
        );
        assert!(!redacted.contains("secret"));
        assert!(!redacted.contains("abc"));
        assert!(!redacted.contains("pem"));
        assert!(redacted.contains("safe"));
    }

    #[test]
    fn ui_log_lines_are_unicode_safe_and_bounded() {
        let line = "界".repeat(MAX_LOG_LINE_CHARS + 100);
        let truncated = truncate_log_line(&line);
        assert!(truncated.ends_with("[truncated]"));
        assert!(truncated.chars().count() <= MAX_LOG_LINE_CHARS + 16);
        assert_eq!(truncate_log_line("ok"), "ok");
    }

    #[test]
    fn window_min_matches_ui_constant() {
        // Keep main window settings aligned with shared tokens.
        assert!(ui::WINDOW_MIN_W < ui::SHELL_COMPACT_W);
    }

    #[test]
    fn quote_autostart_path_wraps_spaces() {
        let p = r#"C:\Users\Test User\App\sing-box-gui.exe"#;
        assert_eq!(quote_autostart_path(p), format!("\"{}\"", p));
    }

    #[test]
    fn pick_release_asset_prefers_platform_suffix() {
        let assets = vec![
            GithubAsset {
                name: "sing-box-gui-v2026.7.11-windows-amd64.exe".into(),
                browser_download_url: "https://example.com/win".into(),
                digest: None,
                size: None,
            },
            GithubAsset {
                name: "sing-box-gui-v2026.7.11-linux-amd64".into(),
                browser_download_url: "https://example.com/linux".into(),
                digest: None,
                size: None,
            },
        ];
        assert_eq!(
            pick_release_asset_url(&assets, "windows-amd64.exe").as_deref(),
            Some("https://example.com/win")
        );
        assert_eq!(
            pick_release_asset_url(&assets, "linux-amd64").as_deref(),
            Some("https://example.com/linux")
        );
        assert!(pick_release_asset_url(&assets, "macos-universal").is_none());
    }

    #[test]
    fn update_asset_url_and_digest_validation_fail_closed() {
        let tag = "v2026.7.24";
        let name = expected_update_asset_name(tag);
        let official = url::Url::parse(&format!(
            "https://github.com/zangge8855/sing-box-gui/releases/download/{tag}/{name}"
        ))
        .unwrap();
        assert!(is_official_update_asset_url(&official, tag));
        let wrong_repo = url::Url::parse(&format!(
            "https://github.com/attacker/sing-box-gui/releases/download/{tag}/{name}"
        ))
        .unwrap();
        assert!(!is_official_update_asset_url(&wrong_repo, tag));
        let wrong_asset = url::Url::parse(&format!(
            "https://github.com/zangge8855/sing-box-gui/releases/download/{tag}/SHA256SUMS.txt"
        ))
        .unwrap();
        assert!(!is_official_update_asset_url(&wrong_asset, tag));
        assert_eq!(
            normalize_update_digest(&format!("sha256:{}", "A".repeat(64))).unwrap(),
            "a".repeat(64)
        );
        assert!(normalize_update_digest(&"a".repeat(64)).is_err());
        assert!(normalize_update_digest("sha256:abcd").is_err());
    }

    #[test]
    fn updater_scripts_keep_parameterization_cleanup_and_rollback_guards() {
        let source = include_str!("main.rs");
        assert!(source.contains("[int]$ProcessId"));
        assert!(source.contains("-ProcessId"));
        let forbidden_legacy_parameter = ["[int]", "$", "Pid"].concat();
        assert!(!source.contains(&forbidden_legacy_parameter));
        assert!(source.contains("-LiteralPath $Target"));
        assert!(source.contains("$replaced = $true"));
        assert!(source.contains("rm -f -- \"$TARGET\""));
        assert!(source.contains("mv -f -- \"$BACKUP\" \"$TARGET\""));
    }

    #[test]
    fn should_poll_api_is_tab_aware() {
        // Active data pages poll at a restrained cadence.
        let (p, c) = should_poll_api(state::Tab::Proxies, 3);
        assert!(p && !c);
        let (p, c) = should_poll_api(state::Tab::Connections, 3);
        assert!(!p && c);
        let (p, c) = should_poll_api(state::Tab::Connections, 1);
        assert!(!p && !c);
        // Inactive pages do no API work.
        let (p, c) = should_poll_api(state::Tab::Logs, 5);
        assert!(!p && !c);
        let (p, c) = should_poll_api(state::Tab::Dashboard, 15);
        assert!(p && c);
    }

    #[test]
    fn memory_bounds_are_conservative() {
        assert!(MAX_LOG_LINES <= 500);
        assert!(MAX_LOG_LINE_CHARS <= 4096);
        assert!(MAX_CONNECTION_SNAPSHOT <= 1_000);
        let manifest = include_str!("../Cargo.toml");
        assert!(manifest.contains("default-features = false"));
        assert!(manifest.contains("\"tiny-skia\""));
        assert!(!manifest.contains("\"canvas\""));
    }

    #[test]
    fn proxy_history_compaction_keeps_only_latest_sample() {
        let mut groups = std::collections::HashMap::from([(
            "node".to_string(),
            api::ProxyInfo {
                name: "node".to_string(),
                proxy_type: "ss".to_string(),
                udp: None,
                history: Some(vec![
                    serde_json::json!({"delay": 10}),
                    serde_json::json!({"delay": 20}),
                ]),
                now: None,
                all: None,
            },
        )]);
        compact_proxy_history(&mut groups);
        let history = groups["node"].history.as_ref().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0]["delay"], 20);
    }

    #[test]
    fn is_remote_version_newer_compares_numerically() {
        // Same version → not newer.
        assert!(!update::is_remote_version_newer("2026.7.9", "v2026.7.9"));
        // Remote strictly less → not newer.
        assert!(!update::is_remote_version_newer("2026.7.9", "v2026.7.8"));
        // Remote strictly greater → newer.
        assert!(update::is_remote_version_newer("2026.7.9", "v2026.8.0"));
        // Prefix-trim equality of the trailing v vs no-v must not false-positive.
        assert!(!update::is_remote_version_newer("1.0.0", "v1.0.0"));
        // Older per-component case.
        assert!(update::is_remote_version_newer("1.0.0", "v2.0.0"));
        assert!(update::is_remote_version_newer(
            "2026.7.13-1",
            "v2026.7.13-2"
        ));
    }
}

/// Whether Tick should poll proxies / connections for the given tab and tick.
fn should_poll_api(tab: state::Tab, tick: u32) -> (bool, bool) {
    let want_proxies = match tab {
        state::Tab::Proxies => tick.is_multiple_of(3),
        state::Tab::Dashboard => tick.is_multiple_of(3),
        _ => false,
    };
    let want_connections = match tab {
        state::Tab::Connections => tick.is_multiple_of(3),
        state::Tab::Dashboard => tick.is_multiple_of(5),
        _ => false,
    };
    (want_proxies, want_connections)
}

fn compact_proxy_history(groups: &mut std::collections::HashMap<String, api::ProxyInfo>) {
    for proxy in groups.values_mut() {
        if let Some(history) = proxy.history.as_mut()
            && history.len() > 1
        {
            let latest = history.pop();
            history.clear();
            history.extend(latest);
        }
    }
}

#[cfg(test)]
fn quote_autostart_path(path: &str) -> String {
    format!("\"{}\"", path)
}

fn task_open_path(path: std::path::PathBuf) -> Task<Message> {
    Task::perform(
        async move {
            let _ = tokio::task::spawn_blocking(move || open::that(path)).await;
        },
        |_| Message::NoOp,
    )
}

/// Preferred release asset name fragment for this build target.
/// Matches CI artifact names: `sing-box-gui-v{VERSION}-{suffix}`.
fn platform_asset_suffix() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "windows-amd64.exe"
    }
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        "windows-arm64.exe"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "macos-universal"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "macos-universal"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-amd64"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "linux-arm64"
    }
    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
        target_os = "macos",
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
    )))]
    {
        "unknown"
    }
}

fn expected_update_asset_name(tag: &str) -> String {
    let version = tag.trim().trim_start_matches('v');
    format!("sing-box-gui-v{version}-{}", platform_asset_suffix())
}

fn is_official_update_asset_url(url: &url::Url, tag: &str) -> bool {
    if url.scheme() != "https" || url.host_str() != Some("github.com") {
        return false;
    }
    let mut segments = match url.path_segments() {
        Some(segments) => segments,
        None => return false,
    };
    let expected_tag = tag.trim();
    let expected_name = expected_update_asset_name(tag);
    matches!(segments.next(), Some("zangge8855"))
        && matches!(segments.next(), Some("sing-box-gui"))
        && matches!(segments.next(), Some("releases"))
        && matches!(segments.next(), Some("download"))
        && segments
            .next()
            .is_some_and(|release_tag| release_tag == expected_tag)
        && segments.next().is_some_and(|asset| asset == expected_name)
        && segments.next().is_none()
}

#[derive(Debug, Clone, serde::Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    digest: Option<String>,
    #[serde(default)]
    size: Option<u64>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[cfg(test)]
fn pick_release_asset_url(assets: &[GithubAsset], suffix: &str) -> Option<String> {
    assets
        .iter()
        .find(|asset| asset.name.ends_with(suffix))
        .map(|asset| asset.browser_download_url.clone())
}

async fn check_app_update() -> Result<message::AppUpdateInfo, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let res = client
        .get("https://api.github.com/repos/zangge8855/sing-box-gui/releases/latest")
        .header("User-Agent", "sing-box-gui")
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;

    // 404 from /releases/latest = the repo has published *no* GitHub release yet.
    let status = res.status();
    if status.as_u16() == 404 {
        return Err("No GitHub release published yet".to_string());
    }
    if !status.is_success() {
        return Err(format!("Server returned error status: {}", status));
    }

    let release: GithubRelease = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let expected_name = expected_update_asset_name(&release.tag_name);
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == expected_name);
    Ok(message::AppUpdateInfo {
        tag_name: release.tag_name,
        download_url: asset.map(|asset| asset.browser_download_url.clone()),
        sha256: asset.and_then(|asset| asset.digest.clone()),
        size: asset.and_then(|asset| asset.size),
    })
}

fn normalize_update_digest(expected_sha256: &str) -> Result<String, String> {
    let digest = expected_sha256
        .strip_prefix("sha256:")
        .filter(|digest| digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| "Update asset is missing a valid SHA-256 digest".to_string())?;
    Ok(digest.to_ascii_lowercase())
}

/// Download the release binary to a temp path next to the current executable.
async fn download_app_update_binary(
    tag: String,
    url: String,
    expected_sha256: String,
    expected_size: u64,
) -> Result<std::path::PathBuf, String> {
    use sha2::{Digest, Sha256};
    use tokio::io::AsyncWriteExt;
    const MAX_UPDATE_BYTES: u64 = 128 * 1024 * 1024;
    if expected_size == 0 || expected_size > MAX_UPDATE_BYTES {
        return Err("Update asset has an invalid size".to_string());
    }
    let expected_hex = normalize_update_digest(&expected_sha256)?;
    let parsed_url = url::Url::parse(&url).map_err(|e| format!("Invalid update URL: {e}"))?;
    if !is_official_update_asset_url(&parsed_url, &tag) {
        return Err("Update URL is not an official sing-box-gui release asset".to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let res = client
        .get(&url)
        .header("User-Agent", "sing-box-gui")
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Download failed with status: {}", res.status()));
    }

    if res.content_length() != Some(expected_size) {
        return Err("Update asset size does not match release metadata".to_string());
    }

    let current = std::env::current_exe()
        .map_err(|e| format!("Failed to resolve current executable: {}", e))?;
    let dir = current
        .parent()
        .ok_or_else(|| "Current executable has no parent directory".to_string())?;

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    #[cfg(target_os = "windows")]
    let file_name = format!(".sing-box-gui.update-{}-{nonce}.exe", std::process::id());
    #[cfg(not(target_os = "windows"))]
    let file_name = format!(".sing-box-gui.update-{}-{nonce}.bin", std::process::id());
    let beside = dir.join(&file_name);
    let (dest, mut file) = match tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&beside)
        .await
    {
        Ok(file) => (beside, file),
        Err(e_beside) => {
            let fallback = std::env::temp_dir().join(&file_name);
            let file = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&fallback)
                .await
                .map_err(|e| {
                    format!("Failed to create update file (beside exe: {e_beside}; temp: {e})")
                })?;
            (fallback, file)
        }
    };

    let download_result: Result<(), String> = async {
        let mut stream = res.bytes_stream();
        let mut downloaded = 0u64;
        let mut hasher = Sha256::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Failed to read download body: {e}"))?;
            downloaded = downloaded.saturating_add(chunk.len() as u64);
            if downloaded > MAX_UPDATE_BYTES || downloaded > expected_size {
                return Err("Update asset exceeds its declared size limit".to_string());
            }
            hasher.update(&chunk);
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to write update file: {e}"))?;
        }
        file.flush()
            .await
            .map_err(|e| format!("Failed to flush update file: {e}"))?;
        file.sync_all()
            .await
            .map_err(|e| format!("Failed to sync update file: {e}"))?;
        drop(file);
        if downloaded != expected_size {
            return Err("Downloaded update size does not match release metadata".to_string());
        }
        if format!("{:x}", hasher.finalize()) != expected_hex {
            return Err("Update SHA-256 verification failed".to_string());
        }
        if downloaded < 1024 {
            return Err("Downloaded update is too small to be a binary".to_string());
        }
        crate::core::validate_binary_magic(&dest)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&dest)
                .await
                .map_err(|e| format!("Failed to stat update file: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&dest, perms)
                .await
                .map_err(|e| format!("Failed to chmod update file: {}", e))?;
        }
        Ok(())
    }
    .await;
    if let Err(error) = download_result {
        let _ = tokio::fs::remove_file(&dest).await;
        return Err(error);
    }
    Ok(dest)
}

/// Schedule replacement of the running binary and relaunch after this process exits.
fn apply_update_and_restart(new_binary: &std::path::Path) -> Result<(), String> {
    let current = std::env::current_exe()
        .map_err(|e| format!("Failed to resolve current executable: {}", e))?;
    let pid = std::process::id();

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let script_path =
            current.with_file_name(format!(".sing-box-gui-update-{}.ps1", std::process::id()));
        let bak = current.with_extension("exe.bak");
        // PowerShell receives paths as arguments, so quotes and shell
        // metacharacters in an installation directory are never interpolated
        // into executable script text.
        let script = r#"param(
    [Parameter(Mandatory=$true)][string]$Target,
    [Parameter(Mandatory=$true)][string]$New,
    [Parameter(Mandatory=$true)][string]$Backup,
    [Parameter(Mandatory=$true)][int]$ProcessId
)
$ErrorActionPreference = 'Stop'
$replaced = $false
try {
    while (Get-Process -Id $ProcessId -ErrorAction SilentlyContinue) {
        Start-Sleep -Milliseconds 250
    }
    if (Test-Path -LiteralPath $Backup) { Remove-Item -LiteralPath $Backup -Force }
    if (Test-Path -LiteralPath $Target) { Move-Item -LiteralPath $Target -Destination $Backup -Force }
    Move-Item -LiteralPath $New -Destination $Target -Force
    $replaced = $true
    $started = Start-Process -FilePath $Target -PassThru
    Start-Sleep -Milliseconds 5000
    if ($started.HasExited) { throw "The updated application exited during startup" }
    $replaced = $false
    if (Test-Path -LiteralPath $Backup) {
        Remove-Item -LiteralPath $Backup -Force -ErrorAction SilentlyContinue
    }
} catch {
    try {
        if ($replaced -and (Test-Path -LiteralPath $Target)) {
            Remove-Item -LiteralPath $Target -Force
        }
        if (Test-Path -LiteralPath $Backup) {
            Move-Item -LiteralPath $Backup -Destination $Target -Force
        }
    } catch { }
    exit 1
} finally {
    Remove-Item -LiteralPath $New -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $MyInvocation.MyCommand.Path -Force -ErrorAction SilentlyContinue
}
"#;
        if let Err(error) = std::fs::write(&script_path, script) {
            let _ = std::fs::remove_file(&script_path);
            return Err(format!("Failed to write update script: {}", error));
        }

        let spawn_result = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                &script_path.to_string_lossy(),
                "-Target",
                &current.to_string_lossy(),
                "-New",
                &new_binary.to_string_lossy(),
                "-Backup",
                &bak.to_string_lossy(),
                "-ProcessId",
                &pid.to_string(),
            ])
            // DETACHED_PROCESS (0x8) | CREATE_NO_WINDOW (0x08000000)
            .creation_flags(0x08000008)
            .spawn();
        if let Err(error) = spawn_result {
            let _ = std::fs::remove_file(&script_path);
            let _ = std::fs::remove_file(new_binary);
            return Err(format!("Failed to spawn update script: {}", error));
        }
        Ok(())
    }

    #[cfg(unix)]
    {
        let script_path =
            current.with_file_name(format!(".sing-box-gui-update-{}.sh", std::process::id()));
        fn shell_quote(value: &std::path::Path) -> String {
            format!("'{}'", value.to_string_lossy().replace('\'', "'\\''"))
        }
        let script = format!(
            r#"#!/bin/sh
TARGET={current}
NEW={new}
BACKUP="$TARGET.bak"
PID={pid}
cleanup() {{
  rm -f -- "$0" "$NEW"
}}
trap cleanup EXIT
while kill -0 "$PID" 2>/dev/null; do sleep 0.25; done
rm -f -- "$BACKUP"
if [ -e "$TARGET" ]; then mv -f -- "$TARGET" "$BACKUP" || exit 1; fi
if ! mv -f -- "$NEW" "$TARGET"; then
  [ -e "$BACKUP" ] && mv -f -- "$BACKUP" "$TARGET"
  exit 1
fi
if ! chmod +x -- "$TARGET"; then
  rm -f -- "$TARGET"
  [ -e "$BACKUP" ] && mv -f -- "$BACKUP" "$TARGET"
  exit 1
fi
"$TARGET" &
NEW_PID=$!
sleep 5
if kill -0 "$NEW_PID" 2>/dev/null; then
  rm -f -- "$BACKUP"
  exit 0
fi
wait "$NEW_PID" 2>/dev/null || true
if [ -e "$TARGET" ]; then
  rm -f -- "$TARGET"
fi
[ -e "$BACKUP" ] && mv -f -- "$BACKUP" "$TARGET"
exit 1
"#,
            current = shell_quote(&current),
            new = shell_quote(new_binary),
            pid = pid,
        );
        if let Err(error) = std::fs::write(&script_path, &script) {
            let _ = std::fs::remove_file(&script_path);
            return Err(format!("Failed to write update script: {}", error));
        }
        use std::os::unix::fs::PermissionsExt;
        let mut perms = match std::fs::metadata(&script_path) {
            Ok(metadata) => metadata.permissions(),
            Err(error) => {
                let _ = std::fs::remove_file(&script_path);
                return Err(format!("Failed to stat update script: {}", error));
            }
        };
        perms.set_mode(0o755);
        if let Err(error) = std::fs::set_permissions(&script_path, perms) {
            let _ = std::fs::remove_file(&script_path);
            return Err(format!("Failed to chmod update script: {}", error));
        }

        let spawn_result = std::process::Command::new("sh").arg(&script_path).spawn();
        if let Err(error) = spawn_result {
            let _ = std::fs::remove_file(&script_path);
            let _ = std::fs::remove_file(new_binary);
            return Err(format!("Failed to spawn update script: {}", error));
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "windows", unix)))]
    {
        let _ = (new_binary, pid, current);
        Err("In-app update is not supported on this platform".to_string())
    }
}

fn main_window_settings(size: iced::Size) -> iced::window::Settings {
    iced::window::Settings {
        size,
        min_size: Some(iced::Size::new(ui::WINDOW_MIN_W, ui::WINDOW_MIN_H)),
        icon: iced::window::icon::from_file_data(include_bytes!("../assets/app-icon.png"), None)
            .ok(),
        exit_on_close_request: false,
        #[cfg(target_os = "linux")]
        platform_specific: iced::window::settings::PlatformSpecific {
            application_id: "sing-box-gui".into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn is_ui_smoke() -> bool {
    #[cfg(feature = "ui-smoke")]
    {
        smoke::requested()
    }
    #[cfg(not(feature = "ui-smoke"))]
    {
        false
    }
}

fn main() -> iced::Result {
    if !is_ui_smoke() {
        core::cleanup_stale_temp_binaries();
        config::cleanup_stale_temp_files();
    }

    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Only a main-thread panic tears the app down; tokio catches worker
        // panics and the app keeps running, so cleaning up there would
        // silently desync the OS proxy from the still-alive UI.
        if !is_ui_smoke() && std::thread::current().name() == Some("main") {
            let config = config::load_gui_config();
            if config.system_proxy_owned
                || sysproxy::is_system_proxy_owned()
                || sysproxy::has_persisted_backup()
            {
                let _ = sysproxy::set_system_proxy(false, config.mixed_port);
            }
            core::stop_core();
        }
        default_panic(info);
    }));

    // Bundle a consistent CJK-capable UI face. Relying on the platform's
    // generic SansSerif fallback caused Chinese text and symbols to render as
    // tofu boxes on some Windows installations.
    let default_font = ui::theme::ui_font(iced::font::Weight::Normal);

    let res = iced::daemon(App::new, App::update, App::view)
        .title("sing-box GUI")
        .theme(|app: &App, _| app.theme())
        .default_font(default_font)
        .font(include_bytes!("../assets/NotoSansCJK-Regular.ttc").as_slice())
        .font(include_bytes!("../assets/material-icons.ttf").as_slice())
        .subscription(App::subscription)
        .run();

    if is_ui_smoke() {
        return res;
    }

    // CRITICAL EXIT CLEANUP with timeout. Disable the proxy before stopping
    // the core: stop_core alone can take ~2 s on unix (SIGTERM grace) and a
    // timeout must never strand the OS behind a dead proxy endpoint.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let config = config::load_gui_config();
        if config.system_proxy_owned
            || sysproxy::is_system_proxy_owned()
            || sysproxy::has_persisted_backup()
        {
            let _ = sysproxy::set_system_proxy(false, config.mixed_port);
        }
        if config.close_core_on_exit {
            core::stop_core();
        }
        let _ = tx.send(());
    });
    let _ = rx.recv_timeout(std::time::Duration::from_secs(5));

    res
}
