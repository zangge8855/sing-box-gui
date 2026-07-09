use iced::widget::{button, column, container, row, scrollable, text, text_input, responsive, pick_list, toggler};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::state::{GuiConfig, Language, AppTheme, UpdateStatus};
use crate::ui::theme;
use crate::ui::page_header;

pub fn render<'a>(
    gui_config: &'a GuiConfig,
    mixed_port_str: &'a str,
    api_port_str: &'a str,
    dns_local_str: &'a str,
    dns_remote_str: &'a str,
    core_path_str: &'a str,
    core_installed: bool,
    install_message: Option<&'a str>,
    core_version: Option<&'a str>,
    update_status: &'a UpdateStatus,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    
    let lang = gui_config.language;
    use crate::ui::i18n::tr;
    
    let theme_cloned = theme.clone();
    
    let main_content = responsive(move |size| {
        let is_compact = size.width < 800.0;
        let theme = &theme_cloned;
        
        let make_toggle_row = move |label_key: &'static str, is_on: bool, msg: Message| {
            let label_el: Element<'_, Message> = text(tr(lang, label_key))
                .color(theme::text_primary(theme))
                .size(13)
                .width(Length::Fill)
                .into();
                
            let switch_el: Element<'_, Message> = toggler(is_on)
                .on_toggle(move |_| msg.clone())
                .size(20)
                .into();
                
            let r: Element<'_, Message> = row![label_el, switch_el]
                .align_y(Alignment::Center)
                .spacing(20)
                .width(Length::Fill)
                .into();
            r
        };
        
        let text_primary = theme::text_primary(theme);
        let text_muted = theme::text_muted(theme);
        
        // 1. System Integration Card (Left Column)
        let settings_card = container(
            column![
                text(tr(lang, "sys_integration")).color(text_muted).size(13),
                column![make_toggle_row("tun_mode_label", gui_config.tun_mode, Message::ToggleTun), text(tr(lang, "help_tun_mode")).color(text_muted).size(11)].spacing(4),
                column![make_toggle_row("autostart_label", gui_config.start_on_boot, Message::ToggleAutostart), text(tr(lang, "help_autostart")).color(text_muted).size(11)].spacing(4),
                column![make_toggle_row("close_core_on_exit_label", gui_config.close_core_on_exit, Message::ToggleCloseCoreOnExit), text(tr(lang, "help_close_core")).color(text_muted).size(11)].spacing(4),
                column![make_toggle_row("auto_start_core_label", gui_config.auto_start_core, Message::ToggleAutoStartCore), text(tr(lang, "help_auto_start_core")).color(text_muted).size(11)].spacing(4),
                column![make_toggle_row("auto_sys_proxy_label", gui_config.auto_sys_proxy, Message::ToggleAutoSysProxy), text(tr(lang, "help_auto_sys_proxy")).color(text_muted).size(11)].spacing(4),
                column![make_toggle_row("tcp_fast_open_label", gui_config.tcp_fast_open, Message::ToggleTcpFastOpen), text(tr(lang, "help_tcp_fast_open")).color(text_muted).size(11)].spacing(4),
                column![make_toggle_row("tcp_multipath_label", gui_config.tcp_multipath, Message::ToggleTcpMultipath), text(tr(lang, "help_tcp_multipath")).color(text_muted).size(11)].spacing(4),
                column![make_toggle_row("disable_proxy_on_stop_label", gui_config.disable_proxy_on_core_stop, Message::ToggleDisableProxyOnCoreStop), text(tr(lang, "help_disable_proxy_on_stop")).color(text_muted).size(11)].spacing(4),
            ]
            .spacing(16)
        )
        .padding(25)
        .width(Length::Fill)
        .style(theme::card_bg);
        
        // 2. Network & DNS Settings Card (Middle Column)
        let mixed_port_input = text_input("2080", mixed_port_str)
            .on_input(Message::MixedPortChanged)
            .on_submit(Message::SaveSettings)
            .padding(10)
            .style(theme::input_field);
            
        let api_port_input = text_input("9090", api_port_str)
            .on_input(Message::ApiPortChanged)
            .on_submit(Message::SaveSettings)
            .padding(10)
            .style(theme::input_field);
            
        let ports_row = row![
            column![
                text(tr(lang, "mixed_port_label")).color(text_muted).size(12),
                mixed_port_input.width(Length::Fill)
            ]
            .spacing(5)
            .width(Length::FillPortion(1)),
            column![
                text(tr(lang, "api_port_label")).color(text_muted).size(12),
                api_port_input.width(Length::Fill)
            ]
            .spacing(5)
            .width(Length::FillPortion(1)),
        ]
        .spacing(16)
        .width(Length::Fill);
        
        let dns_local_input = text_input("223.5.5.5", dns_local_str)
            .on_input(Message::DnsLocalChanged)
            .on_submit(Message::SaveSettings)
            .padding(10)
            .style(theme::input_field);
            
        let dns_remote_input = text_input("8.8.8.8", dns_remote_str)
            .on_input(Message::DnsRemoteChanged)
            .on_submit(Message::SaveSettings)
            .padding(10)
            .style(theme::input_field);
            
        let dns_row = row![
            column![
                text(tr(lang, "dns_local")).color(text_muted).size(12),
                dns_local_input.width(Length::Fill)
            ]
            .spacing(5)
            .width(Length::FillPortion(1)),
            column![
                text(tr(lang, "dns_remote")).color(text_muted).size(12),
                dns_remote_input.width(Length::Fill)
            ]
            .spacing(5)
            .width(Length::FillPortion(1)),
        ]
        .spacing(16)
        .width(Length::Fill);
        
        let network_dns_card = container(
            column![
                text(tr(lang, "ports_config")).color(text_muted).size(13),
                ports_row,
                text(tr(lang, "dns_servers")).color(text_muted).size(13),
                dns_row,
                column![
                    make_toggle_row("fake_ip_label", gui_config.fake_ip, Message::ToggleFakeIp),
                    text(tr(lang, "help_fake_ip")).color(text_muted).size(11)
                ]
                .spacing(4),
            ]
            .spacing(20)
        )
        .padding(25)
        .width(Length::Fill)
        .style(theme::card_bg);
        
        // 3. App & Core Preferences Card (Right Column)
        let lang_label = text(tr(lang, "app_language")).color(text_muted).size(13);
        
        let lang_options = vec![
            LanguageOption { lang: Language::Zh, label: "简体中文" },
            LanguageOption { lang: Language::En, label: "English" },
        ];
        
        let selected_lang_opt = lang_options.iter()
            .find(|o| o.lang == gui_config.language)
            .cloned();
            
        let lang_selector = pick_list(
            lang_options,
            selected_lang_opt,
            move |opt| Message::SetLanguage(opt.lang)
        )
        .width(Length::Fill)
        .padding(8)
        .style(theme::pick_list);
        
        let theme_label = text(tr(lang, "app_theme")).color(text_muted).size(13);
        
        let theme_options = vec![
            ThemeOption { theme: AppTheme::Auto, label: tr(lang, "theme_auto") },
            ThemeOption { theme: AppTheme::Dark, label: tr(lang, "theme_dark") },
            ThemeOption { theme: AppTheme::Light, label: tr(lang, "theme_light") },
        ];
        
        let selected_theme_opt = theme_options.iter()
            .find(|o| o.theme == gui_config.theme)
            .cloned();
            
        let theme_selector = pick_list(
            theme_options,
            selected_theme_opt,
            move |opt| Message::SetTheme(opt.theme)
        )
        .width(Length::Fill)
        .padding(8)
        .style(theme::pick_list);
        
        let app_prefs_content: Element<'_, Message> = if is_compact {
            column![
                column![lang_label, lang_selector].spacing(8),
                column![theme_label, theme_selector].spacing(8)
            ]
            .spacing(15)
            .into()
        } else {
            row![
                column![lang_label, lang_selector].spacing(8).width(Length::FillPortion(1)),
                column![theme_label, theme_selector].spacing(8).width(Length::FillPortion(1))
            ]
            .spacing(20)
            .width(Length::Fill)
            .into()
        };
        
        let app_prefs_card = container(app_prefs_content)
            .padding(25)
            .width(Length::Fill)
            .style(theme::card_bg);
        
        // Core Management Card
        let interval_options = vec![
            IntervalOption { hours: 0, label: tr(lang, "auto_update_off") },
            IntervalOption { hours: 6, label: tr(lang, "auto_update_6h") },
            IntervalOption { hours: 12, label: tr(lang, "auto_update_12h") },
            IntervalOption { hours: 24, label: tr(lang, "auto_update_24h") },
        ];
        let selected_interval = interval_options
            .iter()
            .find(|o| o.hours == gui_config.auto_update_interval_hours)
            .cloned()
            .or_else(|| interval_options.first().cloned());
        let interval_picker = pick_list(
            interval_options,
            selected_interval,
            |opt| Message::AutoUpdateIntervalChanged(opt.hours)
        )
        .width(Length::Fill)
        .padding(8)
        .style(theme::pick_list);

        let auto_update_block = column![
            text(tr(lang, "auto_update_interval")).color(text_muted).size(12),
            text(tr(lang, "help_auto_update")).color(text_muted).size(11),
            interval_picker,
        ]
        .spacing(6)
        .width(Length::Fill);

        let version_label = if let Some(v) = core_version {
            format!("{}: {}", tr(lang, "core_version_label"), v)
        } else if core_installed {
            tr(lang, "core_ver_stable").to_string()
        } else {
            String::new()
        };

        let core_downloader = if core_installed {
            row![
                text(tr(lang, "core_installed_status")).color(theme::SUCCESS).size(13).width(Length::Fill),
                text(version_label).color(text_muted).size(11)
            ]
            .width(Length::Fill)
            .spacing(5)
            .align_y(Alignment::Center)
        } else {
            let btn = button(
                text(tr(lang, "btn_download_core"))
                    .size(12)
                    .width(Length::Fill)
                    .align_x(Alignment::Center)
            )
            .padding([6, 12])
            .width(Length::Fill)
            .style(theme::button_primary)
            .on_press(Message::TriggerCoreDownload);
                
            row![
                text(tr(lang, "core_not_found")).color(theme::DANGER).size(13).width(Length::Fill),
                btn
            ]
            .width(Length::Fill)
            .spacing(10)
            .align_y(Alignment::Center)
        };
        
        let install_status_row = if let Some(msg) = install_message {
            row![text(msg).color(theme::WARNING).size(11)]
        } else {
            row![]
        };
        
        let open_dir_btn = button(
            text(tr(lang, "btn_open_data_dir"))
                .size(12)
                .width(Length::Fill)
                .align_x(Alignment::Center)
        )
        .padding([8, 12])
        .width(Length::Fill)
        .style(theme::button_secondary)
        .on_press(Message::OpenDataDir);

        let core_path_input = text_input(tr(lang, "placeholder_core_path"), core_path_str)
            .on_input(Message::CorePathChanged)
            .on_submit(Message::SaveSettings)
            .padding(10)
            .style(theme::input_field);

        let clear_core_path_btn = button(text(tr(lang, "btn_clear_core_path")).size(12))
            .padding([8, 12])
            .style(theme::button_secondary)
            .on_press(Message::ClearCorePath);

        let core_path_row = column![
            text(tr(lang, "core_path_label")).color(text_muted).size(12),
            text(tr(lang, "help_core_path")).color(text_muted).size(11),
            core_path_input.width(Length::Fill),
            clear_core_path_btn,
        ]
        .spacing(6)
        .width(Length::Fill);

        let core_mgmt_card = container(
            column![
                text(tr(lang, "core_components")).color(text_muted).size(13),
                core_downloader,
                core_path_row,
                auto_update_block,
                open_dir_btn,
                install_status_row
            ]
            .spacing(15)
        )
        .padding(25)
        .width(Length::Fill)
        .style(theme::card_bg);
        
        let app_update_card = {
            let update_info: Element<'_, Message> = match update_status {
                UpdateStatus::NotChecked => {
                    let btn: Element<'_, Message> = button(text(tr(lang, "btn_check_update")).size(12))
                        .padding([6, 12])
                        .style(theme::button_primary)
                        .on_press(Message::CheckUpdate)
                        .into();
                    row![
                        text(format!("{}: v{}", tr(lang, "current_ver_label"), env!("CARGO_PKG_VERSION")))
                            .color(text_muted)
                            .size(13)
                            .width(Length::Fill),
                        btn
                    ]
                    .width(Length::Fill)
                    .align_y(Alignment::Center)
                    .into()
                }
                UpdateStatus::Checking => {
                    row![
                        text(tr(lang, "status_checking"))
                            .color(text_muted)
                            .size(13)
                            .width(Length::Fill)
                    ]
                    .width(Length::Fill)
                    .align_y(Alignment::Center)
                    .into()
                }
                UpdateStatus::UpToDate => {
                    let btn: Element<'_, Message> = button(text(tr(lang, "btn_check_update")).size(12))
                        .padding([6, 12])
                        .style(theme::button_secondary)
                        .on_press(Message::CheckUpdate)
                        .into();
                    row![
                        text(tr(lang, "status_uptodate"))
                            .color(theme::SUCCESS)
                            .size(13)
                            .width(Length::Fill),
                        btn
                    ]
                    .width(Length::Fill)
                    .align_y(Alignment::Center)
                    .into()
                }
                UpdateStatus::NewVersion(tag_name) => {
                    let btn: Element<'_, Message> = button(text(tr(lang, "btn_goto_github")).size(12))
                        .padding([6, 12])
                        .style(theme::button_primary)
                        .on_press(Message::OpenUrl("https://github.com/zangge8855/sing-box-gui/releases/latest".to_string()))
                        .into();
                    row![
                        text(format!("{} {}", tr(lang, "status_new_available"), tag_name))
                            .color(theme::WARNING)
                            .size(13)
                            .width(Length::Fill),
                        btn
                    ]
                    .width(Length::Fill)
                    .align_y(Alignment::Center)
                    .into()
                }
                UpdateStatus::Error(err) => {
                    let btn: Element<'_, Message> = button(text(tr(lang, "btn_check_update")).size(12))
                        .padding([6, 12])
                        .style(theme::button_secondary)
                        .on_press(Message::CheckUpdate)
                        .into();
                    column![
                        row![
                            text(tr(lang, "status_check_failed"))
                                .color(theme::DANGER)
                                .size(13)
                                .width(Length::Fill),
                            btn
                        ]
                        .width(Length::Fill)
                        .align_y(Alignment::Center),
                        text(err.clone())
                            .color(theme::DANGER)
                            .size(11)
                    ]
                    .spacing(8)
                    .width(Length::Fill)
                    .into()
                }
            };
            
            container(
                column![
                    text(tr(lang, "app_update")).color(text_muted).size(13),
                    update_info
                ]
                .spacing(15)
            )
            .padding(25)
            .width(Length::Fill)
            .style(theme::card_bg)
        };
        
        // Layout columns responsively
        let left_col = column![
            settings_card,
        ]
        .spacing(20)
        .width(if is_compact { Length::Fill } else { Length::FillPortion(1) });

        let mid_col = column![
            network_dns_card,
        ]
        .spacing(20)
        .width(if is_compact { Length::Fill } else { Length::FillPortion(1) });

        let app_and_core_col = column![
            app_prefs_card,
            core_mgmt_card,
            app_update_card
        ]
        .spacing(20)
        .width(if is_compact { Length::Fill } else { Length::FillPortion(1) });

        let main_row_layout: Element<'_, Message> = if size.width < 800.0 {
            column![
                left_col,
                mid_col,
                app_and_core_col
            ]
            .spacing(20)
            .width(Length::Fill)
            .into()
        } else if size.width < 1150.0 {
            column![
                row![left_col, mid_col].spacing(20).width(Length::Fill),
                app_and_core_col
            ]
            .spacing(20)
            .width(Length::Fill)
            .into()
        } else {
            row![
                left_col,
                mid_col,
                app_and_core_col
            ]
            .spacing(20)
            .width(Length::Fill)
            .into()
        };

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
                .style(theme::console_bg)
            ]
            .spacing(20)
        )
        .padding([20, 30])
        .width(Length::Fill);

        let content_col = column![
            main_row_layout,
            preview_card
        ]
        .spacing(20)
        .width(Length::Fill);
        
        let save_btn = button(text(tr(lang, "btn_save_apply")).size(14))
            .padding([10, 20])
            .style(theme::button_primary)
            .on_press(Message::SaveSettings);

        let header = page_header("settings", lang, Some(save_btn.into()), theme, is_compact);
        
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageOption {
    pub lang: Language,
    pub label: &'static str,
}

impl std::fmt::Display for LanguageOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeOption {
    pub theme: AppTheme,
    pub label: &'static str,
}

impl std::fmt::Display for ThemeOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntervalOption {
    pub hours: u32,
    pub label: &'static str,
}

impl std::fmt::Display for IntervalOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}
