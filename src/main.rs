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
mod state;
mod sysproxy;
mod ui;
mod update;

use futures::{SinkExt, StreamExt};
use iced::widget::{button, column, container, responsive, row, text, tooltip};
use iced::{Alignment, Element, Font, Length, Subscription, Task};
use message::Message;
use state::{Bandwidth, GuiConfig, Profile, ProxyNode, Tab, Toast};
use std::sync::{Mutex, OnceLock};
use tokio::sync::mpsc;

// OnceLocks for streaming logs and traffic stats asynchronously
static LOG_RX: OnceLock<Mutex<Option<mpsc::Receiver<String>>>> = OnceLock::new();
static LOG_TX: OnceLock<mpsc::Sender<String>> = OnceLock::new();

static TRAFFIC_RX: OnceLock<Mutex<Option<mpsc::Receiver<api::TrafficInfo>>>> = OnceLock::new();
static TRAFFIC_TX: OnceLock<mpsc::Sender<api::TrafficInfo>> = OnceLock::new();

const MAX_LOG_LINES: usize = 500;
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
    url_input: String,
    core_installed: bool,
    core_install_msg: Option<String>,
    update_status: state::UpdateStatus,
    node_search: String,
    profile_error: Option<String>,
    selected_group: String,
    proxy_groups: std::collections::HashMap<String, crate::api::ProxyInfo>,
    bypass_domain_input: String,
    proxy_domain_input: String,
    bypass_ip_input: String,
    proxy_ip_input: String,
    mixed_port_input_str: String,
    api_port_input_str: String,
    dns_server_local_input_str: String,
    dns_server_remote_input_str: String,
    connections_search: String,
    _tray_icon: Option<tray_icon::TrayIcon>,
    tray_menu_show: tray_icon::menu::MenuItem,
    tray_menu_exit: tray_icon::menu::MenuItem,
    tray_menu_submenu: tray_icon::menu::Submenu,
    tray_menu_toggle_core: tray_icon::menu::MenuItem,
    tray_menu_rule_mode: tray_icon::menu::CheckMenuItem,
    tray_menu_global_mode: tray_icon::menu::CheckMenuItem,
    tray_menu_direct_mode: tray_icon::menu::CheckMenuItem,
    tray_menu_system_proxy: tray_icon::menu::CheckMenuItem,
    window_id: Option<iced::window::Id>,

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
    core_path_input_str: String,
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

    connections_sort: state::ConnectionSort,
    connections_sort_desc: bool,
    proxy_sort: state::ProxySort,

    cached_system_is_light: bool,
    theme_check_counter: u32,
    config_dirty: bool,
    pending_exit: bool,
    pending_update_path: Option<std::path::PathBuf>,
}

impl App {
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
        let gui_config = config::load_gui_config();
        let cached_system_is_light = detect_system_theme();
        let core_installed = core::is_core_installed(&gui_config);
        let selected_node_tag = gui_config.selected_node_tag.clone();

        let mixed_port_input_str = gui_config.mixed_port.to_string();
        let api_port_input_str = gui_config.api_port.to_string();
        let dns_server_local_input_str = gui_config.dns_server_local.clone();
        let dns_server_remote_input_str = gui_config.dns_server_remote.clone();

        use tray_icon::{
            TrayIconBuilder,
            menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
        };

        // Create the tray menu (labels match current language; update_tray_menu refreshes later)
        let is_zh = gui_config.language == state::Language::Zh;
        let tray_menu = Menu::new();
        let show_item = MenuItem::with_id(
            "show_window",
            if is_zh {
                "显示主界面"
            } else {
                "Show Window"
            },
            true,
            None,
        );
        let toggle_core_item = MenuItem::with_id(
            "toggle_core",
            if is_zh { "启动内核" } else { "Start Core" },
            true,
            None,
        );

        let rule_mode_item = CheckMenuItem::with_id(
            "mode_rule",
            if is_zh { "规则分流" } else { "Rules" },
            true,
            false,
            None,
        );
        let global_mode_item = CheckMenuItem::with_id(
            "mode_global",
            if is_zh { "全局代理" } else { "Global" },
            true,
            false,
            None,
        );
        let direct_mode_item = CheckMenuItem::with_id(
            "mode_direct",
            if is_zh { "直接连接" } else { "Direct" },
            true,
            false,
            None,
        );

        let system_proxy_item = CheckMenuItem::with_id(
            "toggle_system_proxy",
            if is_zh {
                "系统代理"
            } else {
                "System Proxy"
            },
            true,
            gui_config.system_proxy_enabled,
            None,
        );

        let mode_submenu =
            tray_icon::menu::Submenu::new(if is_zh { "代理模式" } else { "Proxy Mode" }, true);
        let _ = mode_submenu.append(&rule_mode_item);
        let _ = mode_submenu.append(&global_mode_item);
        let _ = mode_submenu.append(&direct_mode_item);

        let exit_item =
            MenuItem::with_id("exit_app", if is_zh { "退出" } else { "Exit" }, true, None);

        let _ = tray_menu.append(&show_item);
        let _ = tray_menu.append(&PredefinedMenuItem::separator());
        let _ = tray_menu.append(&toggle_core_item);
        let _ = tray_menu.append(&system_proxy_item);
        let _ = tray_menu.append(&mode_submenu);
        let _ = tray_menu.append(&PredefinedMenuItem::separator());
        let _ = tray_menu.append(&exit_item);

        let tray_icon = load_icon_safe().and_then(|icon| {
            TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                .with_tooltip("sing-box GUI")
                .with_icon(icon)
                .build()
                .ok()
        });

        let initial_routing_mode = gui_config.routing_mode;
        let core_path_input_str = gui_config.core_path.clone().unwrap_or_default();

        let mut app = Self {
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
            url_input: String::new(),
            core_installed,
            core_install_msg: None,
            update_status: state::UpdateStatus::NotChecked,
            node_search: String::new(),
            profile_error: None,
            selected_group: String::new(),
            proxy_groups: std::collections::HashMap::new(),
            bypass_domain_input: String::new(),
            proxy_domain_input: String::new(),
            bypass_ip_input: String::new(),
            proxy_ip_input: String::new(),
            mixed_port_input_str,
            api_port_input_str,
            dns_server_local_input_str,
            dns_server_remote_input_str,
            connections_search: String::new(),
            _tray_icon: tray_icon,
            tray_menu_show: show_item,
            tray_menu_exit: exit_item,
            tray_menu_submenu: mode_submenu,
            tray_menu_toggle_core: toggle_core_item,
            tray_menu_rule_mode: rule_mode_item,
            tray_menu_global_mode: global_mode_item,
            tray_menu_direct_mode: direct_mode_item,
            tray_menu_system_proxy: system_proxy_item,
            window_id: None,

            traffic_cancel_tx: None,
            confirm_delete_profile_id: None,
            editing_profile_id: None,
            editing_profile_name: String::new(),
            editing_profile_url: String::new(),

            last_core_running: false,
            last_sys_proxy_enabled: false,
            last_routing_mode: initial_routing_mode,
            toast: None,
            core_path_input_str,
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
            connections_sort: state::ConnectionSort::None,
            connections_sort_desc: false,
            proxy_sort: state::ProxySort::Latency,

            cached_system_is_light,
            theme_check_counter: 0,
            config_dirty: false,
            pending_exit: false,
            pending_update_path: None,
        };

        // Force initialization of log and traffic streams on startup
        let _ = get_log_tx();
        let _ = get_traffic_tx();

        // Sync system proxy checkbox status with system state
        let sys_proxy = sysproxy::check_system_proxy(app.gui_config.mixed_port).unwrap_or(false);
        app.sys_proxy_enabled = sys_proxy;
        app.gui_config.system_proxy_enabled = sys_proxy;

