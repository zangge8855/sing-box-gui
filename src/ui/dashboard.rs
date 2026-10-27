use crate::message::Message;
use crate::state::{Bandwidth, GuiConfig, RoutingMode, Tab};
use crate::ui::i18n::tr;
use crate::ui::page_header;
use crate::ui::theme;
use crate::ui::util::{format_size, format_speed, truncate_width};
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
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;

    // Owned copies so the responsive closure can be re-run on resize.
    let theme_cloned = theme.clone();
    let speed = current_speed.clone();
    let history = speed_history.to_vec();
    let core_busy = core_starting || core_stopping;
    let has_profile = gui_config.active_profile_id.is_some();

    let main_content = responsive(move |size| {
        let is_compact = size.width < crate::ui::DASHBOARD_COMPACT_W;
        let theme = &theme_cloned;
        let text_primary = theme::text_primary(theme);
        let text_muted = theme::text_muted(theme);
        let text_tertiary = theme::text_tertiary(theme);

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
        let power_size = if is_compact { 56.0 } else { 64.0 };
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
        let headline_cols = if is_compact { 22 } else { 34 };
        let headline_short = truncate_width(&headline, headline_cols);
        let headline_truncated = headline_short != headline;
        let headline_text = text(headline_short)
            .size(if is_compact {
                theme::TYPE_TITLE
            } else {
                theme::TYPE_DISPLAY
            })
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
        let fact = |label: &'static str, value: String| {
            row![
                text(label).size(theme::TYPE_CAPTION).color(text_tertiary),
                text(value)
                    .size(theme::TYPE_BTN_SM)
                    .font(theme::metric_font())
                    .color(text_primary),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        };
        let mut facts = row![].spacing(crate::ui::SP_20).align_y(Alignment::Center);
        if let Some(desc) = subline {
            facts = facts.push(text(desc).size(theme::TYPE_BTN_SM).color(text_muted));
        } else {
            facts = facts.push(fact(
                tr(lang, "listen_port"),
                if core_running {
                    gui_config.mixed_port.to_string()
                } else {
                    "—".to_string()
                },
            ));
            facts = facts.push(
                button(fact(
                    tr(lang, "dash_connections"),
                    active_connections.to_string(),
                ))
                .padding([2, 6])
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

        let identity = column![
            container(status_line).padding([0, 8]),
            headline_el,
            container(facts).padding([0, 8]),
        ]
        .spacing(6)
        .width(Length::Fill);

        // A clear next step when the power control cannot act yet.
        let cta: Option<Element<'_, Message>> = if core_running || core_busy {
            None
        } else if !has_profile {
            Some(
                button(text(tr(lang, "btn_goto_profiles")).size(theme::TYPE_BTN_MD))
                    .padding(theme::BTN_PAD_MD)
                    .style(theme::button_primary)
                    .on_press(Message::TabChanged(Tab::Profiles))
                    .into(),
            )
        } else if !core_installed {
            Some(
                button(text(tr(lang, "btn_goto_settings")).size(theme::TYPE_BTN_MD))
                    .padding(theme::BTN_PAD_MD)
                    .style(theme::button_primary)
                    .on_press(Message::TabChanged(Tab::Settings))
                    .into(),
            )
        } else {
            None
        };

        let mut hero_col = column![].spacing(crate::ui::SP_16);
        if is_compact {
            hero_col = hero_col.push(
                row![power, identity]
                    .spacing(crate::ui::SP_16)
                    .align_y(Alignment::Center),
            );
            if let Some(cta) = cta {
                hero_col = hero_col.push(cta);
            }
        } else {
            let mut hero_row = row![power, identity]
                .spacing(crate::ui::SP_20)
                .align_y(Alignment::Center);
            if let Some(cta) = cta {
                hero_row = hero_row.push(cta);
            }
            hero_col = hero_col.push(hero_row);
        }
        if core_starting {
            hero_col = hero_col.push(crate::ui::loading_row(
                tr(lang, "core_starting_hint"),
                theme,
            ));
        }
        let hero = container(hero_col)
            .padding(if is_compact {
                theme::CARD_PAD
            } else {
                theme::CARD_PAD + 4.0
            })
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
        let chart = traffic_waveform(
            &history,
            crate::ui::page_content_width(size.width, is_compact) - theme::CARD_PAD * 2.0,
            if is_compact { 120.0 } else { 148.0 },
            theme,
            lang,
        );
        let chart_el: Element<'_, Message> = if idle {
            stack![
                chart,
                container(
                    text(tr(lang, "chart_waiting_traffic"))
                        .size(theme::TYPE_CAPTION)
                        .color(text_tertiary),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
            ]
            .into()
        } else {
            chart
        };

        let traffic = container(
            column![chart_header, figures, chart_el]
                .spacing(crate::ui::SP_16)
                .width(Length::Fill),
        )
        .padding(theme::CARD_PAD)
        .width(Length::Fill)
        .style(theme::card_bg);

        // ── Controls: routing mode (segmented) and system proxy (switch) ──
        let current_mode = gui_config.routing_mode;
        let segment = |mode: RoutingMode, key: &'static str| {
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
            .padding([7, 8])
            .style(theme::button_tab(active));
            if !active {
                b = b.on_press(Message::RoutingModeChanged(mode));
            }
            b
        };
        let mode_desc_key = match current_mode {
            RoutingMode::Rule => "mode_rules_desc",
            RoutingMode::Global => "mode_global_desc",
            RoutingMode::Direct => "mode_direct_desc",
        };
        let card_title = |key: &'static str| {
            text(tr(lang, key))
                .size(theme::TYPE_HEADING)
                .font(theme::ui_font(iced::font::Weight::Semibold))
                .color(text_primary)
        };
        // Both control cards share one skeleton — title row, a 3px-inset track,
        // caption — so they line up row-for-row without a fixed card height.
        let title_row = |key: &'static str, trailing: Element<'static, Message>| {
            row![card_title(key), Space::new().width(Length::Fill), trailing]
                .height(Length::Fixed(22.0))
                .align_y(Alignment::Center)
        };
        let routing_card = container(
            column![
                title_row("active_mode", Space::new().into()),
                container(
                    row![
                        segment(RoutingMode::Rule, "mode_rules"),
                        segment(RoutingMode::Global, "mode_global"),
                        segment(RoutingMode::Direct, "mode_direct"),
                    ]
                    .spacing(3),
                )
                .padding(3)
                .width(Length::Fill)
                .style(theme::segmented_track),
                text(tr(lang, mode_desc_key))
                    .size(theme::TYPE_CAPTION)
                    .color(text_tertiary),
            ]
            .spacing(crate::ui::SP_12),
        )
        .padding(theme::CARD_PAD)
        .width(Length::FillPortion(1))
        .style(theme::card_bg);

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
        let proxy_card = container(
            column![
                title_row("system_proxy", proxy_switch.into()),
                container(
                    row![
                        text(proxy_state)
                            .size(theme::TYPE_BTN_SM)
                            .font(theme::ui_font(iced::font::Weight::Medium))
                            .color(proxy_state_color),
                        Space::new().width(Length::Fill),
                        text(format!("127.0.0.1:{}", gui_config.mixed_port))
                            .size(theme::TYPE_BTN_SM)
                            .font(theme::metric_font())
                            .color(text_muted),
                    ]
                    .align_y(Alignment::Center)
                    .padding([7, 12]),
                )
                .padding(3)
                .width(Length::Fill)
                .style(theme::segmented_track),
                text(tr(lang, "sys_proxy_desc"))
                    .size(theme::TYPE_CAPTION)
                    .color(text_tertiary),
            ]
            .spacing(crate::ui::SP_12),
        )
        .padding(theme::CARD_PAD)
        .width(Length::FillPortion(1))
        .style(theme::card_bg);

        let controls: Element<'_, Message> = if is_compact {
            column![routing_card, proxy_card]
                .spacing(theme::GRID_GAP)
                .width(Length::Fill)
                .into()
        } else {
            row![routing_card, proxy_card]
                .spacing(theme::GRID_GAP)
                .width(Length::Fill)
                .into()
        };

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
        content_col = content_col.push(hero).push(traffic).push(controls);

        let header = page_header("tab_dashboard", lang, None, theme, is_compact);
        crate::ui::page_shell_with_pad(
            header,
            content_col.into(),
            crate::ui::is_pad_compact(size.width),
        )
    });

    main_content.into()
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

        bars = bars.push(tooltip(column_el, tip, tooltip::Position::Top).gap(6));
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
