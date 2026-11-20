use crate::message::Message;
use crate::state::GuiConfig;
use crate::ui::theme;
use crate::ui::util::{
    SEMIBOLD_WIDTH, fit_mono, fit_width, format_size, labelled_size, text_width,
    traffic_usage_ratio,
};
use crate::ui::{PAGE_COMPACT_W, page_header};
use iced::widget::{
    Column, Space, button, column, container, responsive, row, text, text_input, tooltip,
};
use iced::{Alignment, Element, Length};

#[allow(clippy::too_many_arguments)]
pub fn render<'a>(
    gui_config: &'a GuiConfig,
    url_input: &'a str,
    downloading: bool,
    profile_error: Option<&'a str>,
    confirm_delete_id: Option<&'a str>,
    editing_profile_id: Option<&'a str>,
    editing_profile_name: &'a str,
    editing_profile_url: &'a str,
    profile_more_id: Option<&'a str>,
    updating_profile_id: Option<&'a str>,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    use crate::ui::i18n::tr;

    let theme_cloned = theme.clone();
    let editing_id = editing_profile_id.map(|s| s.to_string());
    let editing_name = editing_profile_name.to_string();
    let editing_url = editing_profile_url.to_string();
    let more_id = profile_more_id.map(|s| s.to_string());
    let updating_id = updating_profile_id.map(|s| s.to_string());

    let main_content = responsive(move |size| {
        let theme = &theme_cloned;
        let is_compact = size.width < PAGE_COMPACT_W;
        // Room for a card's name and URL: the card's inner width, less the
        // action buttons that share the row on wide layouts.
        let card_inner = (crate::ui::page_content_width(size.width, is_compact)
            - theme::CARD_PAD * 2.0)
            .max(60.0);
        let text_primary = theme::text_primary(theme);
        let text_muted = theme::text_muted(theme);

        let can_import = !downloading && !url_input.trim().is_empty();
        // Import: one input that takes any link or document, with the other
        // sources tucked beside the title instead of a half-empty second row.
        let mut input = text_input(tr(lang, "sub_url_placeholder"), url_input)
            .on_input(Message::SubscriptionInputChanged)
            .padding([10.0, crate::ui::SP_12])
            .width(Length::Fill)
            .size(theme::TYPE_BODY)
            .style(theme::input_field);
        if can_import {
            input = input.on_submit(Message::DownloadSubscription);
        }

        let mut import_btn = button(
            text(tr(
                lang,
                if downloading {
                    "btn_downloading"
                } else {
                    "btn_download"
                },
            ))
            .size(theme::TYPE_BTN_LG)
            .align_x(Alignment::Center),
        )
        .padding(theme::BTN_PAD_LG)
        .style(if downloading {
            theme::button_secondary
        } else {
            theme::button_primary
        });
        if can_import {
            import_btn = import_btn.on_press(Message::DownloadSubscription);
        }

        let open_folder_btn = button(
            row![
                text(crate::ui::icons::ICON_FOLDER_OPEN)
                    .font(iced::Font::with_name("Material Icons"))
                    .size(crate::ui::ICON_SIZE),
                text(tr(lang, "btn_open_folder"))
                    .size(theme::TYPE_BTN_MD)
                    .align_x(Alignment::Center),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(theme::BTN_PAD_MD)
        .style(theme::button_secondary)
        .on_press(Message::OpenProfilesFolder);

        let source_btn = |key: &'static str, msg: Message| {
            let mut b = button(text(tr(lang, key)).size(theme::TYPE_BTN_SM))
                .padding(theme::BTN_PAD_SM)
                .style(theme::button_secondary);
            if !downloading {
                b = b.on_press(msg);
            }
            b
        };

        // A wrapping row would push the Fill spacer, and with it both
        // buttons, onto a second line; the title takes the slack instead.
        let add_header = row![
            text(tr(lang, "import_sub"))
                .color(text_primary)
                .size(theme::TYPE_HEADING)
                .font(theme::ui_font(iced::font::Weight::Semibold))
                .width(Length::Fill),
            source_btn("btn_import_clipboard", Message::ImportFromClipboard),
            source_btn("btn_import_file", Message::ImportLocalFile),
        ]
        .spacing(crate::ui::SP_8)
        .align_y(Alignment::Center);

        let add_form = container(
            column![
                add_header,
                row![input, import_btn]
                    .spacing(crate::ui::SP_12)
                    .align_y(Alignment::Center),
                text(tr(lang, "import_formats_hint"))
                    .size(theme::TYPE_CAPTION)
                    .color(theme::text_tertiary(theme)),
            ]
            .spacing(crate::ui::SP_12),
        )
        .padding(theme::CARD_PAD)
        .style(theme::card_bg);

        let error_banner = profile_error.map(|err| {
            container(
                row![
                    crate::ui::material_icon(crate::ui::icons::ICON_WARNING)
                        .size(theme::TYPE_HEADING)
                        .color(theme::danger_text(theme)),
                    text(err)
                        .size(theme::TYPE_SECTION)
                        .color(theme::danger_text(theme))
                        .width(Length::Fill)
                ]
                .spacing(crate::ui::SP_8)
                .align_y(Alignment::Center),
            )
            .padding(crate::ui::SP_12)
            .width(Length::Fill)
            .style(|t| theme::tinted_banner(t, theme::DANGER))
        });

        // Grid system for profiles list (Responsive Grid)
        let grid_content: Element<'a, Message> = if gui_config.subscriptions.is_empty() {
            crate::ui::empty_state_with_icon(
                Some(crate::ui::icons::ICON_PROFILES),
                tr(lang, "no_profiles"),
                Some(tr(lang, "empty_profiles_hint")),
                None,
                theme,
            )
        } else {
            let mut card_elements: Vec<Element<'a, Message>> = Vec::new();

            // Each action owns a fixed-width slot so Use / Update / More sit
            // in the same columns on every card, whichever buttons a card has.
            let btn_pad_x = f32::from(theme::BTN_PAD_SM[1]) * 2.0;
            let label_w = |keys: &[&'static str]| {
                keys.iter()
                    .map(|k| text_width(tr(lang, k), theme::TYPE_BTN_SM) * SEMIBOLD_WIDTH)
                    .fold(0.0_f32, f32::max)
                    .ceil()
            };
            let use_slot_w = label_w(&["btn_select"]) + btn_pad_x + 4.0;
            let update_slot_w = label_w(&["btn_update", "btn_updating"]) + btn_pad_x + 4.0;
            let more_slot_w =
                label_w(&["btn_more", "btn_less"]) + crate::ui::ICON_SIZE + 2.0 + btn_pad_x + 4.0;
            let actions_w = use_slot_w + update_slot_w + more_slot_w + crate::ui::SP_8 * 2.0;

            for profile in &gui_config.subscriptions {
                let is_active = Some(&profile.id) == gui_config.active_profile_id.as_ref();

                // Only the card whose update is in flight reads "Updating…";
                // non-subscription local/share-link profiles omit Update.
                let is_updating = updating_id.as_deref() == Some(profile.id.as_str());
                let update_btn = profile.is_subscription.then(|| {
                    let mut b = button(
                        text(tr(
                            lang,
                            if is_updating {
                                "btn_updating"
                            } else {
                                "btn_update"
                            },
                        ))
                        .size(theme::TYPE_BTN_SM)
                        .width(Length::Fill)
                        .align_x(Alignment::Center),
                    )
                    .width(Length::Fixed(update_slot_w))
                    .padding(theme::BTN_PAD_SM)
                    .style(theme::button_secondary);
                    if !downloading {
                        b = b.on_press(Message::UpdateSubscription(profile.id.clone()));
                    }
                    b
                });

                // A pending delete confirmation keeps the More row open.
                let confirming_delete = confirm_delete_id == Some(profile.id.as_str());
                let show_more =
                    confirming_delete || more_id.as_deref() == Some(profile.id.as_str());
                let more_btn = button(
                    row![
                        text(tr(lang, if show_more { "btn_less" } else { "btn_more" }))
                            .size(theme::TYPE_BTN_SM),
                        crate::ui::material_icon(if show_more {
                            crate::ui::icons::ICON_EXPAND_LESS
                        } else {
                            crate::ui::icons::ICON_EXPAND_MORE
                        })
                        .size(crate::ui::ICON_SIZE),
                    ]
                    .spacing(2)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fixed(more_slot_w))
                .padding(theme::BTN_PAD_SM)
                .style(theme::button_secondary)
                .on_press(Message::ToggleProfileMore(profile.id.clone()));

                // At most one primary per card: switching profiles is the real
                // decision, refreshing is routine, and edit/delete are rare
                // enough to live behind More.
                // Wide cards keep empty slots so the columns line up; compact
                // cards pack their buttons from the left edge instead.
                let slot = |w: f32| Space::new().width(Length::Fixed(w));
                let mut actions_row = row![].spacing(crate::ui::SP_8).align_y(Alignment::Center);
                if !is_active {
                    actions_row = actions_row.push(
                        button(
                            text(tr(lang, "btn_select"))
                                .size(theme::TYPE_BTN_SM)
                                .width(Length::Fill)
                                .align_x(Alignment::Center),
                        )
                        .width(Length::Fixed(use_slot_w))
                        .padding(theme::BTN_PAD_SM)
                        .style(theme::button_primary)
                        .on_press(Message::SelectProfile(profile.id.clone())),
                    );
                } else if !is_compact {
                    actions_row = actions_row.push(slot(use_slot_w));
                }
                if let Some(b) = update_btn {
                    actions_row = actions_row.push(b);
                } else if !is_compact {
                    actions_row = actions_row.push(slot(update_slot_w));
                }
                actions_row = actions_row.push(more_btn);

                let secondary_row: Option<Element<'a, Message>> = if show_more {
                    let edit_url_btn =
                        button(text(tr(lang, "btn_edit_url")).size(theme::TYPE_BTN_SM))
                            .padding(theme::BTN_PAD_SM)
                            .style(theme::button_secondary)
                            .on_press(Message::StartEditProfile(profile.id.clone()));
                    let edit_btn = button(text(tr(lang, "btn_edit")).size(theme::TYPE_BTN_SM))
                        .padding(theme::BTN_PAD_SM)
                        .style(theme::button_secondary)
                        .on_press(Message::EditProfile(profile.id.clone()));
                    let delete_actions: Element<'a, Message> = if confirming_delete {
                        row![
                            button(text(tr(lang, "btn_cancel")).size(theme::TYPE_BTN_SM))
                                .padding(theme::BTN_PAD_SM)
                                .style(theme::button_secondary)
                                .on_press(Message::CancelDeleteProfile),
                            button(text(tr(lang, "btn_confirm_delete")).size(theme::TYPE_BTN_SM))
                                .padding(theme::BTN_PAD_SM)
                                .style(theme::button_danger)
                                .on_press(Message::ConfirmDeleteProfile),
                        ]
                        .spacing(crate::ui::SP_8)
                        .align_y(Alignment::Center)
                        .into()
                    } else {
                        button(text(tr(lang, "btn_delete")).size(theme::TYPE_BTN_SM))
                            .padding(theme::BTN_PAD_SM)
                            .style(theme::button_danger_soft)
                            .on_press(Message::RequestDeleteProfile(profile.id.clone()))
                            .into()
                    };
                    if is_compact {
                        Some(
                            row![edit_url_btn, edit_btn, delete_actions]
                                .spacing(crate::ui::SP_8)
                                .align_y(Alignment::Center)
                                .wrap()
                                .vertical_spacing(crate::ui::SP_8)
                                .into(),
                        )
                    } else {
                        Some(
                            row![
                                edit_url_btn,
                                edit_btn,
                                Space::new().width(Length::Fill),
                                delete_actions,
                            ]
                            .spacing(crate::ui::SP_8)
                            .align_y(Alignment::Center)
                            .into(),
                        )
                    }
                } else {
                    None
                };

                let masked_url = mask_sensitive_url(&profile.url);
                let info_w = if is_compact {
                    card_inner
                } else {
                    // Every wide card reserves the same three action slots.
                    (card_inner - actions_w - crate::ui::SP_12).max(60.0)
                };
                let display_url = fit_mono(&masked_url, info_w, theme::TYPE_CAPTION);
                let url_truncated = display_url != masked_url;

                let is_editing = editing_id.as_deref() == Some(profile.id.as_str());

                let profile_card = if is_editing {
                    let name_input =
                        text_input(tr(lang, "placeholder_profile_name"), &editing_name)
                            .on_input(Message::EditProfileNameChanged)
                            .padding(10)
                            .size(theme::TYPE_SECTION)
                            .style(theme::input_field);

                    let url_input_field =
                        text_input(tr(lang, "placeholder_profile_url"), &editing_url)
                            .on_input(Message::EditProfileUrlChanged)
                            .padding(10)
                            .size(theme::TYPE_SECTION)
                            .style(theme::input_field);

                    let save_btn: Element<'a, Message> =
                        button(text(tr(lang, "btn_save")).size(theme::TYPE_BTN_SM))
                            .padding(theme::BTN_PAD_SM)
                            .style(theme::button_primary)
                            .on_press(Message::SaveProfileEdit)
                            .into();

                    let cancel_btn: Element<'a, Message> =
                        button(text(tr(lang, "btn_cancel")).size(theme::TYPE_BTN_SM))
                            .padding(theme::BTN_PAD_SM)
                            .style(theme::button_secondary)
                            .on_press(Message::CancelProfileEdit)
                            .into();

                    let form_col = column![
                        text(tr(lang, "edit_link_title"))
                            .color(text_primary)
                            .size(theme::TYPE_HEADING)
                            .font(theme::ui_font(iced::font::Weight::Semibold)),
                        column![
                            text(tr(lang, "placeholder_profile_name"))
                                .color(text_muted)
                                .size(theme::TYPE_CAPTION),
                            name_input.on_submit(Message::SaveProfileEdit)
                        ]
                        .spacing(4),
                        column![
                            text(tr(lang, "placeholder_profile_url"))
                                .color(text_muted)
                                .size(theme::TYPE_CAPTION),
                            url_input_field.on_submit(Message::SaveProfileEdit)
                        ]
                        .spacing(4),
                        row![
                            iced::widget::Space::new().width(Length::Fill),
                            cancel_btn,
                            save_btn
                        ]
                        .spacing(crate::ui::SP_12)
                    ]
                    .spacing(crate::ui::SP_12);

                    container(form_col)
                        .padding(theme::CARD_PAD)
                        .width(Length::Fill)
                        .style(theme::card_selected)
                } else {
                    let traffic_block: Option<Element<'a, Message>> =
                        match (profile.traffic_upload, profile.traffic_download) {
                            (Some(u), Some(d)) => {
                                let total = profile.traffic_total.unwrap_or(0);
                                let used = u.saturating_add(d);
                                let caption =
                                    |s: String| text(s).color(text_muted).size(theme::TYPE_CAPTION);
                                if let Some(ratio) = traffic_usage_ratio(u, d, total) {
                                    let labels = row![
                                        text(format!(
                                            "{} / {}",
                                            format_size(used),
                                            format_size(total)
                                        ))
                                        .font(theme::metric_font())
                                        .color(text_primary)
                                        .size(theme::TYPE_BTN_SM),
                                        Space::new().width(Length::Fill),
                                        caption(labelled_size(
                                            lang,
                                            "traffic_left",
                                            total.saturating_sub(used)
                                        )),
                                    ]
                                    .align_y(Alignment::Center);
                                    Some(
                                        column![labels, crate::ui::usage_bar(ratio, theme)]
                                            .spacing(6)
                                            .width(Length::Fill)
                                            .into(),
                                    )
                                } else {
                                    Some(caption(labelled_size(lang, "traffic_used", used)).into())
                                }
                            }
                            _ => None,
                        };

                    let mut meta_items: Vec<Element<'_, Message>> = vec![
                        text(format!(
                            "{} {}",
                            tr(lang, "updated_at_label"),
                            profile.updated_at
                        ))
                        .color(text_muted)
                        .size(theme::TYPE_CAPTION)
                        .into(),
                    ];
                    if !profile.is_subscription {
                        meta_items.push(
                            text("·")
                                .color(theme::text_tertiary(theme))
                                .size(theme::TYPE_CAPTION)
                                .into(),
                        );
                        meta_items.push(
                            text(tr(lang, "profile_kind_local"))
                                .color(theme::text_tertiary(theme))
                                .size(theme::TYPE_CAPTION)
                                .into(),
                        );
                    }
                    if let Some(ts) = profile.expire_at
                        && let Some(dt) = chrono::DateTime::from_timestamp(ts, 0)
                    {
                        let near =
                            ts.saturating_sub(chrono::Utc::now().timestamp()) < 7 * 24 * 3600;
                        meta_items.push(
                            text("·")
                                .color(theme::text_tertiary(theme))
                                .size(theme::TYPE_CAPTION)
                                .into(),
                        );
                        meta_items.push(
                            text(format!(
                                "{} {}",
                                tr(lang, "expire_at_label"),
                                dt.with_timezone(&chrono::Local).format("%Y-%m-%d")
                            ))
                            .color(if near {
                                theme::warning_text(theme)
                            } else {
                                text_muted
                            })
                            .size(theme::TYPE_CAPTION)
                            .into(),
                        );
                    }
                    let meta_line = iced::widget::Row::with_children(meta_items)
                        .spacing(6)
                        .align_y(Alignment::Center)
                        .wrap()
                        .vertical_spacing(4);

                    // Status reads as a small pill beside the name, not as a
                    // control-sized block that looks like a disabled button.
                    let short_name = fit_width(
                        &profile.name,
                        (info_w - if is_active { 100.0 } else { 0.0 }).max(40.0),
                        theme::TYPE_HEADING * SEMIBOLD_WIDTH,
                    );
                    let name_truncated = short_name != profile.name;
                    let name_label = text(short_name)
                        .shaping(text::Shaping::Advanced)
                        .wrapping(text::Wrapping::None)
                        .color(text_primary)
                        .size(theme::TYPE_HEADING)
                        .font(theme::ui_font(iced::font::Weight::Semibold));
                    let name_el: Element<'a, Message> = if name_truncated {
                        tooltip(
                            name_label,
                            container(
                                text(&profile.name)
                                    .size(theme::TYPE_BTN_SM)
                                    .color(text_primary),
                            )
                            .padding(theme::BTN_PAD_SM)
                            .style(theme::popover_bg),
                            tooltip::Position::Top,
                        )
                        .into()
                    } else {
                        name_label.into()
                    };
                    let mut name_row = row![name_el]
                        .spacing(crate::ui::SP_8)
                        .align_y(Alignment::Center);
                    if is_active {
                        name_row = name_row.push(
                            container(
                                row![
                                    crate::ui::material_icon(crate::ui::icons::ICON_CHECK_CIRCLE)
                                        .size(theme::TYPE_CAPTION),
                                    text(tr(lang, "active_profile"))
                                        .size(theme::TYPE_CAPTION)
                                        .font(theme::ui_font(iced::font::Weight::Semibold)),
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center),
                            )
                            .padding([2, 8])
                            .style(theme::badge_success),
                        );
                    }

                    let url_label = text(display_url)
                        .color(theme::text_tertiary(theme))
                        .size(theme::TYPE_CAPTION)
                        .font(theme::mono_font());
                    let url_el: Element<'a, Message> = if url_truncated {
                        tooltip(
                            url_label,
                            container(
                                text(masked_url)
                                    .size(theme::TYPE_CAPTION)
                                    .font(theme::mono_font())
                                    .color(text_primary),
                            )
                            .padding(theme::BTN_PAD_SM)
                            .max_width(480)
                            .style(theme::popover_bg),
                            tooltip::Position::Bottom,
                        )
                        .into()
                    } else {
                        url_label.into()
                    };

                    let info_col = column![name_row, url_el, meta_line,]
                        .spacing(6)
                        .width(Length::Fill);

                    // Wide: actions sit beside the identity block. Compact: they
                    // follow the usage bar so the name keeps the full width.
                    let mut card_layout =
                        Column::new().spacing(crate::ui::SP_12).width(Length::Fill);
                    if is_compact {
                        card_layout = card_layout.push(info_col);
                        if let Some(t) = traffic_block {
                            card_layout = card_layout.push(t);
                        }
                        card_layout = card_layout.push(actions_row);
                    } else {
                        card_layout = card_layout.push(
                            row![info_col, actions_row]
                                .spacing(crate::ui::SP_12)
                                .align_y(Alignment::Center),
                        );
                        if let Some(t) = traffic_block {
                            card_layout = card_layout.push(t);
                        }
                    }
                    if let Some(sec) = secondary_row {
                        card_layout = card_layout
                            .push(
                                container(Space::new())
                                    .width(Length::Fill)
                                    .height(1)
                                    .style(theme::hairline),
                            )
                            .push(sec);
                    }

                    container(card_layout)
                        .padding(theme::CARD_PAD)
                        .width(Length::Fill)
                        .style(move |theme| {
                            if is_active {
                                theme::card_selected(theme)
                            } else {
                                theme::card_bg(theme)
                            }
                        })
                };

                card_elements.push(container(profile_card).width(Length::Fill).into());
            }

            // Profiles are operational records, not promotional cards. A
            // single aligned list scans faster and avoids a dashboard mosaic.
            Column::with_children(card_elements)
                .spacing(theme::GRID_GAP)
                .width(Length::Fill)
                .into()
        };

        let mut main_layout_col = column![add_form,]
            .spacing(crate::ui::SP_20)
            .width(Length::Fill);

        if let Some(banner) = error_banner {
            main_layout_col = main_layout_col.push(banner);
        }

        main_layout_col = main_layout_col.push(
            text(tr(lang, "imported_profiles"))
                .color(text_primary)
                .size(theme::TYPE_HEADING)
                .font(theme::ui_font(iced::font::Weight::Semibold)),
        );
        main_layout_col = main_layout_col.push(grid_content);

        // A single small action fits beside the title even at 640px.
        let header = page_header(
            "tab_profiles",
            lang,
            Some(open_folder_btn.into()),
            theme,
            false,
        );

        // Scroll the import form with the records. The shell keeps the
        // scrollbar in its own lane, so cards end flush with the header action.
        crate::ui::page_shell_with_pad(
            header,
            container(main_layout_col).width(Length::Fill).into(),
            is_compact,
        )
    });

    main_content.into()
}

