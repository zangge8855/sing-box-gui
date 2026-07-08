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
#![windows_subsystem = "windows"]

mod state;
mod message;
mod config;
mod core;
mod api;
mod sysproxy;
mod ui;

use std::sync::{Mutex, OnceLock};
use tokio::sync::mpsc;
use iced::{Alignment, Element, Length, Subscription, Task, Font};
use iced::widget::{button, column, container, row, text, responsive};
use state::{Bandwidth, GuiConfig, Profile, ProxyNode, Tab};
use message::Message;
use futures::SinkExt;

// OnceLocks for streaming logs and traffic stats asynchronously
static LOG_RX: OnceLock<Mutex<Option<mpsc::UnboundedReceiver<String>>>> = OnceLock::new();
static LOG_TX: OnceLock<mpsc::UnboundedSender<String>> = OnceLock::new();

static TRAFFIC_RX: OnceLock<Mutex<Option<mpsc::UnboundedReceiver<api::TrafficInfo>>>> = OnceLock::new();
static TRAFFIC_TX: OnceLock<mpsc::UnboundedSender<api::TrafficInfo>> = OnceLock::new();

pub fn get_log_tx() -> mpsc::UnboundedSender<String> {
    LOG_TX.get_or_init(|| {
        let (tx, rx) = mpsc::unbounded_channel();
        let _ = LOG_RX.set(Mutex::new(Some(rx)));
        tx
    }).clone()
}

pub fn get_traffic_tx() -> mpsc::UnboundedSender<api::TrafficInfo> {
    TRAFFIC_TX.get_or_init(|| {
        let (tx, rx) = mpsc::unbounded_channel();
        let _ = TRAFFIC_RX.set(Mutex::new(Some(rx)));
        tx
    }).clone()
}

struct App {
    current_tab: Tab,
    gui_config: GuiConfig,
    core_running: bool,
    sys_proxy_enabled: bool,
    log_lines: Vec<String>,
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
}

impl App {
    fn theme(&self) -> iced::Theme {
        match self.gui_config.theme {
            state::AppTheme::Dark => iced::Theme::Dark,
            state::AppTheme::Light => iced::Theme::Light,
            state::AppTheme::Auto => {
                if detect_system_theme() {
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
                if detect_system_theme() {
                    &LIGHT
                } else {
                    &DARK
                }
            }
        }
    }

    fn new() -> (Self, Task<Message>) {
        let gui_config = config::load_gui_config();
        let core_installed = core::is_core_installed(&gui_config);
        let selected_node_tag = gui_config.selected_node_tag.clone();
        
        let mixed_port_input_str = gui_config.mixed_port.to_string();
        let api_port_input_str = gui_config.api_port.to_string();
        let dns_server_local_input_str = gui_config.dns_server_local.clone();
        let dns_server_remote_input_str = gui_config.dns_server_remote.clone();
        
        use tray_icon::{
            menu::{Menu, MenuItem, CheckMenuItem, PredefinedMenuItem},
            TrayIconBuilder,
        };

        // Create the tray menu
        let tray_menu = Menu::new();
        let show_item = MenuItem::with_id("show_window", "显示主界面 (Show Window)", true, None);
        let toggle_core_item = MenuItem::with_id("toggle_core", "启动内核 (Start Core)", true, None);
        
        let rule_mode_item = CheckMenuItem::with_id("mode_rule", "规则分流 (Rules)", true, false, None);
        let global_mode_item = CheckMenuItem::with_id("mode_global", "全局代理 (Global)", true, false, None);
        let direct_mode_item = CheckMenuItem::with_id("mode_direct", "直接连接 (Direct)", true, false, None);
        
        let system_proxy_item = CheckMenuItem::with_id("toggle_system_proxy", "系统代理 (System Proxy)", true, gui_config.system_proxy_enabled, None);
        
        let mode_submenu = tray_icon::menu::Submenu::new("代理模式 (Proxy Mode)", true);
        let _ = mode_submenu.append(&rule_mode_item);
        let _ = mode_submenu.append(&global_mode_item);
        let _ = mode_submenu.append(&direct_mode_item);
        
        let exit_item = MenuItem::with_id("exit_app", "退出 (Exit)", true, None);
        
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

        let mut app = Self {
            current_tab: Tab::Dashboard,
            gui_config,
            core_running: false,
            sys_proxy_enabled: false,
            log_lines: Vec::new(),
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
            last_routing_mode: state::RoutingMode::Rule,
        };
        
        // Force initialization of log and traffic streams on startup
        let _ = get_log_tx();
        let _ = get_traffic_tx();
        
        // Load active profile nodes if profile exists
        app.reload_active_nodes();
        
        // Sync system proxy checkbox status with system state
        let sys_proxy = sysproxy::check_system_proxy(app.gui_config.mixed_port).unwrap_or(false);
        app.sys_proxy_enabled = sys_proxy;
        
        // Check if core is running on start
        app.core_running = core::is_core_running();
        
        // Initial tray menu synchronization
        app.update_tray_menu();
        
        (app, Task::none())
    }
    
    fn reload_active_nodes(&mut self) {
        if let Some(ref active_id) = self.gui_config.active_profile_id {
            let path = config::get_profile_path(active_id);
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(path) {
                    let trimmed = content.trim();
                    let nodes = if trimmed.starts_with('{') || trimmed.starts_with('[') {
                        config::parse_native_json_nodes(&content).unwrap_or_default()
                    } else {
                        config::parse_clash_yaml_nodes(&content).unwrap_or_default()
                    };
                    self.active_profile_nodes = nodes;
                }
            }
        }
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

        self.tray_menu_rule_mode.set_checked(self.gui_config.routing_mode == state::RoutingMode::Rule);
        self.tray_menu_global_mode.set_checked(self.gui_config.routing_mode == state::RoutingMode::Global);
        self.tray_menu_direct_mode.set_checked(self.gui_config.routing_mode == state::RoutingMode::Direct);
        self.tray_menu_system_proxy.set_checked(self.gui_config.system_proxy_enabled);
    }
    
