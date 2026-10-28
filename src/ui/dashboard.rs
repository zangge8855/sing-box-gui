use crate::message::Message;
use crate::state::{Bandwidth, GuiConfig, Language, Profile, RoutingMode, Tab};
use crate::ui::i18n::tr;
use crate::ui::page_header;
use crate::ui::theme;
use crate::ui::util::{
    format_size, format_speed, labelled_size, traffic_usage_ratio, truncate_width,
};
use iced::widget::{
    Space, button, column, container, responsive, row, stack, text, toggler, tooltip,
};
use iced::{Alignment, Color, Element, Length};

fn icon(unicode: char, size: f32) -> text::Text<'static> {
    text(unicode.to_string())
        .font(iced::Font::with_name("Material Icons"))
        .size(size)
}

/// Splits "1.24 MB/s" into the figure and its unit so the unit can sit
/// quietly beside a large number.
fn split_figure(formatted: &str) -> (String, String) {
    match formatted.rsplit_once(' ') {
        Some((value, unit)) => (value.to_string(), unit.to_string()),
        None => (formatted.to_string(), String::new()),
    }
}

/// Smallest page height at which the dashboard fits without scrolling, so the
/// traffic chart can take whatever is left instead of leaving a blank tail.
fn fill_height_threshold(tun_banner: bool, profile_strip: bool) -> f32 {
    let mut h = 590.0;
    if tun_banner {
        h += 57.0;
    }
    if profile_strip {
        h += 88.0;
    }
    h
}

