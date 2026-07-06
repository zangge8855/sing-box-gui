use iced::widget::{button, column, container, row, svg, text};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::state::{Bandwidth, GuiConfig};
use crate::ui::theme;

fn format_speed(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B/s", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB/s", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB/s", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub fn render<'a>(
    _gui_config: &'a GuiConfig,
    core_running: bool,
    sys_proxy_enabled: bool,
    current_speed: &Bandwidth,
    speed_history: &[(u64, u64)],
) -> Element<'a, Message> {
    
    // Core Status Section
    let status_indicator = if core_running {
        text("Running").color(theme::SUCCESS).size(18)
    } else {
        text("Stopped").color(theme::DANGER).size(18)
    };
    
    let core_control_btn = if core_running {
        button(text("Stop Core").size(14))
            .padding([10, 20])
            .style(theme::button_danger)
            .on_press(Message::ToggleCore)
    } else {
        button(text("Start Core").size(14))
            .padding([10, 20])
            .style(theme::button_primary)
            .on_press(Message::ToggleCore)
    };
    
    let core_card = container(
        column![
            text("sing-box Core").color(theme::TEXT_MUTED).size(14),
            row![
                status_indicator,
                core_control_btn
            ]
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
        text("Active").color(theme::SUCCESS).size(18)
    } else {
        text("Inactive").color(theme::TEXT_MUTED).size(18)
    };
    
    let sys_proxy_btn = button(
        text(if sys_proxy_enabled { "Disable Proxy" } else { "Enable Proxy" }).size(14)
    )
    .padding([10, 20])
    .style(if sys_proxy_enabled { theme::button_danger } else { theme::button_primary })
    .on_press(Message::ToggleSystemProxy);
    
    let proxy_card = container(
        column![
            text("System Proxy").color(theme::TEXT_MUTED).size(14),
            row![
                sys_proxy_indicator,
                sys_proxy_btn
            ]
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
    
    // Speed Stats cards
    let download_card = container(
        column![
            text("Download").color(theme::TEXT_MUTED).size(14),
            text(format_speed(current_speed.down))
                .color(theme::ACCENT_BLUE)
                .size(28),
        ]
        .spacing(5)
    )
    .padding(20)
    .width(Length::FillPortion(1))
    .style(theme::card_bg);
    
    let upload_card = container(
        column![
            text("Upload").color(theme::TEXT_MUTED).size(14),
            text(format_speed(current_speed.up))
                .color(theme::ACCENT_PURPLE)
                .size(28),
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
    
    let svg_xml = format!(
        r##"<svg viewBox="0 0 300 100" xmlns="http://www.w3.org/2000/svg">
             <defs>
               <linearGradient id="downGrad" x1="0" y1="0" x2="0" y2="1">
                 <stop offset="0%" stop-color="#3b82f6" stop-opacity="0.2"/>
                 <stop offset="100%" stop-color="#3b82f6" stop-opacity="0"/>
               </linearGradient>
             </defs>
             <path d="{}" fill="url(#downGrad)" stroke="#3b82f6" stroke-width="1.5"/>
             <path d="{}" fill="none" stroke="#8b5cf6" stroke-width="1.5"/>
           </svg>"##,
        down_path, up_path
    );
    
    let chart_handle = svg::Handle::from_memory(svg_xml.into_bytes());
    let chart_svg = svg(chart_handle)
        .width(Length::Fill)
        .height(Length::Fixed(180.0));
        
    let chart_card = container(
        column![
            text("Real-time Speed History").color(theme::TEXT_MUTED).size(14),
            chart_svg
        ]
        .spacing(10)
    )
    .padding(20)
    .style(theme::card_bg);
    
    // Overall view layout
    container(
        column![
            text("Dashboard").size(24).color(theme::TEXT_PRIMARY),
            controls_row,
            speed_row,
            chart_card
        ]
        .spacing(25)
        .max_width(800)
    )
    .padding(20)
    .into()
}