    fn restart_core(&mut self) -> Task<Message> {
        if self.core_running {
            core::stop_core();
            self.core_running = false;
            self.log_lines.push("[GUI] Restarting core to apply new settings...".to_string());
            Task::done(Message::ToggleCore)
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
            Message::TrayMenuClicked(id_str) => {
                match id_str.as_str() {
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
                    "toggle_core" => {
                        Task::done(Message::ToggleCore)
                    }
                    "toggle_system_proxy" => {
                        Task::done(Message::ToggleSystemProxy)
                    }
                    "mode_rule" => {
                        Task::done(Message::RoutingModeChanged(state::RoutingMode::Rule))
                    }
                    "mode_global" => {
                        Task::done(Message::RoutingModeChanged(state::RoutingMode::Global))
                    }
                    "mode_direct" => {
                        Task::done(Message::RoutingModeChanged(state::RoutingMode::Direct))
                    }
                    "exit_app" => {
                        if self.gui_config.close_core_on_exit {
                            core::stop_core();
                        }
                        let _ = sysproxy::set_system_proxy(false, self.gui_config.mixed_port);
                        iced::exit()
                    }
                    _ => Task::none(),
                }
            }
            Message::WindowOpened(id) => {
                self.window_id = Some(id);
                Task::none()
            }
            Message::WindowCloseRequested(id) => {
                self.window_id = Some(id);
                iced::window::set_mode(id, iced::window::Mode::Hidden)
            }
            Message::TabChanged(tab) => {
                self.current_tab = tab;
                Task::none()
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
                
                Task::perform(async move {
                    api::select_proxy(api_port, &group_clone, &node_clone).await
                }, move |res| {
                    Message::GroupNodeSelected {
                        group: group.clone(),
                        node: node.clone(),
                        error: res.err(),
                    }
                })
            }
            Message::GroupNodeSelected { group, node, error } => {
                if let Some(err) = error {
                    self.log_lines.push(format!("[GUI] Failed to select node {} for group {}: {}", node, group, err));
                } else {
                    if let Some(g_info) = self.proxy_groups.get_mut(&group) {
                        g_info.now = Some(node.clone());
                    }
                    if group == "Proxy" {
                        self.selected_node_tag = Some(node.clone());
                        self.gui_config.selected_node_tag = Some(node.clone());
                        let _ = config::save_gui_config(&self.gui_config);
                    }
                    self.log_lines.push(format!("[GUI] Selected node: {} for group {}", node, group));
                }
                Task::none()
            }
            Message::RulesInputChanged { field, value } => {
                match field.as_str() {
                    "bypass_domains" => self.bypass_domain_input = value,
                    "proxy_domains" => self.proxy_domain_input = value,
                    "bypass_ips" => self.bypass_ip_input = value,
                    "proxy_ips" => self.proxy_ip_input = value,
                    _ => {}
                }
                Task::none()
            }
            Message::AddRule { field } => {
                let (val, list) = match field.as_str() {
                    "bypass_domains" => (&mut self.bypass_domain_input, &mut self.gui_config.custom_bypass_domains),
                    "proxy_domains" => (&mut self.proxy_domain_input, &mut self.gui_config.custom_proxy_domains),
                    "bypass_ips" => (&mut self.bypass_ip_input, &mut self.gui_config.custom_bypass_ips),
                    "proxy_ips" => (&mut self.proxy_ip_input, &mut self.gui_config.custom_proxy_ips),
                    _ => return Task::none(),
                };
                let trimmed = val.trim().to_string();
                if !trimmed.is_empty() && !list.contains(&trimmed) {
                    list.push(trimmed.clone());
                    val.clear();
                    let _ = config::save_gui_config(&self.gui_config);
                    self.log_lines.push(format!("[GUI] Added custom rule to {}: {}", field, trimmed));
                    return self.restart_core();
                }
                Task::none()
            }
            Message::RemoveRule { field, index } => {
                let list = match field.as_str() {
                    "bypass_domains" => &mut self.gui_config.custom_bypass_domains,
                    "proxy_domains" => &mut self.gui_config.custom_proxy_domains,
                    "bypass_ips" => &mut self.gui_config.custom_bypass_ips,
                    "proxy_ips" => &mut self.gui_config.custom_proxy_ips,
                    _ => return Task::none(),
                };
                if index < list.len() {
                    let removed = list.remove(index);
                    let _ = config::save_gui_config(&self.gui_config);
                    self.log_lines.push(format!("[GUI] Removed custom rule from {}: {}", field, removed));
                    return self.restart_core();
                }
                Task::none()
            }
            Message::ProxiesFetched(res) => {
                match res {
                    Ok(groups_map) => {
                        self.proxy_groups = groups_map;
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
                if self.core_running {
                    core::stop_core();
                    self.core_running = false;
                    
                    // Stop traffic monitor stream
                    if let Some(cancel_tx) = self.traffic_cancel_tx.take() {
                        let _ = cancel_tx.send(());
                    }
                    
                    // Turn off system proxy automatically
                    if self.sys_proxy_enabled {
                        let _ = sysproxy::set_system_proxy(false, self.gui_config.mixed_port);
                        self.sys_proxy_enabled = false;
                    }
                    self.log_lines.push("[GUI] sing-box core stopped.".to_string());
                    self.current_speed = Bandwidth::default();
                    self.total_uploaded = 0;
                    self.total_downloaded = 0;
                    Task::none()
                } else {
                    let log_tx = get_log_tx();
                    match core::start_core(&self.gui_config, log_tx) {
                        Ok(_) => {
                            self.core_running = true;
                            self.log_lines.push("[GUI] sing-box core started successfully.".to_string());
                            
                            // Stop existing traffic monitor if any
                            if let Some(cancel_tx) = self.traffic_cancel_tx.take() {
                                let _ = cancel_tx.send(());
                            }
                            
                            let (tx, rx) = tokio::sync::oneshot::channel();
                            self.traffic_cancel_tx = Some(tx);
                            
                            // Start traffic monitor stream
                            let traffic_tx = get_traffic_tx();
                            api::spawn_traffic_monitor(self.gui_config.api_port, traffic_tx, rx);
                            
                            // Trigger latency load
                            Task::perform(async move {
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                            }, |_| Message::Tick)
                        }
                        Err(e) => {
                            self.log_lines.push(format!("[GUI] Error starting core: {}", e));
                            Task::none()
                        }
                    }
                }
            }
            Message::CoreStatusChanged(status) => {
                self.core_running = status;
                Task::none()
            }
            Message::NewLogLine(line) => {
                self.log_lines.push(line);
                if self.log_lines.len() > 1000 {
                    self.log_lines.drain(0..100); // Amortized shifting cost optimization
                }
                
                // Automatically scroll logs scrollable to bottom on new logs
                iced::widget::operation::snap_to(
                    ui::logs::get_logs_scrollable_id().clone(),
                    iced::widget::scrollable::RelativeOffset::END
                )
            }
            Message::ClearLogs => {
                self.log_lines.clear();
                Task::none()
            }
            Message::TriggerCoreDownload => {
                let log_tx = get_log_tx();
                self.log_lines.push("[GUI] Starting sing-box core download...".to_string());
                self.core_install_msg = Some("Downloading...".to_string());
                Task::perform(async move {
                    core::download_core(log_tx).await
                }, Message::CoreDownloaded)
            }
            Message::CoreDownloaded(res) => {
                match res {
                    Ok(_) => {
                        self.core_installed = true;
                        self.core_install_msg = None;
                        self.log_lines.push("[GUI] sing-box core downloaded and installed successfully.".to_string());
                    }
                    Err(e) => {
                        self.core_install_msg = Some(e.clone());
                        self.log_lines.push(format!("[GUI ERROR] Failed to download core: {}", e));
                    }
                }
                Task::none()
            }
            Message::TrafficUpdated { up, down } => {
                self.current_speed = Bandwidth { up, down };
                self.speed_history.push((up, down));
                if self.speed_history.len() > 30 {
                    self.speed_history.remove(0);
                }
                Task::none()
            }
            Message::ToggleSystemProxy => {
                let target = !self.sys_proxy_enabled;
                match sysproxy::set_system_proxy(target, self.gui_config.mixed_port) {
                    Ok(_) => {
                        self.sys_proxy_enabled = target;
                        self.gui_config.system_proxy_enabled = target;
                        let _ = config::save_gui_config(&self.gui_config);
                        self.log_lines.push(format!("[GUI] System proxy toggled to: {}", target));
                    }
                    Err(e) => {
                        self.log_lines.push(format!("[GUI] System proxy error: {}", e));
                    }
                }
                Task::none()
            }
            Message::SystemProxyStatusChanged(status) => {
                self.sys_proxy_enabled = status;
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
                
                Task::perform(download_profile(url), |res| {
                    match res {
                        Ok((id, _name)) => Message::SubscriptionDownloaded { id, error: None },
                        Err(e) => Message::SubscriptionDownloaded { id: String::new(), error: Some(e) },
                    }
                })
            }
            Message::SubscriptionDownloaded { id, error } => {
                self.downloading = false;
                if let Some(err) = error {
                    self.profile_error = Some(err.clone());
                    self.log_lines.push(format!("[GUI] Download failed: {}", err));
                } else {
                    self.profile_error = None;
                    self.url_input.clear();
                    
                    // Reload profiles list safely without overwriting other in-memory settings
                    let loaded = config::load_gui_config();
                    self.gui_config.subscriptions = loaded.subscriptions;
                    self.sync_input_buffers();
                    
                    // If no active profile, select this one
                    if self.gui_config.active_profile_id.is_none() {
                        self.gui_config.active_profile_id = Some(id.clone());
                        let _ = config::save_gui_config(&self.gui_config);
                        self.reload_active_nodes();
                    }
                    self.log_lines.push("[GUI] Subscription downloaded successfully.".to_string());
                }
                Task::none()
            }
            Message::SelectProfile(id) => {
                self.confirm_delete_profile_id = None;
                self.gui_config.active_profile_id = Some(id);
                let _ = config::save_gui_config(&self.gui_config);
                self.reload_active_nodes();
                self.log_lines.push("[GUI] Active profile updated.".to_string());
                return self.restart_core();
            }
            Message::DeleteProfile(id) => {
                if id.starts_with("confirm:") {
                    let real_id = id.trim_start_matches("confirm:").to_string();
                    self.confirm_delete_profile_id = Some(real_id);
                    Task::none()
                } else {
                    self.confirm_delete_profile_id = None;
                    let path = config::get_profile_path(&id);
                    let _ = std::fs::remove_file(path);
                    
                    self.gui_config.subscriptions.retain(|p| p.id != id);
                    if self.gui_config.active_profile_id.as_ref() == Some(&id) {
                        self.gui_config.active_profile_id = None;
                        self.active_profile_nodes.clear();
                    }
                    let _ = config::save_gui_config(&self.gui_config);
                    self.log_lines.push("[GUI] Profile deleted.".to_string());
                    Task::none()
                }
            }
            Message::SelectNode(tag) => {
                if !self.core_running {
                    self.selected_node_tag = Some(tag.clone());
                    self.gui_config.selected_node_tag = Some(tag.clone());
                    let _ = config::save_gui_config(&self.gui_config);
                    return Task::none();
                }
                let tag_clone = tag.clone();
                let api_port = self.gui_config.api_port;
                
                Task::perform(async move {
                    api::select_proxy(api_port, "Proxy", &tag_clone).await
                }, move |res| {
                    match res {
                        Ok(_) => Message::NodeSelected { tag: tag.clone(), error: None },
                        Err(e) => Message::NodeSelected { tag: tag.clone(), error: Some(e) },
                    }
                })
            }
            Message::NodeSelected { tag, error } => {
                if let Some(err) = error {
                    self.log_lines.push(format!("[GUI] Failed to select node: {}", err));
                } else {
                    self.selected_node_tag = Some(tag.clone());
                    self.gui_config.selected_node_tag = Some(tag.clone());
                    let _ = config::save_gui_config(&self.gui_config);
                    self.log_lines.push(format!("[GUI] Selected node: {}", tag));
                }
                Task::none()
            }
            Message::StartLatencyTest => {
                if self.active_profile_nodes.is_empty() {
                    return Task::none();
                }
                self.latency_testing = true;
                let api_port = self.gui_config.api_port;
                
                let tasks = self.active_profile_nodes.iter().map(|node| {
                    let tag = node.name.clone();
                    Task::perform(async move {
                        let latency = api::test_node_latency(api_port, &tag).await;
                        (tag, latency)
                    }, |(tag, res)| {
                        Message::NodeLatencyTested {
                            tag,
                            latency: res.ok(),
                        }
                    })
                }).collect::<Vec<_>>();
                
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
                    self.log_lines.push(format!("[GUI] Updating subscription: {}", url));
                    
                    Task::perform(async move {
                        let client = reqwest::Client::new();
                        let res = client.get(&url)
                            .header("User-Agent", "clash")
                            .send()
                            .await
                            .map_err(|e| format!("Download failed: {}", e))?;
                        if !res.status().is_success() {
                            return Err(format!("Server returned: {}", res.status()));
                        }
                        let content = res.text().await
                            .map_err(|e| format!("Read failed: {}", e))?;
                        let _ = crate::config::validate_profile_content(&content)
                            .map_err(|e| format!("Invalid config: {}", e))?;
                        let path = crate::config::get_profile_path(&id_clone);
                        std::fs::write(&path, &content)
                            .map_err(|e| format!("Save failed: {}", e))?;
                        Ok::<String, String>(id_clone)
                    }, |res| match res {
                        Ok(id) => Message::SubscriptionDownloaded { id, error: None },
                        Err(e) => Message::SubscriptionDownloaded { id: String::new(), error: Some(e) },
                    })
                } else {
                    self.log_lines.push("[GUI] Subscription not found.".to_string());
                    Task::none()
                }
            }
            Message::Tick => {
                self.core_running = core::is_core_running();
                self.core_installed = core::is_core_installed(&self.gui_config);
                self.confirm_delete_profile_id = None; // Reset delete confirmation timeout
                
                let mut tasks = Vec::new();
                
                // Periodically fetch active node and connection totals from Clash API if core is running
                if self.core_running {
                    let api_port = self.gui_config.api_port;
                    tasks.push(Task::perform(async move {
                        api::fetch_proxies(api_port).await
                    }, |res| {
                        Message::ProxiesFetched(res.map(|r| r.proxies).map_err(|e| e))
                    }));
                    
                    tasks.push(Task::done(Message::FetchConnections));
                }
                Task::batch(tasks)
            }
            Message::FetchConnections => {
                if self.core_running {
                    let api_port = self.gui_config.api_port;
                    return Task::perform(async move {
                        api::fetch_connections(api_port).await
                    }, Message::ConnectionsFetched);
                }
                Task::none()
            }
            Message::ConnectionsFetched(Ok(res)) => {
                self.active_connections = res.connections.unwrap_or_default();
                self.total_downloaded = res.download_total;
                self.total_uploaded = res.upload_total;
                Task::none()
            }
            Message::ConnectionsFetched(Err(_e)) => {
                // Suppress background polling HTTP errors
                Task::none()
            }
            Message::CloseConnection(id) => {
                if self.core_running {
                    let api_port = self.gui_config.api_port;
                    let id_clone = id.clone();
                    return Task::perform(async move {
                        match api::close_connection(api_port, &id_clone).await {
                            Ok(_) => Ok(id_clone),
                            Err(e) => Err(e),
                        }
                    }, Message::ConnectionClosed);
                }
                Task::none()
            }
            Message::ConnectionClosed(Ok(id)) => {
                self.log_lines.push(format!("[GUI] Closed connection {}", id));
                self.active_connections.retain(|c| c.id != id);
                Task::none()
            }
            Message::ConnectionClosed(Err(e)) => {
                self.log_lines.push(format!("[GUI] Failed to close connection: {}", e));
                Task::none()
            }
            Message::ErrorOccurred(err) => {
                self.log_lines.push(format!("[GUI ERROR] {}", err));
                self.core_install_msg = Some(err);
                Task::none()
            }
            Message::RoutingModeChanged(mode) => {
                self.gui_config.routing_mode = mode;
                let _ = config::save_gui_config(&self.gui_config);
                return self.restart_core();
            }
            
            // New type-safe configuration messages
            Message::MixedPortChanged(val) => {
                self.mixed_port_input_str = val.clone();
                if let Ok(p) = val.parse::<u16>() {
                    if p > 0 {
                        self.gui_config.mixed_port = p;
                    }
                }
                Task::none()
            }
            Message::ApiPortChanged(val) => {
                self.api_port_input_str = val.clone();
                if let Ok(p) = val.parse::<u16>() {
                    if p > 0 {
                        self.gui_config.api_port = p;
                    }
                }
                Task::none()
            }
            Message::DnsLocalChanged(val) => {
                self.dns_server_local_input_str = val.clone();
                self.gui_config.dns_server_local = val;
                Task::none()
            }
            Message::DnsRemoteChanged(val) => {
                self.dns_server_remote_input_str = val.clone();
                self.gui_config.dns_server_remote = val;
                Task::none()
            }
            Message::ToggleTun => {
                self.gui_config.tun_mode = !self.gui_config.tun_mode;
                let _ = config::save_gui_config(&self.gui_config);
                self.restart_core()
            }
            Message::ToggleAutostart => {
                self.gui_config.start_on_boot = !self.gui_config.start_on_boot;
                let _ = config::save_gui_config(&self.gui_config);
                let _ = set_windows_autostart(self.gui_config.start_on_boot);
                Task::none()
            }
            Message::SetLanguage(lang) => {
                self.gui_config.language = lang;
                let _ = config::save_gui_config(&self.gui_config);
                self.update_tray_menu();
                Task::none()
            }
            Message::SetTheme(theme) => {
                self.gui_config.theme = theme;
                let _ = config::save_gui_config(&self.gui_config);
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
                    if let Some(profile) = self.gui_config.subscriptions.iter_mut().find(|p| p.id == id) {
                        profile.name = self.editing_profile_name.clone();
                        profile.url = self.editing_profile_url.clone();
                        let _ = config::save_gui_config(&self.gui_config);
                    }
                    self.editing_profile_id = None;
                }
                Task::none()
            }
            Message::CancelProfileEdit => {
                self.editing_profile_id = None;
                Task::none()
            }
            Message::ToggleCloseCoreOnExit => {
                self.gui_config.close_core_on_exit = !self.gui_config.close_core_on_exit;
                let _ = config::save_gui_config(&self.gui_config);
                Task::none()
            }
            Message::ToggleFakeIp => {
                self.gui_config.fake_ip = !self.gui_config.fake_ip;
                let _ = config::save_gui_config(&self.gui_config);
                self.restart_core()
            }
            Message::ToggleTcpFastOpen => {
                self.gui_config.tcp_fast_open = !self.gui_config.tcp_fast_open;
                let _ = config::save_gui_config(&self.gui_config);
                self.restart_core()
            }
            Message::ToggleTcpMultipath => {
                self.gui_config.tcp_multipath = !self.gui_config.tcp_multipath;
                let _ = config::save_gui_config(&self.gui_config);
                self.restart_core()
            }
            Message::SaveSettings => {
                if self.mixed_port_input_str.trim().is_empty() || self.api_port_input_str.trim().is_empty() {
                    let err = if self.gui_config.language == state::Language::Zh {
                        "端口号不能为空！".to_string()
                    } else {
                        "Port numbers cannot be empty!".to_string()
                    };
                    self.log_lines.push(format!("[GUI ERROR] {}", err));
                    self.core_install_msg = Some(err);
                    return Task::none();
                }
                
                let mixed_parsed = self.mixed_port_input_str.trim().parse::<u16>();
                let api_parsed = self.api_port_input_str.trim().parse::<u16>();
                
                if mixed_parsed.is_err() || api_parsed.is_err() {
                    let err = if self.gui_config.language == state::Language::Zh {
                        "端口必须是 1 到 65535 之间的有效数字！".to_string()
                    } else {
                        "Ports must be valid numbers between 1 and 65535!".to_string()
                    };
                    self.log_lines.push(format!("[GUI ERROR] {}", err));
                    self.core_install_msg = Some(err);
                    return Task::none();
                }
                
                self.core_install_msg = None;
                let _ = config::save_gui_config(&self.gui_config);
                self.log_lines.push("[GUI] Settings saved and applied successfully.".to_string());
                self.restart_core()
            }
            Message::CheckUpdate => {
                self.update_status = state::UpdateStatus::Checking;
                Task::perform(check_app_update(), Message::UpdateChecked)
            }
            Message::UpdateChecked(result) => {
                match result {
                    Ok(tag_name) => {
                        let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));
                        if tag_name.trim() == current_version.trim() {
                            self.update_status = state::UpdateStatus::UpToDate;
                        } else {
                            self.update_status = state::UpdateStatus::NewVersion(tag_name);
                        }
                    }
                    Err(e) => {
                        self.update_status = state::UpdateStatus::Error(e);
                    }
                }
                Task::none()
            }
            Message::OpenUrl(url) => {
                #[cfg(target_os = "windows")]
                let _ = std::process::Command::new("cmd")
                    .args(&["/c", "start", "", &url])
                    .spawn();
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open")
                    .arg(&url)
                    .spawn();
                #[cfg(target_os = "linux")]
                let _ = std::process::Command::new("xdg-open")
                    .arg(&url)
                    .spawn();
                Task::none()
            }
        }
    }
    
    fn view(&self) -> Element<'_, Message> {
        let lang = self.gui_config.language;
        let theme_ref = self.theme_ref();
        
        let current_tab = self.current_tab;
        let core_running = self.core_running;
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

        let main_content = responsive(move |size| {
            let theme = theme_ref;
            let is_compact = size.width < 750.0;
            let text_muted = ui::theme::text_muted(theme);
            
            // Render active tab view
            let content = match current_tab {
                Tab::Dashboard => ui::dashboard::render(
                    gui_config_ref,
                    core_running,
                    sys_proxy_enabled,
                    current_speed_ref,
                    speed_history_ref,
                    total_uploaded,
                    total_downloaded,
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
                    connections_search_ref,
                    theme,
                ),
                Tab::Logs => ui::logs::render(gui_config_ref, log_lines_ref, theme),
                Tab::Settings => ui::settings::render(
                    gui_config_ref,
                    mixed_port_input_str_ref,
                    api_port_input_str_ref,
                    dns_server_local_input_str_ref,
                    dns_server_remote_input_str_ref,
                    core_installed,
                    core_install_msg_ref,
                    update_status_ref,
                    theme,
                ),
            };

            let make_tab_btn = |tab: Tab, icon_char: char, key: &'static str| {
                let active = current_tab == tab;
                
                let indicator = container(iced::widget::Space::new())
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
                    });
                
                let btn_content = if is_compact {
                    row![
                        indicator,
                        text(icon_char.to_string())
                            .font(Font::with_name("Material Icons"))
                            .size(18),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                } else {
                    row![
                        indicator,
                        text(icon_char.to_string())
                            .font(Font::with_name("Material Icons"))
                            .size(16),
                        text(ui::i18n::tr(lang, key))
                            .size(15)
                            .font(Font {
                                weight: if active { iced::font::Weight::Bold } else { iced::font::Weight::Medium },
                                ..Default::default()
                            })
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center)
                };
                
                button(btn_content)
                    .padding(if is_compact { [14, 0] } else { [14, 16] })
                    .width(Length::Fill)
                    .style(ui::theme::button_tab(active))
                    .on_press(Message::TabChanged(tab))
            };

            let sidebar = if is_compact {
                container(
                    column![
                        column![
                            text("S")
                                .size(24)
                                .font(Font {
                                    weight: iced::font::Weight::Bold,
                                    ..Default::default()
                                })
                                .color(ui::theme::ACCENT_PURPLE)
                                .width(Length::Fill)
                                .align_x(Alignment::Center),
                            column![
                                make_tab_btn(Tab::Dashboard, '\u{E871}', "tab_dashboard"),
                                make_tab_btn(Tab::Proxies, '\u{EA0B}', "tab_proxies"),
                                make_tab_btn(Tab::Profiles, '\u{E2C7}', "tab_profiles"),
                                make_tab_btn(Tab::Rules, '\u{E41E}', "tab_rules"),
                                make_tab_btn(Tab::Connections, '\u{E894}', "tab_connections"),
                                make_tab_btn(Tab::Logs, '\u{E85D}', "tab_logs"),
                                make_tab_btn(Tab::Settings, '\u{E8B8}', "tab_settings"),
                            ]
                            .spacing(10)
                            .width(Length::Fill)
                        ]
                        .spacing(24)
                        .width(Length::Fill),
                        iced::widget::Space::new().height(Length::Fill),
                    ]
                    .width(Length::Fill)
                )
                .width(Length::Fixed(64.0))
                .height(Length::Fill)
                .padding([24, 0])
                .style(ui::theme::sidebar_bg)
            } else {
                container(
                    column![
                        column![
                            row![
                                text("sing-box")
                                    .size(24)
                                    .font(Font {
                                        weight: iced::font::Weight::Bold,
                                        ..Default::default()
                                    })
                                    .color(ui::theme::ACCENT_PURPLE),
                                text("GUI")
                                    .size(12)
                                    .font(Font {
                                        weight: iced::font::Weight::Light,
                                        ..Default::default()
                                    })
                                    .color(ui::theme::ACCENT_BLUE),
                            ]
                            .spacing(6)
                            .align_y(Alignment::End),
                            column![
                                make_tab_btn(Tab::Dashboard, '\u{E871}', "tab_dashboard"),
                                make_tab_btn(Tab::Proxies, '\u{EA0B}', "tab_proxies"),
                                make_tab_btn(Tab::Profiles, '\u{E2C7}', "tab_profiles"),
                                make_tab_btn(Tab::Rules, '\u{E41E}', "tab_rules"),
                                make_tab_btn(Tab::Connections, '\u{E894}', "tab_connections"),
                                make_tab_btn(Tab::Logs, '\u{E85D}', "tab_logs"),
                                make_tab_btn(Tab::Settings, '\u{E8B8}', "tab_settings"),
                            ]
                            .spacing(10)
                            .width(Length::Fill)
                        ]
                        .spacing(36)
                        .width(Length::Fill),
                        iced::widget::Space::new().height(Length::Fill),
                        text(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .size(12)
                            .color(text_muted)
                    ]
                )
                .width(Length::Fixed(240.0))
                .height(Length::Fill)
                .padding(24)
                .style(ui::theme::sidebar_bg)
            };

            let main_layout = container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(ui::theme::main_bg);
                
            row![
                sidebar,
                main_layout
            ]
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
    iced::stream::channel(100, |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
        let rx = {
            let lock_opt = LOG_RX.get();
            if let Some(lock) = lock_opt {
                lock.lock().unwrap_or_else(|e| e.into_inner()).take()
            } else {
                None
            }
        };
        if let Some(mut r) = rx {
            while let Some(line) = r.recv().await {
                let _ = output.send(Message::NewLogLine(line)).await;
            }
        }
    })
}

