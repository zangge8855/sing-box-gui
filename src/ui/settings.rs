use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::state::{GuiConfig, RoutingMode};
use crate::ui::theme;

pub fn render<'a>(
    gui_config: &'a GuiConfig,
    mixed_port_str: &'a str,
    api_port_str: &'a str,
    dns_local_str: &'a str,
    dns_remote_str: &'a str,
    core_installed: bool,
    install_message: Option<&'a str>,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    
    let lang = gui_config.language;
    use crate::ui::i18n::tr;
    
    let text_primary = theme::text_primary(theme);
    let text_muted = theme::text_muted(theme);
    
    // Core Status / Downloader Card
    let core_downloader = if core_installed {
        row![
            text(tr(lang, "core_installed_status")).color(theme::SUCCESS).size(14),
            text(tr(lang, "core_ver_stable")).color(text_muted).size(12)
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
            text(tr(lang, "core_components")).color(text_muted).size(13),
            core_downloader,
            open_dir_btn,
            install_status_row
        ]
        .spacing(15)
    )
    .padding(20)
    .width(Length::Fill)
    .style(theme::card_bg);
    
    // Routing Mode Selector
    let routing_label = text(tr(lang, "routing_rule_mode")).color(text_muted).size(13);
    
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
        make_mode_btn(RoutingMode::Rule, "mode_rules"),
        make_mode_btn(RoutingMode::Global, "mode_global"),
        make_mode_btn(RoutingMode::Direct, "mode_direct")
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
    .width(Length::Fill)
    .style(theme::card_bg);
    
    // Inbound & Port Settings Card
    let mixed_port_input = text_input("2080", mixed_port_str)
        .on_input(|s| Message::PortInputChanged(format!("mixed:{}", s)))
        .on_submit(Message::SaveSettings)
        .padding(10)
        .style(theme::input_field)
        .width(100);
        
    let api_port_input = text_input("9090", api_port_str)
        .on_input(|s| Message::PortInputChanged(format!("api:{}", s)))
        .on_submit(Message::SaveSettings)
        .padding(10)
        .style(theme::input_field)
        .width(100);
        
    let ports_row = row![
        column![
            text(tr(lang, "mixed_port_label")).color(text_muted).size(12),
            mixed_port_input
        ].spacing(5),
        column![
            text(tr(lang, "api_port_label")).color(text_muted).size(12),
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
            text(tr(lang, "ports_config")).color(text_muted).size(13),
            ports_row,
            toggles_row,
            performance_row
        ]
        .spacing(15)
    )
    .padding(20)
    .width(Length::Fill)
    .style(theme::card_bg);
    
    // DNS settings card
    let dns_local_input = text_input("223.5.5.5", dns_local_str)
        .on_input(|s| Message::PortInputChanged(format!("dns_local:{}", s)))
        .on_submit(Message::SaveSettings)
        .padding(10)
        .style(theme::input_field)
        .width(220);
        
    let dns_remote_input = text_input("8.8.8.8", dns_remote_str)
        .on_input(|s| Message::PortInputChanged(format!("dns_remote:{}", s)))
        .on_submit(Message::SaveSettings)
        .padding(10)
        .style(theme::input_field)
        .width(220);
        
    let dns_row = row![
        column![
            text(tr(lang, "dns_local")).color(text_muted).size(12),
            dns_local_input
        ].spacing(5),
        column![
            text(tr(lang, "dns_remote")).color(text_muted).size(12),
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
            text(tr(lang, "dns_servers")).color(text_muted).size(13),
            dns_row,
            fakeip_btn
        ]
        .spacing(15)
    )
    .padding(20)
    .width(Length::Fill)
    .style(theme::card_bg);
    
    // Language card
    let lang_label = text(tr(lang, "app_language")).color(text_muted).size(13);
    
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
    .width(Length::Fill)
    .style(theme::card_bg);
    
    // Theme card
    let theme_label = text(tr(lang, "app_theme")).color(text_muted).size(13);
    
    let make_theme_btn = |target_theme: crate::state::AppTheme, label_key: &'static str| {
        let active = gui_config.theme == target_theme;
        let btn = button(text(tr(lang, label_key)).size(13))
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
            let msg_str = match target_theme {
                crate::state::AppTheme::Auto => "theme:auto",
                crate::state::AppTheme::Dark => "theme:dark",
                crate::state::AppTheme::Light => "theme:light",
            };
            btn.on_press(Message::PortInputChanged(msg_str.to_string()))
        }
    };
    
    let theme_row = row![
        make_theme_btn(crate::state::AppTheme::Auto, "theme_auto"),
        make_theme_btn(crate::state::AppTheme::Dark, "theme_dark"),
        make_theme_btn(crate::state::AppTheme::Light, "theme_light")
    ]
    .spacing(15);
    
    let theme_card = container(
        column![
            theme_label,
            theme_row
        ]
        .spacing(10)
    )
    .padding(20)
    .width(Length::Fill)
    .style(theme::card_bg);
    
    // Save button
    let save_btn = button(text(tr(lang, "btn_save_apply")).size(14))
        .padding([10, 20])
        .style(theme::button_primary)
        .on_press(Message::SaveSettings);
        
    let left_column = column![
        routing_card,
        settings_card,
        dns_card,
    ]
    .spacing(20)
    .width(Length::FillPortion(1));

    let right_column = column![
        core_card,
        lang_card,
        theme_card,
    ]
    .spacing(20)
    .width(Length::FillPortion(1));

    let columns_row = row![
        left_column,
        right_column
    ]
    .spacing(20)
    .width(Length::Fill);

    // Generate config preview string
    let preview_json = crate::config::generate_preview_config(gui_config);
    
    let preview_card = container(
        column![
            text(if lang == crate::state::Language::Zh { "内核配置预览 (只读)" } else { "Configuration Preview (Read-only)" })
                .color(text_muted)
                .size(13),
            container(
                scrollable(
                    text(preview_json)
                        .font(iced::Font::MONOSPACE)
                        .size(12)
                        .color(text_primary)
                )
                .height(Length::Fixed(300.0))
                .width(Length::Fill)
            )
            .padding(15)
            .style(move |t| {
                let base = theme::status_card(t);
                container::Style {
                    background: Some(iced::Background::Color(if theme::is_dark(t) {
                        iced::Color::from_rgb(0.08, 0.08, 0.1)
                    } else {
                        iced::Color::from_rgb(0.95, 0.95, 0.97)
                    })),
                    ..base
                }
            })
        ]
        .spacing(10)
    )
    .padding(20)
    .width(Length::Fill)
    .style(theme::card_bg);

    let scroll_content = scrollable(
        column![
            columns_row,
            preview_card
        ]
        .spacing(20)
        .width(Length::Fill)
    )
    .height(Length::Fill)
    .width(Length::Fill);
        
    container(
        column![
            row![
                text(tr(lang, "tab_settings")).size(24).color(text_primary).width(Length::Fill),
                save_btn
            ]
            .align_y(Alignment::Center)
            .spacing(20),
            scroll_content
        ]
        .spacing(20)
        .max_width(1000)
        .height(Length::Fill)
    )
    .padding(20)
    .into()
}