        // Initial tray menu synchronization
        app.update_tray_menu();

        let mut tasks = Vec::new();
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

    fn sync_input_buffers(&mut self) {
        self.mixed_port_input_str = self.gui_config.mixed_port.to_string();
        self.api_port_input_str = self.gui_config.api_port.to_string();
        self.dns_server_local_input_str = self.gui_config.dns_server_local.clone();
        self.dns_server_remote_input_str = self.gui_config.dns_server_remote.clone();
    }

    fn update_tray_menu(&self) {
        let is_zh = self.gui_config.language == state::Language::Zh;

        if is_zh {
            self.tray_menu_show.set_text("显示主界面");
            if self.core_running {
                self.tray_menu_toggle_core.set_text("关闭内核");
            } else {
                self.tray_menu_toggle_core.set_text("启动内核");
            }
            self.tray_menu_system_proxy.set_text("系统代理");
            self.tray_menu_submenu.set_text("代理模式");
            self.tray_menu_rule_mode.set_text("规则分流");
            self.tray_menu_global_mode.set_text("全局代理");
            self.tray_menu_direct_mode.set_text("直接连接");
            self.tray_menu_exit.set_text("退出");
        } else {
            self.tray_menu_show.set_text("Show Window");
            if self.core_running {
                self.tray_menu_toggle_core.set_text("Stop Core");
            } else {
                self.tray_menu_toggle_core.set_text("Start Core");
            }
            self.tray_menu_system_proxy.set_text("System Proxy");
            self.tray_menu_submenu.set_text("Proxy Mode");
            self.tray_menu_rule_mode.set_text("Rules");
            self.tray_menu_global_mode.set_text("Global");
            self.tray_menu_direct_mode.set_text("Direct");
            self.tray_menu_exit.set_text("Exit");
        }

        self.tray_menu_rule_mode
            .set_checked(self.gui_config.routing_mode == state::RoutingMode::Rule);
        self.tray_menu_global_mode
            .set_checked(self.gui_config.routing_mode == state::RoutingMode::Global);
        self.tray_menu_direct_mode
            .set_checked(self.gui_config.routing_mode == state::RoutingMode::Direct);
        self.tray_menu_system_proxy
            .set_checked(self.gui_config.system_proxy_enabled);
    }

    fn core_busy(&self) -> bool {
        self.core_starting || self.core_stopping
    }

    fn request_exit(&mut self) -> Task<Message> {
        if !self.gui_config.close_core_on_exit {
            return iced::exit();
        }

        self.pending_exit = true;
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

        if sysproxy::is_system_proxy_owned() {
            let _ = sysproxy::set_system_proxy(false, self.gui_config.mixed_port);
        }
        iced::exit()
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
        self.log_lines
            .push_back("[GUI] sing-box core started successfully.".to_string());

        if self.gui_config.auto_sys_proxy && !self.sys_proxy_enabled {
            match sysproxy::set_system_proxy(true, self.gui_config.mixed_port) {
                Ok(_) => {
                    self.sys_proxy_enabled = true;
                    self.gui_config.system_proxy_enabled = true;
                    self.config_dirty = true;
                    self.log_lines
                        .push_back("[GUI] System proxy auto-enabled on core start.".to_string());
                }
                Err(e) => {
                    let msg = format!("Failed to auto-enable system proxy: {}", e);
                    self.log_lines.push_back(format!("[GUI] {}", msg));
                    self.toast_error(if self.gui_config.language == state::Language::Zh {
                        format!("自动开启系统代理失败: {}", e)
                    } else {
                        msg
                    });
                }
            }
        }

        if let Some(cancel_tx) = self.traffic_cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.traffic_cancel_tx = Some(tx);
        let traffic_tx = get_traffic_tx();
        api::spawn_traffic_monitor(self.gui_config.api_port, traffic_tx, rx);

        Task::perform(
            async {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            },
            |_| Message::Tick,
        )
    }

    /// Tear down traffic monitor and optionally disable system proxy after stop.
    fn on_core_stopped_cleanup(&mut self) {
        self.core_running = false;
        if let Some(cancel_tx) = self.traffic_cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
        if self.gui_config.disable_proxy_on_core_stop
            && self.sys_proxy_enabled
            && sysproxy::is_system_proxy_owned()
        {
            match sysproxy::set_system_proxy(false, self.gui_config.mixed_port) {
                Ok(_) => {
                    self.sys_proxy_enabled = false;
                    self.gui_config.system_proxy_enabled = false;
                    self.config_dirty = true;
                }
                Err(e) => {
                    self.log_lines
                        .push_back(format!("[GUI] Failed to disable system proxy: {}", e));
                    self.toast_error(if self.gui_config.language == state::Language::Zh {
                        format!("关闭系统代理失败: {}", e)
                    } else {
                        format!("Failed to disable system proxy: {}", e)
                    });
                }
            }
        }
        self.current_speed = Bandwidth::default();
        self.total_uploaded = 0;
        self.total_downloaded = 0;
        self.log_lines
            .push_back("[GUI] sing-box core stopped.".to_string());
    }

