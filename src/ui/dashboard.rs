use crate::message::Message;
use crate::state::{Bandwidth, CoreInstallState, GuiConfig, Language, Profile, RoutingMode, Tab};
use crate::ui::i18n::tr;
use crate::ui::page_header;
use crate::ui::theme;
use crate::ui::util::{
    LIGHT_WIDTH, SEMIBOLD_WIDTH, fit_width, format_size, format_speed, labelled_size, text_width,
    traffic_usage_ratio,
};
use iced::widget::{
    Space, button, column, container, responsive, row, stack, text, text_input, toggler, tooltip,
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
fn fill_height_threshold(tun_banner: bool, profile_strip: bool, onboarding: bool) -> f32 {
    if onboarding {
        return f32::INFINITY;
    }
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
    core_install_state: &'a CoreInstallState,
    sys_proxy_enabled: bool,
    system_proxy_busy: bool,
    pending_routing_mode: Option<RoutingMode>,
    current_speed: &Bandwidth,
    speed_history: &[(u64, u64)],
    total_uploaded: u64,
    total_downloaded: u64,
    selected_node: Option<&'a str>,
    active_connections: usize,
    url_input: &'a str,
    downloading: bool,
    profile_error: Option<&'a str>,
    updating_profile_id: Option<&'a str>,
    anim_tick: u8,
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
    let show_onboarding = !has_profile || !core_installed;

    let main_content = responsive(move |size| {
        let is_compact = size.width < crate::ui::DASHBOARD_COMPACT_W;
        let pad_compact = crate::ui::is_pad_compact(size.width);
        let fill_height = !is_compact
            && size.height
                >= fill_height_threshold(
                    gui_config.tun_mode,
                    active_profile.is_some(),
                    show_onboarding,
                );
        let theme = &theme_cloned;
        let text_primary = theme::text_primary(theme);
        let text_muted = theme::text_muted(theme);
        let text_tertiary = theme::text_tertiary(theme);

        let content_w = crate::ui::page_content_width(size.width, pad_compact)
            - if fill_height {
                0.0
            } else {
                crate::ui::SCROLL_GUTTER
            };
        let hero_pad = if is_compact {
            theme::CARD_PAD
        } else {
            theme::CARD_PAD + 4.0
        };
        let hero_inner = (content_w - hero_pad * 2.0).max(0.0);
        let power_size = if is_compact { 56.0 } else { 64.0 };
        let controls_w = (hero_inner * 0.44).clamp(296.0, 384.0);
        let identity_w = if is_compact {
            (hero_inner - power_size - crate::ui::SP_16).max(80.0)
        } else {
            (hero_inner - power_size - crate::ui::SP_20 * 3.0 - 1.0 - controls_w).max(80.0)
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
        let can_start = has_profile && core_installed;
        // While the core downloads, the setup card already shows progress; a
        // "install core first" warning beside it would read as a contradiction.
        let core_install_busy = matches!(
            core_install_state,
            CoreInstallState::Downloading
                | CoreInstallState::Verifying
                | CoreInstallState::Extracting
                | CoreInstallState::Installing
        );
        let disabled_reason = (!core_running && !core_busy && !can_start && !core_install_busy)
            .then_some(if !core_installed {
                "btn_install_core_first"
            } else {
                "btn_import_profile_first"
            });
        let mut status_items: Vec<Element<'_, Message>> = vec![
            crate::ui::status_dot(
                dot_color,
                dot_active,
                status_label,
                status_text_color,
                theme::TYPE_BTN_SM,
            ),
        ];
        if let Some(reason_key) = disabled_reason {
            status_items.push(
                text(format!("·  {}", tr(lang, reason_key)))
                    .size(theme::TYPE_BTN_SM)
                    .color(text_muted)
                    .into(),
            );
        }
        let status_line = iced::widget::Row::with_children(status_items)
            .spacing(crate::ui::SP_8)
            .align_y(Alignment::Center)
            .wrap()
            .vertical_spacing(4);

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
        .style(move |t, status| {
            if core_busy {
                theme::power_button_busy(t, status)
            } else {
                theme::power_button(core_running)(t, status)
            }
        });
        if !core_busy && (core_running || can_start) {
            power = power.on_press(Message::ToggleCore);
        }
        let power_hint = if core_starting {
            tr(lang, "status_starting")
        } else if core_stopping {
            tr(lang, "status_stopping")
        } else if core_running {
            tr(lang, "btn_stop_core")
        } else if let Some(reason_key) = disabled_reason {
            tr(lang, reason_key)
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
        // A long name steps down a size before it is cut: a slightly smaller
        // headline reads better than "Premiu…". The ghost button insets the
        // text 8px on each side.
        let headline_room = (identity_w - 16.0 - if has_profile { 22.0 } else { 0.0 }).max(80.0);
        let headline_sizes: &[f32] = if is_compact || identity_w < 320.0 {
            &[theme::TYPE_TITLE, theme::TYPE_TITLE_SM]
        } else {
            &[theme::TYPE_DISPLAY, theme::TYPE_TITLE, theme::TYPE_TITLE_SM]
        };
        let headline_size = headline_sizes
            .iter()
            .copied()
            .find(|&s| text_width(&headline, s * LIGHT_WIDTH) <= headline_room)
            .unwrap_or(theme::TYPE_TITLE_SM);
        let headline_short = fit_width(&headline, headline_room, headline_size * LIGHT_WIDTH);
        let headline_truncated = headline_short != headline;
        let headline_text = text(headline_short)
            .size(headline_size)
            .font(theme::display_font())
            .shaping(text::Shaping::Advanced)
            .wrapping(text::Wrapping::None)
            .color(headline_color);
        let headline_el: Element<'_, Message> = if has_profile {
            let btn = button(
                row![
                    headline_text,
                    icon(crate::ui::icons::ICON_CHEVRON_RIGHT, 18.0).color(text_tertiary),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            )
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
        } else if headline_truncated {
            tooltip(
                container(headline_text).padding([2, 8]),
                container(text(headline).size(theme::TYPE_BTN_SM).color(text_primary))
                    .padding([6, 10])
                    .style(theme::popover_bg),
                tooltip::Position::Bottom,
            )
            .into()
        } else {
            container(headline_text).padding([2, 8]).into()
        };

        // Quiet facts beneath the headline: label in grey, value in ink.
        // Every fact sits in the same 8px inset as the headline button, so a
        // wrapped second line starts on the same edge as the first.
        let fact = |label: &'static str, value: String| {
            row![
                text(tr(lang, label))
                    .size(theme::TYPE_CAPTION)
                    .color(text_tertiary),
                text(value)
                    .size(theme::TYPE_BTN_SM)
                    .font(theme::metric_font())
                    .color(text_primary),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        };
        let mut fact_items: Vec<Element<'_, Message>> = Vec::new();
        if let Some(desc) = subline {
            fact_items.push(
                container(text(desc).size(theme::TYPE_BTN_SM).color(text_muted))
                    .padding([2, 8])
                    .into(),
            );
        } else {
            let shown_connections = if core_running { active_connections } else { 0 };
            fact_items.push(
                button(
                    row![
                        fact("dash_connections", shown_connections.to_string()),
                        icon(crate::ui::icons::ICON_CHEVRON_RIGHT, 14.0).color(text_tertiary),
                    ]
                    .spacing(2)
                    .align_y(Alignment::Center),
                )
                .padding([2, 8])
                .style(theme::button_ghost)
                .on_press(Message::TabChanged(Tab::Connections))
                .into(),
            );
            // The active profile has its own strip below; the hero keeps to
            // what the core is doing right now.
            fact_items.push(
                container(fact("fact_port", gui_config.mixed_port.to_string()))
                    .padding([2, 8])
                    .into(),
            );
            if gui_config.tun_mode {
                fact_items.push(
                    container(
                        container(
                            text("TUN")
                                .size(theme::TYPE_MICRO)
                                .font(theme::ui_font(iced::font::Weight::Semibold))
                                .color(text_muted),
                        )
                        .padding([2, 6])
                        .style(theme::badge_bg),
                    )
                    .padding([2, 8])
                    .into(),
                );
            }
        }
        let facts = iced::widget::Row::with_children(fact_items)
            .spacing(4)
            .align_y(Alignment::Center)
            .wrap()
            .vertical_spacing(2);

        let identity = column![container(status_line).padding([0, 8]), headline_el, facts,]
            .spacing(6)
            .width(Length::Fill);

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
        let current_mode = pending_routing_mode.unwrap_or(gui_config.routing_mode);
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
                    .wrapping(text::Wrapping::None)
                    .align_x(Alignment::Center)
                    .width(Length::Fill),
            )
            .width(Length::FillPortion(1))
            .padding([6, 4])
            .style(theme::button_tab(active));
            if !active && pending_routing_mode.is_none() {
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
            column![
                text(proxy_state)
                    .size(theme::TYPE_BTN_SM)
                    .font(theme::ui_font(iced::font::Weight::Medium))
                    .color(proxy_state_color),
                text(format!("127.0.0.1:{}", gui_config.mixed_port))
                    .size(theme::TYPE_MICRO)
                    .font(theme::metric_font())
                    .color(text_tertiary),
            ]
            .spacing(1),
            Space::new().width(Length::Fill),
            proxy_switch,
        ]
        .spacing(crate::ui::SP_12)
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
            hero_col = hero_col.push(crate::ui::loading_row_animated(
                tr(lang, "core_starting_hint"),
                anim_tick,
                theme,
            ));
        } else if core_stopping {
            hero_col = hero_col.push(crate::ui::loading_row_animated(
                tr(lang, "status_stopping"),
                anim_tick,
                theme,
            ));
        }
        if pending_routing_mode.is_some() {
            hero_col = hero_col.push(crate::ui::loading_row_animated(
                tr(lang, "mode_switching_hint"),
                anim_tick,
                theme,
            ));
        }
        let hero = container(hero_col)
            .padding(hero_pad)
            .width(Length::Fill)
            .style(theme::card_bg);

        // ── Traffic: two large figures over a live chart ──────────────────
        let (down_color, up_color) = theme::chart_series(theme);
        let live_down = if core_running { speed.down } else { 0 };
        let live_up = if core_running { speed.up } else { 0 };
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
                        container(text(unit).size(theme::TYPE_BTN_SM).color(text_muted)).padding(
                            iced::Padding {
                                bottom: if is_compact { 3.0 } else { 4.5 },
                                ..Default::default()
                            }
                        ),
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
        let figures = if is_compact {
            row![
                figure_block(
                    "download",
                    crate::ui::icons::ICON_DOWN,
                    down_color,
                    live_down,
                    total_downloaded,
                ),
                figure_block(
                    "upload",
                    crate::ui::icons::ICON_UP,
                    up_color,
                    live_up,
                    total_uploaded,
                ),
            ]
            .spacing(crate::ui::SP_20)
            .width(Length::Fill)
        } else {
            row![
                figure_block(
                    "download",
                    crate::ui::icons::ICON_DOWN,
                    down_color,
                    live_down,
                    total_downloaded,
                ),
                container(Space::new())
                    .width(1)
                    .height(Length::Fill)
                    .style(theme::hairline),
                figure_block(
                    "upload",
                    crate::ui::icons::ICON_UP,
                    up_color,
                    live_up,
                    total_uploaded,
                ),
            ]
            .spacing(crate::ui::SP_20)
            .height(Length::Shrink)
            .width(Length::Fill)
        };

        let chart_header = text(tr(lang, "speed_history_chart"))
            .size(theme::TYPE_HEADING)
            .font(theme::ui_font(iced::font::Weight::Semibold))
            .color(text_primary);

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

        // Note: hero is always shown first above onboarding/traffic.
        let mut final_col = if gui_config.tun_mode {
            column![
                container(
                    text(tr(lang, "tun_admin_banner"))
                        .size(theme::TYPE_BTN_SM)
                        .color(theme::warning_text(theme)),
                )
                .padding(crate::ui::SP_12)
                .width(Length::Fill)
                .style(|t| theme::tinted_banner(t, theme::WARNING)),
                hero,
            ]
        } else {
            column![hero]
        }
        .spacing(theme::GRID_GAP)
        .width(Length::Fill);
        if show_onboarding {
            final_col = final_col.push(onboarding_card(
                core_installed,
                core_install_state,
                active_profile,
                url_input,
                downloading,
                profile_error,
                can_start,
                lang,
                anim_tick,
                theme,
            ));
        }
        final_col = final_col.push(traffic);
        if let Some(profile) = active_profile {
            final_col = final_col.push(profile_strip(
                profile,
                lang,
                theme,
                downloading,
                updating_profile_id == Some(profile.id.as_str()),
                is_compact,
                content_w,
            ));
        }

        let header = page_header("tab_dashboard", lang, None, theme, pad_compact);
        if fill_height {
            crate::ui::page_shell_fixed_with_pad(
                header,
                final_col.height(Length::Fill).into(),
                pad_compact,
            )
        } else {
            crate::ui::page_shell_with_pad(header, final_col.into(), pad_compact)
        }
    });

    main_content.into()
}

