use crate::message::Message;
use crate::state::GuiConfig;
use crate::ui::theme;
use crate::ui::util::{format_traffic_usage_lang, traffic_usage_ratio, truncate_chars};
use crate::ui::{PAGE_COMPACT_W, page_header};
use iced::widget::{
    Column, Space, button, column, container, responsive, row, scrollable, text, text_input,
};
use iced::{Alignment, Color, Element, Length};

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
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    use crate::ui::i18n::tr;

    let theme_cloned = theme.clone();
    let editing_id = editing_profile_id.map(|s| s.to_string());
    let editing_name = editing_profile_name.to_string();
    let editing_url = editing_profile_url.to_string();
    let more_id = profile_more_id.map(|s| s.to_string());

    let main_content = responsive(move |size| {
        let theme = &theme_cloned;
        let is_compact = size.width < PAGE_COMPACT_W;
        let text_primary = theme::text_primary(theme);
        let text_muted = theme::text_muted(theme);

        // Add subscription input form
        let input = text_input(tr(lang, "sub_url_placeholder"), url_input)
            .on_input(Message::SubscriptionInputChanged)
            .on_submit(Message::DownloadSubscription)
            .padding(crate::ui::SP_12)
            .width(Length::Fill)
            .style(theme::input_field);

        let download_btn = if downloading {
            button(
                text(tr(lang, "btn_downloading"))
                    .size(theme::TYPE_BTN_LG)
                    .align_x(Alignment::Center),
            )
            .padding(theme::BTN_PAD_LG)
            .style(theme::button_secondary)
        } else {
            button(
                text(tr(lang, "btn_download"))
                    .size(theme::TYPE_BTN_LG)
                    .align_x(Alignment::Center),
            )
            .padding(theme::BTN_PAD_LG)
            .style(theme::button_primary)
            .on_press(Message::DownloadSubscription)
        };

        let open_folder_btn = button(
            text(tr(lang, "btn_open_folder"))
                .size(theme::TYPE_BTN_MD)
                .align_x(Alignment::Center),
        )
        .padding(theme::BTN_PAD_MD)
        .style(theme::button_secondary)
        .on_press(Message::OpenProfilesFolder);

        let clipboard_btn = button(
            text(tr(lang, "btn_import_clipboard"))
                .size(theme::TYPE_BTN_LG)
                .align_x(Alignment::Center),
        )
        .padding(theme::BTN_PAD_LG)
        .style(theme::button_secondary)
        .on_press(Message::ImportFromClipboard);

        let file_btn = button(
            text(tr(lang, "btn_import_file"))
                .size(theme::TYPE_BTN_LG)
                .align_x(Alignment::Center),
        )
        .padding(theme::BTN_PAD_LG)
        .style(theme::button_secondary)
        .on_press(Message::ImportLocalFile);

        let form_layout: Element<'a, Message> = if is_compact {
            column![
                input,
                download_btn.width(Length::Fill),
                row![
                    clipboard_btn.width(Length::Fill),
                    file_btn.width(Length::Fill)
                ]
                .spacing(crate::ui::SP_12)
                .width(Length::Fill)
            ]
            .spacing(crate::ui::SP_12)
            .width(Length::Fill)
            .into()
        } else {
            column![
                row![input, download_btn]
                    .spacing(crate::ui::SP_12)
                    .align_y(Alignment::Center)
                    .width(Length::Fill),
                row![
                    clipboard_btn,
                    file_btn,
                    iced::widget::Space::new().width(Length::Fill),
                ]
                .spacing(crate::ui::SP_12)
                .align_y(Alignment::Center)
            ]
            .spacing(crate::ui::SP_12)
            .width(Length::Fill)
            .into()
        };

        let add_form = container(
            column![
                text(tr(lang, "import_sub"))
                    .color(text_primary)
                    .size(theme::TYPE_HEADING)
                    .font(theme::ui_font(iced::font::Weight::Semibold)),
                form_layout
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
                        .color(theme::DANGER),
                    text(err).size(theme::TYPE_SECTION).color(theme::DANGER)
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
            crate::ui::empty_state(
                tr(lang, "no_profiles"),
                Some(tr(lang, "empty_profiles_hint")),
                None,
                theme,
            )
        } else {
            let mut card_elements: Vec<Element<'a, Message>> = Vec::new();

            for profile in &gui_config.subscriptions {
                let is_active = Some(&profile.id) == gui_config.active_profile_id.as_ref();

                let update_btn = if downloading {
                    button(text(tr(lang, "btn_downloading")).size(theme::TYPE_BTN_SM))
                        .padding(theme::BTN_PAD_SM)
                        .style(theme::button_secondary)
                } else {
                    button(text(tr(lang, "btn_update")).size(theme::TYPE_BTN_SM))
                        .padding(theme::BTN_PAD_SM)
                        .style(theme::button_primary)
                        .on_press(Message::UpdateSubscription(profile.id.clone()))
                };

                let delete_actions: Element<'a, Message> =
                    if confirm_delete_id == Some(profile.id.as_str()) {
                        row![
                            button(text(tr(lang, "btn_confirm_delete")).size(theme::TYPE_BTN_SM))
                                .padding(theme::BTN_PAD_SM)
                                .style(theme::button_danger)
                                .on_press(Message::ConfirmDeleteProfile),
                            button(text(tr(lang, "btn_cancel")).size(theme::TYPE_BTN_SM))
                                .padding(theme::BTN_PAD_SM)
                                .style(theme::button_secondary)
                                .on_press(Message::CancelDeleteProfile),
                        ]
                        .spacing(crate::ui::SP_8)
                        .align_y(Alignment::Center)
                        .into()
                    } else {
                        button(text(tr(lang, "btn_delete")).size(theme::TYPE_BTN_SM))
                            .padding(theme::BTN_PAD_SM)
                            .style(theme::button_secondary)
                            .on_press(Message::RequestDeleteProfile(profile.id.clone()))
                            .into()
                    };

                let show_more = more_id.as_deref() == Some(profile.id.as_str());
                let more_btn = button(
                    text(if show_more {
                        tr(lang, "btn_less")
                    } else {
                        tr(lang, "btn_more")
                    })
                    .size(theme::TYPE_BTN_SM),
                )
                .padding(theme::BTN_PAD_SM)
                .style(theme::button_secondary)
                .on_press(Message::ToggleProfileMore(profile.id.clone()));

                let badge_or_spacer: Element<'a, Message> = if is_active {
                    container(
                        text(tr(lang, "active_profile"))
                            .color(Color::WHITE)
                            .size(theme::TYPE_BTN_SM),
                    )
                    .padding(theme::BTN_PAD_SM)
                    .style(theme::badge_success)
                    .into()
                } else {
                    iced::widget::Space::new().width(Length::Shrink).into()
                };

                // Primary actions only: select / update / delete (+ more toggle)
                let mut actions_row = row![].spacing(crate::ui::SP_8).align_y(Alignment::Center);
                if !is_active {
                    let select_btn = button(text(tr(lang, "btn_select")).size(theme::TYPE_BTN_SM))
                        .padding(theme::BTN_PAD_SM)
                        .style(theme::button_secondary)
                        .on_press(Message::SelectProfile(profile.id.clone()));
                    actions_row = actions_row.push(select_btn);
                }
                actions_row = actions_row
                    .push(update_btn)
                    .push(delete_actions)
                    .push(more_btn);

                let secondary_row: Option<Element<'a, Message>> = if show_more {
                    Some(
                        row![
                            button(text(tr(lang, "btn_edit_url")).size(theme::TYPE_BTN_SM))
                                .padding(theme::BTN_PAD_SM)
                                .style(theme::button_secondary)
                                .on_press(Message::StartEditProfile(profile.id.clone())),
                            button(text(tr(lang, "btn_edit")).size(theme::TYPE_BTN_SM))
                                .padding(theme::BTN_PAD_SM)
                                .style(theme::button_secondary)
                                .on_press(Message::EditProfile(profile.id.clone())),
                        ]
                        .spacing(crate::ui::SP_8)
                        .align_y(Alignment::Center)
                        .into(),
                    )
                } else {
                    None
                };

                let masked_url = mask_sensitive_url(&profile.url);
                let display_url = truncate_chars(&masked_url, 40);

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
                            .font(theme::ui_font(iced::font::Weight::Bold)),
                        column![
                            text(tr(lang, "placeholder_profile_name"))
                                .color(text_muted)
                                .size(theme::TYPE_CAPTION),
                            name_input
                        ]
                        .spacing(4),
                        column![
                            text(tr(lang, "placeholder_profile_url"))
                                .color(text_muted)
                                .size(theme::TYPE_CAPTION),
                            url_input_field
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
                                let label = text(format_traffic_usage_lang(lang, u, d, total))
                                    .color(theme::ACCENT_BLUE)
                                    .size(theme::TYPE_CAPTION);
                                if let Some(ratio) = traffic_usage_ratio(u, d, total) {
                                    let bar_color = if ratio >= 0.9 {
                                        theme::DANGER
                                    } else if ratio >= 0.75 {
                                        theme::WARNING
                                    } else {
                                        theme::ACCENT_BLUE
                                    };
                                    // Simple fill bar without relying on ProgressBar private height API
                                    let bar = container(
                                        row![
                                            container(Space::new())
                                                .width(Length::FillPortion(
                                                    (ratio * 1000.0).max(1.0) as u16
                                                ))
                                                .height(6.0)
                                                .style(move |_t| container::Style {
                                                    background: Some(iced::Background::Color(
                                                        bar_color
                                                    )),
                                                    border: iced::Border {
                                                        radius: theme::RADIUS_MICRO.into(),
                                                        ..Default::default()
                                                    },
                                                    ..Default::default()
                                                }),
                                            container(Space::new())
                                                .width(Length::FillPortion(
                                                    ((1.0 - ratio) * 1000.0).max(1.0) as u16
                                                ))
                                                .height(6.0),
                                        ]
                                        .width(Length::Fill),
                                    )
                                    .width(Length::Fill)
                                    .padding(0)
                                    .style(|t| {
                                        container::Style {
                                            background: Some(iced::Background::Color(
                                                theme::input_surface(t),
                                            )),
                                            border: iced::Border {
                                                radius: theme::RADIUS_MICRO.into(),
                                                ..Default::default()
                                            },
                                            ..Default::default()
                                        }
                                    });
                                    Some(column![label, bar].spacing(4).width(Length::Fill).into())
                                } else {
                                    Some(label.into())
                                }
                            }
                            _ => None,
                        };

                    let expire_line = profile.expire_at.and_then(|ts| {
                        chrono::DateTime::from_timestamp(ts, 0).map(|dt| {
                            let near = {
                                let now = chrono::Utc::now().timestamp();
                                ts.saturating_sub(now) < 7 * 24 * 3600
                            };
                            text(format!(
                                "{}: {}",
                                tr(lang, "expire_at_label"),
                                dt.with_timezone(&chrono::Local).format("%Y-%m-%d")
                            ))
                            .color(if near { theme::WARNING } else { text_muted })
                            .size(theme::TYPE_CAPTION)
                        })
                    });

                    let mut meta_col = column![
                        text(truncate_chars(&profile.name, 36))
                            .color(text_primary)
                            .size(theme::TYPE_BODY)
                            .font(theme::ui_font(iced::font::Weight::Bold)),
                        text(display_url).color(text_muted).size(theme::TYPE_BTN_SM),
                        text(format!(
                            "{}: {}",
                            tr(lang, "updated_at_label"),
                            profile.updated_at
                        ))
                        .color(text_muted)
                        .size(theme::TYPE_CAPTION),
                    ]
                    .spacing(crate::ui::SP_8)
                    .width(Length::Fill);

                    if let Some(t) = traffic_block {
                        meta_col = meta_col.push(t);
                    }
                    if let Some(e) = expire_line {
                        meta_col = meta_col.push(e);
                    }

                    let mut card_layout = column![
                        meta_col,
                        row![
                            badge_or_spacer,
                            iced::widget::Space::new().width(Length::Fill),
                            actions_row
                        ]
                        .align_y(Alignment::Center)
                        .width(Length::Fill)
                    ]
                    .spacing(crate::ui::SP_12);

                    if let Some(sec) = secondary_row {
                        card_layout = card_layout.push(sec);
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

            // Calculate columns based on width
            // Profiles are operational records, not promotional cards. A
            // single aligned list scans faster and avoids a dashboard mosaic.
            let cols = 1;

            let mut grid_rows = Column::new().spacing(theme::GRID_GAP);
            let mut current_row = iced::widget::Row::new().spacing(theme::GRID_GAP);
            let total_cards = card_elements.len();

            for (i, card) in card_elements.into_iter().enumerate() {
                current_row = current_row.push(container(card).width(Length::FillPortion(1)));
                if (i + 1) % cols == 0 {
                    grid_rows = grid_rows.push(current_row);
                    current_row = iced::widget::Row::new().spacing(theme::GRID_GAP);
                }
            }

            let remaining_elements = total_cards % cols;
            if remaining_elements > 0 {
                for _ in remaining_elements..cols {
                    current_row =
                        current_row.push(container(text("")).width(Length::FillPortion(1)));
                }
                grid_rows = grid_rows.push(current_row);
            }
            scrollable(grid_rows)
                .style(theme::scrollbar_style)
                .height(Length::Fill)
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

        let header = page_header(
            "tab_profiles",
            lang,
            Some(open_folder_btn.into()),
            theme,
            is_compact,
        );

        let col = column![header, main_layout_col]
            .spacing(crate::ui::SP_20)
            .width(Length::Fill)
            .height(Length::Fill);

        crate::ui::page_body_fixed_with_pad(col.into(), is_compact)
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
    };
    let has_password = parsed.password().is_some();
    if has_password {
        let _ = parsed.set_password(Some("******"));
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
    if has_password || query_changed || parsed.fragment() == Some("******") {
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