    fn restart_core(&mut self) -> Task<Message> {
        if self.core_busy() {
            // Coalesce: ensure we restart once the in-flight transition ends.
            self.pending_core_restart = true;
            return Task::none();
        }
        if self.core_running {
            self.pending_core_restart = true;
            self.log_lines
                .push_back("[GUI] Restarting core to apply new settings...".to_string());
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
            Message::TrayIconClicked => {
                if let Some(id) = self.window_id {
                    Task::batch(vec![
                        iced::window::set_mode(id, iced::window::Mode::Windowed),
                        iced::window::gain_focus(id),
                    ])
                } else {
                    Task::none()
                }
            }
            Message::TrayMenuClicked(id_str) => match id_str.as_str() {
                "show_window" => {
                    if let Some(id) = self.window_id {
                        Task::batch(vec![
                            iced::window::set_mode(id, iced::window::Mode::Windowed),
                            iced::window::gain_focus(id),
                        ])
                    } else {
                        Task::none()
                    }
                }
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
                Task::none()
            }
            Message::WindowCloseRequested(id) => {
                self.window_id = Some(id);
                if self._tray_icon.is_some() {
                    iced::window::set_mode(id, iced::window::Mode::Hidden)
                } else {
                    self.request_exit()
                }
            }
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
                    self.log_lines.push_back(format!(
                        "[GUI] Failed to select node {} for group {}: {}",
                        node, group, err
                    ));
                } else {
                    if let Some(g_info) = self.proxy_groups.get_mut(&group) {
                        g_info.now = Some(node.clone());
                    }
                    if group == "Proxy" {
                        self.selected_node_tag = Some(node.clone());
                        self.gui_config.selected_node_tag = Some(node.clone());
                        self.config_dirty = true;
                    }
                    self.log_lines
                        .push_back(format!("[GUI] Selected node: {} for group {}", node, group));
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
                    self.log_lines.push_back(format!(
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
                    self.log_lines.push_back(format!(
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
                        self.core_running = false;
                        self.force_stop_after_start = false;
                        self.log_lines
                            .push_back(format!("[GUI] Error starting core: {}", e));
                        self.toast_error(e);
                        // Drop pending restart so we don't loop on a broken config.
                        self.pending_core_restart = false;
                        if self.pending_exit {
                            if sysproxy::is_system_proxy_owned() {
                                let _ =
                                    sysproxy::set_system_proxy(false, self.gui_config.mixed_port);
                            }
                            return iced::exit();
                        }
                        Task::none()
                    }
                }
            }
            Message::CoreStopFinished => {
                self.core_stopping = false;
                self.on_core_stopped_cleanup();
                if self.pending_exit {
                    if sysproxy::is_system_proxy_owned() {
                        let _ = sysproxy::set_system_proxy(false, self.gui_config.mixed_port);
                    }
                    return iced::exit();
                }

                if let Some(path) = self.pending_update_path.take() {
                    if sysproxy::is_system_proxy_owned() {
                        let _ = sysproxy::set_system_proxy(false, self.gui_config.mixed_port);
                    }
                    match apply_update_and_restart(&path) {
                        Ok(()) => {
                            self.log_lines
                                .push_back("[GUI] Update scheduled; exiting to apply.".to_string());
                            return iced::exit();
                        }
                        Err(e) => {
                            self.log_lines
                                .push_back(format!("[GUI] Failed to apply update: {}", e));
                            self.toast_error(format!("Update failed: {}", e));
                        }
                    }
                }

                if self.pending_core_restart {
                    self.pending_core_restart = false;
                    return self.task_start_core();
                }
                Task::none()
            }
            Message::NewLogLine(line) => {
                self.log_lines.push_back(line);
                // When the GUI log window oversize its cap, drop the oldest 10%
                // entries in a single drain (one memmove) to keep memory bounded
                // without paying per-line overhead on every message.
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
                self.log_lines
                    .push_back(format!("[GUI] Logs exported to {}", path));
                self.toast_success(if self.gui_config.language == state::Language::Zh {
                    format!("日志已导出: {}", path)
                } else {
                    format!("Logs exported: {}", path)
                });
                open_path_in_system(std::path::Path::new(&path));
                Task::none()
            }
            Message::LogsExported(Err(e)) => {
                self.toast_error(e);
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
                self.gui_config.auto_update_interval_hours = hours;
                self.config_dirty = true;
                Task::none()
            }
            Message::ToggleDisableProxyOnCoreStop => {
                self.gui_config.disable_proxy_on_core_stop =
                    !self.gui_config.disable_proxy_on_core_stop;
                self.config_dirty = true;
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
                self.toast_info(if self.gui_config.language == state::Language::Zh {
                    "无法读取剪贴板"
                } else {
                    "Could not read clipboard"
                });
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
                let log_tx = get_log_tx();
                self.log_lines
                    .push_back("[GUI] Starting sing-box core download...".to_string());
                self.core_install_msg = Some("Downloading...".to_string());
                Task::perform(
                    async move { core::download_core(log_tx, false).await },
                    Message::CoreDownloaded,
                )
            }
            Message::ForceCoreDownload => {
                let log_tx = get_log_tx();
                self.log_lines
                    .push_back("[GUI] Force reinstalling sing-box core...".to_string());
                self.core_install_msg = Some("Reinstalling...".to_string());
                Task::perform(
                    async move { core::download_core(log_tx, true).await },
                    Message::CoreDownloaded,
                )
            }
            Message::LatencyTestUrlChanged(url) => {
                self.gui_config.latency_test_url = url;
                self.config_dirty = true;
                Task::none()
            }
            Message::LatencyTestTimeoutChanged(s) => {
                if let Ok(ms) = s.parse::<u32>() {
                    self.gui_config.latency_test_timeout_ms = ms.clamp(500, 30_000);
                    self.config_dirty = true;
                }
                Task::none()
            }
            Message::CoreDownloaded(res) => {
                match res {
                    Ok(_) => {
                        self.core_installed = true;
                        self.core_install_msg = None;
                        self.log_lines.push_back(
                            "[GUI] sing-box core downloaded and installed successfully."
                                .to_string(),
                        );
                        self.toast_success(if self.gui_config.language == state::Language::Zh {
                            "内核下载安装成功"
                        } else {
                            "Core downloaded successfully"
                        });
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
                        self.core_install_msg = Some(e.clone());
                        self.log_lines
                            .push_back(format!("[GUI ERROR] Failed to download core: {}", e));
                        self.toast_error(e);
                    }
                }
                Task::none()
            }
            Message::TrafficUpdated { up, down } => {
                if self.current_speed.up == up && self.current_speed.down == down {
                    return Task::none();
                }
                self.current_speed = Bandwidth { up, down };
                self.speed_history.push((up, down));
                if self.speed_history.len() > 30 {
                    let excess = self.speed_history.len() - 30;
                    self.speed_history.drain(..excess);
                }
                Task::none()
            }
            Message::ToggleSystemProxy => {
                let target = !self.sys_proxy_enabled;
                match sysproxy::set_system_proxy(target, self.gui_config.mixed_port) {
                    Ok(_) => {
                        self.sys_proxy_enabled = target;
                        self.gui_config.system_proxy_enabled = target;
                        self.config_dirty = true;
                        self.log_lines
                            .push_back(format!("[GUI] System proxy toggled to: {}", target));
                    }
                    Err(e) => {
                        self.log_lines
                            .push_back(format!("[GUI] System proxy error: {}", e));
                        self.toast_error(if self.gui_config.language == state::Language::Zh {
                            format!("系统代理切换失败: {}", e)
                        } else {
                            format!("System proxy error: {}", e)
                        });
                    }
                }
                Task::none()
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
                let mut load_nodes = false;
                if let Some(err) = error {
                    self.profile_error = Some(err.clone());
                    self.log_lines
                        .push_back(format!("[GUI] Download failed: {}", err));
                    self.toast_error(err);
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

                    self.sync_input_buffers();

                    if self.gui_config.active_profile_id.is_none() && !id.is_empty() {
                        self.gui_config.active_profile_id = Some(id.clone());
                        self.config_dirty = true;
                        load_nodes = true;
                    }
                    self.log_lines
                        .push_back("[GUI] Subscription downloaded successfully.".to_string());
                    self.toast_success(self.tr("toast_sub_ok"));
                }
                let next_update = self.kick_pending_auto_update();
                if load_nodes {
                    Task::batch(vec![self.load_active_nodes_task(), next_update])
                } else {
                    next_update
                }
            }
            Message::SelectProfile(id) => {
                self.confirm_delete_profile_id = None;
                self.gui_config.active_profile_id = Some(id);
                self.active_profile_nodes.clear();
                self.proxy_groups.clear();
                self.selected_group.clear();
                self.config_dirty = true;
                self.log_lines
                    .push_back("[GUI] Active profile updated.".to_string());
                self.toast_success(self.tr("profile_selected_toast"));
                Task::batch(vec![self.load_active_nodes_task(), self.restart_core()])
            }
            Message::ActiveNodesLoaded { profile_id, result } => {
                if self.gui_config.active_profile_id.as_deref() != Some(profile_id.as_str()) {
                    return Task::none();
                }
                match result {
                    Ok(nodes) => self.active_profile_nodes = nodes,
                    Err(error) => {
                        self.active_profile_nodes.clear();
                        self.log_lines
                            .push_back(format!("[GUI] Failed to load active nodes: {error}"));
                    }
                }
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
                if let Some(id) = self.confirm_delete_profile_id.take() {
                    let path = config::get_profile_path(&id);
                    let _ = std::fs::remove_file(path);

                    let was_active = self.gui_config.active_profile_id.as_ref() == Some(&id);
                    self.gui_config.subscriptions.retain(|p| p.id != id);
                    if was_active {
                        self.gui_config.active_profile_id = None;
                        self.active_profile_nodes.clear();
                    }
                    self.config_dirty = true;
                    self.log_lines
                        .push_back("[GUI] Profile deleted.".to_string());
                    self.toast_success(self.tr("toast_profile_deleted"));

                    // Active profile deleted while core runs → stop to avoid orphan config.
                    if was_active {
                        self.pending_core_restart = false;
                        if self.core_starting {
                            self.force_stop_after_start = true;
                            self.toast_info(self.tr("toast_core_stopped_profile_deleted"));
                        } else if self.core_running && !self.core_stopping {
                            self.toast_info(self.tr("toast_core_stopped_profile_deleted"));
                            return self.task_stop_core();
                        }
                    }
                }
                Task::none()
            }
            Message::SelectNode(tag) => {
                if !self.core_running {
                    self.selected_node_tag = Some(tag.clone());
                    self.gui_config.selected_node_tag = Some(tag.clone());
                    self.config_dirty = true;
                    return Task::none();
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
                    self.log_lines
                        .push_back(format!("[GUI] Failed to select node: {}", err));
                } else {
                    self.selected_node_tag = Some(tag.clone());
                    self.gui_config.selected_node_tag = Some(tag.clone());
                    self.config_dirty = true;
                    self.log_lines
                        .push_back(format!("[GUI] Selected node: {}", tag));
                }
                Task::none()
            }
            Message::StartLatencyTest => {
                if !self.core_running {
                    self.toast_info(self.tr("toast_start_core_first"));
                    return Task::none();
                }
                if self.active_profile_nodes.is_empty() {
                    self.toast_info(self.tr("toast_no_nodes"));
                    return Task::none();
                }
                self.latency_testing = true;
                let api_port = self.gui_config.api_port;
                let test_url = self.gui_config.latency_test_url.clone();
                let timeout_ms = self.gui_config.latency_test_timeout_ms;
                // Cap concurrent delay probes so Clash API / UI are not stampeded.
                let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(8));

                let tasks = self
                    .active_profile_nodes
                    .iter()
                    .map(|node| {
                        let tag = node.name.clone();
                        let test_url = test_url.clone();
                        let sem = std::sync::Arc::clone(&sem);
                        Task::perform(
                            async move {
                                let _permit = sem.acquire().await;
                                let latency =
                                    api::test_node_latency(api_port, &tag, &test_url, timeout_ms)
                                        .await;
                                (tag, latency)
                            },
                            |(tag, res)| Message::NodeLatencyTested {
                                tag,
                                latency: res.ok(),
                            },
                        )
                    })
                    .collect::<Vec<_>>();

                Task::batch(tasks).chain(Task::done(Message::LatencyTestComplete))
            }
            Message::NodeLatencyTested { tag, latency } => {
                for node in &mut self.active_profile_nodes {
                    if node.name == tag {
                        node.latency = latency;
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
                self.confirm_delete_profile_id = None;
                let sub = self.gui_config.subscriptions.iter().find(|p| p.id == id);
                if let Some(profile) = sub {
                    let url = profile.url.clone();
                    let id_clone = id.clone();
                    self.downloading = true;
                    self.profile_error = None;
                    self.log_lines
                        .push_back(format!("[GUI] Updating subscription: {}", url));

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
                    self.log_lines
                        .push_back("[GUI] Subscription not found.".to_string());
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
                self.theme_check_counter = self.theme_check_counter.wrapping_add(1);

                let mut tasks = Vec::new();
                if self.config_dirty && !self.config_save_in_flight {
                    let cfg = self.gui_config.clone();
                    self.config_dirty = false;
                    self.config_save_in_flight = true;
                    tasks.push(Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || config::save_gui_config(&cfg))
                                .await
                                .map_err(|e| e.to_string())
                                .and_then(|result| result)
                        },
                        Message::ConfigSaved,
                    ));
                }

                // Lock-free fast path; during async start/stop prefer the transition flags.
                if !self.core_busy() {
                    self.core_running = core::is_core_running_fast();
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
                if self.gui_config.theme == state::AppTheme::Auto
                    && self.theme_check_counter.is_multiple_of(15)
                {
                    tasks.push(Task::perform(
                        async {
                            tokio::task::spawn_blocking(detect_system_theme)
                                .await
                                .unwrap_or(false)
                        },
                        Message::SystemThemeDetected,
                    ));
                }
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
            Message::ConfigSaved(result) => {
                self.config_save_in_flight = false;
                if let Err(error) = result {
                    self.config_dirty = true;
                    self.log_lines
                        .push_back(format!("[GUI] Failed to save settings: {error}"));
                }
                self.refresh_config_preview()
            }
            Message::SystemThemeDetected(is_light) => {
                self.cached_system_is_light = is_light;
                Task::none()
            }
            Message::CoreLivenessChecked(running) => {
                let was_running = self.core_running;
                self.core_running = running;
                if was_running
                    && !self.core_running
                    && let Some(msg) = core::take_unexpected_core_exit()
                {
                    self.log_lines.push_back(format!("[GUI] {}", msg));
                    self.toast_error(msg);
                    if self.gui_config.disable_proxy_on_core_stop
                        && self.sys_proxy_enabled
                        && sysproxy::is_system_proxy_owned()
                    {
                        let _ = sysproxy::set_system_proxy(false, self.gui_config.mixed_port);
                        self.sys_proxy_enabled = false;
                        self.gui_config.system_proxy_enabled = false;
                        self.config_dirty = true;
                    }
                    if let Some(cancel_tx) = self.traffic_cancel_tx.take() {
                        let _ = cancel_tx.send(());
                    }
                    self.current_speed = Bandwidth::default();
                }
                Task::none()
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
                self.log_lines
                    .push_back(format!("[GUI] Closed connection {}", id));
                self.active_connections.retain(|c| c.id != id);
                Task::none()
            }
            Message::ConnectionClosed(Err(e)) => {
                self.log_lines
                    .push_back(format!("[GUI] Failed to close connection: {}", e));
                self.toast_error(e);
                Task::none()
            }
            Message::CloseAllConnections => {
                if !self.core_running {
                    self.toast_info(if self.gui_config.language == state::Language::Zh {
                        "内核未运行"
                    } else {
                        "Core is not running"
                    });
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
                self.log_lines
                    .push_back("[GUI] Closed all connections.".to_string());
                self.toast_success(if self.gui_config.language == state::Language::Zh {
                    "已关闭全部连接"
                } else {
                    "All connections closed"
                });
                Task::none()
            }
            Message::AllConnectionsClosed(Err(e)) => {
                self.log_lines
                    .push_back(format!("[GUI] Failed to close all connections: {}", e));
                self.toast_error(e);
                Task::none()
            }
            Message::RoutingModeChanged(mode) => {
                self.gui_config.routing_mode = mode;
                self.config_dirty = true;
                let mode_label = mode.as_clash_mode();
                if self.core_running {
                    let api_port = self.gui_config.api_port;
                    let mode_str = mode_label.to_string();
                    Task::perform(
                        async move { api::set_mode(api_port, &mode_str).await.map(|_| mode_str) },
                        Message::ModeSet,
                    )
                } else {
                    self.log_lines.push_back(format!(
                        "[GUI] Routing mode set to {} (will apply on next core start).",
                        mode_label
                    ));
                    self.toast_success(if self.gui_config.language == state::Language::Zh {
                        format!("路由模式已设为 {}（下次启动生效）", mode_label)
                    } else {
                        format!("Routing mode set to {} (applies on next start)", mode_label)
                    });
                    Task::none()
                }
            }
            Message::ModeSet(Ok(mode)) => {
                self.log_lines
                    .push_back(format!("[GUI] Routing mode switched to {}.", mode));
                self.toast_success(if self.gui_config.language == state::Language::Zh {
                    format!("路由模式已切换为 {}", mode)
                } else {
                    format!("Routing mode: {}", mode)
                });
                Task::none()
            }
            Message::ModeSet(Err(e)) => {
                self.log_lines
                    .push_back(format!("[GUI] Failed to set routing mode: {}", e));
                self.toast_error(if self.gui_config.language == state::Language::Zh {
                    format!("切换路由模式失败: {}", e)
                } else {
                    format!("Failed to set mode: {}", e)
                });
                Task::none()
            }
            Message::DismissToast => {
                self.toast = None;
                Task::none()
            }

            // New type-safe configuration messages
            Message::MixedPortChanged(val) => {
                self.mixed_port_input_str = val;
                Task::none()
            }
            Message::ApiPortChanged(val) => {
                self.api_port_input_str = val;
                Task::none()
            }
            Message::DnsLocalChanged(val) => {
                self.dns_server_local_input_str = val;
                Task::none()
            }
            Message::DnsRemoteChanged(val) => {
                self.dns_server_remote_input_str = val;
                Task::none()
            }
            Message::CorePathChanged(val) => {
                self.core_path_input_str = val.clone();
                let trimmed = val.trim();
                if trimmed.is_empty() {
                    self.gui_config.core_path = None;
                } else {
                    self.gui_config.core_path = Some(trimmed.to_string());
                }
                self.core_installed = core::is_core_installed(&self.gui_config);
                Task::none()
            }
            Message::ClearCorePath => {
                self.core_path_input_str.clear();
                self.gui_config.core_path = None;
                self.core_installed = core::is_core_installed(&self.gui_config);
                self.config_dirty = true;
                self.toast_info(if self.gui_config.language == state::Language::Zh {
                    "已恢复默认内核路径"
                } else {
                    "Restored default core path"
                });
                Task::none()
            }
            Message::ToggleTun => {
                self.gui_config.tun_mode = !self.gui_config.tun_mode;
                self.config_dirty = true;
                // Admin-elevation sanity check: TUN inbound needs CAP_NET_ADMIN /
                // the Windows wintun driver via an elevated process. Tell the user
                // *now* instead of letting them discover it from a FATAL toast.
                if self.gui_config.tun_mode && !platform::is_running_elevated() {
                    self.toast_info(if self.gui_config.language == state::Language::Zh {
                        "TUN 模式需要管理员权限，内核可能启动失败。请以管理员身份重启本程序。"
                    } else {
                        "TUN mode requires Administrator privileges. Restart this app elevated to enable it."
                    });
                }
                self.restart_core()
            }
            Message::ToggleAutostart => {
                let target = !self.gui_config.start_on_boot;
                match platform::set_autostart(target) {
                    Ok(()) => {
                        self.gui_config.start_on_boot = target;
                        self.config_dirty = true;
                    }
                    Err(e) => self.toast_error(e),
                }
                Task::none()
            }
            Message::ToggleAutoStartCore => {
                self.gui_config.auto_start_core = !self.gui_config.auto_start_core;
                self.config_dirty = true;
                Task::none()
            }
            Message::ToggleAutoSysProxy => {
                self.gui_config.auto_sys_proxy = !self.gui_config.auto_sys_proxy;
                self.config_dirty = true;
                Task::none()
            }
            Message::SetLanguage(lang) => {
                self.gui_config.language = lang;
                self.config_dirty = true;
                self.update_tray_menu();
                Task::none()
            }
            Message::SetTheme(theme) => {
                self.gui_config.theme = theme;
                self.config_dirty = true;
                Task::none()
            }
            Message::OpenDataDir => {
                open_path_in_system(&config::get_app_dir());
                Task::none()
            }
            Message::OpenProfilesFolder => {
                open_path_in_system(&config::get_app_dir().join("profiles"));
                Task::none()
            }
            Message::EditProfile(id) => {
                let path = config::get_profile_path(&id);
                open_path_in_system(&path);
                Task::none()
            }
            Message::StartEditProfile(id) => {
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
                if let Some(id) = self.editing_profile_id.clone() {
                    if let Some(profile) = self
                        .gui_config
                        .subscriptions
                        .iter_mut()
                        .find(|p| p.id == id)
                    {
                        profile.name = self.editing_profile_name.clone();
                        profile.url = self.editing_profile_url.clone();
                        self.config_dirty = true;
                    }
                    self.editing_profile_id = None;
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
                if self.profile_more_id.as_deref() == Some(id.as_str()) {
                    self.profile_more_id = None;
                } else {
                    self.profile_more_id = Some(id);
                }
                Task::none()
            }
            Message::ToggleCloseCoreOnExit => {
                self.gui_config.close_core_on_exit = !self.gui_config.close_core_on_exit;
                self.config_dirty = true;
                Task::none()
            }
            Message::ToggleFakeIp => {
                self.gui_config.fake_ip = !self.gui_config.fake_ip;
                self.config_dirty = true;
                self.restart_core()
            }
            Message::ToggleTcpFastOpen => {
                self.gui_config.tcp_fast_open = !self.gui_config.tcp_fast_open;
                self.config_dirty = true;
                self.restart_core()
            }
            Message::ToggleTcpMultipath => {
                self.gui_config.tcp_multipath = !self.gui_config.tcp_multipath;
                self.config_dirty = true;
                self.restart_core()
            }
            Message::SaveSettings => {
                if self.mixed_port_input_str.trim().is_empty()
                    || self.api_port_input_str.trim().is_empty()
                {
                    let err = self.tr("port_empty_error").to_string();
                    self.log_lines.push_back(format!("[GUI ERROR] {}", err));
                    self.core_install_msg = Some(err);
                    return Task::none();
                }

                let (mixed_p, api_p) = match (
                    self.mixed_port_input_str.trim().parse::<u16>(),
                    self.api_port_input_str.trim().parse::<u16>(),
                ) {
                    (Ok(mixed), Ok(api)) => (mixed, api),
                    _ => {
                        let err = self.tr("port_invalid_error").to_string();
                        self.log_lines.push_back(format!("[GUI ERROR] {}", err));
                        self.core_install_msg = Some(err);
                        return Task::none();
                    }
                };

                // Reject reserved (0..1024) and identical mixed/api ports —
                // the latter would collide on 127.0.0.1:port and FATAL the core.
                let reserved_msg = self.tr("port_reserved_error").to_string();
                if mixed_p < 1024 || api_p < 1024 {
                    self.log_lines
                        .push_back(format!("[GUI ERROR] {}", reserved_msg));
                    self.core_install_msg = Some(reserved_msg);
                    return Task::none();
                }
                if mixed_p == api_p {
                    let err = self.tr("port_conflict_error").to_string();
                    self.log_lines.push_back(format!("[GUI ERROR] {}", err));
                    self.core_install_msg = Some(err);
                    return Task::none();
                }

                let dns_local = self.dns_server_local_input_str.trim();
                let dns_remote = self.dns_server_remote_input_str.trim();
                if !is_valid_dns_server_address(dns_local)
                    || !is_valid_dns_server_address(dns_remote)
                {
                    let err = self.tr("invalid_dns_server").to_string();
                    self.log_lines.push_back(format!("[GUI ERROR] {err}"));
                    self.core_install_msg = Some(err.clone());
                    self.toast_error(err);
                    return Task::none();
                }

                self.core_install_msg = None;
                self.gui_config.mixed_port = mixed_p;
                self.gui_config.api_port = api_p;
                self.gui_config.dns_server_local = dns_local.to_string();
                self.gui_config.dns_server_remote = dns_remote.to_string();
                let trimmed_core = self.core_path_input_str.trim();
                if trimmed_core.is_empty() {
                    self.gui_config.core_path = None;
                } else {
                    self.gui_config.core_path = Some(trimmed_core.to_string());
                }
                self.core_installed = core::is_core_installed(&self.gui_config);
                self.config_dirty = true;
                self.log_lines
                    .push_back("[GUI] Settings saved and applied successfully.".to_string());
                self.toast_success(self.tr("settings_saved_toast"));
                self.restart_core()
            }
            Message::CheckUpdate => {
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
            Message::DownloadAppUpdate { tag, url } => {
                self.update_status = state::UpdateStatus::Downloading { tag: tag.clone() };
                let url_for_msg = url.clone();
                Task::perform(download_app_update_binary(url), move |result| {
                    Message::AppUpdateDownloaded {
                        tag,
                        url: url_for_msg,
                        result,
                    }
                })
            }
            Message::AppUpdateDownloaded { tag, url, result } => {
                match result {
                    Ok(path) => {
                        self.log_lines.push_back(format!(
                            "[GUI] Update {} downloaded to {}",
                            tag,
                            path.display()
                        ));
                        self.toast_info(self.tr("toast_update_installing"));
                        // Stop core and clear proxy before replacing the binary.
                        self.force_stop_after_start = false;
                        self.pending_core_restart = false;
                        self.pending_update_path = Some(path);
                        self.task_stop_core()
                    }
                    Err(e) => {
                        self.log_lines
                            .push_back(format!("[GUI] Update download failed: {}", e));
                        // Keep download URL so the user can retry in-app.
                        self.update_status = state::UpdateStatus::NewVersion {
                            tag,
                            download_url: Some(url),
                        };
                        self.toast_error(e);
                        Task::none()
                    }
                }
            }
            Message::OpenUrl(url) => {
                // `open::that` delegates to ShellExecuteW / open / xdg-open
                // safely, avoiding cmd-shell injection on user-controlled URLs.
                let _ = open::that(&url);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
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
        let mixed_port_input_str_ref = &self.mixed_port_input_str;
        let api_port_input_str_ref = &self.api_port_input_str;
        let dns_server_local_input_str_ref = &self.dns_server_local_input_str;
        let dns_server_remote_input_str_ref = &self.dns_server_remote_input_str;
        let core_install_msg_ref = self.core_install_msg.as_deref();
        let update_status_ref = &self.update_status;
        let editing_profile_id_ref = self.editing_profile_id.as_deref();
        let editing_profile_name_ref = &self.editing_profile_name;
        let editing_profile_url_ref = &self.editing_profile_url;
        let core_path_input_str_ref = &self.core_path_input_str;
        let toast_ref = self.toast.as_ref();
        let log_filter = self.log_filter;
        let log_search_ref = &self.log_search;
        let core_version_ref = self.core_version.as_deref();
        let active_connections_count = self.active_connections.len();
        let selected_node_for_dash = self.selected_node_tag.as_deref();
        let profile_more_id_ref = self.profile_more_id.as_deref();
        let config_preview_expanded = self.config_preview_expanded;

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
                    theme,
                ),
                Tab::Settings => ui::settings::render(
                    gui_config_ref,
                    mixed_port_input_str_ref,
                    api_port_input_str_ref,
                    dns_server_local_input_str_ref,
                    dns_server_remote_input_str_ref,
                    core_path_input_str_ref,
                    core_installed,
                    core_install_msg_ref,
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

                    let indicator: Element<'_, Message> = container(iced::widget::Space::new())
                        .width(4)
                        .height(20)
                        .style(move |_theme| container::Style {
                            background: if active {
                                Some(iced::Background::Color(ui::theme::ACCENT_PURPLE))
                            } else {
                                None
                            },
                            border: iced::Border {
                                radius: 2.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .into();

                    let btn_content: Element<'_, Message> = if is_compact {
                        row![
                            indicator,
                            container(
                                text(icon_char.to_string())
                                    .font(Font::with_name("Material Icons"))
                                    .size(ui::ICON_SIZE_LG)
                            )
                            .width(Length::Fill)
                            .align_x(Alignment::Center)
                        ]
                        .align_y(Alignment::Center)
                        .into()
                    } else {
                        row![
                            indicator,
                            text(icon_char.to_string())
                                .font(Font::with_name("Material Icons"))
                                .size(ui::ICON_SIZE),
                            text(ui::i18n::tr(lang, key))
                                .size(ui::theme::TYPE_BODY)
                                .font(ui::theme::ui_font(if active {
                                    iced::font::Weight::Bold
                                } else {
                                    iced::font::Weight::Medium
                                }))
                        ]
                        .spacing(ui::SP_12)
                        .align_y(Alignment::Center)
                        .into()
                    };

                    let btn = button(btn_content)
                        .padding(if is_compact { [14, 0] } else { [14, 16] })
                        .width(Length::Fill)
                        .style(ui::theme::button_tab(active))
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
                            .style(ui::theme::card_bg),
                            tooltip::Position::Right,
                        )
                        .into()
                    } else {
                        btn.into()
                    }
                };

            let logo_handle = iced::widget::image::Handle::from_bytes(
                include_bytes!("../assets/app-icon.png").as_slice(),
            );
            let status_dot_color = if core_running {
                ui::theme::SUCCESS
            } else {
                // Neutral resting state — not an error
                ui::theme::text_muted(theme)
            };

            let logo_rounded = |handle: iced::widget::image::Handle, size: f32| {
                container(
                    iced::widget::image(handle)
                        .width(size)
                        .height(size)
                        .content_fit(iced::ContentFit::Cover),
                )
                .width(size)
                .height(size)
                .clip(true)
                .style(|_t| container::Style {
                    border: iced::Border {
                        radius: ui::theme::RADIUS_SM.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
            };

            let sidebar = if is_compact {
                container(
                    column![
                        column![
                            container(logo_rounded(logo_handle.clone(), 36.0))
                                .width(Length::Fill)
                                .center_x(Length::Fill),
                            container(iced::widget::Space::new())
                                .height(1)
                                .width(Length::Fill)
                                .style(|t| container::Style {
                                    background: Some(iced::Background::Color(
                                        ui::theme::border_color(t)
                                    )),
                                    ..Default::default()
                                }),
                            column![
                                make_tab_btn(Tab::Dashboard, '\u{E871}', "tab_dashboard"),
                                make_tab_btn(Tab::Proxies, '\u{EA0B}', "tab_proxies"),
                                make_tab_btn(Tab::Profiles, '\u{E2C7}', "tab_profiles"),
                                make_tab_btn(Tab::Rules, '\u{E41E}', "tab_rules"),
                                make_tab_btn(Tab::Connections, '\u{E894}', "tab_connections"),
                                make_tab_btn(Tab::Logs, '\u{E85D}', "tab_logs"),
                                make_tab_btn(Tab::Settings, '\u{E8B8}', "tab_settings"),
                            ]
                            .spacing(6)
                            .width(Length::Fill)
                        ]
                        .spacing(16)
                        .width(Length::Fill),
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
                .padding([24, 0])
                .style(ui::theme::sidebar_bg)
            } else {
                container(column![
                    column![
                        row![
                            logo_rounded(logo_handle, 32.0),
                            column![
                                text("sing-box")
                                    .size(18)
                                    .font(ui::theme::ui_font(iced::font::Weight::Semibold))
                                    .color(ui::theme::text_primary(theme)),
                                text("GUI")
                                    .size(ui::theme::TYPE_CAPTION)
                                    .font(ui::theme::ui_font(iced::font::Weight::Medium))
                                    .color(ui::theme::text_tertiary(theme)),
                            ]
                            .spacing(0)
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                        container(iced::widget::Space::new())
                            .height(1)
                            .width(Length::Fill)
                            .style(|t| container::Style {
                                background: Some(iced::Background::Color(ui::theme::border_color(
                                    t
                                ))),
                                ..Default::default()
                            }),
                        column![
                            make_tab_btn(Tab::Dashboard, '\u{E871}', "tab_dashboard"),
                            make_tab_btn(Tab::Proxies, '\u{EA0B}', "tab_proxies"),
                            make_tab_btn(Tab::Profiles, '\u{E2C7}', "tab_profiles"),
                            make_tab_btn(Tab::Rules, '\u{E41E}', "tab_rules"),
                            make_tab_btn(Tab::Connections, '\u{E894}', "tab_connections"),
                            make_tab_btn(Tab::Logs, '\u{E85D}', "tab_logs"),
                            make_tab_btn(Tab::Settings, '\u{E8B8}', "tab_settings"),
                        ]
                        .spacing(6)
                        .width(Length::Fill)
                    ]
                    .spacing(16)
                    .width(Length::Fill),
                    iced::widget::Space::new().height(Length::Fill),
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
                                ui::theme::SUCCESS
                            } else {
                                text_muted
                            },
                            ui::theme::TYPE_CAPTION
                        ),
                        iced::widget::Space::new().width(Length::Fill),
                        text(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .size(ui::theme::TYPE_CAPTION)
                            .color(text_muted)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                ])
                .width(Length::Fixed(224.0))
                .height(Length::Fill)
                .padding(20)
                .style(ui::theme::sidebar_bg)
            };

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

            row![sidebar, main_layout]
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
        });

        main_content.into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![
            iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick),
            iced::window::close_requests().map(Message::WindowCloseRequested),
            iced::window::events().filter_map(|(id, event)| {
                if let iced::window::Event::Opened { .. } = event {
                    Some(Message::WindowOpened(id))
                } else {
                    None
                }
            }),
        ];

        // Live streams for logs, traffic stats, and tray events
        subs.push(Subscription::run(log_subscription));
        subs.push(Subscription::run(traffic_subscription));
        subs.push(Subscription::run(tray_subscription));

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
                    let _ = output.send(Message::NewLogLine(line)).await;
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

// Subscription worker for streaming real-time system tray menu and clicks
fn tray_subscription() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(
        100,
        |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
            let (tx, mut rx) = tokio::sync::mpsc::channel(100);
            // Shared stop flag — set when the iced side drops, lets the two
            // blocking `recv()` loops below exit instead of leaking threads.
            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let stop_menu = stop.clone();
            let stop_tray = stop.clone();

            let tx_clone = tx.clone();
            std::thread::spawn(move || {
                let menu_channel = tray_icon::menu::MenuEvent::receiver();
                while !stop_menu.load(std::sync::atomic::Ordering::SeqCst) {
                    // Non-blocking poll with a short sleep so the stop flag is honored.
                    while let Ok(event) = menu_channel.try_recv() {
                        // Drop on closed channel = iced subscription ended — stop polling.
                        if tx_clone
                            .blocking_send(Message::TrayMenuClicked(event.id.0))
                            .is_err()
                        {
                            stop_menu.store(true, std::sync::atomic::Ordering::SeqCst);
                            return;
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            });

            let tx_clone2 = tx.clone();
            std::thread::spawn(move || {
                let tray_channel = tray_icon::TrayIconEvent::receiver();
                while !stop_tray.load(std::sync::atomic::Ordering::SeqCst) {
                    while let Ok(event) = tray_channel.try_recv() {
                        let to_send = match event {
                            tray_icon::TrayIconEvent::Click {
                                button: tray_icon::MouseButton::Left,
                                ..
                            } => Some(Message::TrayIconClicked),
                            tray_icon::TrayIconEvent::DoubleClick {
                                button: tray_icon::MouseButton::Left,
                                ..
                            } => Some(Message::TrayIconClicked),
                            _ => None,
                        };
                        if let Some(msg) = to_send
                            && tx_clone2.blocking_send(msg).is_err()
                        {
                            stop_tray.store(true, std::sync::atomic::Ordering::SeqCst);
                            return;
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            });

            while let Some(msg) = rx.recv().await {
                // iced sender closed (subscription dropped); signal workers to exit.
                if output.send(msg).await.is_err() {
                    stop.store(true, std::sync::atomic::Ordering::SeqCst);
                    break;
                }
            }
            // The async task returns — and the two std threads observe the stop flag
            // next time they check, normally within 50ms.
            stop.store(true, std::sync::atomic::Ordering::SeqCst);
        },
    )
}

fn load_icon_safe() -> Option<tray_icon::Icon> {
    let icon_bytes = include_bytes!("../assets/app-icon.png");
    match image::load_from_memory(icon_bytes) {
        Ok(img) => {
            let rgba_img = img
                .resize(32, 32, image::imageops::FilterType::Lanczos3)
                .into_rgba8();
            let (width, height) = rgba_img.dimensions();
            let rgba = rgba_img.into_raw();
            tray_icon::Icon::from_rgba(rgba, width, height).ok()
        }
        Err(_) => {
            // Fallback transparent 16x16 icon
            let rgba = vec![0; 16 * 16 * 4];
            tray_icon::Icon::from_rgba(rgba, 16, 16).ok()
        }
    }
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
    let write_path = path.clone();
    tokio::task::spawn_blocking(move || config::atomic_write(&write_path, content.as_bytes()))
        .await
        .map_err(|e| format!("Profile save task failed: {e}"))??;

    Ok(ProfileFetchResult {
        id,
        source_url: url,
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
    let mut meta = ProfileContentMeta {
        display_name: None,
        traffic_upload: None,
        traffic_download: None,
        traffic_total: None,
        expire_at: None,
    };

    if std::path::Path::new(url).exists() {
        let bytes = tokio::fs::read(url)
            .await
            .map_err(|e| format!("Failed to read local file: {}", e))?;
        if bytes.len() > MAX_PROFILE_BYTES {
            return Err("Profile is larger than the 16 MiB safety limit".to_string());
        }
        let content = decode_profile_bytes(&bytes)?;
        return Ok((content, meta));
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

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use state::{GuiConfig, Profile};

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
            },
            GithubAsset {
                name: "sing-box-gui-v2026.7.11-linux-amd64".into(),
                browser_download_url: "https://example.com/linux".into(),
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

fn detect_system_theme() -> bool {
    #[cfg(target_os = "windows")]
    {
        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;
        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize")
            && let Ok(val) = hkcu.get_value::<u32, _>("AppsUseLightTheme")
        {
            return val == 1; // 1 = Light Mode, 0 = Dark Mode
        }
    }
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("defaults")
            .args(&["read", "-g", "AppleInterfaceStyle"])
            .output()
        {
            let style = String::from_utf8_lossy(&output.stdout);
            return !style.trim().contains("Dark");
        }
    }
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("gsettings")
            .args(&["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
        {
            let scheme = String::from_utf8_lossy(&output.stdout);
            return !scheme.trim().contains("dark");
        }
    }
    false // Default to dark mode (0)
}

/// Cheap elevation check so TUN mode can warn the user *before* the core
/// FATALs on permission denied / missing wintun privileges.
fn open_path_in_system(path: &std::path::Path) {
    // Use the `open` crate rather than shelling out to `cmd /c start`, which
    // mishandles paths containing spaces or shell metacharacters like `&`.
    let _ = open::that(path);
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

fn pick_release_asset_url(assets: &[GithubAsset], suffix: &str) -> Option<String> {
    // Prefer exact suffix match (CI naming), then fallback to contains.
    assets
        .iter()
        .find(|a| a.name.ends_with(suffix))
        .or_else(|| assets.iter().find(|a| a.name.contains(suffix)))
        .map(|a| a.browser_download_url.clone())
}

#[derive(Debug, Clone, serde::Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    assets: Vec<GithubAsset>,
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

    let download_url = pick_release_asset_url(&release.assets, platform_asset_suffix());
    Ok(message::AppUpdateInfo {
        tag_name: release.tag_name,
        download_url,
    })
}

/// Download the release binary to a temp path next to the current executable.
async fn download_app_update_binary(url: String) -> Result<std::path::PathBuf, String> {
    use tokio::io::AsyncWriteExt;
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

    const MAX_UPDATE_BYTES: usize = 128 * 1024 * 1024;
    if res
        .content_length()
        .is_some_and(|size| size > MAX_UPDATE_BYTES as u64)
    {
        return Err("Update package exceeds the 128 MiB safety limit".to_string());
    }

    let current = std::env::current_exe()
        .map_err(|e| format!("Failed to resolve current executable: {}", e))?;
    let dir = current
        .parent()
        .ok_or_else(|| "Current executable has no parent directory".to_string())?;

    #[cfg(target_os = "windows")]
    let file_name = "sing-box-gui.update.exe";
    #[cfg(not(target_os = "windows"))]
    let file_name = "sing-box-gui.update.bin";

    // Prefer beside the running binary (portable installs); fall back to temp.
    let beside = dir.join(file_name);
    let (dest, mut file) = match tokio::fs::File::create(&beside).await {
        Ok(file) => (beside, file),
        Err(e_beside) => {
            let fallback = std::env::temp_dir().join(file_name);
            let file = tokio::fs::File::create(&fallback).await.map_err(|e| {
                format!("Failed to create update file (beside exe: {e_beside}; temp: {e})")
            })?;
            (fallback, file)
        }
    };

    let mut stream = res.bytes_stream();
    let mut downloaded = 0usize;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Failed to read download body: {e}"))?;
        downloaded = downloaded.saturating_add(chunk.len());
        if downloaded > MAX_UPDATE_BYTES {
            drop(file);
            let _ = tokio::fs::remove_file(&dest).await;
            return Err("Update package exceeds the 128 MiB safety limit".to_string());
        }
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write update file: {e}"))?;
    }
    file.flush()
        .await
        .map_err(|e| format!("Failed to flush update file: {e}"))?;
    drop(file);

    if downloaded < 1024 {
        let _ = tokio::fs::remove_file(&dest).await;
        return Err(format!(
            "Downloaded file too small ({downloaded} bytes) — likely not a binary"
        ));
    }

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
        let script_path = current.with_extension("update.cmd");
        let current_s = current.to_string_lossy().replace('"', "");
        let new_s = new_binary.to_string_lossy().replace('"', "");
        let bak = current.with_extension("exe.bak");
        let bak_s = bak.to_string_lossy().replace('"', "");

        // Wait for this PID to exit, swap binaries, relaunch, clean up.
        let script = format!(
            r#"@echo off
setlocal
set "TARGET={current}"
set "NEW={new}"
set "BAK={bak}"
set "PID={pid}"
:wait
tasklist /FI "PID eq %PID%" /NH /FO CSV 2>NUL | findstr /B /I "%PID%," >NUL
if not errorlevel 1 (
  timeout /t 1 /nobreak >NUL
  goto wait
)
if exist "%BAK%" del /F /Q "%BAK%" >NUL 2>&1
move /Y "%TARGET%" "%BAK%" >NUL 2>&1
move /Y "%NEW%" "%TARGET%"
if errorlevel 1 (
  if exist "%BAK%" move /Y "%BAK%" "%TARGET%" >NUL 2>&1
  del /F /Q "%~f0" >NUL 2>&1
  exit /b 1
)
start "" "%TARGET%"
if exist "%BAK%" del /F /Q "%BAK%" >NUL 2>&1
del /F /Q "%~f0" >NUL 2>&1
"#,
            current = current_s,
            new = new_s,
            bak = bak_s,
            pid = pid,
        );
        std::fs::write(&script_path, script)
            .map_err(|e| format!("Failed to write update script: {}", e))?;

        std::process::Command::new("cmd")
            .args(["/C", &script_path.to_string_lossy()])
            // DETACHED_PROCESS (0x8) | CREATE_NO_WINDOW (0x08000000)
            .creation_flags(0x08000008)
            .spawn()
            .map_err(|e| format!("Failed to spawn update script: {}", e))?;
        Ok(())
    }

    #[cfg(unix)]
    {
        let script_path = current.with_extension("update.sh");
        let current_s = current.to_string_lossy();
        let new_s = new_binary.to_string_lossy();
        let script = format!(
            r#"#!/bin/sh
TARGET="{current}"
NEW="{new}"
PID={pid}
cleanup() {{
  rm -f -- "$0"
}}
trap cleanup EXIT
while kill -0 "$PID" 2>/dev/null; do sleep 1; done
mv -f "$NEW" "$TARGET" || exit 1
chmod +x "$TARGET"
exec "$TARGET"
"#,
            current = current_s.replace('"', "\\\""),
            new = new_s.replace('"', "\\\""),
            pid = pid,
        );
        std::fs::write(&script_path, &script)
            .map_err(|e| format!("Failed to write update script: {}", e))?;
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&script_path)
            .map_err(|e| format!("Failed to stat update script: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms)
            .map_err(|e| format!("Failed to chmod update script: {}", e))?;

        std::process::Command::new("sh")
            .arg(&script_path)
            .spawn()
            .map_err(|e| format!("Failed to spawn update script: {}", e))?;
        // Best-effort: remove script after a delay is handled by not needing self-delete on unix
        // (script is overwritten next time).
        return Ok(());
    }

    #[cfg(not(any(target_os = "windows", unix)))]
    {
        let _ = (new_binary, pid, current);
        Err("In-app update is not supported on this platform".to_string())
    }
}

/// Normalize a version-like string into comparable dot-separated numeric tokens
/// so we can compare `v2026.7.9` vs `2026.7.9` etc. without false positives.
/// Returns true when `remote_tag` is *strictly* newer than `local_pkg_version`
/// using dotted numeric comparison. Falls back to string inequality when
/// neither side parses at all.
fn main() -> iced::Result {
    let icon_bytes = include_bytes!("../assets/app-icon.png");
    let icon = iced::window::icon::from_file_data(icon_bytes, None).ok();

    let window_settings = iced::window::Settings {
        size: iced::Size::new(1080.0, 750.0),
        // Low enough that SHELL_COMPACT_W icon sidebar is reachable
        min_size: Some(iced::Size::new(ui::WINDOW_MIN_W, ui::WINDOW_MIN_H)),
        icon,
        exit_on_close_request: false,
        ..Default::default()
    };

    // Bundle a consistent CJK-capable UI face. Relying on the platform's
    // generic SansSerif fallback caused Chinese text and symbols to render as
    // tofu boxes on some Windows installations.
    let default_font = iced::Font::with_name(ui::theme::UI_FONT_NAME);

    let res = iced::application(App::new, App::update, App::view)
        .title("sing-box GUI")
        .window(window_settings)
        .theme(App::theme)
        .default_font(default_font)
        .font(include_bytes!("../assets/NotoSansCJK-Regular.ttc").as_slice())
        .font(include_bytes!("../assets/material-icons.ttf").as_slice())
        .subscription(App::subscription)
        .run();

    // CRITICAL EXIT CLEANUP
    let config = config::load_gui_config();
    if config.close_core_on_exit {
        core::stop_core();
        if sysproxy::is_system_proxy_owned() {
            let _ = sysproxy::set_system_proxy(false, config.mixed_port);
        }
    }

    res
}