/// First-launch 1-2-3 setup guide rendered on Dashboard whenever the core or
/// an active profile is missing, keeping setup on one screen.
#[allow(clippy::too_many_arguments)]
fn onboarding_card<'a>(
    core_installed: bool,
    install_state: &'a CoreInstallState,
    active_profile: Option<&'a Profile>,
    url_input: &'a str,
    downloading: bool,
    profile_error: Option<&'a str>,
    can_start: bool,
    lang: Language,
    anim_tick: u8,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let text_primary = theme::text_primary(theme);
    let text_muted = theme::text_muted(theme);
    let text_tertiary = theme::text_tertiary(theme);

    // The marker is the step's number until it is done, then a check —
    // one glyph, no rings or filled discs.
    let step_badge = |n: u8, done: bool, active: bool| -> Element<'a, Message> {
        let inner: Element<'a, Message> = if done {
            crate::ui::material_icon(crate::ui::icons::ICON_CHECK)
                .size(crate::ui::ICON_SIZE)
                .color(theme::success_text(theme))
                .into()
        } else {
            text(n.to_string())
                .size(theme::TYPE_BODY)
                .font(theme::ui_font(iced::font::Weight::Semibold))
                .color(if active { text_primary } else { text_tertiary })
                .into()
        };
        container(inner)
            .width(20)
            .height(20)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
    };

    // Step 1: Install sing-box core
    let step1_body: Element<'a, Message> = if core_installed {
        text(tr(lang, "onboarding_step1_ready"))
            .size(theme::TYPE_CAPTION)
            .color(theme::success_text(theme))
            .into()
    } else {
        match install_state {
            CoreInstallState::Downloading => {
                crate::ui::loading_row_animated(tr(lang, "core_downloading"), anim_tick, theme)
            }
            CoreInstallState::Verifying => {
                crate::ui::loading_row_animated(tr(lang, "core_verifying"), anim_tick, theme)
            }
            CoreInstallState::Extracting => {
                crate::ui::loading_row_animated(tr(lang, "core_extracting"), anim_tick, theme)
            }
            CoreInstallState::Installing => {
                crate::ui::loading_row_animated(tr(lang, "core_installing"), anim_tick, theme)
            }
            CoreInstallState::Error(err) => row![
                column![
                    text(tr(lang, "core_install_failed"))
                        .size(theme::TYPE_CAPTION)
                        .color(theme::danger_text(theme)),
                    text(crate::ui::i18n::localize_error(lang, err).into_owned())
                        .size(theme::TYPE_CAPTION)
                        .color(text_muted),
                ]
                .spacing(2)
                .width(Length::Fill),
                button(text(tr(lang, "btn_retry_core")).size(theme::TYPE_BTN_SM))
                    .padding(theme::BTN_PAD_SM)
                    .style(theme::button_primary)
                    .on_press(Message::TriggerCoreDownload),
            ]
            .spacing(crate::ui::SP_12)
            .align_y(Alignment::Center)
            .into(),
            CoreInstallState::Idle | CoreInstallState::Installed => row![
                text(tr(lang, "core_status_missing"))
                    .size(theme::TYPE_CAPTION)
                    .color(text_muted)
                    .width(Length::Fill),
                button(text(tr(lang, "onboarding_step1_action")).size(theme::TYPE_BTN_SM))
                    .padding(theme::BTN_PAD_SM)
                    .style(theme::button_primary)
                    .on_press(Message::TriggerCoreDownload),
            ]
            .spacing(crate::ui::SP_12)
            .align_y(Alignment::Center)
            .into(),
        }
    };
    let step1 = row![
        step_badge(1, core_installed, !core_installed),
        column![
            text(tr(lang, "onboarding_step1_title"))
                .size(theme::TYPE_BODY)
                .font(theme::ui_font(iced::font::Weight::Semibold))
                .color(text_primary),
            step1_body,
        ]
        .spacing(6)
        .width(Length::Fill),
    ]
    .spacing(crate::ui::SP_12)
    .align_y(Alignment::Start);

    // Step 2: Import profile or subscription
    let has_profile = active_profile.is_some();
    let step2_body: Element<'a, Message> = if let Some(profile) = active_profile {
        text(crate::ui::util::labelled(
            lang,
            tr(lang, "onboarding_step2_ready"),
            &profile.name,
        ))
        .size(theme::TYPE_CAPTION)
        .color(theme::success_text(theme))
        .into()
    } else {
        let can_import = !downloading && !url_input.trim().is_empty();
        let mut input = text_input(tr(lang, "sub_url_placeholder"), url_input)
            .on_input(Message::SubscriptionInputChanged)
            .padding(8)
            .size(theme::TYPE_BODY)
            .width(Length::Fill)
            .style(theme::input_field);
        if can_import {
            input = input.on_submit(Message::DownloadSubscription);
        }
        let mut import_btn = button(
            text(tr(
                lang,
                if downloading {
                    "btn_downloading"
                } else {
                    "btn_download"
                },
            ))
            .size(theme::TYPE_BTN_SM),
        )
        .padding(theme::BTN_PAD_SM)
        .style(if can_import {
            theme::button_primary
        } else {
            theme::button_secondary
        });
        if can_import {
            import_btn = import_btn.on_press(Message::DownloadSubscription);
        }

        let mut clip_btn = button(text(tr(lang, "btn_import_clipboard")).size(theme::TYPE_BTN_SM))
            .padding(theme::BTN_PAD_SM)
            .style(theme::button_secondary);
        let mut file_btn = button(text(tr(lang, "btn_import_file")).size(theme::TYPE_BTN_SM))
            .padding(theme::BTN_PAD_SM)
            .style(theme::button_secondary);
        if !downloading {
            clip_btn = clip_btn.on_press(Message::ImportFromClipboard);
            file_btn = file_btn.on_press(Message::ImportLocalFile);
        }

        let mut col = column![
            row![input, import_btn]
                .spacing(crate::ui::SP_8)
                .align_y(Alignment::Center),
            row![clip_btn, file_btn]
                .spacing(crate::ui::SP_8)
                .wrap()
                .vertical_spacing(crate::ui::SP_8),
        ]
        .spacing(crate::ui::SP_8);
        if let Some(err) = profile_error {
            let dismiss_btn = button(
                text(crate::ui::icons::ICON_CLOSE)
                    .font(iced::Font::with_name("Material Icons"))
                    .size(14.0),
            )
            .padding(2)
            .style(theme::button_dismiss)
            .on_press(Message::DismissProfileError);
            col = col.push(
                row![
                    text(err)
                        .size(theme::TYPE_CAPTION)
                        .color(theme::danger_text(theme))
                        .width(Length::Fill),
                    dismiss_btn,
                ]
                .spacing(crate::ui::SP_8)
                .align_y(Alignment::Center),
            );
        }
        col.into()
    };
    let step2 = row![
        step_badge(2, has_profile, core_installed && !has_profile),
        column![
            text(tr(lang, "onboarding_step2_title"))
                .size(theme::TYPE_BODY)
                .font(theme::ui_font(iced::font::Weight::Semibold))
                .color(text_primary),
            step2_body,
        ]
        .spacing(6)
        .width(Length::Fill),
    ]
    .spacing(crate::ui::SP_12)
    .align_y(Alignment::Start);

    // Step 3: Ready to connect
    let step3 = row![
        step_badge(3, can_start, can_start),
        column![
            text(tr(lang, "onboarding_step3_title"))
                .size(theme::TYPE_BODY)
                .font(theme::ui_font(iced::font::Weight::Semibold))
                .color(text_primary),
            text(tr(
                lang,
                if can_start {
                    "onboarding_step3_ready"
                } else {
                    "onboarding_step3_waiting"
                }
            ))
            .size(theme::TYPE_CAPTION)
            .color(if can_start {
                theme::success_text(theme)
            } else {
                text_tertiary
            }),
        ]
        .spacing(4)
        .width(Length::Fill),
    ]
    .spacing(crate::ui::SP_12)
    .align_y(Alignment::Start);

    container(
        column![
            text(tr(lang, "onboarding_title"))
                .size(theme::TYPE_HEADING)
                .font(theme::ui_font(iced::font::Weight::Semibold))
                .color(text_primary),
            container(Space::new())
                .width(Length::Fill)
                .height(1)
                .style(theme::hairline),
            step1,
            container(Space::new())
                .width(Length::Fill)
                .height(1)
                .style(theme::hairline),
            step2,
            container(Space::new())
                .width(Length::Fill)
                .height(1)
                .style(theme::hairline),
            step3,
        ]
        .spacing(crate::ui::SP_12)
        .width(Length::Fill),
    )
    .padding(theme::CARD_PAD)
    .width(Length::Fill)
    .style(theme::card_bg)
    .into()
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
    content_w: f32,
) -> Element<'a, Message> {
    let text_primary = theme::text_primary(theme);
    let text_muted = theme::text_muted(theme);
    let caption = |s: String| text(s).size(theme::TYPE_CAPTION).color(text_muted);

    // Wide: the name gets a fixed column beside the meter. Compact: it shares
    // a row with the Update button only.
    let ident_w = (content_w * 0.27).clamp(200.0, 300.0);
    let name_room = if is_compact {
        content_w - theme::CARD_PAD * 2.0 - 120.0
    } else {
        ident_w
    } - 16.0
        - 18.0;
    let name = fit_width(
        &profile.name,
        name_room.max(60.0),
        theme::TYPE_BODY * SEMIBOLD_WIDTH,
    );

    let name_truncated = name != profile.name;
    let name_btn = button(
        row![
            text(name)
                .size(theme::TYPE_BODY)
                .font(theme::ui_font(iced::font::Weight::Semibold))
                .shaping(text::Shaping::Advanced)
                .wrapping(text::Wrapping::None)
                .color(text_primary),
            icon(crate::ui::icons::ICON_CHEVRON_RIGHT, 15.0).color(theme::text_tertiary(theme)),
        ]
        .spacing(3)
        .align_y(Alignment::Center),
    )
    .padding([2, 6])
    .style(theme::button_ghost)
    .on_press(Message::TabChanged(Tab::Profiles));
    let name_el: Element<'a, Message> = if name_truncated {
        tooltip(
            name_btn,
            container(
                text(&profile.name)
                    .size(theme::TYPE_BTN_SM)
                    .color(text_primary),
            )
            .padding([6, 10])
            .style(theme::popover_bg),
            tooltip::Position::Top,
        )
        .gap(4)
        .into()
    } else {
        name_btn.into()
    };

    let ident = column![
        container(
            text(tr(lang, "dash_current_profile"))
                .size(theme::TYPE_CAPTION)
                .color(theme::text_tertiary(theme)),
        )
        .padding([0, 6]),
        name_el,
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
    let mut label_items: Vec<Element<'a, Message>> = Vec::new();
    match (profile.traffic_upload, profile.traffic_download) {
        (Some(u), Some(d)) => {
            let used = u.saturating_add(d);
            let total = profile.traffic_total.unwrap_or(0);
            if total > 0 {
                label_items.push(
                    text(format!("{} / {}", format_size(used), format_size(total)))
                        .size(theme::TYPE_BTN_SM)
                        .font(theme::metric_font())
                        .color(text_primary)
                        .into(),
                );
                label_items.push(
                    caption(labelled_size(
                        lang,
                        "traffic_left",
                        total.saturating_sub(used),
                    ))
                    .into(),
                );
            } else {
                label_items.push(caption(labelled_size(lang, "traffic_used", used)).into());
            }
        }
        _ => {
            label_items.push(
                caption(format!(
                    "{} {}",
                    tr(lang, "updated_at_label"),
                    profile.updated_at
                ))
                .into(),
            );
        }
    }
    let usage_w = if is_compact {
        content_w - theme::CARD_PAD * 2.0
    } else {
        content_w
            - ident_w
            - if profile.is_subscription { 104.0 } else { 0.0 }
            - crate::ui::SP_20 * 2.0
            - theme::CARD_PAD * 2.0
    };
    let wrap_labels = !is_compact && expiry.is_some() && usage_w < 360.0;
    let labels: Element<'a, Message> = if wrap_labels {
        if let Some(expiry) = expiry {
            label_items.push(expiry.into());
        }
        iced::widget::Row::with_children(label_items)
            .spacing(crate::ui::SP_12)
            .align_y(Alignment::Center)
            .wrap()
            .vertical_spacing(4)
            .into()
    } else {
        label_items.push(Space::new().width(Length::Fill).into());
        if let Some(expiry) = expiry {
            label_items.push(expiry.into());
        }
        iced::widget::Row::with_children(label_items)
            .spacing(crate::ui::SP_12)
            .align_y(Alignment::Center)
            .into()
    };
    let mut usage = column![labels].spacing(8).width(Length::Fill);
    if let Some(ratio) = ratio {
        usage = usage.push(crate::ui::usage_bar(ratio, theme));
    }

    let update_btn = profile.is_subscription.then(|| {
        let mut b = button(
            text(tr(
                lang,
                if updating {
                    "btn_updating"
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

    // The profile name is a ghost button whose 6px padding insets its text;
    // the card's left inset gives those 6px back so the label, the name and
    // the usage line all start on the card margin.
    const GHOST_INSET: f32 = 6.0;
    let body: Element<'a, Message> = if is_compact {
        let mut top = row![ident, Space::new().width(Length::Fill)].align_y(Alignment::Center);
        if let Some(b) = update_btn {
            top = top.push(b);
        }
        let usage = container(usage).padding(iced::Padding {
            left: GHOST_INSET,
            ..Default::default()
        });
        column![top, usage].spacing(crate::ui::SP_12).into()
    } else {
        let mut line = row![ident.width(Length::Fixed(ident_w)), usage]
            .spacing(crate::ui::SP_20)
            .align_y(Alignment::Center);
        if let Some(b) = update_btn {
            line = line.push(b);
        }
        line.into()
    };

    // Dense vertically, but the same side insets as the cards above so the
    // label and the Update button share their content edges.
    container(body)
        .padding(iced::Padding {
            top: theme::CARD_PAD_DENSE,
            right: theme::CARD_PAD,
            bottom: theme::CARD_PAD_DENSE,
            left: theme::CARD_PAD - GHOST_INSET,
        })
        .width(Length::Fill)
        .style(theme::card_bg)
        .into()
}

/// Splits the plot height around the 1px baseline: `(down_area, up_area,
/// px_per_byte)`. Both directions share one scale; the area each side gets
/// follows its peak, so a download-heavy window doesn't leave the upload half
/// as dead space. Each side keeps at least 14%: room for its peak label
/// without parking an empty band under a quiet upload.
fn waveform_split(peak_down: u64, peak_up: u64, height: f32) -> (f32, f32, f32) {
    // 2px breath between each bar and the baseline.
    const BREATH: f32 = 2.0;
    let usable = (height - 1.0 - BREATH * 2.0).max(2.0);
    let sum = peak_down as f64 + peak_up as f64;
    let share = if sum == 0.0 {
        0.5
    } else {
        (peak_down as f64 / sum).clamp(0.14, 0.86) as f32
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

    // Reserved Y-axis rail for the peak labels. They start on the card's left
    // margin, in line with the rate figures above, rather than floating
    // right-aligned in the middle of the rail.
    const Y_AXIS_W: f32 = 80.0;
    let plot_width = (chart_width - Y_AXIS_W - crate::ui::SP_8).max(80.0);
    let n = history.len().max(1);
    let peak_up = history.iter().map(|&(u, _)| u).max().unwrap_or(0);
    let peak_down = history.iter().map(|&(_, d)| d).max().unwrap_or(0);
    let slot = plot_width / n as f32;
    // Thin ticks centred in full-width slots: they read as a measurement
    // rather than a block graphic, and each slot stays a wide hover target.
    let bar_w = (slot * 0.3).clamp(2.0, 6.0).round();
    let height = height.floor();
    let (down_area, up_area, scale) = waveform_split(peak_down, peak_up, height);

    let bar = |value: u64, color: Color, alpha: f32, rising: bool| -> Element<'a, Message> {
        if value == 0 {
            return Space::new().height(0).into();
        }
        // Keep 3px clear of the top/bottom borders and 2px clear of the baseline.
        let reach = (if rising { down_area } else { up_area } - 5.0).max(2.0);
        let h = (value as f32 * scale).clamp(2.0, reach);
        let r = (h / 2.0).min(1.5);
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
            .width(Length::Fixed(bar_w))
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
                .width(Length::Fixed(match lang {
                    Language::Zh => 32.0,
                    Language::En => 60.0,
                })),
            text(format_speed(rate))
                .size(theme::TYPE_CAPTION)
                .font(theme::metric_font())
                .color(text_primary),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
    };

    let mut bars = row![].height(height).width(Length::Fill);
    for (i, &(up, down)) in history.iter().enumerate() {
        let alpha = 1.0;
        let column_el = column![
            container(bar(down, down_color, alpha, true))
                .height(down_area)
                .width(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::End)
                .padding(iced::Padding {
                    top: 3.0,
                    bottom: 2.0,
                    ..Default::default()
                }),
            Space::new().height(1),
            container(bar(up, up_color, alpha, false))
                .height(up_area)
                .width(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Start)
                .padding(iced::Padding {
                    top: 2.0,
                    bottom: 3.0,
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
            .style(|t| container::Style {
                background: Some(iced::Background::Color(theme::border_strong(t))),
                ..Default::default()
            }),
    ]
    .width(Length::Fill)
    .height(height);

    // Each peak label rides level with the tallest bar it describes, so a
    // quiet upload side is labelled next to its bars rather than at the
    // bottom of an otherwise empty band.
    const LABEL_H: f32 = 14.0;
    let reach = |peak: u64, area: f32| {
        if peak == 0 {
            0.0
        } else {
            (peak as f32 * scale).clamp(2.0, (area - 5.0).max(2.0))
        }
    };
    let down_top = (down_area - 2.0 - reach(peak_down, down_area) - LABEL_H / 2.0)
        .clamp(0.0, (down_area - LABEL_H).max(0.0));
    let up_top = (down_area + 3.0 + reach(peak_up, up_area) - LABEL_H / 2.0)
        .clamp(down_area + 1.0, (height - LABEL_H).max(down_area + 1.0));
    let axis_label = |label: String, color: Color| {
        container(
            text(label)
                .size(theme::TYPE_MICRO)
                .font(theme::metric_font())
                .wrapping(text::Wrapping::None)
                .color(color),
        )
        .height(LABEL_H)
        .width(Length::Fill)
        .align_x(Alignment::Start)
        .align_y(Alignment::Center)
    };
    let peak_label = |arrow: &str, peak: u64| {
        if peak > 0 {
            format!("{arrow} {}", format_speed(peak))
        } else {
            String::new()
        }
    };
    let y_axis = column![
        Space::new().height(down_top),
        axis_label(peak_label("↓", peak_down), down_color),
        Space::new().height((up_top - down_top - LABEL_H).max(0.0)),
        axis_label(peak_label("↑", peak_up), up_color),
    ]
    .width(Length::Fixed(Y_AXIS_W))
    .height(height);

    row![y_axis, stack![baseline, bars]]
        .spacing(crate::ui::SP_8)
        .width(Length::Fill)
        .height(height)
        .into()
}

#[cfg(test)]
mod tests {
    use super::{fill_height_threshold, render, split_figure, waveform_split};

    #[test]
    fn waveform_split_fills_height_on_one_scale() {
        for (down, up) in [(0, 0), (8_000_000, 400_000), (100, 5_000_000), (2048, 2048)] {
            let (d, u, scale) = waveform_split(down, up, 148.0);
            assert_eq!(d + u + 1.0, 148.0, "areas plus baseline fill the plot");
            assert!(
                d >= 0.14 * 143.0 && u >= 0.14 * 143.0,
                "quiet side stays legible"
            );
            // Neither peak overflows its side at the shared scale.
            assert!(down as f32 * scale <= d - 2.0 + 0.01);
            assert!(up as f32 * scale <= u - 2.0 + 0.01);
        }
        // A download-heavy window hands the download side most of the height.
        let (d, u, _) = waveform_split(8_000_000, 400_000, 148.0);
        assert!(d > u * 2.0);
        // Tiny heights stay finite and non-negative.
        let (d_min, u_min, scale_min) = waveform_split(10_000, 10_000, 2.0);
        assert!(d_min > 0.0 && u_min > 0.0 && scale_min.is_finite());
        // Huge u64 peaks never overflow u64 + u64.
        let (d_max, u_max, scale_max) = waveform_split(u64::MAX, u64::MAX, 148.0);
        assert!(d_max > 0.0 && u_max > 0.0 && scale_max.is_finite());
    }

    #[test]
    fn split_figure_separates_value_and_unit() {
        assert_eq!(
            split_figure("1.24 MB/s"),
            ("1.24".to_string(), "MB/s".to_string())
        );
        assert_eq!(split_figure("0"), ("0".to_string(), String::new()));
    }

    #[test]
    fn fill_height_threshold_accounts_for_banners_and_onboarding() {
        assert!(fill_height_threshold(false, false, true).is_infinite());
        let base = fill_height_threshold(false, false, false);
        assert!(fill_height_threshold(true, false, false) > base);
        assert!(fill_height_threshold(false, true, false) > base);
    }

    #[test]
    fn dashboard_render_handles_onboarding_and_long_profile_strip() {
        use crate::state::{
            Bandwidth, CoreInstallState, GuiConfig, Language, Profile, RoutingMode,
        };
        let mut cfg = GuiConfig::default();
        let speed = Bandwidth {
            up: 1024,
            down: 4096,
        };
        let _ = render(
            &cfg,
            false,
            false,
            false,
            false,
            &CoreInstallState::Idle,
            false,
            false,
            None,
            &speed,
            &[],
            0,
            0,
            None,
            0,
            "",
            false,
            Some("Profile source is empty"),
            None,
            0,
            &iced::Theme::Light,
        );

        cfg.language = Language::Zh;
        cfg.tun_mode = true;
        cfg.active_profile_id = Some("p1".into());
        cfg.subscriptions.push(Profile {
            id: "p1".into(),
            name: "超长测试订阅名称 / Very Long Subscription Name For Tooltip Coverage".into(),
            url: "https://example.com/sub?token=secret".into(),
            file_path: String::new(),
            is_subscription: true,
            updated_at: "2026-11-18 10:00".into(),
            traffic_upload: Some(1_000_000_000),
            traffic_download: Some(2_000_000_000),
            traffic_total: Some(10_000_000_000),
            expire_at: Some(1_893_456_000),
        });
        let _ = render(
            &cfg,
            true,
            false,
            false,
            true,
            &CoreInstallState::Installed,
            true,
            false,
            Some(RoutingMode::Global),
            &speed,
            &[(100, 200), (300, 400)],
            100,
            200,
            Some("🇯🇵 東京非常长的节点名称 Tokyo Very Long Node Name 01"),
            12,
            "",
            false,
            None,
            None,
            3,
            &iced::Theme::Dark,
        );
    }
}