pub fn mask_sensitive_url(url: &str) -> String {
    let Ok(mut parsed) = url::Url::parse(url) else {
        return url.to_string();
    };

    let sensitive_key = |key: &str| {
        let key = key.to_ascii_lowercase().replace('-', "_");
        key.contains("token")
            || key.contains("uuid")
            || key.contains("key")
            || key.contains("pwd")
            || key.contains("password")
            || key.contains("secret")
            || key.contains("auth")
            || key == "pass"
            || key == "psk"
    };
    let has_password = parsed.password().is_some();
    if has_password {
        let _ = parsed.set_password(Some("******"));
    }
    // Proxy share links (trojan://password@host, vless://uuid@host, etc.) store
    // their secret in `username` without a `:` password separator.
    let is_proxy_share_scheme = matches!(
        parsed.scheme(),
        "vless"
            | "vmess"
            | "trojan"
            | "ss"
            | "shadowsocks"
            | "hy2"
            | "hysteria"
            | "hysteria2"
            | "tuic"
            | "anytls"
            | "naive+https"
            | "naive+quic"
            | "ssh"
            | "shadowtls"
            | "wg"
            | "wireguard"
            | "socks"
            | "socks4"
            | "socks4a"
            | "socks5"
            | "socks5h"
    );
    let masked_username = is_proxy_share_scheme && !parsed.username().is_empty() && !has_password;
    if masked_username {
        let _ = parsed.set_username("******");
    }
    let mut query_changed = false;
    let query_pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .map(|(key, value)| {
            if sensitive_key(&key) {
                query_changed = true;
                (key.into_owned(), "******".to_string())
            } else {
                (key.into_owned(), value.into_owned())
            }
        })
        .collect();
    if query_changed {
        let mut pairs = parsed.query_pairs_mut();
        pairs.clear();
        for (key, value) in query_pairs {
            pairs.append_pair(&key, &value);
        }
    }
    if let Some(fragment) = parsed.fragment().map(str::to_string)
        && (sensitive_key(&fragment) || fragment.len() > 32)
    {
        parsed.set_fragment(Some("******"));
    }
    if has_password || masked_username || query_changed || parsed.fragment() == Some("******") {
        return parsed.to_string();
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_sensitive_url() {
        assert_eq!(
            mask_sensitive_url("https://example.com/sub?token=123456&flag=clash"),
            "https://example.com/sub?token=******&flag=clash"
        );
        assert_eq!(
            mask_sensitive_url("https://example.com/sub?uuid=some-uuid-value&flag=clash"),
            "https://example.com/sub?uuid=******&flag=clash"
        );
        assert_eq!(
            mask_sensitive_url("https://example.com/sub?normal_param=value"),
            "https://example.com/sub?normal_param=value"
        );
        assert_eq!(
            mask_sensitive_url("invalid_url_string"),
            "invalid_url_string"
        );
        let masked = mask_sensitive_url(
            "https://user:pa%20ss@example.com/sub?token=a%2Fb&name=a%20b#fragment-token",
        );
        assert!(masked.contains("user:******@example.com"));
        assert!(masked.contains("token=******"));
        assert!(masked.contains("name=a+b") || masked.contains("name=a%20b"));

        let vless = mask_sensitive_url(
            "vless://11111111-1111-1111-1111-111111111111@node.example:443?api-key=secretkey#HK",
        );
        assert!(!vless.contains("11111111-1111"), "vless={vless}");
        assert!(vless.contains("******@node.example:443"), "vless={vless}");
        assert!(vless.contains("api-key=******"), "vless={vless}");

        let trojan = mask_sensitive_url("trojan://super-secret-pass@node.example:443#JP");
        assert!(!trojan.contains("super-secret-pass"), "trojan={trojan}");
        assert!(trojan.contains("******@node.example:443"));

        let anytls = mask_sensitive_url("anytls://secret-pass@edge.example:443?sni=edge#SG");
        assert!(!anytls.contains("secret-pass"), "anytls={anytls}");
        assert!(anytls.contains("******@edge.example:443"));

        let wg = mask_sensitive_url("wg://privkey-token@wg.example:51820?psk=secret123#WG");
        assert!(!wg.contains("privkey-token"), "wg={wg}");
        assert!(!wg.contains("secret123"), "wg={wg}");
        assert!(wg.contains("psk=******"), "wg={wg}");
    }

    #[test]
    fn profiles_render_handles_empty_editing_deleting_and_long_urls() {
        let mut cfg = GuiConfig::default();
        let _ = render(
            &cfg,
            "",
            false,
            None,
            None,
            None,
            "",
            "",
            None,
            None,
            &iced::Theme::Light,
        );
        cfg.language = crate::state::Language::Zh;
        cfg.active_profile_id = Some("p1".into());
        cfg.subscriptions.push(crate::state::Profile {
            id: "p1".into(),
            name: "Very Long Profile Name For Truncation Tooltip Verification 测试长名称".into(),
            url: "https://example.com/very/long/subscription/path/with/many/segments?token=secret123456".into(),
            file_path: String::new(),
            is_subscription: true,
            updated_at: "2026-11-18 10:00".into(),
            traffic_upload: Some(1000),
            traffic_download: Some(2000),
            traffic_total: Some(10000),
            expire_at: Some(1_893_456_000),
        });
        let _ = render(
            &cfg,
            "https://example.com",
            true,
            Some("Error"),
            Some("p1"),
            None,
            "",
            "",
            Some("p1"),
            Some("p1"),
            &iced::Theme::Dark,
        );
    }

    /// Structural: shipped profiles UI source must not use bare numeric text sizes
    /// for body/button labels — TYPE_* tokens are the source of truth.
    #[test]
    fn profiles_source_uses_type_tokens_not_bare_sizes() {
        let src = include_str!("profiles.rs");
        // Strip this test module so assertions below don't match themselves.
        let code = src
            .split("#[cfg(test)]")
            .next()
            .expect("profiles.rs should have a test module");
        // Bare text/button size literals like .size(12) / .size(14.0)
        let bare_size = regex_lite_count_bare_sizes(code);
        assert_eq!(
            bare_size, 0,
            "profiles.rs still has {bare_size} bare .size(N) literals outside tests"
        );
        assert!(
            code.contains("theme::TYPE_"),
            "profiles.rs must reference theme::TYPE_* tokens"
        );
    }

    fn regex_lite_count_bare_sizes(code: &str) -> usize {
        // Match .size(digits) or .size(digits.digits) — not TYPE_* or variables
        let mut count = 0;
        let bytes = code.as_bytes();
        let needle = b".size(";
        let mut i = 0;
        while i + needle.len() < bytes.len() {
            if &bytes[i..i + needle.len()] == needle {
                let rest = &code[i + needle.len()..];
                let end = rest.find(')').unwrap_or(0);
                let arg = rest[..end].trim();
                if !arg.is_empty() && arg.chars().all(|c| c.is_ascii_digit() || c == '.') {
                    count += 1;
                }
                i += needle.len() + end.max(1);
            } else {
                i += 1;
            }
        }
        count
    }
}
