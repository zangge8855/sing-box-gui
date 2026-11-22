//! Offline visual fixtures. Compiled only with `--features ui-smoke`; normal
//! builds cannot enter this mode or touch its synthetic runtime state.

use crate::{App, api, message::Message, state::*};
use iced::{Size, Task, window};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Action {
    TrayStep,
    Next,
    Prepare,
    Capture,
    Captured(window::Screenshot),
    Saved(Result<(), String>),
}

#[derive(Clone)]
struct Case {
    name: String,
    tab: Tab,
    language: Language,
    theme: AppTheme,
    size: Size,
    variant: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrayStage {
    Online,
    Hidden,
    Restored,
    HiddenForLoss,
    Recovered,
    OnlineAgain,
    Done,
}

pub struct Runner {
    output: PathBuf,
    cases: Vec<Case>,
    index: usize,
    tray_stage: TrayStage,
    tray_attempts: u16,
    control: Option<PathBuf>,
    window_opened: bool,
    previous_window: Option<window::Id>,
    no_tray: bool,
}

pub fn requested() -> bool {
    std::env::var_os("SINGBOX_UI_SMOKE_DIR").is_some()
}

impl Runner {
    fn build_cases() -> Vec<Case> {
        let mut cases = Vec::new();
        for (language, locale) in [(Language::En, "en"), (Language::Zh, "zh")] {
            for (theme, appearance) in [(AppTheme::Light, "light"), (AppTheme::Dark, "dark")] {
                for (width, height) in [(640, 480), (720, 600), (1080, 750), (1600, 1000)] {
                    for tab in [
                        Tab::Dashboard,
                        Tab::Proxies,
                        Tab::Profiles,
                        Tab::Rules,
                        Tab::Connections,
                        Tab::Logs,
                        Tab::Settings,
                    ] {
                        cases.push(Case {
                            name: format!("{locale}-{appearance}-{width}-{tab:?}"),
                            tab,
                            language,
                            theme,
                            size: Size::new(width as f32, height as f32),
                            variant: "filled",
                        });
                    }
                }
            }
        }
        for (language, locale) in [(Language::En, "en"), (Language::Zh, "zh")] {
            for (theme, appearance) in [(AppTheme::Light, "light"), (AppTheme::Dark, "dark")] {
                for (tab, variant) in [
                    (Tab::Dashboard, "empty"),
                    (Tab::Proxies, "empty"),
                    (Tab::Profiles, "empty"),
                    (Tab::Rules, "empty"),
                    (Tab::Connections, "empty"),
                    (Tab::Logs, "empty"),
                    (Tab::Profiles, "delete"),
                    (Tab::Profiles, "edit"),
                    (Tab::Profiles, "updating"),
                    (Tab::Profiles, "more"),
                    (Tab::Rules, "invalid"),
                    (Tab::Settings, "invalid"),
                    (Tab::Logs, "no-matches"),
                    (Tab::Proxies, "no-matches"),
                    (Tab::Connections, "no-matches"),
                    (Tab::Settings, "bottom"),
                    (Tab::Settings, "preview"),
                    (Tab::Profiles, "bottom"),
                    (Tab::Dashboard, "bottom"),
                    (Tab::Rules, "bottom"),
                    (Tab::Dashboard, "toasts"),
                    (Tab::Dashboard, "starting"),
                    (Tab::Dashboard, "stopping"),
                    (Tab::Dashboard, "mode-switching"),
                    (Tab::Dashboard, "installing"),
                    (Tab::Dashboard, "core-error"),
                    (Tab::Settings, "pending"),
                    (Tab::Settings, "update"),
                    (Tab::Settings, "update-downloading"),
                    (Tab::Proxies, "testing"),
                    (Tab::Profiles, "error"),
                ] {
                    // The update row sits below the fold at the compact size.
                    let (width, height) = if matches!(variant, "update" | "update-downloading") {
                        (1080, 750)
                    } else {
                        (640, 480)
                    };
                    cases.push(Case {
                        name: format!("{locale}-{appearance}-{width}-{tab:?}-{variant}"),
                        tab,
                        language,
                        theme,
                        size: Size::new(width as f32, height as f32),
                        variant,
                    });
                }
            }
        }
        cases
    }