// Subscription worker for streaming real-time Clash API traffic stats
fn traffic_subscription() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(100, |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
        let rx = {
            let lock_opt = TRAFFIC_RX.get();
            if let Some(lock) = lock_opt {
                lock.lock().unwrap_or_else(|e| e.into_inner()).take()
            } else {
                None
            }
        };
        if let Some(mut r) = rx {
            while let Some(info) = r.recv().await {
                let _ = output.send(Message::TrafficUpdated { up: info.up, down: info.down }).await;
            }
        }
    })
}

// Subscription worker for streaming real-time system tray menu and clicks
fn tray_subscription() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(100, |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        
        let tx_clone = tx.clone();
        std::thread::spawn(move || {
            let menu_channel = tray_icon::menu::MenuEvent::receiver();
            loop {
                if let Ok(event) = menu_channel.recv() {
                    let _ = tx_clone.blocking_send(Message::TrayMenuClicked(event.id.0));
                }
            }
        });

        let tx_clone2 = tx.clone();
        std::thread::spawn(move || {
            let tray_channel = tray_icon::TrayIconEvent::receiver();
            loop {
                if let Ok(event) = tray_channel.recv() {
                    match event {
                        tray_icon::TrayIconEvent::Click { button, .. } => {
                            if button == tray_icon::MouseButton::Left {
                                let _ = tx_clone2.blocking_send(Message::TrayIconClicked);
                            }
                        }
                        tray_icon::TrayIconEvent::DoubleClick { button, .. } => {
                            if button == tray_icon::MouseButton::Left {
                                let _ = tx_clone2.blocking_send(Message::TrayIconClicked);
                            }
                        }
                        _ => {}
                    }
                }
            }
        });
        
        while let Some(msg) = rx.recv().await {
            let _ = output.send(msg).await;
        }
    })
}

