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
use iced::{Element, Length, Subscription, Task, Font};
use iced::widget::{button, column, container, row, text};
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

    fn new() -> (Self, Task<Message>) {
        let gui_config = config::load_gui_config();
        let core_installed = core::is_core_installed(&gui_config);
        let selected_node_tag = gui_config.selected_node_tag.clone();
        
        let mut app = Self {
            current_tab: Tab::Dashboard,
            gui_config,
            core_running: false,
            sys_proxy_enabled: false,
            log_lines: Vec::new(),
            current_speed: Bandwidth::default(),
            speed_history: vec![(0, 0); 30], // initial 30 empty data points
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
        };
        
        // Load active profile nodes if profile exists
        app.reload_active_nodes();
        
        // Sync system proxy checkbox status with Windows registry state
        let sys_proxy = sysproxy::check_system_proxy().unwrap_or(false);
        app.sys_proxy_enabled = sys_proxy;
        
        // Check if core is running on start
        app.core_running = core::is_core_running();
        
        (app, Task::none())
    }
    
    fn reload_active_nodes(&mut self) {
        if let Some(ref active_id) = self.gui_config.active_profile_id {
            let path = config::get_profile_path(active_id);
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if let Ok(nodes) = config::parse_clash_yaml_nodes(&content) {
                        self.active_profile_nodes = nodes;
                    }
                }
            }
        }
        
        // Retrieve active proxy node tag from Clash API if core running
        if self.core_running {
            // Will update selected node tag inside Tick
        }
    }
    
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TabChanged(tab) => {
                self.current_tab = tab;
                Task::none()
            }
            Message::ToggleCore => {
                if self.core_running {
                    core::stop_core();
                    self.core_running = false;
                    
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
                            
                            // Start traffic monitor stream
                            let traffic_tx = get_traffic_tx();
                            api::spawn_traffic_monitor(self.gui_config.api_port, traffic_tx);
                            
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
                // Check internal triggers
                if line == "TRIGGER_CORE_DOWNLOAD" {
                    let log_tx = get_log_tx();
                    return Task::perform(async move {
                        core::download_core(log_tx)
                    }, |res| match res {
                        Ok(_) => Message::CoreStatusChanged(true), // triggers core install refresh
                        Err(e) => Message::ErrorOccurred(e),
                    });
                } else if line == "CLEAR_LOG_BUFFER" {
                    self.log_lines.clear();
                    return Task::none();
                }
                
                self.log_lines.push(line);
                if self.log_lines.len() > 1000 {
                    self.log_lines.remove(0);
                }
                Task::none()
            }
            Message::TrafficUpdated { up, down } => {
                self.current_speed = Bandwidth { up, down };
                self.total_uploaded += up;
                self.total_downloaded += down;
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
                Task::none()
            }
            Message::DownloadSubscription => {
                if self.url_input.trim().is_empty() {
                    return Task::none();
                }
                self.downloading = true;
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
                    self.log_lines.push(format!("[GUI] Download failed: {}", err));
                } else {
                    self.url_input.clear();
                    // Update the updated_at timestamp for the subscription
                    for sub in &mut self.gui_config.subscriptions {
                        if sub.id == id {
                            sub.updated_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                        }
                    }
                    self.log_lines.push("[GUI] Subscription downloaded successfully.".to_string());
                    
                    // Reload profiles list
                    let loaded = config::load_gui_config();
                    self.gui_config = loaded;
                    
                    // If no active profile, select this one
                    if self.gui_config.active_profile_id.is_none() {
                        self.gui_config.active_profile_id = Some(id.clone());
                        let _ = config::save_gui_config(&self.gui_config);
                        self.reload_active_nodes();
                    }
                }
                Task::none()
            }
            Message::SelectProfile(id) => {
                self.gui_config.active_profile_id = Some(id);
                let _ = config::save_gui_config(&self.gui_config);
                self.reload_active_nodes();
                self.log_lines.push("[GUI] Active profile updated.".to_string());
                
                // If core is running, restart core to apply config
                if self.core_running {
                    core::stop_core();
                    self.core_running = false;
                    return Task::done(Message::ToggleCore);
                }
                Task::none()
            }
            Message::DeleteProfile(id) => {
                // Delete cached file
                let path = config::get_profile_path(&id);
                let _ = std::fs::remove_file(path);
                
                // Remove from list
                self.gui_config.subscriptions.retain(|p| p.id != id);
                if self.gui_config.active_profile_id.as_ref() == Some(&id) {
                    self.gui_config.active_profile_id = None;
                    self.active_profile_nodes.clear();
                }
                let _ = config::save_gui_config(&self.gui_config);
                self.log_lines.push("[GUI] Profile deleted.".to_string());
                Task::none()
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
                
                Task::batch(tasks).chain(Task::perform(async {}, |_| Message::NodeLatencyTested { tag: "TEST_FINISHED".to_string(), latency: None }))
            }
            Message::NodeLatencyTested { tag, latency } => {
                if tag == "TEST_FINISHED" {
                    self.latency_testing = false;
                } else {
                    for node in &mut self.active_profile_nodes {
                        if node.name == tag {
                            node.latency = latency;
                        }
                    }
                }
                Task::none()
            }
            Message::UpdateSubscription(id) => {
                let sub = self.gui_config.subscriptions.iter().find(|p| p.id == id);
                if let Some(profile) = sub {
                    let url = profile.url.clone();
                    let id_clone = id.clone();
                    self.downloading = true;
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
                        let _ = crate::config::parse_clash_yaml_nodes(&content)
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
                
                let mut tasks = Vec::new();
                
                // Periodically fetch active node from Clash API if core running
                if self.core_running {
                    let api_port = self.gui_config.api_port;
                    tasks.push(Task::perform(async move {
                        api::fetch_proxies(api_port).await
                    }, |res| match res {
                        Ok(proxies_res) => {
                            if let Some(proxy_info) = proxies_res.proxies.get("Proxy") {
                                if let Some(ref active_node) = proxy_info.now {
                                    return Message::SelectNode(active_node.clone());
                                }
                            }
                            Message::Tick
                        }
                        Err(_) => Message::Tick,
                    }));
                    
                    if self.current_tab == Tab::Connections {
                        tasks.push(Task::done(Message::FetchConnections));
                    }
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
                self.active_connections = res.connections;
                Task::none()
            }
            Message::ConnectionsFetched(Err(e)) => {
                self.log_lines.push(format!("[GUI] Failed to fetch connections: {}", e));
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
                // Restart core if running to apply changes
                if self.core_running {
                    core::stop_core();
                    self.core_running = false;
                    return Task::done(Message::ToggleCore);
                }
                Task::none()
            }
            Message::PortInputChanged(input) => {
                if input.starts_with("mixed:") {
                    if let Ok(p) = input[6..].parse::<u16>() {
                        self.gui_config.mixed_port = p;
                    }
                } else if input.starts_with("api:") {
                    if let Ok(p) = input[4..].parse::<u16>() {
                        self.gui_config.api_port = p;
                    }
                } else if input.starts_with("dns_local:") {
                    self.gui_config.dns_server_local = input[10..].to_string();
                } else if input.starts_with("dns_remote:") {
                    self.gui_config.dns_server_remote = input[11..].to_string();
                } else if input == "toggle_tun" {
                    self.gui_config.tun_mode = !self.gui_config.tun_mode;
                    let _ = config::save_gui_config(&self.gui_config);
                } else if input == "toggle_autostart" {
                    self.gui_config.start_on_boot = !self.gui_config.start_on_boot;
                    let _ = config::save_gui_config(&self.gui_config);
                    // Apply Windows startup boot register changes
                    let _ = set_windows_autostart(self.gui_config.start_on_boot);
                } else if input == "lang:en" {
                    self.gui_config.language = state::Language::En;
                    let _ = config::save_gui_config(&self.gui_config);
                } else if input == "lang:zh" {
                    self.gui_config.language = state::Language::Zh;
                    let _ = config::save_gui_config(&self.gui_config);
                } else if input == "toggle_fakeip" {
                    self.gui_config.fake_ip = !self.gui_config.fake_ip;
                    let _ = config::save_gui_config(&self.gui_config);
                } else if input == "toggle_tfo" {
                    self.gui_config.tcp_fast_open = !self.gui_config.tcp_fast_open;
                    let _ = config::save_gui_config(&self.gui_config);
                } else if input == "toggle_mptcp" {
                    self.gui_config.tcp_multipath = !self.gui_config.tcp_multipath;
                    let _ = config::save_gui_config(&self.gui_config);
                } else if input == "toggle_close_core" {
                    self.gui_config.close_core_on_exit = !self.gui_config.close_core_on_exit;
                    let _ = config::save_gui_config(&self.gui_config);
                } else if input == "theme:auto" {
                    self.gui_config.theme = state::AppTheme::Auto;
                    let _ = config::save_gui_config(&self.gui_config);
                } else if input == "theme:dark" {
                    self.gui_config.theme = state::AppTheme::Dark;
                    let _ = config::save_gui_config(&self.gui_config);
                } else if input == "theme:light" {
                    self.gui_config.theme = state::AppTheme::Light;
                    let _ = config::save_gui_config(&self.gui_config);
                } else if input == "open_data_dir" {
                    #[cfg(target_os = "windows")]
                    let _ = std::process::Command::new("explorer")
                        .arg(config::get_app_dir())
                        .spawn();
                    #[cfg(target_os = "macos")]
                    let _ = std::process::Command::new("open")
                        .arg(config::get_app_dir())
                        .spawn();
                    #[cfg(target_os = "linux")]
                    let _ = std::process::Command::new("xdg-open")
                        .arg(config::get_app_dir())
                        .spawn();
                } else if input == "open_profiles_folder" {
                    #[cfg(target_os = "windows")]
                    let _ = std::process::Command::new("explorer")
                        .arg(config::get_app_dir().join("profiles"))
                        .spawn();
                    #[cfg(target_os = "macos")]
                    let _ = std::process::Command::new("open")
                        .arg(config::get_app_dir().join("profiles"))
                        .spawn();
                    #[cfg(target_os = "linux")]
                    let _ = std::process::Command::new("xdg-open")
                        .arg(config::get_app_dir().join("profiles"))
                        .spawn();
                } else if input.starts_with("edit_profile:") {
                    let id = &input[13..];
                    let path = config::get_profile_path(id);
                    #[cfg(target_os = "windows")]
                    let _ = std::process::Command::new("cmd")
                        .args(&["/c", "start", "", &path.to_string_lossy()])
                        .spawn();
                    #[cfg(target_os = "macos")]
                    let _ = std::process::Command::new("open")
                        .arg(&path)
                        .spawn();
                    #[cfg(target_os = "linux")]
                    let _ = std::process::Command::new("xdg-open")
                        .arg(&path)
                        .spawn();
                }
                Task::none()
            }
            Message::SaveSettings => {
                let _ = config::save_gui_config(&self.gui_config);
                self.log_lines.push("[GUI] Settings saved and applied successfully.".to_string());
                Task::none()
            }
        }
    }
    
    fn view(&self) -> Element<'_, Message> {
        let lang = self.gui_config.language;
        let active_theme = self.theme();
        
        let make_tab_btn = |tab: Tab, icon: &str, key: &'static str| {
            let active = self.current_tab == tab;
            let label = format!("{} {}", icon, ui::i18n::tr(lang, key));
            button(
                text(label)
                    .size(14)
                    .font(Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
            )
            .padding([12, 16])
            .width(Length::Fill)
            .style(ui::theme::button_tab(active))
            .on_press(Message::TabChanged(tab))
        };
        
        // Sidebar View
        let sidebar = container(
            column![
                column![
                    text("sing-box")
                        .size(24)
                        .font(Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        })
                        .color(ui::theme::ACCENT_PURPLE),
                    column![
                        make_tab_btn(Tab::Dashboard, "📊", "tab_dashboard"),
                        make_tab_btn(Tab::Proxies, "⚡", "tab_proxies"),
                        make_tab_btn(Tab::Profiles, "📂", "tab_profiles"),
                        make_tab_btn(Tab::Connections, "🌐", "tab_connections"),
                        make_tab_btn(Tab::Logs, "📝", "tab_logs"),
                        make_tab_btn(Tab::Settings, "⚙️", "tab_settings"),
                    ]
                    .spacing(8)
                ]
                .spacing(40),
                iced::widget::Space::new().height(Length::Fill),
                text(format!("v{}", env!("CARGO_PKG_VERSION")))
                    .size(12)
                    .color(ui::theme::text_muted(&active_theme))
            ]
        )
        .width(Length::Fixed(200.0))
        .height(Length::Fill)
        .padding(20)
        .style(ui::theme::sidebar_bg);
        
        // Main Tab Content View
        let content = match self.current_tab {
            Tab::Dashboard => ui::dashboard::render(
                &self.gui_config,
                self.core_running,
                self.sys_proxy_enabled,
                &self.current_speed,
                &self.speed_history,
                self.total_uploaded,
                self.total_downloaded,
                &active_theme,
            ),
            Tab::Proxies => ui::proxies::render(
                &self.gui_config,
                &self.active_profile_nodes,
                self.selected_node_tag.as_deref(),
                self.latency_testing,
                &active_theme,
            ),
            Tab::Profiles => ui::profiles::render(
                &self.gui_config,
                &self.url_input,
                self.downloading,
                &active_theme,
            ),
            Tab::Connections => ui::connections::render(&self.gui_config, &self.active_connections, &active_theme),
            Tab::Logs => ui::logs::render(&self.gui_config, &self.log_lines, &active_theme),
            Tab::Settings => ui::settings::render(
                &self.gui_config,
                self.core_installed,
                self.core_install_msg.as_deref(),
                &active_theme,
            ),
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
    }
    
    fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![
            iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick)
        ];
        
        // Live streams for logs and traffic stats
        subs.push(Subscription::run(log_subscription));
        subs.push(Subscription::run(traffic_subscription));
        
        Subscription::batch(subs)
    }
}

// Subscription worker for streaming log lines
fn log_subscription() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(100, |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
        let rx = {
            let lock_opt = LOG_RX.get();
            if let Some(lock) = lock_opt {
                lock.lock().unwrap().take()
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
                lock.lock().unwrap().take()
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
        
    // Verify it parses as Clash YAML (contains proxies key)
    let _nodes = config::parse_clash_yaml_nodes(&content)
        .map_err(|e| format!("Invalid Clash configuration format: {}", e))?;
        
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
    
    // Save raw YAML to profile directory
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
    false // Default to dark mode (0)
}

fn main() -> iced::Result {
    let icon_bytes = include_bytes!("../assets/logo.jpg");
    let icon = iced::window::icon::from_file_data(icon_bytes, None).ok();
    
    let window_settings = iced::window::Settings {
        icon,
        ..Default::default()
    };

    let res = iced::application(App::new, App::update, App::view)
        .title("sing-box GUI")
        .window(window_settings)
        .theme(App::theme)
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
