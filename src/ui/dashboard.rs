use iced::widget::{button, column, container, row, scrollable, svg, text, Space};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::state::{Bandwidth, GuiConfig, RoutingMode};
use crate::ui::theme;

fn icon(unicode: char) -> text::Text<'static> {
    text(unicode.to_string()).font(iced::Font::with_name("Icons")).size(16)
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
    
    let text_primary = theme::text_primary(theme);
    let text_muted = theme::text_muted(theme);
    
    // Core Status Section
    let status_indicator = if core_running {
        text(tr(lang, "status_running")).color(theme::SUCCESS).size(16)
    } else {
        text(tr(lang, "status_stopped")).color(theme::DANGER).size(16)
    };
    
    let core_control_btn = if core_running {
        button(
            row![icon('\u{E047}'), text(tr(lang, "btn_stop_core")).size(13)]
                .spacing(8)
                .align_y(Alignment::Center)
        )
        .padding([8, 16])
        .width(Length::Fixed(120.0))
        .style(theme::button_danger)
        .on_press(Message::ToggleCore)
    } else {
        button(
            row![icon('\u{E037}'), text(tr(lang, "btn_start_core")).size(13)]
                .spacing(8)
                .align_y(Alignment::Center)
        )
        .padding([8, 16])
        .width(Length::Fixed(120.0))
        .style(theme::button_primary)
        .on_press(Message::ToggleCore)
    };
    
    // System Proxy Control Section
    let sys_proxy_indicator = if sys_proxy_enabled {
        text(tr(lang, "enabled")).color(theme::SUCCESS).size(16)
    } else {
        text(tr(lang, "disabled")).color(text_muted).size(16)
    };
    
    let sys_proxy_btn = button(
        row![
            icon(if sys_proxy_enabled { '\u{E047}' } else { '\u{E037}' }),
            text(if sys_proxy_enabled { tr(lang, "btn_disable_proxy") } else { tr(lang, "btn_enable_proxy") }).size(13)
        ]
        .spacing(8)
        .align_y(Alignment::Center)
    )
    .padding([8, 16])
    .width(Length::Fixed(120.0))
    .style(if sys_proxy_enabled { theme::button_danger } else { theme::button_primary })
    .on_press(Message::ToggleSystemProxy);

    let system_control_card = container(
        column![
            row![
                row![icon('\u{E322}'), text(tr(lang, "singbox_core")).color(text_muted).size(13)].spacing(8).align_y(Alignment::Center),
                Space::new().width(Length::Fill),
                status_indicator,
                Space::new().width(Length::Fixed(16.0)),
                core_control_btn
            ]
            .width(Length::Fill)
            .align_y(Alignment::Center),
            
            row![
                row![icon('\u{E32A}'), text(tr(lang, "system_proxy")).color(text_muted).size(13)].spacing(8).align_y(Alignment::Center),
                Space::new().width(Length::Fill),
                sys_proxy_indicator,
                Space::new().width(Length::Fixed(16.0)),
                sys_proxy_btn
            ]
            .width(Length::Fill)
            .align_y(Alignment::Center)
        ]
        .spacing(24) // Increased spacing
    )
    .padding(24) // Increased padding
    .width(Length::Fill)
    .style(theme::card_bg);
    
    // Routing Mode selection card
    let make_mode_btn = |mode: RoutingMode, key: &'static str| {
        let active = gui_config.routing_mode == mode;
        let btn = button(
            text(tr(lang, key))
                .size(12)
                .width(Length::Fill)
                .align_x(Alignment::Center)
        )
        .padding([8, 10])
        .width(Length::Fill)
        .style(move |t, s| {
            if active {
                theme::button_primary(t, s)
            } else {
                theme::button_secondary(t, s)
            }
        });
            
        let btn_el: Element<'a, Message> = if active {
            btn.into()
        } else {
            btn.on_press(Message::RoutingModeChanged(mode)).into()
        };
        btn_el
    };
    
    let mode_buttons = row![
        make_mode_btn(RoutingMode::Rule, "routing_rules_desc"),
        make_mode_btn(RoutingMode::Global, "routing_global_desc"),
        make_mode_btn(RoutingMode::Direct, "routing_direct_desc")
    ]
    .spacing(10)
    .width(Length::Fill);
    
    let mode_card = container(
        column![
            text(tr(lang, "active_mode")).color(text_muted).size(13),
            mode_buttons
        ]
        .spacing(20) // Increased spacing
    )
    .padding(24) // Increased padding
    .width(Length::Fill)
    .style(theme::card_bg);

    // Speed Stats cards
    let download_card = container(
        column![
            row![
                icon('\u{E5DB}').color(theme::ACCENT_BLUE),
                text(tr(lang, "download")).color(text_muted).size(13)
            ]
            .spacing(4)
            .align_y(Alignment::Center),
            text(format_speed(current_speed.down))
                .font(iced::Font {
                    family: iced::font::Family::Monospace,
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::ACCENT_BLUE)
                .size(32),
            text(format!("{} {}", tr(lang, "total_label"), format_size(total_downloaded)))
                .color(theme::text_tertiary(theme))
                .size(11),
        ]
        .spacing(12)
    )
    .padding(24)
    .width(Length::FillPortion(1))
    .style(theme::card_bg);
    
    let upload_card = container(
        column![
            row![
                icon('\u{E5D8}').color(theme::ACCENT_PURPLE),
                text(tr(lang, "upload")).color(text_muted).size(13)
            ]
            .spacing(4)
            .align_y(Alignment::Center),
            text(format_speed(current_speed.up))
                .font(iced::Font {
                    family: iced::font::Family::Monospace,
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(theme::ACCENT_PURPLE)
                .size(32),
            text(format!("{} {}", tr(lang, "total_label"), format_size(total_uploaded)))
                .color(theme::text_tertiary(theme))
                .size(11),
        ]
        .spacing(12)
    )
    .padding(24)
    .width(Length::FillPortion(1))
    .style(theme::card_bg);
    
    let speed_row = row![
        download_card,
        upload_card
    ]
    .spacing(24); // Increased spacing between cards
    
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

    let down_color_hex = format!("rgba({}, {}, {}, {})", (theme::ACCENT_BLUE.r * 255.0) as u8, (theme::ACCENT_BLUE.g * 255.0) as u8, (theme::ACCENT_BLUE.b * 255.0) as u8, 1.0);
    let up_color_hex = format!("rgba({}, {}, {}, {})", (theme::ACCENT_PURPLE.r * 255.0) as u8, (theme::ACCENT_PURPLE.g * 255.0) as u8, (theme::ACCENT_PURPLE.b * 255.0) as u8, 1.0);

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
    .height(Length::Fill)
    .style(theme::card_bg);
    
    // Balanced 2-Column Responsive Dashboard Layout
    let left_col = column![
        system_control_card,
        mode_card
    ]
    .spacing(20)
    .width(Length::FillPortion(5));
    
    let right_col = column![
        speed_row,
        container(chart_card).height(Length::Fixed(220.0))
    ]
    .spacing(20)
    .width(Length::FillPortion(7));
    
    let main_content = container(
        row![
            left_col,
            right_col
        ]
        .spacing(20)
        .width(Length::Fill)
    )
    .max_width(1200.0);

    let header_row = container(
        row![
            text(tr(lang, "tab_dashboard")).size(24).color(text_primary),
        ]
        .align_y(Alignment::Center)
        .width(Length::Fill)
    )
    .max_width(1200.0)
    .padding(iced::Padding { top: 0.0, right: 0.0, bottom: 10.0, left: 0.0 });

    let content_col = column![
        header_row,
        main_content
    ]
    .spacing(10)
    .width(Length::Fill)
    .align_x(Alignment::Center);

    container(
        scrollable(
            container(content_col)
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding(iced::Padding { top: 10.0, right: 20.0, bottom: 30.0, left: 20.0 })
        )
        .height(Length::Fill)
    )
    .padding(0)
    .into()
}
