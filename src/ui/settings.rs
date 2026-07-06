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
    
    let lang = gui_config.language;
    use crate::ui::i18n::tr;
    
    // Core Status / Downloader Card
    let core_downloader = if core_installed {
        row![
            text(tr(lang, "core_installed_status")).color(theme::SUCCESS).size(14),
            text(tr(lang, "core_ver_stable")).color(theme::TEXT_MUTED).size(12)
        ]
        .spacing(5)
        .align_y(Alignment::Center)
    } else {
        let btn = button(text(tr(lang, "btn_download_core")).size(13))
            .padding([8, 16])
            .style(theme::button_primary)
            .on_press(Message::NewLogLine("TRIGGER_CORE_DOWNLOAD".to_string()));
            
        row![
            text(tr(lang, "core_not_found")).color(theme::DANGER).size(14),
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
    
    let open_dir_btn = button(text(tr(lang, "btn_open_data_dir")).size(13))
        .padding([8, 16])
        .style(theme::button_secondary)
        .on_press(Message::PortInputChanged("open_data_dir".to_string()));

    let core_card = container(
        column![
            text(tr(lang, "core_components")).color(theme::TEXT_MUTED).size(13),
            core_downloader,
            open_dir_btn,
            install_status_row
        ]
        .spacing(15)
    )
    .padding(20)
    .style(theme::card_bg);
    
    // Routing Mode Selector
    let routing_label = text(tr(lang, "routing_rule_mode")).color(theme::TEXT_MUTED).size(13);
    
    let make_mode_btn = |mode: RoutingMode, key: &'static str| {
        let active = gui_config.routing_mode == mode;
        let btn = button(text(tr(lang, key)).size(13))
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
        make_mode_btn(RoutingMode::Rule, "routing_rules_desc"),
        make_mode_btn(RoutingMode::Global, "routing_global_desc"),
        make_mode_btn(RoutingMode::Direct, "routing_direct_desc")
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
            text(tr(lang, "mixed_port_label")).color(theme::TEXT_MUTED).size(12),
            mixed_port_input
        ].spacing(5),
        column![
            text(tr(lang, "api_port_label")).color(theme::TEXT_MUTED).size(12),
            api_port_input
        ].spacing(5),
    ]
    .spacing(30);
    
    // TUN mode & autostart checkboxes toggles
    let tun_btn = button(
        text(format!("{}: {}", tr(lang, "tun_mode_label"), if gui_config.tun_mode { "ON" } else { "OFF" })).size(13)
    )
    .padding([8, 16])
    .style(if gui_config.tun_mode { theme::button_primary } else { theme::button_secondary })
    .on_press(Message::PortInputChanged("toggle_tun".to_string()));
    
    let autostart_btn = button(
        text(format!("{}: {}", tr(lang, "autostart_label"), if gui_config.start_on_boot { "ON" } else { "OFF" })).size(13)
    )
    .padding([8, 16])
    .style(if gui_config.start_on_boot { theme::button_primary } else { theme::button_secondary })
    .on_press(Message::PortInputChanged("toggle_autostart".to_string()));
    
    let close_core_btn = button(
        text(format!("{}: {}", tr(lang, "close_core_on_exit_label"), if gui_config.close_core_on_exit { "ON" } else { "OFF" })).size(13)
    )
    .padding([8, 16])
    .style(if gui_config.close_core_on_exit { theme::button_primary } else { theme::button_secondary })
    .on_press(Message::PortInputChanged("toggle_close_core".to_string()));
    
    let toggles_row = row![
        tun_btn,
        autostart_btn,
        close_core_btn
    ]
    .spacing(20);
    
    let tfo_btn = button(
        text(format!("{}: {}", tr(lang, "tcp_fast_open_label"), if gui_config.tcp_fast_open { "ON" } else { "OFF" })).size(13)
    )
    .padding([8, 16])
    .style(if gui_config.tcp_fast_open { theme::button_primary } else { theme::button_secondary })
    .on_press(Message::PortInputChanged("toggle_tfo".to_string()));
    
    let mptcp_btn = button(
        text(format!("{}: {}", tr(lang, "tcp_multipath_label"), if gui_config.tcp_multipath { "ON" } else { "OFF" })).size(13)
    )
    .padding([8, 16])
    .style(if gui_config.tcp_multipath { theme::button_primary } else { theme::button_secondary })
    .on_press(Message::PortInputChanged("toggle_mptcp".to_string()));
    
    let performance_row = row![
        tfo_btn,
        mptcp_btn
    ]
    .spacing(20);
    
    let settings_card = container(
        column![
            text(tr(lang, "ports_config")).color(theme::TEXT_MUTED).size(13),
            ports_row,
            toggles_row,
            performance_row
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
            text(tr(lang, "dns_local")).color(theme::TEXT_MUTED).size(12),
            dns_local_input
        ].spacing(5),
        column![
            text(tr(lang, "dns_remote")).color(theme::TEXT_MUTED).size(12),
            dns_remote_input
        ].spacing(5),
    ]
    .spacing(30);
    
    let fakeip_btn = button(
        text(format!("{}: {}", tr(lang, "fake_ip_label"), if gui_config.fake_ip { "ON" } else { "OFF" })).size(13)
    )
    .padding([8, 16])
    .style(if gui_config.fake_ip { theme::button_primary } else { theme::button_secondary })
    .on_press(Message::PortInputChanged("toggle_fakeip".to_string()));
    
    let dns_card = container(
        column![
            text(tr(lang, "dns_servers")).color(theme::TEXT_MUTED).size(13),
            dns_row,
            fakeip_btn
        ]
        .spacing(15)
    )
    .padding(20)
    .style(theme::card_bg);
    
    // Language card
    let lang_label = text(tr(lang, "app_language")).color(theme::TEXT_MUTED).size(13);
    
    let make_lang_btn = |target_lang: crate::state::Language, label: &'static str| {
        let active = gui_config.language == target_lang;
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
            let msg_str = match target_lang {
                crate::state::Language::En => "lang:en",
                crate::state::Language::Zh => "lang:zh",
            };
            btn.on_press(Message::PortInputChanged(msg_str.to_string()))
        }
    };
    
    let lang_row = row![
        make_lang_btn(crate::state::Language::En, "English"),
        make_lang_btn(crate::state::Language::Zh, "简体中文")
    ]
    .spacing(15);
    
    let lang_card = container(
        column![
            lang_label,
            lang_row
        ]
        .spacing(10)
    )
    .padding(20)
    .style(theme::card_bg);
    
    // Save button
    let save_btn = button(text(tr(lang, "btn_save_apply")).size(14))
        .padding([10, 20])
        .style(theme::button_primary)
        .on_press(Message::SaveSettings);
        
    container(
        column![
            text(tr(lang, "tab_settings")).size(24).color(theme::TEXT_PRIMARY),
            core_card,
            routing_card,
            settings_card,
            dns_card,
            lang_card,
            save_btn
        ]
        .spacing(20)
        .max_width(800)
    )
    .padding(20)
    .into()
}
