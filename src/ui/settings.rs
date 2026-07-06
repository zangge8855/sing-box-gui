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
    
    let make_toggle_row = move |label_key: &'static str, is_on: bool, msg: &'static str| {
        let label_el: Element<'a, Message> = text(tr(lang, label_key)).color(theme::text_primary(theme)).size(13).width(Length::Fill).into();
        let btn_el: Element<'a, Message> = button(
            text(if is_on { "ON" } else { "OFF" })
                .size(12)
                .width(Length::Fixed(50.0))
                .align_x(Alignment::Center)
        )
        .padding([6, 12])
        .style(if is_on { theme::button_primary } else { theme::button_secondary })
        .on_press(Message::PortInputChanged(msg.to_string()))
        .into();
        
        let r: Element<'a, Message> = row![label_el, btn_el]
            .align_y(Alignment::Center)
            .spacing(20)
            .width(Length::Fill)
            .into();
        r
    };
    
    let text_primary = theme::text_primary(theme);
    let text_muted = theme::text_muted(theme);
    
    // Core Status / Downloader Card
    let core_downloader = if core_installed {
        row![
            text(tr(lang, "core_installed_status")).color(theme::SUCCESS).size(14).width(Length::Fill),
            text(tr(lang, "core_ver_stable")).color(text_muted).size(12)
        ]
        .width(Length::Fill)
        .spacing(5)
        .align_y(Alignment::Center)
    } else {
        let btn = button(
            text(tr(lang, "btn_download_core"))
                .size(13)
                .width(Length::Fill)
                .align_x(Alignment::Center)
        )
        .padding([8, 16])
        .width(Length::Fixed(150.0))
        .style(theme::button_primary)
        .on_press(Message::NewLogLine("TRIGGER_CORE_DOWNLOAD".to_string()));
            
        row![
            text(tr(lang, "core_not_found")).color(theme::DANGER).size(14).width(Length::Fill),
            btn
        ]
        .width(Length::Fill)
        .spacing(20)
        .align_y(Alignment::Center)
    };
    
    let install_status_row = if let Some(msg) = install_message {
        row![text(msg).color(theme::WARNING).size(13)]
    } else {
        row![]
    };
    
    let open_dir_btn = button(
        text(tr(lang, "btn_open_data_dir"))
            .size(13)
            .width(Length::Fill)
            .align_x(Alignment::Center)
    )
    .padding([8, 16])
    .width(Length::Fill)
    .style(theme::button_secondary)
    .on_press(Message::PortInputChanged("open_data_dir".to_string()));

    let core_card = container(
        column![
            text(tr(lang, "core_components")).color(text_muted).size(13),
            core_downloader,
            open_dir_btn,
            install_status_row
        ]
        .spacing(20)
    )
    .padding(25)
    .width(Length::Fill)
    .style(theme::card_bg);
    
    // Routing Mode Selector
    let routing_label = text(tr(lang, "routing_rule_mode")).color(text_muted).size(13);
    
    let make_mode_btn = |mode: RoutingMode, key: &'static str| {
        let active = gui_config.routing_mode == mode;
        let btn = button(
            text(tr(lang, key))
                .size(13)
                .width(Length::Fill)
                .align_x(Alignment::Center)
        )
        .padding([8, 16])
        .width(Length::Fill)
        .style(move |theme, status| {
            if active {
                theme::button_primary(theme, status)
            } else {
                theme::button_secondary(theme, status)
            }
        });
            
        let btn_el: Element<'a, Message> = if active {
            btn.into()
        } else {
            btn.on_press(Message::RoutingModeChanged(mode)).into()
        };
        btn_el
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
        .spacing(15)
    )
    .padding(25)
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
    
    let settings_card = container(
        column![
            text(tr(lang, "ports_config")).color(text_muted).size(13),
            ports_row,
            make_toggle_row("tun_mode_label", gui_config.tun_mode, "toggle_tun"),
            make_toggle_row("autostart_label", gui_config.start_on_boot, "toggle_autostart"),
            make_toggle_row("close_core_on_exit_label", gui_config.close_core_on_exit, "toggle_close_core"),
            make_toggle_row("tcp_fast_open_label", gui_config.tcp_fast_open, "toggle_tfo"),
            make_toggle_row("tcp_multipath_label", gui_config.tcp_multipath, "toggle_mptcp"),
        ]
        .spacing(20)
    )
    .padding(25)
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
    
    let dns_card = container(
        column![
            text(tr(lang, "dns_servers")).color(text_muted).size(13),
            dns_row,
            make_toggle_row("fake_ip_label", gui_config.fake_ip, "toggle_fakeip"),
        ]
        .spacing(20)
    )
    .padding(25)
    .width(Length::Fill)
    .style(theme::card_bg);
    
    // Language card
    let lang_label = text(tr(lang, "app_language")).color(text_muted).size(13);
    
    let make_lang_btn = |target_lang: crate::state::Language, label: &'static str| {
        let active = gui_config.language == target_lang;
        let btn = button(
            text(label)
                .size(13)
                .width(Length::Fill)
                .align_x(Alignment::Center)
        )
        .padding([8, 16])
        .width(Length::Fill)
        .style(move |theme, status| {
            if active {
                theme::button_primary(theme, status)
            } else {
                theme::button_secondary(theme, status)
            }
        });
            
        let btn_el: Element<'a, Message> = if active {
            btn.into()
        } else {
            let msg_str = match target_lang {
                crate::state::Language::En => "lang:en",
                crate::state::Language::Zh => "lang:zh",
            };
            btn.on_press(Message::PortInputChanged(msg_str.to_string())).into()
        };
        btn_el
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
        .spacing(15)
    )
    .padding(25)
    .width(Length::Fill)
    .style(theme::card_bg);
    
    // Theme card
    let theme_label = text(tr(lang, "app_theme")).color(text_muted).size(13);
    
    let make_theme_btn = |target_theme: crate::state::AppTheme, label_key: &'static str| {
        let active = gui_config.theme == target_theme;
        let btn = button(
            text(tr(lang, label_key))
                .size(13)
                .width(Length::Fill)
                .align_x(Alignment::Center)
        )
        .padding([8, 16])
        .width(Length::Fill)
        .style(move |theme, status| {
            if active {
                theme::button_primary(theme, status)
            } else {
                theme::button_secondary(theme, status)
            }
        });
            
        let btn_el: Element<'a, Message> = if active {
            btn.into()
        } else {
            let msg_str = match target_theme {
                crate::state::AppTheme::Auto => "theme:auto",
                crate::state::AppTheme::Dark => "theme:dark",
                crate::state::AppTheme::Light => "theme:light",
            };
            btn.on_press(Message::PortInputChanged(msg_str.to_string())).into()
        };
        btn_el
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
        .spacing(15)
    )
    .padding(25)
    .width(Length::Fill)
    .style(theme::card_bg);
    
    // Save button
    let save_btn = button(text(tr(lang, "btn_save_apply")).size(14))
        .padding([10, 20])
        .style(theme::button_primary)
        .on_press(Message::SaveSettings);

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
        .spacing(20)
    )
    .padding([20, 30])
    .width(Length::Fill)
    .max_width(800.0)
    .center_x(Length::Fill);
    
    let main_content = container(
        column![
            routing_card,
            settings_card,
            dns_card,
            core_card,
            lang_card,
            theme_card,
            preview_card
        ]
        .spacing(20)
    )
    .width(Length::Fill)
    .max_width(600.0)
    .center_x(Length::Fill);
    
    // Header row with Title and Save button
    let header_row = container(
        row![
            text(tr(lang, "settings")).size(24).color(text_primary),
            iced::widget::Space::new().width(Length::Fill),
            save_btn
        ]
        .align_y(Alignment::Center)
        .width(Length::Fill)
    )
    .max_width(600.0)
    .center_x(Length::Fill)
    .padding(iced::Padding { top: 0.0, right: 0.0, bottom: 10.0, left: 0.0 });

    let content_col = column![
        header_row,
        main_content
    ]
    .spacing(20)
    .width(Length::Fill)
    .align_x(Alignment::Center);

    let scroll_content = scrollable(
        container(content_col)
            .padding([30, 20])
            .width(Length::Fill)
            .center_x(Length::Fill)
    )
    .height(Length::Fill);

    container(scroll_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
