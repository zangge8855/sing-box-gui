use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Element};
use crate::message::Message;
use crate::state::{GuiConfig, RoutingMode};
use crate::ui::theme;

pub fn render<'a>(
    gui_config: &'a GuiConfig,
    core_installed: bool,
    install_message: Option<&'a str>,
) -> Element<'a, Message> {
    
    // Core Status / Downloader Card
    let core_downloader = if core_installed {
        row![
            text("sing-box core status: Installed").color(theme::SUCCESS).size(14),
            text(" (v1.13.14 stable)").color(theme::TEXT_MUTED).size(12)
        ]
        .spacing(5)
        .align_y(Alignment::Center)
    } else {
        let btn = button(text("Download & Install sing-box Core").size(13))
            .padding([8, 16])
            .style(theme::button_primary)
            .on_press(Message::NewLogLine("TRIGGER_CORE_DOWNLOAD".to_string()));
            
        row![
            text("sing-box core: Not Found").color(theme::DANGER).size(14),
            btn
        ]
        .spacing(20)
        .align_y(Alignment::Center)
    };
    
    let install_status_row = if let Some(msg) = install_message {
        row![text(msg).color(theme::WARNING).size(13)]
    } else {
        row![]
    };
    
    let core_card = container(
        column![
            text("Core Components").color(theme::TEXT_MUTED).size(13),
            core_downloader,
            install_status_row
        ]
        .spacing(15)
    )
    .padding(20)
    .style(theme::card_bg);
    
    // Routing Mode Selector
    let routing_label = text("Routing Rule Mode").color(theme::TEXT_MUTED).size(13);
    
    let make_mode_btn = |mode: RoutingMode, label: &'static str| {
        let active = gui_config.routing_mode == mode;
        let btn = button(text(label).size(13))
            .padding([8, 16])
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
    
    let routing_row = row![
        make_mode_btn(RoutingMode::Rule, "Rules (Bypass LAN/CN)"),
        make_mode_btn(RoutingMode::Global, "Global (All Proxy)"),
        make_mode_btn(RoutingMode::Direct, "Direct (Bypass All)")
    ]
    .spacing(15);
    
    let routing_card = container(
        column![
            routing_label,
            routing_row
        ]
        .spacing(10)
    )
    .padding(20)
    .style(theme::card_bg);
    
    // Inbound & Port Settings Card
    let mixed_port_input = text_input("2080", &gui_config.mixed_port.to_string())
        .on_input(|s| Message::PortInputChanged(format!("mixed:{}", s)))
        .padding(10)
        .style(theme::input_field)
        .width(100);
        
    let api_port_input = text_input("9090", &gui_config.api_port.to_string())
        .on_input(|s| Message::PortInputChanged(format!("api:{}", s)))
        .padding(10)
        .style(theme::input_field)
        .width(100);
        
    let ports_row = row![
        column![
            text("Mixed Port (HTTP+SOCKS)").color(theme::TEXT_MUTED).size(12),
            mixed_port_input
        ].spacing(5),
        column![
            text("Clash API Port").color(theme::TEXT_MUTED).size(12),
            api_port_input
        ].spacing(5),
    ]
    .spacing(30);
    
    // TUN mode & autostart checkboxes toggles
    let tun_btn = button(
        text(if gui_config.tun_mode { "TUN Mode: ON" } else { "TUN Mode: OFF" }).size(13)
    )
    .padding([8, 16])
    .style(if gui_config.tun_mode { theme::button_primary } else { theme::button_secondary })
    .on_press(Message::PortInputChanged("toggle_tun".to_string()));
    
    let autostart_btn = button(
        text(if gui_config.start_on_boot { "Start on Boot: ON" } else { "Start on Boot: OFF" }).size(13)
    )
    .padding([8, 16])
    .style(if gui_config.start_on_boot { theme::button_primary } else { theme::button_secondary })
    .on_press(Message::PortInputChanged("toggle_autostart".to_string()));
    
    let toggles_row = row![
        tun_btn,
        autostart_btn
    ]
    .spacing(20);
    
    let settings_card = container(
        column![
            text("Core Network Settings").color(theme::TEXT_MUTED).size(13),
            ports_row,
            toggles_row
        ]
        .spacing(15)
    )
    .padding(20)
    .style(theme::card_bg);
    
    // DNS settings card
    let dns_local_input = text_input("223.5.5.5", &gui_config.dns_server_local)
        .on_input(|s| Message::PortInputChanged(format!("dns_local:{}", s)))
        .padding(10)
        .style(theme::input_field)
        .width(220);
        
    let dns_remote_input = text_input("8.8.8.8", &gui_config.dns_server_remote)
        .on_input(|s| Message::PortInputChanged(format!("dns_remote:{}", s)))
        .padding(10)
        .style(theme::input_field)
        .width(220);
        
    let dns_row = row![
        column![
            text("Local DNS Server").color(theme::TEXT_MUTED).size(12),
            dns_local_input
        ].spacing(5),
        column![
            text("Remote DNS Server").color(theme::TEXT_MUTED).size(12),
            dns_remote_input
        ].spacing(5),
    ]
    .spacing(30);
    
    let dns_card = container(
        column![
            text("DNS Configuration").color(theme::TEXT_MUTED).size(13),
            dns_row
        ]
        .spacing(15)
    )
    .padding(20)
    .style(theme::card_bg);
    
    // Save button
    let save_btn = button(text("Save & Apply Settings").size(14))
        .padding([10, 20])
        .style(theme::button_primary)
        .on_press(Message::SaveSettings);
        
    container(
        column![
            text("Settings").size(24).color(theme::TEXT_PRIMARY),
            core_card,
            routing_card,
            settings_card,
            dns_card,
            save_btn
        ]
        .spacing(20)
        .max_width(800)
    )
    .padding(20)
    .into()
}
