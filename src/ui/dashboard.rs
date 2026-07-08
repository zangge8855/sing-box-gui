use iced::widget::{button, column, container, row, svg, text, Space, responsive, scrollable, pick_list};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::state::{Bandwidth, GuiConfig, RoutingMode};
use crate::ui::theme;
use crate::ui::page_header;

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
    text(unicode.to_string()).font(iced::Font::with_name("Material Icons")).size(16)
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{:.2} B", bytes as f64)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn format_speed(bytes: u64) -> String {
    format!("{}/s", format_size(bytes))
}

pub fn render<'a>(
    gui_config: &'a GuiConfig,
    core_running: bool,
    sys_proxy_enabled: bool,
    current_speed: &Bandwidth,
    speed_history: &[(u64, u64)],
    total_uploaded: u64,
    total_downloaded: u64,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    use crate::ui::i18n::tr;
    
    // Clone properties to move them into the responsive Fn closure
    let theme_cloned = theme.clone();
    let speed_cloned = current_speed.clone();
    let history_cloned = speed_history.to_vec();
    
    let main_content = responsive(move |size| {
        let is_compact = size.width < 900.0;
        let theme = &theme_cloned;
        let current_speed = &speed_cloned;
        let speed_history = &history_cloned;
        
        let text_muted = theme::text_muted(theme);
        
        // 1. Core Status Card
        let status_indicator = if core_running {
            row![
                container(Space::new())
                    .width(8)
                    .height(8)
                    .style(|_t| container::Style {
                        background: Some(iced::Background::Color(theme::SUCCESS)),
                        border: iced::Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                text(tr(lang, "status_running")).color(theme::SUCCESS).size(14)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        } else {
            row![
                container(Space::new())
                    .width(8)
                    .height(8)
                    .style(|_t| container::Style {
                        background: Some(iced::Background::Color(theme::DANGER)),
                        border: iced::Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                text(tr(lang, "status_stopped")).color(theme::DANGER).size(14)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        };
        
        let core_control_btn = if core_running {
            button(
                row![icon('\u{E047}'), text(tr(lang, "btn_stop_core")).size(12)]
                    .spacing(6)
                    .align_y(Alignment::Center)
            )
            .padding([6, 12])
            .style(theme::button_danger)
            .on_press(Message::ToggleCore)
        } else {
            button(
                row![icon('\u{E037}'), text(tr(lang, "btn_start_core")).size(12)]
                    .spacing(6)
                    .align_y(Alignment::Center)
            )
            .padding([6, 12])
            .style(theme::button_primary)
            .on_press(Message::ToggleCore)
        };

        let core_status_card = container(
            column![
                row![
                    icon('\u{E322}').color(theme::ACCENT_PURPLE),
                    text(tr(lang, "singbox_core")).color(text_muted).size(13)
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                
                row![
                    status_indicator,
                    Space::new().width(Length::Fill),
                    core_control_btn
                ]
                .align_y(Alignment::Center)
                .width(Length::Fill)
            ]
            .spacing(16)
        )
        .padding(20)
        .width(Length::FillPortion(1))
        .style(theme::card_bg);

        // 2. System Proxy Card
        let sys_proxy_indicator = if sys_proxy_enabled {
            row![
                container(Space::new())
                    .width(8)
                    .height(8)
                    .style(|_t| container::Style {
                        background: Some(iced::Background::Color(theme::SUCCESS)),
                        border: iced::Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                text(tr(lang, "enabled")).color(theme::SUCCESS).size(14)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        } else {
            row![
                container(Space::new())
                    .width(8)
                    .height(8)
                    .style(move |_t| container::Style {
                        background: Some(iced::Background::Color(text_muted)),
                        border: iced::Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                text(tr(lang, "disabled")).color(text_muted).size(14)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        };
        
        let sys_proxy_btn = button(
            row![
                icon(if sys_proxy_enabled { '\u{E047}' } else { '\u{E037}' }),
                text(if sys_proxy_enabled { tr(lang, "btn_disable_proxy") } else { tr(lang, "btn_enable_proxy") }).size(12)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        )
        .padding([6, 12])
        .style(if sys_proxy_enabled { theme::button_danger } else { theme::button_primary })
        .on_press(Message::ToggleSystemProxy);

        let proxy_status_card = container(
            column![
                row![
                    icon('\u{E32A}').color(theme::ACCENT_BLUE),
                    text(tr(lang, "system_proxy")).color(text_muted).size(13)
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                
                row![
                    sys_proxy_indicator,
                    Space::new().width(Length::Fill),
                    sys_proxy_btn
                ]
                .align_y(Alignment::Center)
                .width(Length::Fill)
            ]
            .spacing(16)
        )
        .padding(20)
        .width(Length::FillPortion(1))
        .style(theme::card_bg);

        // 3. Download Speed Card
        let download_card = container(
            column![
                row![
                    icon('\u{E5DB}').color(theme::ACCENT_BLUE),
                    text(tr(lang, "download")).color(text_muted).size(13)
                ]
                .spacing(6)
                .align_y(Alignment::Center),
                
                row![
                    text(format_speed(current_speed.down))
                        .font(iced::Font {
                            family: iced::font::Family::Monospace,
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        })
                        .color(theme::ACCENT_BLUE)
                        .size(22),
                    Space::new().width(Length::Fill),
                    text(format!("{} {}", tr(lang, "total_label"), format_size(total_downloaded)))
                        .color(theme::text_tertiary(theme))
                        .size(11)
                ]
                .align_y(Alignment::Center)
                .width(Length::Fill)
            ]
            .spacing(16)
        )
        .padding(20)
        .width(Length::FillPortion(1))
        .style(theme::card_bg);

        // 4. Upload Speed Card
        let upload_card = container(
            column![
                row![
                    icon('\u{E5D8}').color(theme::ACCENT_PURPLE),
                    text(tr(lang, "upload")).color(text_muted).size(13)
                ]
                .spacing(6)
                .align_y(Alignment::Center),
                
                row![
                    text(format_speed(current_speed.up))
                        .font(iced::Font {
                            family: iced::font::Family::Monospace,
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        })
                        .color(theme::ACCENT_PURPLE)
                        .size(22),
                    Space::new().width(Length::Fill),
                    text(format!("{} {}", tr(lang, "total_label"), format_size(total_uploaded)))
                        .color(theme::text_tertiary(theme))
                        .size(11)
                ]
                .align_y(Alignment::Center)
                .width(Length::Fill)
            ]
            .spacing(16)
        )
        .padding(20)
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
                    icon('\u{E8B8}').color(theme::ACCENT_BLUE),
                    text(tr(lang, "active_mode")).color(text_muted).size(13),
                    Space::new().width(Length::Fill),
                    text(port_text).color(text_muted).size(12)
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill),
                
                row![
                    mode_selector
                ]
                .align_y(Alignment::Center)
                .width(Length::Fill)
            ]
            .spacing(16)
        )
        .padding(20)
        .width(Length::FillPortion(1))
        .style(theme::card_bg);

        // Layout the control status cards responsively
        let control_row: Element<'_, Message> = if is_compact {
            column![
                core_status_card,
                proxy_status_card,
                routing_mode_card
            ]
            .spacing(16)
            .width(Length::Fill)
            .into()
        } else {
            row![
                core_status_card,
                proxy_status_card,
                routing_mode_card
            ]
            .spacing(16)
            .width(Length::Fill)
            .into()
        };

        // Layout the traffic metric cards responsively
        let traffic_row: Element<'_, Message> = if is_compact {
            column![
                download_card,
                upload_card
            ]
            .spacing(16)
            .width(Length::Fill)
            .into()
        } else {
            row![
                download_card,
                upload_card
            ]
            .spacing(16)
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
            
        let chart_card = container(
            column![
                text(tr(lang, "speed_history_chart")).color(text_muted).size(13),
                container(chart_svg)
                    .width(Length::Fill)
                    .height(Length::Fill)
            ]
            .spacing(10)
            .height(Length::Fill)
        )
        .padding(20)
        .width(Length::Fill)
        .height(if is_compact { Length::Fixed(220.0) } else { Length::Fixed(240.0) })
        .style(theme::card_bg);

        let content_col = column![
            control_row,
            traffic_row,
            chart_card
        ]
        .spacing(20)
        .width(Length::Fill);
        
        let header = page_header("tab_dashboard", lang, None, theme, is_compact);
        
        let col = column![header, content_col].spacing(20).width(Length::Fill);

        let inner = container(col)
            .width(Length::Fill)
            .max_width(1200.0)
            .center_x(Length::Fill)
            .padding(crate::ui::page_padding());

        container(
            scrollable(inner).height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    });
    
    main_content.into()
}