#[allow(clippy::too_many_arguments)]
pub fn render<'a>(
    gui_config: &'a GuiConfig,
    core_running: bool,
    core_starting: bool,
    core_stopping: bool,
    core_installed: bool,
    sys_proxy_enabled: bool,
    system_proxy_busy: bool,
    current_speed: &Bandwidth,
    speed_history: &[(u64, u64)],
    total_uploaded: u64,
    total_downloaded: u64,
    selected_node: Option<&'a str>,
    active_connections: usize,
    downloading: bool,
    updating_profile_id: Option<&'a str>,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;

    // Owned copies so the responsive closure can be re-run on resize.
    let theme_cloned = theme.clone();
    let speed = current_speed.clone();
    let history = speed_history.to_vec();
    let core_busy = core_starting || core_stopping;
    let has_profile = gui_config.active_profile_id.is_some();
    let active_profile = gui_config
        .active_profile_id
        .as_deref()
        .and_then(|id| gui_config.subscriptions.iter().find(|p| p.id == id));

    let main_content = responsive(move |size| {
        let is_compact = size.width < crate::ui::DASHBOARD_COMPACT_W;
        let pad_compact = crate::ui::is_pad_compact(size.width);
        let fill_height = !is_compact
            && size.height >= fill_height_threshold(gui_config.tun_mode, active_profile.is_some());
        let theme = &theme_cloned;
        let text_primary = theme::text_primary(theme);
        let text_muted = theme::text_muted(theme);
        let text_tertiary = theme::text_tertiary(theme);

        let content_w = crate::ui::page_content_width(size.width, pad_compact);
        let hero_pad = if is_compact {
            theme::CARD_PAD
        } else {
            theme::CARD_PAD + 4.0
        };
        let hero_inner = content_w - hero_pad * 2.0;
        let power_size = if is_compact { 56.0 } else { 64.0 };
        let controls_w = (hero_inner * 0.42).clamp(280.0, 360.0);
        let identity_w = if is_compact {
            hero_inner - power_size - crate::ui::SP_16
        } else {
            hero_inner - power_size - crate::ui::SP_20 * 3.0 - 1.0 - controls_w
        };

        // ── Hero: power control, status, current node ─────────────────────
        let (dot_color, dot_active, status_label, status_text_color) = if core_starting {
            (
                theme::WARNING,
                true,
                tr(lang, "status_starting"),
                theme::warning_text(theme),
            )
        } else if core_stopping {
            (
                theme::WARNING,
                true,
                tr(lang, "status_stopping"),
                theme::warning_text(theme),
            )
        } else if core_running {
            (
                theme::SUCCESS,
                true,
                tr(lang, "status_running"),
                theme::success_text(theme),
            )
        } else {
            (text_tertiary, false, tr(lang, "status_stopped"), text_muted)
        };
        let status_line = row![
            crate::ui::status_dot(
                dot_color,
                dot_active,
                status_label,
                status_text_color,
                theme::TYPE_BTN_SM,
            ),
            text(tr(lang, "singbox_core"))
                .size(theme::TYPE_CAPTION)
                .color(text_tertiary),
        ]
        .spacing(crate::ui::SP_12)
        .align_y(Alignment::Center);

        let can_start = has_profile && core_installed;
        let mut power = button(
            container(icon(
                crate::ui::icons::ICON_POWER,
                if is_compact { 24.0 } else { 28.0 },
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
        )
        .width(power_size)
        .height(power_size)
        .padding(0)
        .style(theme::power_button(core_running));
        if !core_busy && (core_running || can_start) {
            power = power.on_press(Message::ToggleCore);
        }
        let power_hint = if core_starting {
            tr(lang, "status_starting")
        } else if core_stopping {
            tr(lang, "status_stopping")
        } else if core_running {
            tr(lang, "btn_stop_core")
        } else {
            tr(lang, "btn_start_core")
        };
        let power: Element<'_, Message> = tooltip(
            power,
            container(
                text(power_hint)
                    .size(theme::TYPE_BTN_SM)
                    .color(text_primary),
            )
            .padding([6, 10])
            .style(theme::popover_bg),
            tooltip::Position::Bottom,
        )
        .into();

        // The headline: current node, or what to do next.
        let (headline, headline_color, subline) = if !has_profile {
            (
                tr(lang, "empty_dashboard_title").to_string(),
                text_primary,
                Some(tr(lang, "empty_dashboard_desc")),
            )
        } else if let Some(node) = selected_node {
            (node.to_string(), text_primary, None)
        } else {
            (tr(lang, "dash_no_node").to_string(), text_muted, None)
        };
        // Size and truncation follow the room actually left beside the
        // controls, so a long node name never pushes them off the card.
        let headline_size = if is_compact || identity_w < 340.0 {
            theme::TYPE_TITLE
        } else {
            theme::TYPE_DISPLAY
        };
        let headline_cols = ((identity_w - 16.0) / (headline_size * 0.6)).max(8.0) as usize;
        let headline_short = truncate_width(&headline, headline_cols);
        let headline_truncated = headline_short != headline;
        let headline_text = text(headline_short)
            .size(headline_size)
            .font(theme::display_font())
            .shaping(text::Shaping::Advanced)
            .wrapping(text::Wrapping::None)
            .color(headline_color);
        let headline_el: Element<'_, Message> = if has_profile {
            let btn = button(headline_text)
                .padding([2, 8])
                .style(theme::button_ghost)
                .on_press(Message::TabChanged(Tab::Proxies));
            if headline_truncated {
                tooltip(
                    btn,
                    container(text(headline).size(theme::TYPE_BTN_SM).color(text_primary))
                        .padding([6, 10])
                        .style(theme::popover_bg),
                    tooltip::Position::Bottom,
                )
                .into()
            } else {
                btn.into()
            }
        } else {
            container(headline_text).padding([2, 8]).into()
        };

        // Quiet facts beneath the headline: label in grey, value in ink.
        let mut facts = row![].spacing(crate::ui::SP_16).align_y(Alignment::Center);
        if let Some(desc) = subline {
            facts = facts.push(text(desc).size(theme::TYPE_BTN_SM).color(text_muted));
        } else {
            facts = facts.push(
                button(
                    row![
                        text(tr(lang, "dash_connections"))
                            .size(theme::TYPE_CAPTION)
                            .color(text_tertiary),
                        text(active_connections.to_string())
                            .size(theme::TYPE_BTN_SM)
                            .font(theme::metric_font())
                            .color(text_primary),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .padding([2, 8])
                .style(theme::button_ghost)
                .on_press(Message::TabChanged(Tab::Connections)),
            );
            if gui_config.tun_mode {
                facts = facts.push(
                    container(
                        text("TUN")
                            .size(theme::TYPE_MICRO)
                            .font(theme::ui_font(iced::font::Weight::Semibold))
                            .color(theme::badge_text(theme::INFO, theme)),
                    )
                    .padding([2, 6])
                    .style(|t| theme::badge_colored(theme::INFO, t)),
                );
            }
        }

        let mut identity = column![
            container(status_line).padding([0, 8]),
            headline_el,
            container(facts).padding(iced::Padding {
                left: if subline.is_some() { 8.0 } else { 0.0 },
                ..Default::default()
            }),
        ]
        .spacing(6)
        .width(Length::Fill);

        // A clear next step when the power control cannot act yet.
        let cta_key = if core_running || core_busy {
            None
        } else if !has_profile {
            Some(("btn_goto_profiles", Tab::Profiles))
        } else if !core_installed {
            Some(("btn_goto_settings", Tab::Settings))
        } else {
            None
        };
        if let Some((key, tab)) = cta_key {
            identity = identity.push(
                container(
                    button(text(tr(lang, key)).size(theme::TYPE_BTN_MD))
                        .padding(theme::BTN_PAD_MD)
                        .style(theme::button_primary)
                        .on_press(Message::TabChanged(tab)),
                )
                .padding(iced::Padding {
                    top: 6.0,
                    left: 8.0,
                    ..Default::default()
                }),
            );
        }

        // ── Controls: routing mode and system proxy, beside the power ─────
        let label_w = match lang {
            Language::Zh => 60.0,
            Language::En => 96.0,
        };
        let control_label = |key: &'static str| {
            text(tr(lang, key))
                .size(theme::TYPE_BTN_SM)
                .color(text_muted)
                .width(Length::Fixed(label_w))
        };
        let current_mode = gui_config.routing_mode;
        let segment = |mode: RoutingMode, key: &'static str, desc_key: &'static str| {
            let active = mode == current_mode;
            let mut b = button(
                text(tr(lang, key))
                    .size(theme::TYPE_BTN_SM)
                    .font(theme::ui_font(if active {
                        iced::font::Weight::Semibold
                    } else {
                        iced::font::Weight::Normal
                    }))
                    .align_x(Alignment::Center)
                    .width(Length::Fill),
            )
            .width(Length::FillPortion(1))
            .padding([6, 8])
            .style(theme::button_tab(active));
            if !active {
                b = b.on_press(Message::RoutingModeChanged(mode));
            }
            tooltip(
                b,
                container(
                    text(tr(lang, desc_key))
                        .size(theme::TYPE_CAPTION)
                        .color(text_primary),
                )
                .padding([6, 10])
                .max_width(260)
                .style(theme::popover_bg),
                tooltip::Position::Bottom,
            )
            .gap(6)
        };
        let routing_row = row![
            control_label("active_mode"),
            container(
                row![
                    segment(RoutingMode::Rule, "mode_rules", "mode_rules_desc"),
                    segment(RoutingMode::Global, "mode_global", "mode_global_desc"),
                    segment(RoutingMode::Direct, "mode_direct", "mode_direct_desc"),
                ]
                .spacing(3),
            )
            .padding(3)
            .width(Length::Fill)
            .style(theme::segmented_track),
        ]
        .spacing(crate::ui::SP_12)
        .align_y(Alignment::Center);

        let mut proxy_switch = toggler(sys_proxy_enabled)
            .size(20)
            .style(theme::toggler_style);
        if !system_proxy_busy {
            proxy_switch = proxy_switch.on_toggle(|_| Message::ToggleSystemProxy);
        }
        let (proxy_state, proxy_state_color) = if system_proxy_busy {
            (tr(lang, "proxy_applying"), theme::warning_text(theme))
        } else if sys_proxy_enabled {
            (tr(lang, "enabled"), theme::success_text(theme))
        } else {
            (tr(lang, "disabled"), text_muted)
        };
        let proxy_switch: Element<'_, Message> = tooltip(
            proxy_switch,
            container(
                text(tr(lang, "sys_proxy_desc"))
                    .size(theme::TYPE_CAPTION)
                    .color(text_primary),
            )
            .padding([6, 10])
            .max_width(260)
            .style(theme::popover_bg),
            tooltip::Position::Bottom,
        )
        .gap(6)
        .into();
        let proxy_row = row![
            control_label("system_proxy"),
            text(proxy_state)
                .size(theme::TYPE_BTN_SM)
                .font(theme::ui_font(iced::font::Weight::Medium))
                .color(proxy_state_color),
            text(format!("127.0.0.1:{}", gui_config.mixed_port))
                .size(theme::TYPE_CAPTION)
                .font(theme::metric_font())
                .color(text_tertiary),
            Space::new().width(Length::Fill),
            proxy_switch,
        ]
        .spacing(crate::ui::SP_12)
        .height(Length::Fixed(35.0))
        .align_y(Alignment::Center);

        let controls = column![routing_row, proxy_row].spacing(crate::ui::SP_12);

        let mut hero_col = column![].spacing(crate::ui::SP_16);
        if is_compact {
            hero_col = hero_col
                .push(
                    row![power, identity]
                        .spacing(crate::ui::SP_16)
                        .align_y(Alignment::Center),
                )
                .push(
                    container(Space::new())
                        .width(Length::Fill)
                        .height(1)
                        .style(theme::hairline),
                )
                .push(controls.width(Length::Fill));
        } else {
            // The hairline is Fill-tall inside a Shrink row, so it spans
            // exactly the taller of the identity and control blocks.
            hero_col = hero_col.push(
                row![
                    power,
                    identity,
                    container(Space::new())
                        .width(1)
                        .height(Length::Fill)
                        .style(theme::hairline),
                    controls.width(Length::Fixed(controls_w)),
                ]
                .spacing(crate::ui::SP_20)
                .height(Length::Shrink)
                .align_y(Alignment::Center),
            );
        }
        if core_starting {
            hero_col = hero_col.push(crate::ui::loading_row(
                tr(lang, "core_starting_hint"),
                theme,
            ));
        }
        let hero = container(hero_col)
            .padding(hero_pad)
            .width(Length::Fill)
            .style(theme::hero_panel(core_running));

        // ── Traffic: two large figures over a live chart ──────────────────
        let (down_color, up_color) = theme::chart_series(theme);
        let figure_block =
            |label_key: &'static str, arrow: char, series: Color, rate: u64, total: u64| {
                let (value, unit) = split_figure(&format_speed(rate));
                column![
                    row![
                        icon(arrow, crate::ui::ICON_SIZE).color(series),
                        text(tr(lang, label_key))
                            .size(theme::TYPE_BTN_SM)
                            .color(text_muted),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                    row![
                        text(value)
                            .size(if is_compact {
                                theme::TYPE_TITLE
                            } else {
                                theme::TYPE_DISPLAY
                            })
                            .font(theme::display_font())
                            .color(text_primary),
                        text(unit).size(theme::TYPE_BTN_SM).color(text_muted),
                    ]
                    .spacing(6)
                    .align_y(Alignment::End),
                    text(format!(
                        "{} {}",
                        tr(lang, "total_label"),
                        format_size(total)
                    ))
                    .size(theme::TYPE_CAPTION)
                    .color(text_tertiary),
                ]
                .spacing(4)
                .width(Length::FillPortion(1))
            };
        let figures = row![
            figure_block(
                "download",
                crate::ui::icons::ICON_DOWN,
                down_color,
                speed.down,
                total_downloaded,
            ),
            figure_block(
                "upload",
                crate::ui::icons::ICON_UP,
                up_color,
                speed.up,
                total_uploaded,
            ),
        ]
        .spacing(crate::ui::SP_20)
        .width(Length::Fill);

        let legend_swatch = |color: Color| {
            container(Space::new())
                .width(10)
                .height(3)
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(color)),
                    border: iced::Border {
                        radius: 1.5.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
        };
        let peak = history.iter().map(|&(u, d)| u.max(d)).max().unwrap_or(0);
        let mut chart_header = row![
            text(tr(lang, "speed_history_chart"))
                .size(theme::TYPE_HEADING)
                .font(theme::ui_font(iced::font::Weight::Semibold))
                .color(text_primary),
            Space::new().width(Length::Fill),
        ]
        .spacing(crate::ui::SP_12)
        .align_y(Alignment::Center);
        if peak > 0 {
            chart_header = chart_header.push(
                text(format!("{} {}", tr(lang, "chart_peak"), format_speed(peak)))
                    .size(theme::TYPE_CAPTION)
                    .font(theme::metric_font())
                    .color(text_tertiary),
            );
        }
        chart_header = chart_header.push(
            row![
                legend_swatch(down_color),
                text(tr(lang, "chart_legend_down"))
                    .size(theme::TYPE_CAPTION)
                    .color(text_muted),
                Space::new().width(6),
                legend_swatch(up_color),
                text(tr(lang, "chart_legend_up"))
                    .size(theme::TYPE_CAPTION)
                    .color(text_muted),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );

        let idle = !core_running || history.iter().all(|&(u, d)| u == 0 && d == 0);
        let chart_el: Element<'_, Message> = if fill_height {
            // Measure the space the card leaves and draw the chart into it.
            let history = history.clone();
            let theme = theme.clone();
            responsive(move |chart| {
                traffic_chart(&history, chart.width, chart.height, idle, &theme, lang)
            })
            .into()
        } else {
            traffic_chart(
                &history,
                content_w - theme::CARD_PAD * 2.0,
                if is_compact { 120.0 } else { 148.0 },
                idle,
                theme,
                lang,
            )
        };

        let mut traffic = container(
            column![chart_header, figures, chart_el]
                .spacing(crate::ui::SP_16)
                .width(Length::Fill),
        )
        .padding(theme::CARD_PAD)
        .width(Length::Fill)
        .style(theme::card_bg);
        if fill_height {
            traffic = traffic.height(Length::Fill);
        }

        let mut content_col = column![].spacing(theme::GRID_GAP).width(Length::Fill);
        if gui_config.tun_mode {
            content_col = content_col.push(
                container(
                    text(tr(lang, "tun_admin_banner"))
                        .size(theme::TYPE_BTN_SM)
                        .color(theme::warning_text(theme)),
                )
                .padding(crate::ui::SP_12)
                .width(Length::Fill)
                .style(|t| theme::tinted_banner(t, theme::WARNING)),
            );
        }
        content_col = content_col.push(hero).push(traffic);
        if let Some(profile) = active_profile {
            content_col = content_col.push(profile_strip(
                profile,
                lang,
                theme,
                downloading,
                updating_profile_id == Some(profile.id.as_str()),
                is_compact,
            ));
        }

        let header = page_header("tab_dashboard", lang, None, theme, pad_compact);
        if fill_height {
            crate::ui::page_shell_fixed_with_pad(
                header,
                content_col.height(Length::Fill).into(),
                pad_compact,
            )
        } else {
            crate::ui::page_shell_with_pad(header, content_col.into(), pad_compact)
        }
    });

    main_content.into()
}

