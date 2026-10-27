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
    pub fn from_env() -> Option<Self> {
        let output = PathBuf::from(std::env::var_os("SINGBOX_UI_SMOKE_DIR")?);
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
                    (Tab::Connections, "empty"),
                    (Tab::Logs, "empty"),
                    (Tab::Profiles, "delete"),
                    (Tab::Profiles, "edit"),
                    (Tab::Settings, "invalid"),
                    (Tab::Logs, "no-matches"),
                    (Tab::Proxies, "no-matches"),
                    (Tab::Settings, "bottom"),
                    (Tab::Settings, "preview"),
                    (Tab::Profiles, "bottom"),
                    (Tab::Dashboard, "bottom"),
                    (Tab::Rules, "bottom"),
                ] {
                    cases.push(Case {
                        name: format!("{locale}-{appearance}-640-{tab:?}-{variant}"),
                        tab,
                        language,
                        theme,
                        size: Size::new(640.0, 480.0),
                        variant,
                    });
                }
            }
        }
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
            runner.tray_stage = TrayStage::Hidden;
            close = true;
        }
        TrayStage::Hidden if app.window_id.is_none() => {
            runner.tray_stage = TrayStage::Restored;
            command = Some("activate");
        }
        TrayStage::Restored if runner.window_opened && app.window_id != runner.previous_window => {
            if app.url_input != "unsaved import survives closing the window" {
                return fail("window recreation lost the unsaved form");
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
        if let Err(error) = std::fs::write(control.join("command"), command) {
            return fail(error);
        }
    }
    if close {
        runner.window_opened = false;
        runner.previous_window = app.window_id;
        let id = app.window_id.expect("open test window");
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
                    return app.handle_update(Message::WindowCloseRequested(
                        app.window_id.expect("open test window"),
                    ));
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
            let scroll = match case.variant {
                "bottom" | "preview" => operation::snap_to_end("page-body"),
                "edit" | "delete" => {
                    operation::scroll_to("page-body", AbsoluteOffset { x: 0.0, y: 288.0 })
                }
                "invalid" => operation::scroll_to("page-body", AbsoluteOffset { x: 0.0, y: 900.0 }),
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
            if actual != expected {
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
    app.core_version = app.core_running.then(|| "1.13.14".into());
    app.sys_proxy_enabled = app.core_running;
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
    app.node_search.clear();
    app.log_search.clear();
    app.connections_search.clear();
    app.confirm_delete_profile_id = None;
    app.editing_profile_id = None;
    app.profile_more_id = None;
    app.toast = None;
    app.config_preview_expanded = false;
    app.config_preview = None;
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
        app.gui_config.subscriptions.push(Profile {
            id: format!("fixture-{i}"),
            name: name.into(),
            url: "https://example.com/subscription?token=example-redacted-token".into(),
            file_path: String::new(),
            is_subscription: true,
            updated_at: "2026-09-06 14:30".into(),
            traffic_upload: Some(4_500_000_000),
            traffic_download: Some(37_800_000_000),
            traffic_total: Some(200_000_000_000),
            expire_at: Some(1_810_944_000),
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
        app.active_profile_nodes.push(ProxyNode {
            name: name.into(),
            node_type: if i % 2 == 0 { "vless" } else { "hysteria2" }.into(),
            server: format!("node-{i}.example.com"),
            port: 443,
            latency: Some([42, 68, 108, 230, 9999, 18, 0, 580][i]),
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
        }
        _ => {}
    }
}