fn load_icon_safe() -> Option<tray_icon::Icon> {
    let logo_bytes = include_bytes!("../assets/logo.jpg");
    match image::load_from_memory(logo_bytes) {
        Ok(img) => {
            let rgba_img = img.resize(32, 32, image::imageops::FilterType::Lanczos3).into_rgba8();
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

// Download subscription configuration task implementation
async fn download_profile(url: String) -> Result<(String, String), String> {
    let content = if std::path::Path::new(&url).exists() {
        std::fs::read_to_string(&url)
            .map_err(|e| format!("Failed to read local file: {}", e))?
    } else {
        let client = reqwest::Client::new();
        let res = client.get(&url)
            .header("User-Agent", "clash")
            .send()
            .await
            .map_err(|e| format!("Download request failed: {}", e))?;
            
        if !res.status().is_success() {
            return Err(format!("Download failed with status: {}", res.status()));
        }
        
        res.text()
            .await
            .map_err(|e| format!("Failed to read content: {}", e))?
    };
        
    // Verify profile content is valid (either Sing-Box JSON or Clash YAML)
    config::validate_profile_content(&content)?;
        
    // Generate an ID and Name
    let id = chrono::Utc::now().timestamp_millis().to_string();
    let name = if std::path::Path::new(&url).exists() {
        std::path::Path::new(&url)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("Local_Config")
            .to_string()
    } else {
        format!("Sub_{}", &id[id.len()-6..])
    };
    
    // Save raw YAML/JSON to profile directory
    let path = config::get_profile_path(&id);
    std::fs::write(&path, &content)
        .map_err(|e| format!("Failed to save profile: {}", e))?;
        
    // Add to configurations registry list
    let mut config = config::load_gui_config();
    config.subscriptions.push(Profile {
        id: id.clone(),
        name: name.clone(),
        url: url.clone(),
        file_path: path.to_string_lossy().to_string(),
        is_subscription: true,
        updated_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    });
    
    let _ = config::save_gui_config(&config);
    Ok((id, name))
}

fn set_windows_autostart(enable: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
        use winreg::RegKey;
        
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let run_key = hkcu.open_subkey_with_flags(
            r#"Software\Microsoft\Windows\CurrentVersion\Run"#,
            KEY_WRITE
        ).map_err(|e| format!("Failed to open registry key: {}", e))?;
        
        if enable {
            let exe_path = std::env::current_exe()
                .map_err(|e| format!("Failed to resolve current exe path: {}", e))?;
            run_key.set_value("sing-box-gui", &exe_path.to_string_lossy().to_string())
                .map_err(|e| format!("Failed to write autostart registry: {}", e))?;
        } else {
            let _ = run_key.delete_value("sing-box-gui");
        }
    }
    Ok(())
}

fn detect_system_theme() -> bool {
    #[cfg(target_os = "windows")]
    {
        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;
        if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize") {
            if let Ok(val) = hkcu.get_value::<u32, _>("AppsUseLightTheme") {
                return val == 1; // 1 = Light Mode, 0 = Dark Mode
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("defaults").args(&["read", "-g", "AppleInterfaceStyle"]).output() {
            let style = String::from_utf8_lossy(&output.stdout);
            return !style.trim().contains("Dark");
        }
    }
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("gsettings").args(&["get", "org.gnome.desktop.interface", "color-scheme"]).output() {
            let scheme = String::from_utf8_lossy(&output.stdout);
            return !scheme.trim().contains("dark");
        }
    }
    false // Default to dark mode (0)
}

fn open_path_in_system(path: &std::path::Path) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(&["/c", "start", "", &path.to_string_lossy()])
        .spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open")
        .arg(path)
        .spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open")
        .arg(path)
        .spawn();
}

async fn check_app_update() -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
        
    let res = client.get("https://api.github.com/repos/zangge8855/sing-box-gui/releases/latest")
        .header("User-Agent", "sing-box-gui")
        .send()
        .await
        .map_err(|e| format!("Network request failed: {}", e))?;
        
    if !res.status().is_success() {
        return Err(format!("Server returned error status: {}", res.status()));
    }
    
    #[derive(serde::Deserialize)]
    struct GithubRelease {
        tag_name: String,
    }
    
    let release: GithubRelease = res.json()
        .await
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
        
    Ok(release.tag_name)
}

fn main() -> iced::Result {
    let icon_bytes = include_bytes!("../assets/logo.jpg");
    let icon = iced::window::icon::from_file_data(icon_bytes, None).ok();
    
    let window_settings = iced::window::Settings {
        size: iced::Size::new(1080.0, 750.0),
        min_size: Some(iced::Size::new(960.0, 680.0)),
        icon,
        exit_on_close_request: false,
        ..Default::default()
    };

    let default_font = iced::Font {
        family: iced::font::Family::SansSerif,
        ..Default::default()
    };

    let res = iced::application(App::new, App::update, App::view)
        .title("sing-box GUI")
        .window(window_settings)
        .theme(App::theme)
        .default_font(default_font)
        .font(include_bytes!("../assets/material-icons.ttf").as_slice())
        .subscription(App::subscription)
        .run();
        
    // CRITICAL EXIT CLEANUP
    let config = config::load_gui_config();
    if config.close_core_on_exit {
        core::stop_core();
    }
    let _ = sysproxy::set_system_proxy(false, config.mixed_port);
    
    res
}