    pub fn from_env() -> Option<Self> {
        let output = PathBuf::from(std::env::var_os("SINGBOX_UI_SMOKE_DIR")?);
        let mut cases = Self::build_cases();
        if let Ok(filter) = std::env::var("SINGBOX_UI_SMOKE_FILTER") {
            cases.retain(|case| case.name.contains(&filter));
        }
        Some(Self {
            output,
            cases,
            index: 0,
            tray_stage: if std::env::var("SINGBOX_UI_SMOKE_TRAY").as_deref() == Ok("1") {
                TrayStage::Online
            } else {
                TrayStage::Done
            },
            tray_attempts: 0,
            control: std::env::var_os("SINGBOX_UI_SMOKE_CONTROL_DIR").map(PathBuf::from),
            window_opened: false,
            previous_window: None,
            no_tray: std::env::var("SINGBOX_UI_SMOKE_TRAY").as_deref() == Ok("0"),
        })
    }
}

pub fn window_opened(app: &mut App) -> Task<Message> {
    let runner = app.smoke.as_mut().expect("smoke runner");
    runner.window_opened = true;
    match runner.tray_stage {
        TrayStage::Online => Task::done(Message::Smoke(Action::TrayStep)),
        TrayStage::Done => Task::done(Message::Smoke(Action::Next)),
        _ => Task::none(),
    }
}

fn after(milliseconds: u64, action: Action) -> Task<Message> {
    Task::perform(
        async move { tokio::time::sleep(std::time::Duration::from_millis(milliseconds)).await },
        move |_| Message::Smoke(action.clone()),
    )
}

fn fail(error: impl std::fmt::Display) -> Task<Message> {
    eprintln!("UI_SMOKE_FAILED: {error}");
    iced::exit()
}

// The Python runner hosts an actual D-Bus StatusNotifierWatcher. Activate and
// watcher loss arrive through ksni and the real Iced tray subscription.
fn test_tray(app: &mut App) -> Task<Message> {
    let available = app.tray.as_ref().is_some_and(crate::tray::Tray::available);
    let runner = app.smoke.as_mut().expect("smoke runner");
    runner.tray_attempts += 1;
    if runner.tray_attempts > 120 {
        return fail(format!(
            "tray lifecycle timed out at {:?}",
            runner.tray_stage
        ));
    }
    let mut command = None;
    let mut close = false;
    match runner.tray_stage {
        TrayStage::Online if available && runner.window_opened => {
            app.url_input = "unsaved import survives closing the window".into();
            app.settings_draft.mixed_port = "7891".into();
            runner.tray_stage = TrayStage::Hidden;
            close = true;
        }
        TrayStage::Hidden if app.window_id.is_none() => {
            runner.tray_stage = TrayStage::Restored;
            command = Some("activate");
        }
        TrayStage::Restored if runner.window_opened && app.window_id != runner.previous_window => {
            if app.url_input != "unsaved import survives closing the window"
                || app.settings_draft.mixed_port != "7891"
            {
                return fail("window recreation lost the unsaved form or settings draft");
            }
            runner.tray_stage = TrayStage::HiddenForLoss;
            close = true;
        }
        TrayStage::HiddenForLoss if app.window_id.is_none() => {
            runner.tray_stage = TrayStage::Recovered;
            command = Some("remove-watcher");
        }
        TrayStage::Recovered
            if runner.window_opened && app.window_id != runner.previous_window && !available =>
        {
            runner.tray_stage = TrayStage::OnlineAgain;
            command = Some("restore-watcher");
        }
        TrayStage::OnlineAgain if available => {
            runner.tray_stage = TrayStage::Done;
            app.url_input.clear();
            app.settings_draft = RuntimeSettingsDraft::from_config(&app.gui_config);
            println!("UI_TRAY_OK: activation, state preservation, watcher loss and recovery");
            return Task::done(Message::Smoke(Action::Next));
        }
        _ => return after(100, Action::TrayStep),
    }
    runner.tray_attempts = 0;
    if let Some(command) = command {
        let Some(control) = &runner.control else {
            return fail("missing tray control directory");
        };
        let temp_path = control.join("command.tmp");
        let target_path = control.join("command");
        if let Err(error) = std::fs::write(&temp_path, command)
            .and_then(|()| std::fs::rename(&temp_path, &target_path))
        {
            return fail(error);
        }
    }
    if close {
        runner.window_opened = false;
        runner.previous_window = app.window_id;
        let Some(id) = app.window_id else {
            return fail("missing open test window");
        };
        return app
            .handle_update(Message::WindowCloseRequested(id))
            .chain(after(150, Action::TrayStep));
    }
    after(100, Action::TrayStep)
}

pub fn update(app: &mut App, action: Action) -> Task<Message> {
    match action {
        Action::TrayStep => test_tray(app),
        Action::Next => {
            let Some(runner) = &app.smoke else {
                return Task::none();
            };
            let Some(case) = runner.cases.get(runner.index).cloned() else {
                if runner.no_tray {
                    if app.tray.as_ref().is_some_and(crate::tray::Tray::available) {
                        return fail("tray reported available without a host");
                    }
                    println!("UI_TRAY_NONE_OK: close exits when no tray host is available");
                    println!("UI_SMOKE_OK: {} screenshots", runner.index);
                    if let Some(id) = app.window_id {
                        return app.handle_update(Message::WindowCloseRequested(id));
                    }
                    return iced::exit();
                }
                println!("UI_SMOKE_OK: {} screenshots", runner.index);
                return iced::exit();
            };
            seed(app, &case);
            // Winit on Wayland can return a synchronous resize without an
            // event for Iced to consume. A new surface guarantees the actual
            // requested viewport and also exercises our tray restore path.
            if app.window_size != case.size {
                app.window_size = case.size;
                let close = app
                    .window_id
                    .take()
                    .map(window::close)
                    .unwrap_or_else(Task::none);
                return close.chain(app.open_main_window());
            }
            let Some(id) = app.window_id else {
                return app.open_main_window();
            };
            Task::batch([window::resize(id, case.size), after(200, Action::Prepare)])
        }
        Action::Prepare => {
            use iced::widget::operation::{self, AbsoluteOffset};
            let runner = app.smoke.as_ref().expect("smoke runner");
            let case = &runner.cases[runner.index];
            let scroll = match (case.tab, case.variant) {
                (_, "bottom" | "preview") => operation::snap_to_end("page-body"),
                (Tab::Profiles, "edit" | "delete" | "updating" | "more") => {
                    operation::scroll_to("page-body", AbsoluteOffset { x: 0.0, y: 288.0 })
                }
                (Tab::Settings, "invalid") => {
                    operation::scroll_to("page-body", AbsoluteOffset { x: 0.0, y: 900.0 })
                }
                _ => operation::scroll_to("page-body", AbsoluteOffset { x: 0.0, y: 0.0 }),
            };
            scroll.chain(after(250, Action::Capture))
        }
        Action::Capture => app
            .window_id
            .map(|id| window::screenshot(id).map(|shot| Message::Smoke(Action::Captured(shot))))
            .unwrap_or_else(Task::none),
        Action::Captured(shot) => {
            let Some(runner) = &app.smoke else {
                return Task::none();
            };
            let path = runner
                .output
                .join(format!("{}.png", runner.cases[runner.index].name));
            let expected = runner.cases[runner.index].size;
            let actual = Size::new(
                shot.size.width as f32 / shot.scale_factor,
                shot.size.height as f32 / shot.scale_factor,
            );
            if (actual.width - expected.width).abs() > 0.5
                || (actual.height - expected.height).abs() > 0.5
            {
                return fail(format!("expected {expected:?}, captured {actual:?}"));
            }
            if let Ok(scale) = std::env::var("SINGBOX_UI_SMOKE_SCALE")
                && scale.parse::<f32>().ok() != Some(shot.scale_factor)
            {
                return fail(format!(
                    "expected {scale}x scaling, got {}",
                    shot.scale_factor
                ));
            }
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        std::fs::create_dir_all(
                            path.parent().ok_or("Missing screenshot directory")?,
                        )
                        .map_err(|e| e.to_string())?;
                        if shot.size.width == 0 || shot.size.height == 0 {
                            return Err("Empty screenshot".into());
                        }
                        use image::ImageEncoder;
                        use image::codecs::png::{CompressionType, FilterType, PngEncoder};
                        use std::io::Write;
                        let mut file = std::io::BufWriter::new(
                            std::fs::File::create(&path).map_err(|e| e.to_string())?,
                        );
                        // Fast lossless encoding keeps the full HiDPI matrix
                        // practical even in unoptimized development builds.
                        PngEncoder::new_with_quality(
                            &mut file,
                            CompressionType::Fast,
                            FilterType::Sub,
                        )
                        .write_image(
                            shot.as_ref(),
                            shot.size.width,
                            shot.size.height,
                            image::ExtendedColorType::Rgba8,
                        )
                        .map_err(|e| e.to_string())?;
                        file.flush().map_err(|e| e.to_string())
                    })
                    .await
                    .map_err(|e| e.to_string())?
                },
                |result| Message::Smoke(Action::Saved(result)),
            )
        }
        Action::Saved(result) => {
            if let Err(error) = result {
                eprintln!("UI_SMOKE_FAILED: {error}");
                return iced::exit();
            }
            if let Some(runner) = &mut app.smoke {
                runner.index += 1;
            }
            Task::done(Message::Smoke(Action::Next))
        }
    }
}

