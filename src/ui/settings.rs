use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::state::{GuiConfig, RoutingMode, Language, AppTheme};
use crate::ui::theme;
use crate::ui::{page_header, page_shell};

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
    
    let make_toggle_row = move |label_key: &'static str, is_on: bool, msg: Message| {
        let label_el: Element<'a, Message> = text(tr(lang, label_key)).color(theme::text_primary(theme)).size(13).width(Length::Fill).into();
        let btn_el: Element<'a, Message> = button(
            text(tr(lang, if is_on { "ON" } else { "OFF" }))
                .size(12)
                .width(Length::Fixed(50.0))
                .align_x(Alignment::Center)
        )
        .padding([6, 12])
        .style(if is_on { theme::button_primary } else { theme::button_secondary })
        .on_press(msg)
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
        .on_press(Message::TriggerCoreDownload);
            
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
    .on_press(Message::OpenDataDir);

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
        .on_input(Message::MixedPortChanged)
        .on_submit(Message::SaveSettings)
        .padding(10)
        .style(theme::input_field)
        .width(100);
        
    let api_port_input = text_input("9090", api_port_str)
        .on_input(Message::ApiPortChanged)
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
            make_toggle_row("tun_mode_label", gui_config.tun_mode, Message::ToggleTun),
            make_toggle_row("autostart_label", gui_config.start_on_boot, Message::ToggleAutostart),
            make_toggle_row("close_core_on_exit_label", gui_config.close_core_on_exit, Message::ToggleCloseCoreOnExit),
            make_toggle_row("tcp_fast_open_label", gui_config.tcp_fast_open, Message::ToggleTcpFastOpen),
            make_toggle_row("tcp_multipath_label", gui_config.tcp_multipath, Message::ToggleTcpMultipath),
        ]
        .spacing(20)
    )
    .padding(25)
    .width(Length::Fill)
    .style(theme::card_bg);
    
    // DNS settings card
    let dns_local_input = text_input("223.5.5.5", dns_local_str)
        .on_input(Message::DnsLocalChanged)
        .on_submit(Message::SaveSettings)
        .padding(10)
        .style(theme::input_field)
        .width(220);
        
    let dns_remote_input = text_input("8.8.8.8", dns_remote_str)
        .on_input(Message::DnsRemoteChanged)
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
            make_toggle_row("fake_ip_label", gui_config.fake_ip, Message::ToggleFakeIp),
        ]
        .spacing(20)
    )
    .padding(25)
    .width(Length::Fill)
    .style(theme::card_bg);
    
    // Language card
    let lang_label = text(tr(lang, "app_language")).color(text_muted).size(13);
    
    let make_lang_btn = |target_lang: Language, label: &'static str| {
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
            btn.on_press(Message::SetLanguage(target_lang)).into()
        };
        btn_el
    };
    
    let lang_row = row![
        make_lang_btn(Language::En, "English"),
        make_lang_btn(Language::Zh, "简体中文")
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
    
    let make_theme_btn = |target_theme: AppTheme, label_key: &'static str| {
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
            btn.on_press(Message::SetTheme(target_theme)).into()
        };
        btn_el
    };
    
    let theme_row = row![
        make_theme_btn(AppTheme::Auto, "theme_auto"),
        make_theme_btn(AppTheme::Dark, "theme_dark"),
        make_theme_btn(AppTheme::Light, "theme_light")
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
            text(tr(lang, "core_config_preview"))
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
    .width(Length::Fill);
    
    // Modern 2-Column Settings layout
    let left_col = column![
        routing_card,
        settings_card,
    ]
    .spacing(20)
    .width(Length::FillPortion(1));

    let right_col = column![
        dns_card,
        core_card,
        lang_card,
        theme_card,
    ]
    .spacing(20)
    .width(Length::FillPortion(1));

    let two_cols = row![
        left_col,
        right_col
    ]
    .spacing(20)
    .width(Length::Fill);

    let main_content = column![
        two_cols,
        preview_card
    ]
    .spacing(20)
    .width(Length::Fill);

    let header = page_header("settings", lang, Some(save_btn.into()), theme);
    page_shell(header, main_content.into())
}
