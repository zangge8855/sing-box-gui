use iced::widget::{button, column, container, row, svg, text, Space, responsive, pick_list};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::state::{Bandwidth, GuiConfig, RoutingMode};
use crate::ui::theme;
use crate::ui::page_header;
use crate::ui::util::{format_size_precise as format_size, format_speed};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutingModeOption {
    pub mode: RoutingMode,
    pub label: &'static str,
}

impl std::fmt::Display for RoutingModeOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}
fn icon(unicode: char) -> text::Text<'static> {
    text(unicode.to_string())
        .font(iced::Font::with_name("Material Icons"))
        .size(crate::ui::ICON_SIZE)
}

#[allow(clippy::too_many_arguments)]
pub fn render<'a>(
    gui_config: &'a GuiConfig,
    core_running: bool,
    core_starting: bool,
    core_stopping: bool,
    sys_proxy_enabled: bool,
    current_speed: &Bandwidth,
    speed_history: &[(u64, u64)],
    total_uploaded: u64,
    total_downloaded: u64,
    selected_node: Option<&'a str>,
    active_connections: usize,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    use crate::ui::i18n::tr;
    
    // Clone properties to move them into the responsive Fn closure
    let theme_cloned = theme.clone();
    let speed_cloned = current_speed.clone();
    let history_cloned = speed_history.to_vec();
    let core_busy = core_starting || core_stopping;
    
    let main_content = responsive(move |size| {
        let is_compact = size.width < crate::ui::DASHBOARD_COMPACT_W;
        let theme = &theme_cloned;
        let current_speed = &speed_cloned;
        let speed_history = &history_cloned;
        
        let text_muted = theme::text_muted(theme);
        
        let make_icon_badge = |unicode: char, color: iced::Color| {
            container(
                text(unicode.to_string())
                    .font(iced::Font::with_name("Material Icons"))
                    .size(crate::ui::ICON_SIZE)
                    .color(color)
            )
            .padding(6)
            .style(move |t| container::Style {
                background: Some(iced::Background::Color(theme::with_alpha(color, if theme::is_dark(t) { 0.16 } else { 0.12 }))),
                border: iced::Border {
                    radius: theme::RADIUS_SM.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
        };

        // 1. Core Status Card
        let (status_color, status_active, status_label) = if core_starting {
            (theme::WARNING, true, tr(lang, "status_starting"))
        } else if core_stopping {
            (theme::WARNING, true, tr(lang, "status_stopping"))
        } else if core_running {
            (theme::SUCCESS, true, tr(lang, "status_running"))
        } else {
            (text_muted, false, tr(lang, "status_stopped"))
        };
        let status_indicator = crate::ui::status_dot(
            status_color,
            status_active,
            status_label,
            status_color,
            theme::TYPE_BODY,
        );
        
        let core_control_btn = if core_busy {
            button(
                row![
                    icon(crate::ui::icons::ICON_UPDATE),
                    text(if core_starting {
                        tr(lang, "status_starting")
                    } else {
                        tr(lang, "status_stopping")
                    })
                    .size(theme::TYPE_BTN_SM)
                ]
                .spacing(crate::ui::SP_8)
                .align_y(Alignment::Center)
            )
            .padding(theme::BTN_PAD_SM)
            .style(theme::button_secondary)
            // No on_press — disabled while transitioning
        } else if core_running {
            button(
                row![icon(crate::ui::icons::ICON_STOP), text(tr(lang, "btn_stop_core")).size(theme::TYPE_BTN_SM)]
                    .spacing(crate::ui::SP_8)
                    .align_y(Alignment::Center)
            )
            .padding(theme::BTN_PAD_SM)
            .style(theme::button_danger)
            .on_press(Message::ToggleCore)
        } else {
            button(
                row![icon(crate::ui::icons::ICON_START), text(tr(lang, "btn_start_core")).size(theme::TYPE_BTN_SM)]
                    .spacing(crate::ui::SP_8)
                    .align_y(Alignment::Center)
            )
            .padding(theme::BTN_PAD_SM)
            .style(theme::button_primary)
            .on_press(Message::ToggleCore)
        };

        let mut core_card_col = column![
            row![
                make_icon_badge(crate::ui::icons::ICON_VPN, theme::ACCENT_PURPLE),
                text(tr(lang, "singbox_core")).color(text_muted).size(theme::TYPE_SECTION)
            ]
            .spacing(crate::ui::SP_8)
            .align_y(Alignment::Center),
            
            row![
                status_indicator,
                Space::new().width(Length::Fill),
                core_control_btn
            ]
            .align_y(Alignment::Center)
            .width(Length::Fill)
        ]
        .spacing(theme::GRID_GAP);

        if core_starting {
            core_card_col = core_card_col.push(crate::ui::loading_row(
                tr(lang, "core_starting_hint"),
                theme,
            ));
        }

        let core_status_card = container(core_card_col)
        .padding(theme::CARD_PAD)
        .width(Length::FillPortion(1))
        .style(theme::card_bg);

        // 2. System Proxy Card
        let sys_proxy_indicator = crate::ui::status_dot(
            if sys_proxy_enabled { theme::SUCCESS } else { text_muted },
            sys_proxy_enabled,
            if sys_proxy_enabled {
                tr(lang, "enabled")
            } else {
                tr(lang, "disabled")
            },
            if sys_proxy_enabled { theme::SUCCESS } else { text_muted },
            theme::TYPE_BODY,
        );
        
        let sys_proxy_btn = button(
            row![
                icon(if sys_proxy_enabled { crate::ui::icons::ICON_STOP } else { crate::ui::icons::ICON_START }),
                text(if sys_proxy_enabled { tr(lang, "btn_disable_proxy") } else { tr(lang, "btn_enable_proxy") }).size(theme::TYPE_BTN_SM)
            ]
            .spacing(crate::ui::SP_8)
            .align_y(Alignment::Center)
        )
        .padding(theme::BTN_PAD_SM)
        .style(if sys_proxy_enabled { theme::button_danger } else { theme::button_primary })
        .on_press(Message::ToggleSystemProxy);

        let proxy_status_card = container(
            column![
                row![
                    make_icon_badge(crate::ui::icons::ICON_SPEED, theme::ACCENT_BLUE),
                    text(tr(lang, "system_proxy")).color(text_muted).size(theme::TYPE_SECTION)
                ]
                .spacing(crate::ui::SP_8)
                .align_y(Alignment::Center),
                
                row![
                    sys_proxy_indicator,
                    Space::new().width(Length::Fill),
                    sys_proxy_btn
                ]
                .align_y(Alignment::Center)
                .width(Length::Fill)
            ]
            .spacing(theme::GRID_GAP)
        )
        .padding(theme::CARD_PAD)
        .width(Length::FillPortion(1))
        .style(theme::card_bg);

        // 3. Download Speed Card
        let download_card = container(
            column![
                row![
                    make_icon_badge(crate::ui::icons::ICON_DOWN, theme::ACCENT_BLUE),
                    text(tr(lang, "download")).color(text_muted).size(theme::TYPE_SECTION)
                ]
                .spacing(crate::ui::SP_8)
                .align_y(Alignment::Center),
                
                row![
                    text(format_speed(current_speed.down))
                        .font(theme::metric_font())
                        .color(theme::ACCENT_BLUE)
                        .size(theme::TYPE_METRIC),
                    Space::new().width(Length::Fill),
                    text(format!("{} {}", tr(lang, "total_label"), format_size(total_downloaded)))
                        .color(theme::text_tertiary(theme))
                        .size(theme::TYPE_CAPTION)
                ]
                .align_y(Alignment::Center)
                .width(Length::Fill)
            ]
            .spacing(theme::GRID_GAP)
        )
        .padding(theme::CARD_PAD)
        .width(Length::FillPortion(1))
        .style(theme::card_bg);

        // 4. Upload Speed Card
        let upload_card = container(
            column![
                row![
                    make_icon_badge(crate::ui::icons::ICON_UP, theme::ACCENT_PURPLE),
                    text(tr(lang, "upload")).color(text_muted).size(theme::TYPE_SECTION)
                ]
                .spacing(crate::ui::SP_8)
                .align_y(Alignment::Center),
                
                row![
                    text(format_speed(current_speed.up))
                        .font(theme::metric_font())
                        .color(theme::ACCENT_PURPLE)
                        .size(theme::TYPE_METRIC),
                    Space::new().width(Length::Fill),
                    text(format!("{} {}", tr(lang, "total_label"), format_size(total_uploaded)))
                        .color(theme::text_tertiary(theme))
                        .size(theme::TYPE_CAPTION)
                ]
                .align_y(Alignment::Center)
                .width(Length::Fill)
            ]
            .spacing(theme::GRID_GAP)
        )
        .padding(theme::CARD_PAD)
        .width(Length::FillPortion(1))
        .style(theme::card_bg);

        // 5. Routing Mode Card
        let mode_options = vec![
            RoutingModeOption { mode: RoutingMode::Rule, label: tr(lang, "mode_rules") },
            RoutingModeOption { mode: RoutingMode::Global, label: tr(lang, "mode_global") },
            RoutingModeOption { mode: RoutingMode::Direct, label: tr(lang, "mode_direct") },
        ];
        
        let selected_mode_opt = mode_options.iter()
            .find(|o| o.mode == gui_config.routing_mode)
            .cloned();
            
        let mode_selector = pick_list(
            mode_options,
            selected_mode_opt,
            move |opt| Message::RoutingModeChanged(opt.mode)
        )
        .width(Length::Fill)
        .padding(8)
        .style(theme::pick_list);

        let port_text = if core_running {
            format!("{}: {}", tr(lang, "listen_port"), gui_config.mixed_port)
        } else {
            format!("{}: -", tr(lang, "listen_port"))
        };

        let routing_mode_card = container(
            column![
                row![
                    make_icon_badge(crate::ui::icons::ICON_SETTINGS, theme::ACCENT_BLUE),
                    text(tr(lang, "active_mode")).color(text_muted).size(theme::TYPE_SECTION),
                    Space::new().width(Length::Fill),
                    text(port_text).color(text_muted).size(theme::TYPE_BTN_SM)
                ]
                .spacing(crate::ui::SP_8)
                .align_y(Alignment::Center)
                .width(Length::Fill),
                
                row![
                    mode_selector
                ]
                .align_y(Alignment::Center)
                .width(Length::Fill)
            ]
            .spacing(theme::GRID_GAP)
        )
        .padding(theme::CARD_PAD)
        .width(Length::FillPortion(1))
        .style(theme::card_bg);

        // Layout the control status cards responsively
        let control_row: Element<'_, Message> = if is_compact {
            column![
                core_status_card,
                proxy_status_card,
                routing_mode_card
            ]
            .spacing(theme::GRID_GAP)
            .width(Length::Fill)
            .into()
        } else {
            row![
                core_status_card,
                proxy_status_card,
                routing_mode_card
            ]
            .spacing(theme::GRID_GAP)
            .width(Length::Fill)
            .into()
        };

        // Layout the traffic metric cards responsively
        let traffic_row: Element<'_, Message> = if is_compact {
            column![
                download_card,
                upload_card
            ]
            .spacing(theme::GRID_GAP)
            .width(Length::Fill)
            .into()
        } else {
            row![
                download_card,
                upload_card
            ]
            .spacing(theme::GRID_GAP)
            .width(Length::Fill)
            .into()
        };

        // Render dynamic SVG chart of speed history
        let max_speed = speed_history.iter()
            .map(|&(u, d)| u.max(d))
            .max()
            .unwrap_or(0)
            .max(1024 * 10); // 10 KB/s dynamic min scale
            
        let points_count = speed_history.len();
        
        let mut down_path = String::new();
        let mut up_path = String::new();
        
        if points_count > 1 {
            let get_x = |i: usize| i as f32 * (300.0 / (points_count - 1) as f32);
            let get_y_down = |down: u64| 100.0 - (down as f32 / max_speed as f32 * 80.0);
            let get_y_up = |up: u64| 100.0 - (up as f32 / max_speed as f32 * 80.0);
            
            down_path.push_str(&format!("M 0 100 L 0 {:.2}", get_y_down(speed_history[0].1)));
            
            for i in 0..points_count - 1 {
                let x0 = get_x(i);
                let y0 = get_y_down(speed_history[i].1);
                let x1 = get_x(i + 1);
                let y1 = get_y_down(speed_history[i + 1].1);
                let cx = (x0 + x1) / 2.0;
                down_path.push_str(&format!(" C {:.2} {:.2}, {:.2} {:.2}, {:.2} {:.2}", cx, y0, cx, y1, x1, y1));
            }
            down_path.push_str(" L 300 100 Z");
            
            up_path.push_str(&format!("M 0 {:.2}", get_y_up(speed_history[0].0)));
            for i in 0..points_count - 1 {
                let x0 = get_x(i);
                let y0 = get_y_up(speed_history[i].0);
                let x1 = get_x(i + 1);
                let y1 = get_y_up(speed_history[i + 1].0);
                let cx = (x0 + x1) / 2.0;
                up_path.push_str(&format!(" C {:.2} {:.2}, {:.2} {:.2}, {:.2} {:.2}", cx, y0, cx, y1, x1, y1));
            }
        } else {
            down_path = "M 0 100 L 300 100 Z".to_string();
            up_path = "M 0 100 L 300 100".to_string();
        }
        
        let grid_color = if theme::is_dark(theme) {
            "rgba(255, 255, 255, 0.05)"
        } else {
            "rgba(0, 0, 0, 0.04)"
        };

        let mut grid_lines = String::new();
        for y in [20, 40, 60, 80] {
            grid_lines.push_str(&format!(r#"<line x1="-5" y1="{}" x2="305" y2="{}" stroke="{}" stroke-dasharray="2 2" stroke-width="0.5"/>"#, y, y, grid_color));
        }
        for x in [50, 100, 150, 200, 250] {
            grid_lines.push_str(&format!(r#"<line x1="{}" y1="-5" x2="{}" y2="105" stroke="{}" stroke-dasharray="2 2" stroke-width="0.5"/>"#, x, x, grid_color));
        }

        let down_color_hex = format!("rgba({},{},{},1)", (theme::ACCENT_BLUE.r * 255.0).round() as u8, (theme::ACCENT_BLUE.g * 255.0).round() as u8, (theme::ACCENT_BLUE.b * 255.0).round() as u8);
        let up_color_hex = format!("rgba({},{},{},1)", (theme::ACCENT_PURPLE.r * 255.0).round() as u8, (theme::ACCENT_PURPLE.g * 255.0).round() as u8, (theme::ACCENT_PURPLE.b * 255.0).round() as u8);

        let svg_xml = format!(
            r##"<svg viewBox="-5 -5 310 110" xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="none">
                 <defs>
                   <linearGradient id="downGrad" x1="0" y1="0" x2="0" y2="1">
                     <stop offset="0%" stop-color="{}" stop-opacity="0.4"/>
                     <stop offset="100%" stop-color="{}" stop-opacity="0"/>
                   </linearGradient>
                 </defs>
                 {}
                 <path d="{}" fill="url(#downGrad)" stroke="{}" stroke-width="2.5"/>
                 <path d="{}" fill="none" stroke="{}" stroke-width="2.5"/>
               </svg>"##,
              down_color_hex, down_color_hex, grid_lines, down_path, down_color_hex, up_path, up_color_hex
        );
        
        let chart_handle = svg::Handle::from_memory(svg_xml.into_bytes());
        let chart_svg = svg(chart_handle)
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(iced::ContentFit::Fill);
            
        let legend = row![
            container(Space::new()).width(12).height(4).style(|_t| container::Style {
                background: Some(iced::Background::Color(theme::ACCENT_BLUE)),
                border: iced::Border { radius: (theme::RADIUS_MICRO / 2.0).into(), ..Default::default() },
                ..Default::default()
            }),
            text(tr(lang, "chart_legend_down")).color(text_muted).size(theme::TYPE_BTN_SM),
            Space::new().width(12),
            container(Space::new()).width(12).height(4).style(|_t| container::Style {
                background: Some(iced::Background::Color(theme::ACCENT_PURPLE)),
                border: iced::Border { radius: (theme::RADIUS_MICRO / 2.0).into(), ..Default::default() },
                ..Default::default()
            }),
            text(tr(lang, "chart_legend_up")).color(text_muted).size(theme::TYPE_BTN_SM),
        ]
        .spacing(crate::ui::SP_8)
        .align_y(Alignment::Center);

        let idle_chart = !core_running
            || speed_history.iter().all(|&(u, d)| u == 0 && d == 0);
        let chart_caption = if idle_chart {
            Some(
                text(tr(lang, "chart_waiting_traffic"))
                    .color(theme::text_tertiary(theme))
                    .size(theme::TYPE_CAPTION),
            )
        } else {
            None
        };

        let mut chart_col = column![
            row![
                text(tr(lang, "speed_history_chart")).color(text_muted).size(theme::TYPE_SECTION).width(Length::Fill),
                legend,
            ]
            .align_y(Alignment::Center),
            container(chart_svg)
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .spacing(crate::ui::SP_12)
        .height(Length::Fill);
        if let Some(cap) = chart_caption {
            chart_col = chart_col.push(cap);
        }

        let chart_card = container(chart_col)
        .padding(theme::CARD_PAD)
        .width(Length::Fill)
        .height(if is_compact { Length::Fixed(220.0) } else { Length::Fixed(240.0) })
        .style(theme::card_bg);

        // Summary: current node + active connections (clickable → Proxies / Connections)
        let node_label = selected_node.unwrap_or_else(|| {
            if gui_config.active_profile_id.is_none() {
                tr(lang, "dash_no_profile")
            } else {
                tr(lang, "dash_no_node")
            }
        });
        let node_btn = button(
            column![
                text(tr(lang, "dash_current_node")).color(text_muted).size(theme::TYPE_BTN_SM),
                text(crate::ui::util::truncate_chars(node_label, 36))
                    .color(theme::text_primary(theme))
                    .size(theme::TYPE_BODY)
                    .font(iced::Font {
                        weight: iced::font::Weight::Medium,
                        ..Default::default()
                    }),
            ]
            .spacing(4)
            .width(Length::Fill)
        )
        .padding([8, 12])
        .style(theme::button_ghost)
        .on_press(Message::TabChanged(crate::state::Tab::Proxies))
        .width(Length::FillPortion(2));

        let conn_btn = button(
            column![
                text(tr(lang, "dash_connections")).color(text_muted).size(theme::TYPE_BTN_SM),
                text(format!("{}", active_connections))
                    .color(theme::ACCENT_BLUE)
                    .size(theme::TYPE_TITLE)
                    .font(theme::metric_font()),
            ]
            .spacing(4)
            .width(Length::Fill)
        )
        .padding([8, 12])
        .style(theme::button_ghost)
        .on_press(Message::TabChanged(crate::state::Tab::Connections))
        .width(Length::FillPortion(1));

        let summary_card = container(
            row![node_btn, conn_btn]
            .spacing(crate::ui::SP_20)
            .align_y(Alignment::Center)
            .width(Length::Fill)
        )
        .padding(theme::CARD_PAD)
        .width(Length::Fill)
        .style(theme::card_bg);

        let empty_hint: Option<Element<'_, Message>> = if gui_config.active_profile_id.is_none() {
            let cta = button(text(tr(lang, "btn_goto_profiles")).size(theme::TYPE_BTN_MD))
                .padding(theme::BTN_PAD_MD)
                .style(theme::button_primary)
                .on_press(Message::TabChanged(crate::state::Tab::Profiles));
            Some(crate::ui::empty_state(
                tr(lang, "empty_dashboard_title"),
                Some(tr(lang, "empty_dashboard_desc")),
                Some(cta.into()),
                theme,
            ))
        } else {
            None
        };

        let mut content_col = column![
            control_row,
            summary_card,
            traffic_row,
            chart_card
        ]
        .spacing(crate::ui::SP_20)
        .width(Length::Fill);

        if gui_config.tun_mode {
            content_col = content_col.push(
                container(text(tr(lang, "tun_admin_banner")).size(theme::TYPE_BTN_SM).color(theme::WARNING))
                    .padding(crate::ui::SP_12)
                    .width(Length::Fill)
                    .style(|t| theme::tinted_banner(t, theme::WARNING)),
            );
        }

        if let Some(hint) = empty_hint {
            content_col = content_col.push(hint);
        }
        
        let header = page_header("tab_dashboard", lang, None, theme, is_compact);
        
        crate::ui::page_shell_with_pad(header, content_col.into(), is_compact)
    });
    
    main_content.into()
}
