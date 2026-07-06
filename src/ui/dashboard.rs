use iced::widget::{button, column, container, row, scrollable, svg, text};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::state::{Bandwidth, GuiConfig, RoutingMode};
use crate::ui::theme;

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
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
        text(tr(lang, "status_running")).color(theme::SUCCESS).size(18).width(Length::Fill)
    } else {
        text(tr(lang, "status_stopped")).color(theme::DANGER).size(18).width(Length::Fill)
    };
    
    let core_control_btn = if core_running {
        button(
            text(tr(lang, "btn_stop_core"))
                .size(14)
                .width(Length::Fill)
                .align_x(Alignment::Center)
        )
        .padding([10, 20])
        .width(Length::Fixed(150.0))
        .style(theme::button_danger)
        .on_press(Message::ToggleCore)
    } else {
        button(
            text(tr(lang, "btn_start_core"))
                .size(14)
                .width(Length::Fill)
                .align_x(Alignment::Center)
        )
        .padding([10, 20])
        .width(Length::Fixed(150.0))
        .style(theme::button_primary)
        .on_press(Message::ToggleCore)
    };
    
    let core_card = container(
        column![
            text(tr(lang, "singbox_core")).color(text_muted).size(14),
            row![
                status_indicator,
                core_control_btn
            ]
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .spacing(20)
        ]
        .spacing(10)
    )
    .padding(20)
    .width(Length::FillPortion(1))
    .style(theme::status_card);
    
    // System Proxy Control Section
    let sys_proxy_indicator = if sys_proxy_enabled {
        text(tr(lang, "enabled")).color(theme::SUCCESS).size(18).width(Length::Fill)
    } else {
        text(tr(lang, "disabled")).color(text_muted).size(18).width(Length::Fill)
    };
    
    let sys_proxy_btn = button(
        text(if sys_proxy_enabled { tr(lang, "btn_disable_proxy") } else { tr(lang, "btn_enable_proxy") })
            .size(14)
            .width(Length::Fill)
            .align_x(Alignment::Center)
    )
    .padding([10, 20])
    .width(Length::Fixed(150.0))
    .style(if sys_proxy_enabled { theme::button_danger } else { theme::button_primary })
    .on_press(Message::ToggleSystemProxy);
    
    let proxy_card = container(
        column![
            text(tr(lang, "system_proxy")).color(text_muted).size(14),
            row![
                sys_proxy_indicator,
                sys_proxy_btn
            ]
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .spacing(20)
        ]
        .spacing(10)
    )
    .padding(20)
    .width(Length::FillPortion(1))
    .style(theme::status_card);
    
    // Combined controls row
    let controls_row = row![
        core_card,
        proxy_card
    ]
    .spacing(20);

    // Connection info summary row
    let make_mode_btn = |mode: RoutingMode, key: &'static str| {
        let active = gui_config.routing_mode == mode;
        let btn = button(
            text(tr(lang, key))
                .size(12)
                .width(Length::Fill)
                .align_x(Alignment::Center)
        )
        .padding([6, 12])
        .width(Length::Fill)
        .style(move |theme, status| {
            if active {
                theme::button_primary(theme, status)
            } else {
                theme::button_secondary(theme, status)
            }
        });
            
        if active {
            btn
        } else {
            btn.on_press(Message::RoutingModeChanged(mode))
        }
    };
    
    let mode_buttons = row![
        make_mode_btn(RoutingMode::Rule, "mode_rules"),
        make_mode_btn(RoutingMode::Global, "mode_global"),
        make_mode_btn(RoutingMode::Direct, "mode_direct")
    ]
    .spacing(8);

    let mode_card = container(
        column![
            text(tr(lang, "active_mode")).color(text_muted).size(12),
            mode_buttons
        ]
        .spacing(8)
    )
    .padding(16)
    .width(Length::FillPortion(1))
    .style(theme::card_bg);

    let port_card = container(
        column![
            text(tr(lang, "listen_port")).color(text_muted).size(12),
            text(format!("{}", gui_config.mixed_port)).color(text_primary).size(20),
        ]
        .spacing(6)
    )
    .padding(16)
    .width(Length::FillPortion(1))
    .style(theme::card_bg);

    let info_row = row![
        mode_card,
        port_card
    ]
    .spacing(20);

    // Speed Stats cards
    let download_card = container(
        column![
            text(tr(lang, "download")).color(text_muted).size(14),
            text(format_speed(current_speed.down))
                .color(theme::ACCENT_BLUE)
                .size(28),
            text(format!("Total: {}", format_size(total_downloaded)))
                .color(text_muted)
                .size(12),
        ]
        .spacing(5)
    )
    .padding(20)
    .width(Length::FillPortion(1))
    .style(theme::card_bg);
    
    let upload_card = container(
        column![
            text(tr(lang, "upload")).color(text_muted).size(14),
            text(format_speed(current_speed.up))
                .color(theme::ACCENT_PURPLE)
                .size(28),
            text(format!("Total: {}", format_size(total_uploaded)))
                .color(text_muted)
                .size(12),
        ]
        .spacing(5)
    )
    .padding(20)
    .width(Length::FillPortion(1))
    .style(theme::card_bg);
    
    let speed_row = row![
        download_card,
        upload_card
    ]
    .spacing(20);
    
    // Render dynamic SVG chart of speed history
    let max_speed = speed_history.iter()
        .map(|&(u, d)| u.max(d))
        .max()
        .unwrap_or(0)
        .max(1024 * 100); // 100 KB/s min scale
        
    let points_count = speed_history.len();
    
    let mut down_path = String::new();
    let mut up_path = String::new();
    
    if points_count > 1 {
        // Build download speed area path
        down_path.push_str("M 0 100");
        for (i, &(_, down)) in speed_history.iter().enumerate() {
            let x = (i as f32 * (300.0 / (points_count - 1) as f32)) as i32;
            let y = (100.0 - (down as f32 / max_speed as f32 * 80.0)) as i32;
            down_path.push_str(&format!(" L {} {}", x, y));
        }
        down_path.push_str(" L 300 100 Z");
        
        // Build upload speed line path
        for (i, &(up, _)) in speed_history.iter().enumerate() {
            let x = (i as f32 * (300.0 / (points_count - 1) as f32)) as i32;
            let y = (100.0 - (up as f32 / max_speed as f32 * 80.0)) as i32;
            if i == 0 {
                up_path.push_str(&format!("M {} {}", x, y));
            } else {
                up_path.push_str(&format!(" L {} {}", x, y));
            }
        }
    } else {
        down_path = "M 0 100 L 300 100 Z".to_string();
        up_path = "M 0 100 L 300 100".to_string();
    }
    
    let grid_color = if theme::is_dark(theme) {
        "rgba(255, 255, 255, 0.08)"
    } else {
        "rgba(0, 0, 0, 0.06)"
    };

    let svg_xml = format!(
        r##"<svg viewBox="0 0 300 100" xmlns="http://www.w3.org/2000/svg">
             <defs>
               <linearGradient id="downGrad" x1="0" y1="0" x2="0" y2="1">
                 <stop offset="0%" stop-color="#3b82f6" stop-opacity="0.2"/>
                 <stop offset="100%" stop-color="#3b82f6" stop-opacity="0"/>
               </linearGradient>
             </defs>
             <line x1="0" y1="20" x2="300" y2="20" stroke="{}" stroke-dasharray="2 2" stroke-width="0.5"/>
             <line x1="0" y1="40" x2="300" y2="40" stroke="{}" stroke-dasharray="2 2" stroke-width="0.5"/>
             <line x1="0" y1="60" x2="300" y2="60" stroke="{}" stroke-dasharray="2 2" stroke-width="0.5"/>
             <line x1="0" y1="80" x2="300" y2="80" stroke="{}" stroke-dasharray="2 2" stroke-width="0.5"/>
             <path d="{}" fill="url(#downGrad)" stroke="#3b82f6" stroke-width="1.5"/>
             <path d="{}" fill="none" stroke="#8b5cf6" stroke-width="1.5"/>
           </svg>"##,
         grid_color, grid_color, grid_color, grid_color, down_path, up_path
    );
    
    let chart_handle = svg::Handle::from_memory(svg_xml.into_bytes());
    let chart_svg = svg(chart_handle)
        .width(Length::Fill)
        .height(Length::Fixed(180.0));
        
    let chart_card = container(
        column![
            text(tr(lang, "speed_history_chart")).color(text_muted).size(14),
            chart_svg
        ]
        .spacing(10)
    )
    .padding(20)
    .style(theme::card_bg);
    
    // Overall view layout
    let main_col = column![
        text(tr(lang, "tab_dashboard")).size(24).color(text_primary),
        controls_row,
        info_row,
        speed_row,
        chart_card
    ]
    .spacing(25)
    .max_width(800);

    container(
        scrollable(main_col)
            .height(Length::Fill)
            .width(Length::Fill)
    )
    .padding(20)
    .into()
}