/// The waveform, with a quiet caption over it while nothing is flowing.
fn traffic_chart<'a>(
    history: &[(u64, u64)],
    width: f32,
    height: f32,
    idle: bool,
    theme: &iced::Theme,
    lang: Language,
) -> Element<'a, Message> {
    let chart = traffic_waveform(history, width, height.max(80.0), theme, lang);
    if !idle {
        return chart;
    }
    stack![
        chart,
        container(
            text(tr(lang, "chart_waiting_traffic"))
                .size(theme::TYPE_CAPTION)
                .color(theme::text_tertiary(theme)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center),
    ]
    .into()
}

/// The active subscription at a glance: which one, how much is left, when it
/// expires, and a one-click refresh — without a trip to the Profiles page.
fn profile_strip<'a>(
    profile: &'a Profile,
    lang: Language,
    theme: &iced::Theme,
    downloading: bool,
    updating: bool,
    is_compact: bool,
) -> Element<'a, Message> {
    let text_primary = theme::text_primary(theme);
    let text_muted = theme::text_muted(theme);
    let caption = |s: String| text(s).size(theme::TYPE_CAPTION).color(text_muted);

    let ident = column![
        container(
            text(tr(lang, "dash_current_profile"))
                .size(theme::TYPE_CAPTION)
                .color(theme::text_tertiary(theme)),
        )
        .padding([0, 6]),
        button(
            text(truncate_width(
                &profile.name,
                if is_compact { 30 } else { 24 }
            ))
            .size(theme::TYPE_BODY)
            .font(theme::ui_font(iced::font::Weight::Semibold))
            .shaping(text::Shaping::Advanced)
            .wrapping(text::Wrapping::None)
            .color(text_primary),
        )
        .padding([2, 6])
        .style(theme::button_ghost)
        .on_press(Message::TabChanged(Tab::Profiles)),
    ]
    .spacing(2);

    let expiry = profile
        .expire_at
        .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0).map(|dt| (ts, dt)))
        .map(|(ts, dt)| {
            let near = ts.saturating_sub(chrono::Utc::now().timestamp()) < 7 * 24 * 3600;
            text(format!(
                "{} {}",
                tr(lang, "expire_at_label"),
                dt.with_timezone(&chrono::Local).format("%Y-%m-%d")
            ))
            .size(theme::TYPE_CAPTION)
            .color(if near {
                theme::warning_text(theme)
            } else {
                text_muted
            })
        });

    let ratio = match (profile.traffic_upload, profile.traffic_download) {
        (Some(u), Some(d)) => traffic_usage_ratio(u, d, profile.traffic_total.unwrap_or(0)),
        _ => None,
    };
    let mut labels = row![].spacing(crate::ui::SP_12).align_y(Alignment::Center);
    match (profile.traffic_upload, profile.traffic_download) {
        (Some(u), Some(d)) => {
            let used = u.saturating_add(d);
            let total = profile.traffic_total.unwrap_or(0);
            if total > 0 {
                labels = labels
                    .push(
                        text(format!("{} / {}", format_size(used), format_size(total)))
                            .size(theme::TYPE_BTN_SM)
                            .font(theme::metric_font())
                            .color(text_primary),
                    )
                    .push(caption(labelled_size(
                        lang,
                        "traffic_left",
                        total.saturating_sub(used),
                    )));
            } else {
                labels = labels.push(caption(labelled_size(lang, "traffic_used", used)));
            }
        }
        _ => {
            labels = labels.push(caption(format!(
                "{} {}",
                tr(lang, "updated_at_label"),
                profile.updated_at
            )));
        }
    }
    labels = labels.push(Space::new().width(Length::Fill));
    if let Some(expiry) = expiry {
        labels = labels.push(expiry);
    }
    let mut usage = column![labels].spacing(8).width(Length::Fill);
    if let Some(ratio) = ratio {
        usage = usage.push(crate::ui::usage_bar(ratio, theme));
    }

    let update_btn = profile.is_subscription.then(|| {
        let mut b = button(
            text(tr(
                lang,
                if updating {
                    "btn_downloading"
                } else {
                    "btn_update"
                },
            ))
            .size(theme::TYPE_BTN_SM),
        )
        .padding(theme::BTN_PAD_SM)
        .style(theme::button_secondary);
        if !downloading {
            b = b.on_press(Message::UpdateSubscription(profile.id.clone()));
        }
        b
    });

    let body: Element<'a, Message> = if is_compact {
        let mut top = row![ident, Space::new().width(Length::Fill)].align_y(Alignment::Center);
        if let Some(b) = update_btn {
            top = top.push(b);
        }
        column![top, usage].spacing(crate::ui::SP_12).into()
    } else {
        let mut line = row![ident.width(Length::Fixed(200.0)), usage]
            .spacing(crate::ui::SP_20)
            .align_y(Alignment::Center);
        if let Some(b) = update_btn {
            line = line.push(b);
        }
        line.into()
    };

    container(body)
        .padding(theme::CARD_PAD_DENSE)
        .width(Length::Fill)
        .style(theme::card_bg)
        .into()
}

