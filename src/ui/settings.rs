use crate::message::Message;
use crate::state::{
    AppTheme, CoreInstallState, GuiConfig, Language, RuntimeSettingsDraft, UpdateStatus,
};
use crate::ui::i18n::tr;
use crate::ui::page_header;
use crate::ui::theme;
use iced::widget::{
    Column, Space, button, column, container, pick_list, responsive, row, scrollable, text,
    text_input, toggler,
};
use iced::{Alignment, Element, Length};

/// Inline pickers and text fields share one width so their edges line up
/// down a card; ports and timeouts take the narrow one.
const CONTROL_W: f32 = 200.0;
const NARROW_CONTROL_W: f32 = 110.0;

fn field<'a>(
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
    invalid: bool,
) -> text_input::TextInput<'a, Message> {
    text_input(placeholder, value)
        .on_input(on_input)
        .on_submit(Message::SaveSettings)
        .size(theme::TYPE_BODY)
        .padding([8, 10])
        .style(if invalid {
            theme::input_field_error
        } else {
            theme::input_field
        })
}

#[allow(clippy::too_many_arguments)]
pub fn render<'a>(
    gui_config: &'a GuiConfig,
    draft: &'a RuntimeSettingsDraft,
    errors: &'a std::collections::BTreeMap<&'static str, &'static str>,
    core_installed: bool,
    install_state: &'a CoreInstallState,
    core_version: Option<&'a str>,
    update_status: &'a UpdateStatus,
    config_preview_expanded: bool,
    config_preview: Option<&'a str>,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    let theme_cloned = theme.clone();

    let main_content = responsive(move |size| {
        let theme = &theme_cloned;
        let text_primary = theme::text_primary(theme);
        let text_muted = theme::text_muted(theme);
        let danger = theme::danger_text(theme);

        // ── Building blocks: a settings row is a label (with optional help
        // and error lines) on the left and its control on the right ────────
        let label_block =
            |key: &'static str, help: Option<&'static str>, error: Option<&'static str>| {
                let mut col = column![
                    text(tr(lang, key))
                        .size(theme::TYPE_BODY)
                        .color(text_primary)
                ]
                .spacing(3)
                .width(Length::Fill);
                if let Some(help) = help {
                    col = col.push(
                        text(tr(lang, help))
                            .size(theme::TYPE_CAPTION)
                            .color(text_muted),
                    );
                }
                if let Some(error) = error {
                    col = col.push(
                        text(tr(lang, error))
                            .size(theme::TYPE_CAPTION)
                            .color(danger),
                    );
                }
                col
            };
        let inline = |key: &'static str,
                      help: Option<&'static str>,
                      error: Option<&'static str>,
                      control: Element<'a, Message>|
         -> Element<'a, Message> {
            row![label_block(key, help, error), control]
                .spacing(crate::ui::SP_16)
                .align_y(Alignment::Center)
                .into()
        };
        // Long values (URLs, paths) get the full card width under the label.
        let stacked = |key: &'static str,
                       help: Option<&'static str>,
                       error: Option<&'static str>,
                       control: Element<'a, Message>|
         -> Element<'a, Message> {
            let mut col = label_block(key, help, None).push(control).spacing(6);
            if let Some(error) = error {
                col = col.push(
                    text(tr(lang, error))
                        .size(theme::TYPE_CAPTION)
                        .color(danger),
                );
            }
            col.into()
        };
        let switch = |on: bool, msg: Message| -> Element<'a, Message> {
            toggler(on)
                .on_toggle(move |_| msg.clone())
                .size(20)
                .style(theme::toggler_style)
                .into()
        };
        let small_btn = |key: &'static str, msg: Option<Message>, primary: bool| {
            let mut b = button(text(tr(lang, key)).size(theme::TYPE_BTN_SM))
                .padding(theme::BTN_PAD_SM)
                .style(if primary {
                    theme::button_primary
                } else {
                    theme::button_secondary
                });
            if let Some(msg) = msg {
                b = b.on_press(msg);
            }
            b
        };
        let section = |title_key: &'static str, rows: Vec<Element<'a, Message>>| {
            let mut col = Column::new().width(Length::Fill).push(
                container(
                    text(tr(lang, title_key))
                        .size(theme::TYPE_HEADING)
                        .font(theme::ui_font(iced::font::Weight::Semibold))
                        .color(text_primary),
                )
                .padding(iced::Padding {
                    bottom: 4.0,
                    ..Default::default()
                }),
            );
            for (i, row_el) in rows.into_iter().enumerate() {
                if i > 0 {
                    col = col.push(
                        container(Space::new())
                            .width(Length::Fill)
                            .height(1)
                            .style(theme::hairline),
                    );
                }
                col = col.push(container(row_el).padding([12, 0]).width(Length::Fill));
            }
            container(col)
                .padding([16.0, theme::CARD_PAD])
                .width(Length::Fill)
                .style(theme::card_bg)
        };
        // A macro, not a closure: each picker has its own option type.
        macro_rules! picker_style {
            ($picker:expr) => {
                $picker
                    .width(Length::Fixed(CONTROL_W))
                    .padding([7, 10])
                    .text_size(theme::TYPE_BODY)
                    .style(theme::pick_list)
                    .menu_style(theme::pick_list_menu)
            };
        }

        // ── General: how the app itself behaves ───────────────────────────
        let lang_options = vec![
            LanguageOption {
                lang: Language::Zh,
                label: "简体中文",
            },
            LanguageOption {
                lang: Language::En,
                label: "English",
            },
        ];
        let selected_lang = lang_options
            .iter()
            .find(|o| o.lang == gui_config.language)
            .cloned();
        let lang_picker = picker_style!(pick_list(lang_options, selected_lang, |opt| {
            Message::SetLanguage(opt.lang)
        }));

        let theme_options = vec![
            ThemeOption {
                theme: AppTheme::Auto,
                label: tr(lang, "theme_auto"),
            },
            ThemeOption {
                theme: AppTheme::Dark,
                label: tr(lang, "theme_dark"),
            },
            ThemeOption {
                theme: AppTheme::Light,
                label: tr(lang, "theme_light"),
            },
        ];
        let selected_theme = theme_options
            .iter()
            .find(|o| o.theme == gui_config.theme)
            .cloned();
        let theme_picker = picker_style!(pick_list(theme_options, selected_theme, |opt| {
            Message::SetTheme(opt.theme)
        }));

        let interval_options = vec![
            IntervalOption {
                hours: 0,
                label: tr(lang, "auto_update_off"),
            },
            IntervalOption {
                hours: 6,
                label: tr(lang, "auto_update_6h"),
            },
            IntervalOption {
                hours: 12,
                label: tr(lang, "auto_update_12h"),
            },
            IntervalOption {
                hours: 24,
                label: tr(lang, "auto_update_24h"),
            },
        ];
        let selected_interval = interval_options
            .iter()
            .find(|o| o.hours == draft.auto_update_interval_hours)
            .cloned()
            .or_else(|| interval_options.first().cloned());
        let interval_picker =
            picker_style!(pick_list(interval_options, selected_interval, |opt| {
                Message::AutoUpdateIntervalChanged(opt.hours)
            },));

        let check_btn = || small_btn("btn_check_update", Some(Message::CheckUpdate), false);
        let current_version =
            tr(lang, "app_version_current").replace("{}", env!("CARGO_PKG_VERSION"));
        let caption = |s: String, color| text(s).size(theme::TYPE_CAPTION).color(color);
        let (update_status_el, update_control): (
            Element<'a, Message>,
            Option<Element<'a, Message>>,
        ) = match update_status {
            UpdateStatus::NotChecked => (
                caption(
                    format!("{current_version} · {}", tr(lang, "status_not_checked")),
                    text_muted,
                )
                .into(),
                Some(check_btn().into()),
            ),
            UpdateStatus::Checking => (
                crate::ui::loading_row(tr(lang, "status_checking"), theme),
                None,
            ),
            UpdateStatus::Downloading { tag } | UpdateStatus::Installing { tag } => {
                let key = if matches!(update_status, UpdateStatus::Downloading { .. }) {
                    "status_downloading_update"
                } else {
                    "status_installing_update"
                };
                (
                    row![
                        crate::ui::loading_row(tr(lang, key), theme),
                        caption(tag.clone(), theme::warning_text(theme)),
                    ]
                    .spacing(crate::ui::SP_8)
                    .align_y(Alignment::Center)
                    .into(),
                    None,
                )
            }
            UpdateStatus::UpToDate => (
                caption(
                    format!("{current_version} · {}", tr(lang, "status_uptodate")),
                    theme::success_text(theme),
                )
                .into(),
                Some(check_btn().into()),
            ),
            UpdateStatus::NewVersion {
                tag,
                download_url,
                sha256,
                size,
            } => {
                let control = if let (Some(url), Some(sha256), Some(size)) =
                    (download_url.clone(), sha256.clone(), *size)
                {
                    small_btn(
                        "btn_download_install",
                        Some(Message::DownloadAppUpdate {
                            tag: tag.clone(),
                            url,
                            sha256,
                            size,
                        }),
                        true,
                    )
                } else {
                    small_btn(
                        "btn_view_release_on_github",
                        Some(Message::OpenUrl(format!(
                            "https://github.com/zangge8855/sing-box-gui/releases/tag/{tag}"
                        ))),
                        false,
                    )
                };
                let missing_key = if download_url.is_none() {
                    Some("update_asset_missing")
                } else if sha256.is_none() {
                    Some("update_digest_missing")
                } else if size.is_none() {
                    Some("update_size_missing")
                } else {
                    None
                };
                let mut col = column![caption(
                    format!(
                        "{}: {tag} · {current_version}",
                        tr(lang, "status_new_version")
                    ),
                    theme::warning_text(theme),
                )]
                .spacing(3);
                if let Some(key) = missing_key {
                    col = col.push(caption(tr(lang, key).to_string(), danger));
                }
                (col.into(), Some(control.into()))
            }
            UpdateStatus::Error(err) => (
                column![
                    caption(tr(lang, "status_check_failed").to_string(), danger),
                    caption(err.clone(), text_muted),
                ]
                .spacing(3)
                .into(),
                Some(check_btn().into()),
            ),
        };
        let mut update_row = row![
            column![
                text(tr(lang, "app_update"))
                    .size(theme::TYPE_BODY)
                    .color(text_primary),
                update_status_el,
            ]
            .spacing(3)
            .width(Length::Fill)
        ]
        .spacing(crate::ui::SP_16)
        .align_y(Alignment::Center);
        if let Some(control) = update_control {
            update_row = update_row.push(control);
        }

        let general = section(
            "settings_general",
            vec![
                inline("app_language", None, None, lang_picker.into()),
                inline("app_theme", None, None, theme_picker.into()),
                inline(
                    "autostart_label",
                    Some("help_autostart"),
                    errors.get("autostart").copied(),
                    switch(draft.start_on_boot, Message::ToggleAutostart),
                ),
                inline(
                    "auto_start_core_label",
                    Some("help_auto_start_core"),
                    None,
                    switch(draft.auto_start_core, Message::ToggleAutoStartCore),
                ),
                inline(
                    "close_core_on_exit_label",
                    Some("help_close_core"),
                    None,
                    switch(draft.close_core_on_exit, Message::ToggleCloseCoreOnExit),
                ),
                inline(
                    "auto_update_interval",
                    Some("help_auto_update"),
                    None,
                    interval_picker.into(),
                ),
                update_row.into(),
            ],
        );

        // ── Proxy: how traffic reaches the core ───────────────────────────
        let mixed_port_result = crate::parse_runtime_port(&draft.mixed_port);
        let api_port_result = crate::parse_runtime_port(&draft.api_port);
        let port_conflict = matches!(
            (&mixed_port_result, &api_port_result),
            (Ok(mixed), Ok(api)) if mixed == api
        );
        let mixed_port_error = if port_conflict {
            Some("port_conflict_error")
        } else {
            mixed_port_result.as_ref().err().copied()
        };
        let api_port_error = if port_conflict {
            Some("port_conflict_error")
        } else {
            api_port_result.as_ref().err().copied()
        };

        let mut tun_row = column![inline(
            "tun_mode_label",
            Some("help_tun_mode"),
            None,
            switch(draft.tun_mode, Message::ToggleTun),
        )]
        .spacing(10);
        if draft.tun_mode {
            tun_row = tun_row.push(
                container(
                    text(tr(lang, "tun_admin_banner"))
                        .size(theme::TYPE_CAPTION)
                        .color(theme::warning_text(theme)),
                )
                .padding([8, 10])
                .width(Length::Fill)
                .style(|t| theme::tinted_banner(t, theme::WARNING)),
            );
        }

        let proxy = section(
            "settings_proxy",
            vec![
                tun_row.into(),
                inline(
                    "auto_sys_proxy_label",
                    Some("help_auto_sys_proxy"),
                    None,
                    switch(draft.auto_sys_proxy, Message::ToggleAutoSysProxy),
                ),
                inline(
                    "disable_proxy_on_stop_label",
                    Some("help_disable_proxy_on_stop"),
                    None,
                    switch(
                        draft.disable_proxy_on_core_stop,
                        Message::ToggleDisableProxyOnCoreStop,
                    ),
                ),
                inline(
                    "mixed_port_label",
                    Some("help_mixed_port"),
                    mixed_port_error,
                    field(
                        "2080",
                        &draft.mixed_port,
                        Message::MixedPortChanged,
                        mixed_port_error.is_some(),
                    )
                    .width(Length::Fixed(NARROW_CONTROL_W))
                    .into(),
                ),
                inline(
                    "api_port_label",
                    Some("help_api_port"),
                    api_port_error,
                    field(
                        "9090",
                        &draft.api_port,
                        Message::ApiPortChanged,
                        api_port_error.is_some(),
                    )
                    .width(Length::Fixed(NARROW_CONTROL_W))
                    .into(),
                ),
            ],
        );

        // ── DNS ───────────────────────────────────────────────────────────
        let dns_local_error = (!crate::is_valid_dns_server_address(draft.dns_server_local.trim()))
            .then_some("invalid_dns_server");
        let dns_remote_error =
            (!crate::is_valid_dns_server_address(draft.dns_server_remote.trim()))
                .then_some("invalid_dns_server");
        let dns = section(
            "dns_servers",
            vec![
                inline(
                    "dns_local",
                    Some("help_dns_local"),
                    dns_local_error,
                    field(
                        "223.5.5.5",
                        &draft.dns_server_local,
                        Message::DnsLocalChanged,
                        dns_local_error.is_some(),
                    )
                    .width(Length::Fixed(CONTROL_W))
                    .into(),
                ),
                inline(
                    "dns_remote",
                    Some("help_dns_remote"),
                    dns_remote_error,
                    field(
                        "8.8.8.8",
                        &draft.dns_server_remote,
                        Message::DnsRemoteChanged,
                        dns_remote_error.is_some(),
                    )
                    .width(Length::Fixed(CONTROL_W))
                    .into(),
                ),
                inline(
                    "fake_ip_label",
                    Some("help_fake_ip"),
                    None,
                    switch(draft.fake_ip, Message::ToggleFakeIp),
                ),
            ],
        );

        // ── Advanced: transport tweaks and latency testing ────────────────
        let latency_url_error = (!url::Url::parse(draft.latency_test_url.trim())
            .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some()))
        .then_some("invalid_latency_url");
        let latency_timeout_error = crate::parse_latency_timeout(&draft.latency_test_timeout_ms)
            .err()
            .map(|_| "invalid_latency_timeout");
        let advanced = section(
            "settings_advanced",
            vec![
                inline(
                    "tcp_fast_open_label",
                    Some("help_tcp_fast_open"),
                    None,
                    switch(draft.tcp_fast_open, Message::ToggleTcpFastOpen),
                ),
                inline(
                    "tcp_multipath_label",
                    Some("help_tcp_multipath"),
                    None,
                    switch(draft.tcp_multipath, Message::ToggleTcpMultipath),
                ),
                stacked(
                    "latency_test_url_label",
                    Some("help_latency_test"),
                    latency_url_error,
                    field(
                        "http://cp.cloudflare.com/generate_204",
                        &draft.latency_test_url,
                        Message::LatencyTestUrlChanged,
                        latency_url_error.is_some(),
                    )
                    .width(Length::Fill)
                    .into(),
                ),
                inline(
                    "latency_test_timeout_label",
                    None,
                    latency_timeout_error,
                    field(
                        "2000",
                        &draft.latency_test_timeout_ms,
                        Message::LatencyTestTimeoutChanged,
                        latency_timeout_error.is_some(),
                    )
                    .width(Length::Fixed(NARROW_CONTROL_W))
                    .into(),
                ),
            ],
        );

        // ── Core: the managed sing-box binary ─────────────────────────────
        let version = core_version
            .map(str::to_string)
            .or_else(|| core_installed.then(|| crate::core::CORE_VERSION.to_string()));
        let mut core_state_line = row![if core_installed {
            caption(
                tr(lang, "core_status_installed").to_string(),
                theme::success_text(theme),
            )
        } else {
            caption(tr(lang, "core_status_missing").to_string(), danger)
        }]
        .spacing(6)
        .align_y(Alignment::Center);
        if let Some(version) = version {
            core_state_line = core_state_line.push(caption(
                format!("· {} {version}", tr(lang, "core_version_label")),
                text_muted,
            ));
        }
        let core_busy = matches!(
            install_state,
            CoreInstallState::Downloading
                | CoreInstallState::Verifying
                | CoreInstallState::Extracting
                | CoreInstallState::Installing
        );
        let core_action = if core_installed {
            small_btn(
                "btn_reinstall_core",
                (!core_busy).then_some(Message::ForceCoreDownload),
                false,
            )
        } else {
            small_btn(
                "btn_download_core",
                (!core_busy).then_some(Message::TriggerCoreDownload),
                true,
            )
        };
        let mut core_status = column![
            row![
                column![
                    text(tr(lang, "singbox_core"))
                        .size(theme::TYPE_BODY)
                        .color(text_primary),
                    core_state_line,
                ]
                .spacing(3)
                .width(Length::Fill),
                core_action,
            ]
            .spacing(crate::ui::SP_16)
            .align_y(Alignment::Center)
        ]
        .spacing(10);
        let install_line: Option<Element<'a, Message>> = match install_state {
            CoreInstallState::Idle => None,
            CoreInstallState::Downloading => {
                Some(crate::ui::loading_row(tr(lang, "core_downloading"), theme))
            }
            CoreInstallState::Verifying => {
                Some(crate::ui::loading_row(tr(lang, "core_verifying"), theme))
            }
            CoreInstallState::Extracting => {
                Some(crate::ui::loading_row(tr(lang, "core_extracting"), theme))
            }
            CoreInstallState::Installing => {
                Some(crate::ui::loading_row(tr(lang, "core_installing"), theme))
            }
            CoreInstallState::Installed => Some(
                caption(
                    tr(lang, "core_install_success").to_string(),
                    theme::success_text(theme),
                )
                .into(),
            ),
            CoreInstallState::Error(error) => Some(
                row![
                    column![
                        caption(tr(lang, "core_install_failed").to_string(), danger),
                        caption(error.clone(), text_muted),
                    ]
                    .spacing(3)
                    .width(Length::Fill),
                    // An error can come from a forced reinstall while the old
                    // binary is still present; retry must not short-circuit on
                    // the existing destination.
                    small_btn("btn_retry_core", Some(Message::ForceCoreDownload), false),
                ]
                .spacing(crate::ui::SP_16)
                .align_y(Alignment::Center)
                .into(),
            ),
        };
        if let Some(line) = install_line {
            core_status = core_status.push(line);
        }

        let core_path_error = errors.get("core_path").copied();
        let core_path_control = row![
            field(
                tr(lang, "placeholder_core_path"),
                &draft.core_path,
                Message::CorePathChanged,
                core_path_error.is_some(),
            )
            .width(Length::Fill),
            small_btn("btn_clear_core_path", Some(Message::ClearCorePath), false),
        ]
        .spacing(crate::ui::SP_8)
        .align_y(Alignment::Center);

        let core = section(
            "settings_core",
            vec![
                core_status.into(),
                stacked(
                    "core_path_label",
                    Some("help_core_path"),
                    core_path_error,
                    core_path_control.into(),
                ),
                inline(
                    "data_dir_label",
                    Some("help_data_dir"),
                    None,
                    small_btn("btn_open", Some(Message::OpenDataDir), false).into(),
                ),
            ],
        );

        // Two balanced columns once each row has room for a label beside its
        // control; one column (Windows-settings style) below that.
        let sections: Element<'_, Message> = if size.width >= crate::ui::SETTINGS_2COL_W {
            row![
                column![general, proxy]
                    .spacing(theme::GRID_GAP)
                    .width(Length::FillPortion(1)),
                column![dns, advanced, core]
                    .spacing(theme::GRID_GAP)
                    .width(Length::FillPortion(1)),
            ]
            .spacing(theme::GRID_GAP)
            .width(Length::Fill)
            .into()
        } else {
            column![general, proxy, dns, advanced, core]
                .spacing(theme::GRID_GAP)
                .width(Length::Fill)
                .into()
        };

        // Collapsible config preview — avoids nested scroll fight when collapsed
        let preview_toggle = button(
            text(if config_preview_expanded {
                tr(lang, "btn_hide_preview")
            } else {
                tr(lang, "btn_show_preview")
            })
            .size(theme::TYPE_BTN_SM),
        )
        .padding(theme::BTN_PAD_SM)
        .style(theme::button_secondary)
        .on_press(Message::ToggleConfigPreview);
        let preview_header = row![
            column![
                text(tr(lang, "core_config_preview"))
                    .size(theme::TYPE_BODY)
                    .color(text_primary),
                text(tr(lang, "help_config_preview"))
                    .size(theme::TYPE_CAPTION)
                    .color(text_muted),
            ]
            .spacing(3)
            .width(Length::Fill),
            preview_toggle,
        ]
        .spacing(crate::ui::SP_16)
        .align_y(Alignment::Center);
        let mut preview_col = column![preview_header].spacing(crate::ui::SP_12);
        if config_preview_expanded {
            preview_col = preview_col.push(
                container(
                    scrollable(
                        text(config_preview.unwrap_or("..."))
                            .font(theme::mono_font())
                            .size(theme::TYPE_MONO)
                            .color(text_primary),
                    )
                    .style(theme::scrollbar_style)
                    .height(Length::Fixed((size.height - 320.0).clamp(120.0, 280.0)))
                    .width(Length::Fill),
                )
                .padding(theme::CARD_PAD_DENSE)
                .style(theme::console_bg),
            );
        }
        let preview_card = container(preview_col)
            .padding([16.0, theme::CARD_PAD])
            .width(Length::Fill)
            .style(theme::card_bg);

        let has_pending_changes = draft.has_pending_changes(gui_config);
        let mut content_col = column![].spacing(theme::GRID_GAP).width(Length::Fill);
        if has_pending_changes {
            content_col = content_col.push(
                container(
                    text(tr(lang, "settings_pending"))
                        .size(theme::TYPE_CAPTION)
                        .color(theme::warning_text(theme)),
                )
                .padding([8, 10])
                .width(Length::Fill)
                .style(|t| theme::tinted_banner(t, theme::WARNING)),
            );
        }
        content_col = content_col.push(sections).push(preview_card);

        // Save only lights up when there is something to save, and a way
        // back sits beside it. Header buttons share the medium size.
        let mut save_btn = button(text(tr(lang, "btn_save_apply")).size(theme::TYPE_BTN_MD))
            .padding(theme::BTN_PAD_MD)
            .style(theme::button_primary);
        let mut header_actions = row![].spacing(crate::ui::SP_8).align_y(Alignment::Center);
        if has_pending_changes {
            save_btn = save_btn.on_press(Message::SaveSettings);
            header_actions = header_actions.push(
                button(text(tr(lang, "btn_discard_changes")).size(theme::TYPE_BTN_MD))
                    .padding(theme::BTN_PAD_MD)
                    .style(theme::button_secondary)
                    .on_press(Message::DiscardSettings),
            );
        }
        header_actions = header_actions.push(save_btn);

        let header = page_header("settings", lang, Some(header_actions.into()), theme, false);
        crate::ui::page_shell_with_pad(
            header,
            content_col.into(),
            crate::ui::is_pad_compact(size.width),
        )
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