fn seed(app: &mut App, case: &Case) {
    app.current_tab = case.tab;
    app.gui_config = GuiConfig {
        language: case.language,
        theme: case.theme,
        ..GuiConfig::default()
    };
    app.settings_draft = RuntimeSettingsDraft::from_config(&app.gui_config);
    app.settings_errors.clear();
    app.core_running = case.variant != "empty";
    app.core_installed = app.core_running;
    app.core_version = app.core_running.then(|| crate::core::CORE_VERSION.into());
    app.sys_proxy_enabled = app.core_running;
    app.system_proxy_busy = false;
    app.core_stopping = false;
    app.current_speed = Bandwidth {
        up: 385_024,
        down: 6_724_096,
    };
    app.speed_history = (0..30)
        .map(|i| {
            (
                (i * 31_337 % 700_000) as u64,
                (i * 725_711 % 9_000_000) as u64,
            )
        })
        .collect();
    app.total_uploaded = 153_429_120;
    app.total_downloaded = 3_368_902_144;
    app.active_profile_nodes.clear();
    app.proxy_groups.clear();
    app.active_connections.clear();
    app.log_lines.clear();
    app.selected_node_tag = None;
    app.selected_group.clear();
    app.pending_routing_mode = None;
    app.url_input.clear();
    app.node_search.clear();
    app.log_search.clear();
    app.log_filter = LogFilter::All;
    app.logs_follow = true;
    app.connections_search.clear();
    app.connections_sort = ConnectionSort::None;
    app.connections_sort_desc = false;
    app.proxy_sort = ProxySort::Latency;
    app.direct_rule_input.clear();
    app.proxy_rule_input.clear();
    app.confirm_delete_profile_id = None;
    app.editing_profile_id = None;
    app.editing_profile_name.clear();
    app.editing_profile_url.clear();
    app.profile_more_id = None;
    app.toasts.clear();
    app.core_starting = false;
    app.core_install_state = CoreInstallState::Idle;
    app.update_status = UpdateStatus::NotChecked;
    app.latency_testing = false;
    app.downloading = false;
    app.updating_profile_id = None;
    app.profile_error = None;
    app.active_profile_is_native = false;
    app.show_all_proxies = false;
    app.config_preview_expanded = false;
    app.config_preview = None;
    app.pending_restart_after_save = false;
    app.pending_proxy_reapply_after_save = false;
    app.pending_settings_save_feedback = false;
    app.pending_proxy_disable_context = None;
    app.pending_proxy_enable_after_recovery = false;
    app.deleting_profile_ids.clear();
    app.pending_auto_updates.clear();
    app.auto_update_failures.clear();
    app.pending_core_restart = false;
    app.force_stop_after_start = false;
    app.autostart_busy = false;
    app.settings_validation_in_flight = false;
    app.detached_core_running = false;
    app.pending_exit = false;
    app.pending_update = None;
    app.clear_url_input_after_download = false;
    if case.variant == "empty" {
        return;
    }

    for (i, name) in [
        "日常订阅 · Personal network",
        "备用线路 / A very long subscription name for international travel",
        "本地配置 · Local configuration",
    ]
    .into_iter()
    .enumerate()
    {
        let is_local = i == 2;
        app.gui_config.subscriptions.push(Profile {
            id: format!("fixture-{i}"),
            name: name.into(),
            url: if is_local {
                "/home/user/profiles/local-nodes.yaml".into()
            } else {
                "https://example.com/subscription?token=example-redacted-token".into()
            },
            file_path: if is_local {
                "/home/user/profiles/local-nodes.yaml".into()
            } else {
                String::new()
            },
            is_subscription: !is_local,
            updated_at: "2026-09-06 14:30:00".into(),
            traffic_upload: (!is_local).then_some(4_500_000_000),
            traffic_download: (!is_local).then_some(37_800_000_000),
            traffic_total: (!is_local).then_some(200_000_000_000),
            expire_at: (!is_local).then_some(2_051_222_400),
        });
    }
    app.gui_config.active_profile_id = Some("fixture-0".into());
    app.gui_config.custom_bypass_domains = vec![
        "localhost".into(),
        "example.cn".into(),
        "very-long-intranet-service-name.example.internal".into(),
    ];
    app.gui_config.custom_proxy_domains = vec!["example.com".into(), "github.com".into()];
    app.gui_config.custom_bypass_ips = vec!["192.168.0.0/16".into(), "10.0.0.0/8".into()];
    app.gui_config.custom_proxy_ips = vec!["1.1.1.1/32".into()];
    let names = [
        "香港 Hong Kong · Premium 01",
        "日本 Tokyo · 日常线路",
        "Singapore / 高速专线",
        "United States - Los Angeles - Very long node name",
        "德国 Frankfurt · Hysteria2",
        "自动选择 Auto",
        "直连 DIRECT",
        "新加坡备用线路 · Singapore backup",
    ];
    for (i, name) in names.into_iter().enumerate() {
        let (node_type, server, port) = match i {
            5 => ("urltest", String::new(), 0),
            6 => ("direct", String::new(), 0),
            _ => (
                if i % 2 == 0 { "vless" } else { "hysteria2" },
                format!("node-{i}.example.com"),
                443,
            ),
        };
        app.active_profile_nodes.push(ProxyNode {
            name: name.into(),
            node_type: node_type.into(),
            server,
            port,
            latency: [
                Some(42),
                Some(68),
                Some(108),
                Some(230),
                Some(9999),
                Some(18),
                Some(0),
                None,
            ][i],
            selectable: true,
        });
    }
    app.selected_node_tag = Some(names[0].into());
    app.selected_group = "Proxy".into();
    for (name, kind) in [
        ("Proxy", "Selector"),
        ("自动选择 Auto", "URLTest"),
        ("Streaming / 流媒体", "Selector"),
    ] {
        app.proxy_groups.insert(
            name.into(),
            api::ProxyInfo {
                name: name.into(),
                proxy_type: kind.into(),
                udp: Some(true),
                history: None,
                now: Some(names[0].into()),
                all: Some(names.iter().map(|s| (*s).into()).collect()),
            },
        );
    }
    for i in 0..12 {
        app.active_connections.push(api::Connection {
            id: format!("connection-{i}"),
            metadata: api::ConnectionMetadata {
                network: if i % 3 == 0 { "udp" } else { "tcp" }.into(),
                host: if i % 3 == 0 {
                    "very-long-hostname-for-a-streaming-service.example.com"
                } else {
                    "github.com"
                }
                .into(),
                destination_ip: "192.0.2.1".into(),
                destination_port: "443".into(),
                process_path: Some("/usr/bin/firefox".into()),
                ..Default::default()
            },
            upload: 42_000 * i,
            download: 721_033 * (i + 1),
            start: "2026-09-06T14:30:00+08:00".into(),
            chains: vec!["Proxy".into(), names[0].into()],
            rule: "DomainSuffix".into(),
        });
    }
    for i in 0..20 {
        app.log_lines.push_back(format!("14:30:{i:02} INFO [tcp] outbound/vless[香港 Hong Kong · Premium 01]: connection to example.com:443"));
    }
    app.log_lines.push_back(
        "14:30:21 WARN DNS request timed out; retrying alternate resolver / DNS 查询超时，正在重试"
            .into(),
    );
    app.log_lines.push_back(
        "14:30:22 ERROR connection refused: upstream.example.com:443 / 上游连接失败".into(),
    );
    match case.variant {
        "delete" => app.confirm_delete_profile_id = Some("fixture-0".into()),
        "edit" => {
            app.editing_profile_id = Some("fixture-0".into());
            app.editing_profile_name = app.gui_config.subscriptions[0].name.clone();
            app.editing_profile_url = app.gui_config.subscriptions[0].url.clone();
        }
        "invalid" if case.tab == Tab::Rules => {
            app.direct_rule_input = "https://bad rule".into();
        }
        "invalid" => {
            app.settings_draft.mixed_port = "9090".into();
            app.settings_draft.tun_mode = true;
            app.settings_draft.dns_server_remote = "invalid resolver".into();
        }
        "preview" => {
            app.config_preview_expanded = true;
            app.config_preview = Some(
                serde_json::to_string_pretty(&serde_json::json!({
                    "log": {"level": "info"},
                    "dns": {"servers": [{"type": "udp", "server": "223.5.5.5"}]},
                    "inbounds": [{"type": "mixed", "listen": "127.0.0.1", "listen_port": 2080}],
                    "outbounds": [{"type": "direct", "tag": "直连 / Direct"}]
                }))
                .expect("fixture JSON"),
            );
        }
        "no-matches" => {
            app.node_search = "no-matching-node".into();
            app.log_search = "no-matching-log".into();
            app.connections_search = "no-matching-connection".into();
        }
        "toasts" => {
            app.toasts.push(Toast::success(crate::ui::i18n::tr(
                case.language,
                "toast_sub_ok",
            )));
            app.toasts.push(Toast::error(app.tr_detail(
                "profile_download_failed",
                crate::ui::i18n::localize_error(
                    case.language,
                    "Download failed with status: 404 Not Found",
                ),
            )));
        }
        "starting" => {
            app.core_running = false;
            app.core_starting = true;
            app.sys_proxy_enabled = false;
            app.active_connections.clear();
            app.current_speed = Bandwidth::default();
        }
        "stopping" => {
            app.core_running = false;
            app.core_stopping = true;
            app.sys_proxy_enabled = false;
            app.active_connections.clear();
            app.current_speed = Bandwidth::default();
        }
        "mode-switching" => {
            app.pending_routing_mode = Some(RoutingMode::Global);
        }
        "installing" => {
            app.core_running = false;
            app.core_installed = false;
            app.core_version = None;
            app.sys_proxy_enabled = false;
            app.active_connections.clear();
            app.current_speed = Bandwidth::default();
            app.core_install_state = CoreInstallState::Downloading;
        }
        "core-error" => {
            app.core_running = false;
            app.sys_proxy_enabled = false;
            app.active_connections.clear();
            app.current_speed = Bandwidth::default();
            app.toasts.push(Toast::error(app.tr_detail(
                "core_unexpected_exit",
                crate::ui::i18n::localize_error(
                    case.language,
                    "bind 127.0.0.1:2080: address already in use",
                ),
            )));
        }
        "updating" => {
            app.downloading = true;
            app.updating_profile_id = Some("fixture-0".into());
        }
        "more" => {
            app.profile_more_id = Some("fixture-0".into());
        }
        "pending" => app.settings_draft.mixed_port = "7890".into(),
        "update" => {
            app.update_status = UpdateStatus::NewVersion {
                tag: "v2026.12.1".into(),
                download_url: Some("https://example.com/sing-box-gui".into()),
                sha256: Some("0".repeat(64)),
                size: Some(9_437_184),
            }
        }
        "update-downloading" => {
            app.update_status = UpdateStatus::Downloading {
                tag: "v2026.12.1".into(),
            };
        }
        "testing" => {
            app.latency_testing = true;
            for node in &mut app.active_profile_nodes {
                node.latency = None;
            }
        }
        "error" => {
            app.profile_error = Some(app.tr_detail(
                "profile_download_failed",
                crate::ui::i18n::localize_error(
                    case.language,
                    "Profile contained no parseable proxy nodes (unsupported schemes or bad links).",
                ),
            ));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_cases_have_unique_names_and_cover_all_tabs() {
        let cases = Runner::build_cases();
        assert!(!cases.is_empty());
        let unique: std::collections::HashSet<&str> =
            cases.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(unique.len(), cases.len(), "duplicate smoke case names");
        for tab in [
            Tab::Dashboard,
            Tab::Proxies,
            Tab::Profiles,
            Tab::Rules,
            Tab::Connections,
            Tab::Logs,
            Tab::Settings,
        ] {
            assert!(cases.iter().any(|c| c.tab == tab), "missing tab {tab:?}");
        }
    }

    #[test]
    fn seed_resets_stale_state_and_renders_all_smoke_cases() {
        let (mut app, startup) = App::new_with_config(GuiConfig::default());
        drop(startup);
        let dummy_window = app.window_id.unwrap_or_else(window::Id::unique);
        let cases = Runner::build_cases();

        // Pollute transient UI fields, then verify seeding an empty case resets them.
        app.url_input = "stale".into();
        app.direct_rule_input = "stale.com".into();
        app.proxy_rule_input = "proxy.com".into();
        app.selected_group = "StaleGroup".into();
        app.log_filter = LogFilter::Error;
        app.connections_sort = ConnectionSort::Upload;
        app.connections_sort_desc = true;
        app.proxy_sort = ProxySort::Name;
        app.core_stopping = true;
        app.system_proxy_busy = true;
        app.pending_restart_after_save = true;
        app.pending_proxy_reapply_after_save = true;
        app.pending_exit = true;
        app.deleting_profile_ids.insert("stale".into());

        let empty_case = cases
            .iter()
            .find(|c| c.variant == "empty")
            .expect("empty case");
        seed(&mut app, empty_case);
        assert!(app.url_input.is_empty());
        assert!(app.direct_rule_input.is_empty());
        assert!(app.proxy_rule_input.is_empty());
        assert!(app.selected_group.is_empty());
        assert_eq!(app.log_filter, LogFilter::All);
        assert_eq!(app.connections_sort, ConnectionSort::None);
        assert!(!app.connections_sort_desc);
        assert_eq!(app.proxy_sort, ProxySort::Latency);
        assert!(!app.core_stopping);
        assert!(!app.system_proxy_busy);
        assert!(!app.pending_restart_after_save);
        assert!(!app.pending_proxy_reapply_after_save);
        assert!(!app.pending_exit);
        assert!(app.deleting_profile_ids.is_empty());

        let filled_case = cases
            .iter()
            .find(|c| c.variant == "filled")
            .expect("filled case");
        seed(&mut app, filled_case);
        let local = &app.gui_config.subscriptions[2];
        assert!(!local.is_subscription);
        assert!(local.traffic_total.is_none());
        assert!(local.expire_at.is_none());
        assert!(app.active_profile_nodes.iter().any(|n| n.latency.is_none()));

        for case in &cases {
            seed(&mut app, case);
            let _element = app.view(dummy_window);
        }
    }
}