/// Splits the plot height around the 1px baseline: `(down_area, up_area,
/// px_per_byte)`. Both directions share one scale; the area each side gets
/// follows its peak, so a download-heavy window doesn't leave the upload half
/// as dead space. Each side keeps at least 22% so the quiet one stays legible.
fn waveform_split(peak_down: u64, peak_up: u64, height: f32) -> (f32, f32, f32) {
    // 2px breath between each bar and the baseline.
    const BREATH: f32 = 2.0;
    let usable = (height - 1.0 - BREATH * 2.0).max(2.0);
    let sum = peak_down + peak_up;
    let share = if sum == 0 {
        0.5
    } else {
        (peak_down as f32 / sum as f32).clamp(0.22, 0.78)
    };
    let down_reach = (usable * share).floor().max(1.0);
    let up_reach = (usable - down_reach).max(1.0);
    // 10 KB/s floor keeps idle noise flat.
    const FLOOR: f32 = 10.0 * 1024.0;
    let scale = (down_reach / (peak_down as f32).max(FLOOR * share))
        .min(up_reach / (peak_up as f32).max(FLOOR * (1.0 - share)));
    (down_reach + BREATH, up_reach + BREATH, scale)
}

/// Live traffic as a mirrored waveform: download rises above a hairline
/// baseline, upload hangs below it, on one shared scale. Built from plain
/// containers — crisp at any scale factor and with no geometry pipeline.
/// Older samples fade so the eye lands on "now" at the right edge.
fn traffic_waveform<'a>(
    history: &[(u64, u64)],
    chart_width: f32,
    height: f32,
    theme: &iced::Theme,
    lang: crate::state::Language,
) -> Element<'a, Message> {
    let (down_color, up_color) = theme::chart_series(theme);
    let text_primary = theme::text_primary(theme);
    let text_muted = theme::text_muted(theme);
    let text_tertiary = theme::text_tertiary(theme);

    let n = history.len().max(1);
    let peak_up = history.iter().map(|&(u, _)| u).max().unwrap_or(0);
    let peak_down = history.iter().map(|&(_, d)| d).max().unwrap_or(0);
    let slot = chart_width / n as f32;
    let gap = (slot * 0.35).clamp(2.0, 8.0);
    let height = height.floor();
    let (down_area, up_area, scale) = waveform_split(peak_down, peak_up, height);

    let bar = |value: u64, color: Color, alpha: f32, rising: bool| -> Element<'a, Message> {
        if value == 0 {
            return Space::new().height(0).into();
        }
        let reach = if rising { down_area } else { up_area } - 2.0;
        let h = (value as f32 * scale).clamp(2.0, reach.max(2.0));
        let r = (h / 2.0).min(3.0);
        let radius = if rising {
            iced::border::Radius {
                top_left: r,
                top_right: r,
                bottom_right: 0.0,
                bottom_left: 0.0,
            }
        } else {
            iced::border::Radius {
                top_left: 0.0,
                top_right: 0.0,
                bottom_right: r,
                bottom_left: r,
            }
        };
        container(Space::new())
            .width(Length::Fill)
            .height(h)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(theme::with_alpha(color, alpha))),
                border: iced::Border {
                    radius,
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    };

    let tip_row = |color: Color, label_key: &'static str, rate: u64| {
        row![
            container(Space::new())
                .width(8)
                .height(8)
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(color)),
                    border: iced::Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            text(tr(lang, label_key))
                .size(theme::TYPE_CAPTION)
                .color(text_muted)
                .width(Length::Fixed(44.0)),
            text(format_speed(rate))
                .size(theme::TYPE_CAPTION)
                .font(theme::metric_font())
                .color(text_primary),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
    };

    let mut bars = row![].spacing(gap).height(height).width(Length::Fill);
    for (i, &(up, down)) in history.iter().enumerate() {
        let recency = if n > 1 {
            i as f32 / (n - 1) as f32
        } else {
            1.0
        };
        let alpha = 0.5 + 0.5 * recency;
        let column_el = column![
            container(bar(down, down_color, alpha, true))
                .height(down_area)
                .width(Length::Fill)
                .align_y(Alignment::End)
                .padding(iced::Padding {
                    bottom: 2.0,
                    ..Default::default()
                }),
            Space::new().height(1),
            container(bar(up, up_color, alpha, false))
                .height(up_area)
                .width(Length::Fill)
                .align_y(Alignment::Start)
                .padding(iced::Padding {
                    top: 2.0,
                    ..Default::default()
                }),
        ]
        .width(Length::FillPortion(1));

        let secs_ago = n - 1 - i;
        let when = if secs_ago == 0 {
            tr(lang, "chart_now").to_string()
        } else {
            tr(lang, "chart_seconds_ago").replace("{}", &secs_ago.to_string())
        };
        let tip = container(
            column![
                text(when).size(theme::TYPE_MICRO).color(text_tertiary),
                tip_row(down_color, "chart_legend_down", down),
                tip_row(up_color, "chart_legend_up", up),
            ]
            .spacing(4),
        )
        .padding([8, 10])
        .style(theme::popover_bg);

        // Open sideways, toward the middle of the chart: a tooltip above the
        // bars would cover the figures it is annotating.
        let side = if i * 2 < n {
            tooltip::Position::Right
        } else {
            tooltip::Position::Left
        };
        bars = bars.push(tooltip(column_el, tip, side).gap(6));
    }

    let baseline = column![
        Space::new().height(down_area),
        container(Space::new())
            .width(Length::Fill)
            .height(1)
            .style(theme::hairline),
    ]
    .width(Length::Fill)
    .height(height);

    stack![baseline, bars].into()
}

#[cfg(test)]
mod tests {
    use super::{split_figure, waveform_split};

    #[test]
    fn waveform_split_fills_height_on_one_scale() {
        for (down, up) in [(0, 0), (8_000_000, 400_000), (100, 5_000_000), (2048, 2048)] {
            let (d, u, scale) = waveform_split(down, up, 148.0);
            assert_eq!(d + u + 1.0, 148.0, "areas plus baseline fill the plot");
            assert!(
                d >= 0.22 * 143.0 && u >= 0.22 * 143.0,
                "quiet side stays legible"
            );
            // Neither peak overflows its side at the shared scale.
            assert!(down as f32 * scale <= d - 2.0 + 0.01);
            assert!(up as f32 * scale <= u - 2.0 + 0.01);
        }
        // A download-heavy window hands the download side most of the height.
        let (d, u, _) = waveform_split(8_000_000, 400_000, 148.0);
        assert!(d > u * 2.0);
    }

    #[test]
    fn split_figure_separates_value_and_unit() {
        assert_eq!(
            split_figure("1.24 MB/s"),
            ("1.24".to_string(), "MB/s".to_string())
        );
        assert_eq!(split_figure("0"), ("0".to_string(), String::new()));
    }
}
