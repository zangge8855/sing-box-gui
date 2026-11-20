use crate::message::Message;
use crate::state::{GuiConfig, Language, RuleField, RuleTarget};
use crate::ui::i18n::tr;
use crate::ui::page_header;
use crate::ui::theme;
use iced::widget::{
    Column, Space, button, column, container, responsive, row, text, text_input, tooltip,
};
use iced::{Alignment, Element, Length};

/// Every list row has the same height, so two cards can be matched by count.
const RULE_ROW_H: f32 = 34.0;

pub fn render<'a>(
    gui_config: &'a GuiConfig,
    direct_input: &'a str,
    proxy_input: &'a str,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    let theme_cloned = theme.clone();

    let main_content = responsive(move |size| {
        let is_compact = size.width < crate::ui::PAGE_NARROW_W;
        let pad_compact = crate::ui::is_pad_compact(size.width);
        let content_w =
            crate::ui::page_content_width(size.width, pad_compact) - crate::ui::SCROLL_GUTTER;
        let card_w = if is_compact {
            content_w
        } else {
            (content_w - theme::GRID_GAP) / 2.0
        }
        .max(60.0);
        // Rule text room: card minus padding, card/item borders (4) and scroll
        // left inset (1), row insets (10 + 4), the close button (~24) and two
        // 12px gaps. The kind tag is subtracted per row.
        let item_px = (card_w - theme::CARD_PAD * 2.0 - 14.0 - 24.0 - 24.0 - 5.0).max(60.0);
        let theme = &theme_cloned;

        // What the built-in rules already cover, as one quiet line rather
        // than a card that competes with the lists.
        let note = container(
            row![
                crate::ui::material_icon(crate::ui::icons::ICON_INFO)
                    .size(crate::ui::ICON_SIZE)
                    .color(theme::text_tertiary(theme)),
                text(tr(lang, "rules_builtin_desc"))
                    .size(theme::TYPE_BTN_SM)
                    .color(theme::text_muted(theme))
                    .width(Length::Fill),
            ]
            .spacing(crate::ui::SP_8)
            .align_y(Alignment::Center),
        )
        .padding([10, 14])
        .width(Length::Fill)
        .style(theme::status_card);

        let direct_lists = [
            (
                RuleField::BypassDomains,
                &gui_config.custom_bypass_domains[..],
            ),
            (RuleField::BypassIps, &gui_config.custom_bypass_ips[..]),
        ];
        let proxy_lists = [
            (
                RuleField::ProxyDomains,
                &gui_config.custom_proxy_domains[..],
            ),
            (RuleField::ProxyIps, &gui_config.custom_proxy_ips[..]),
        ];
        // An empty list shows a placeholder two rows tall.
        let rows = |lists: &[(RuleField, &[String]); 2]| match lists[0].1.len() + lists[1].1.len() {
            0 => 2,
            n => n,
        };
        let (direct_rows, proxy_rows) = (rows(&direct_lists), rows(&proxy_lists));

        // Side by side, the shorter card stretches to the taller one.
        let direct = rule_card(
            RuleTarget::Direct,
            direct_input,
            direct_lists,
            !is_compact && direct_rows < proxy_rows,
            is_compact,
            item_px,
            lang,
            theme,
        );
        let proxy = rule_card(
            RuleTarget::Proxy,
            proxy_input,
            proxy_lists,
            !is_compact && proxy_rows < direct_rows,
            is_compact,
            item_px,
            lang,
            theme,
        );
        let cards: Element<'_, Message> = if is_compact {
            column![direct, proxy].spacing(theme::GRID_GAP).into()
        } else {
            row![direct, proxy]
                .spacing(theme::GRID_GAP)
                .height(Length::Shrink)
                .into()
        };

        let header = page_header("tab_rules", lang, None, theme, pad_compact);
        let content = column![note, cards]
            .spacing(theme::GRID_GAP)
            .width(Length::Fill);
        crate::ui::page_shell_with_pad(header, content.into(), pad_compact)
    });

    main_content.into()
}

/// One target (direct or proxy): a single input that accepts domains and IP
/// ranges alike, over every rule it holds, each tagged with its kind.
#[allow(clippy::too_many_arguments)]
fn rule_card<'a>(
    target: RuleTarget,
    input_value: &'a str,
    lists: [(RuleField, &'a [String]); 2],
    stretch: bool,
    is_compact: bool,
    item_px: f32,
    lang: Language,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let text_primary = theme::text_primary(theme);
    let text_muted = theme::text_muted(theme);
    let text_tertiary = theme::text_tertiary(theme);
    let (title_key, desc_key) = match target {
        RuleTarget::Direct => ("rules_direct_title", "rules_direct_desc"),
        RuleTarget::Proxy => ("rules_proxy_title", "rules_proxy_desc"),
    };
    let count = lists[0].1.len() + lists[1].1.len();

    let target_dot_color = match target {
        RuleTarget::Direct => theme::SUCCESS,
        RuleTarget::Proxy => theme::accent(theme),
    };
    let title_row = row![
        container(Space::new())
            .width(7)
            .height(7)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(target_dot_color)),
                border: iced::Border {
                    radius: 3.5.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        text(tr(lang, title_key))
            .size(theme::TYPE_HEADING)
            .font(theme::ui_font(iced::font::Weight::Semibold))
            .color(text_primary),
        container(
            text(count.to_string())
                .size(theme::TYPE_MICRO)
                .font(theme::metric_font())
                .color(text_muted),
        )
        .padding([1, 7])
        .style(theme::badge_bg),
    ]
    .spacing(crate::ui::SP_8)
    .align_y(Alignment::Center);

    let validation_err =
        crate::validate_rule_draft(target, input_value, lists[0].1, lists[1].1).err();
    let can_add = !input_value.trim().is_empty() && validation_err.is_none();

    let mut input = text_input(tr(lang, "placeholder_rule"), input_value)
        .on_input(move |value| Message::RulesInputChanged { target, value })
        .padding([8, 10])
        .size(theme::TYPE_BTN_SM)
        .width(Length::Fill)
        .style(if validation_err.is_some() {
            theme::input_field_error
        } else {
            theme::input_field
        });
    if can_add {
        input = input.on_submit(Message::AddRule { target });
    }
    let mut add_btn = button(
        row![
            text(crate::ui::icons::ICON_ADD)
                .font(iced::Font::with_name("Material Icons"))
                .size(theme::TYPE_BTN_MD),
            text(tr(lang, "btn_add_rule")).size(theme::TYPE_BTN_SM),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding(theme::BTN_PAD_SM)
    .style(if can_add {
        theme::button_primary
    } else {
        theme::button_secondary
    });
    if can_add {
        add_btn = add_btn.on_press(Message::AddRule { target });
    }

    // One pill width for both kinds, so the item text and the delete glyphs
    // form straight columns no matter which tags a list mixes.
    let tag_w = ["rule_tag_ip", "rule_tag_domain"]
        .iter()
        .map(|k| crate::ui::util::text_width(tr(lang, k), theme::TYPE_MICRO))
        .fold(0.0_f32, f32::max)
        .ceil()
        + 14.0;

    let mut list = Column::new().spacing(0).width(Length::Fill);
    let mut rendered_rows = 0usize;
    for (field, items) in lists {
        let is_ip = field.is_ip();
        let tag = tr(
            lang,
            if is_ip {
                "rule_tag_ip"
            } else {
                "rule_tag_domain"
            },
        );
        for (index, item) in items.iter().enumerate() {
            if rendered_rows > 0 {
                list = list.push(
                    container(Space::new())
                        .height(1)
                        .width(Length::Fill)
                        .style(theme::hairline),
                );
            }
            rendered_rows += 1;
            // Quiet until hovered, like the connection table's close glyph.
            let delete = button(
                text(crate::ui::icons::ICON_CLOSE)
                    .font(iced::Font::with_name("Material Icons"))
                    .size(crate::ui::ICON_SIZE),
            )
            .style(theme::button_dismiss)
            .padding(3)
            .on_press(Message::RemoveRule { field, index });
            let room = item_px - tag_w - 4.0;
            let short_item = crate::ui::util::fit_mono(item, room.max(60.0), theme::TYPE_BTN_SM);
            let is_truncated = short_item != *item;
            let item_text = container(
                text(short_item)
                    .size(theme::TYPE_BTN_SM)
                    .font(theme::mono_font())
                    .color(text_primary)
                    .wrapping(text::Wrapping::None),
            )
            .width(Length::Fill)
            .clip(true);
            let item_el: Element<'a, Message> = if is_truncated {
                tooltip(
                    item_text,
                    container(
                        text(item.as_str())
                            .size(theme::TYPE_BTN_SM)
                            .font(theme::mono_font())
                            .color(text_primary),
                    )
                    .padding([6, 10])
                    .style(theme::popover_bg),
                    tooltip::Position::Top,
                )
                .gap(4)
                .into()
            } else {
                item_text.into()
            };
            let tag_pill = container(
                text(tag)
                    .size(theme::TYPE_MICRO)
                    .font(theme::ui_font(iced::font::Weight::Medium))
                    .color(if is_ip {
                        theme::badge_text(theme::INFO, theme)
                    } else {
                        text_muted
                    })
                    .width(Length::Fill)
                    .align_x(Alignment::Center),
            )
            .width(Length::Fixed(tag_w))
            .padding([1, 0])
            .style(move |t| {
                if is_ip {
                    theme::badge_colored(theme::INFO, t)
                } else {
                    theme::badge_bg(t)
                }
            });
            let line = row![item_el, tag_pill, delete]
                .spacing(crate::ui::SP_8)
                .align_y(Alignment::Center);
            list = list.push(
                container(line)
                    .padding(iced::Padding {
                        left: 12.0,
                        right: 6.0,
                        ..Default::default()
                    })
                    .height(RULE_ROW_H)
                    .align_y(Alignment::Center)
                    .width(Length::Fill),
            );
        }
    }
    let list: Element<'a, Message> = if count == 0 {
        container(
            text(tr(lang, "no_custom_rules"))
                .size(theme::TYPE_CAPTION)
                .color(text_tertiary),
        )
        .width(Length::Fill)
        .height(RULE_ROW_H * 2.0 + 1.0)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(theme::status_card)
        .into()
    } else {
        container(list)
            .width(Length::Fill)
            .style(|t| iced::widget::container::Style {
                background: Some(iced::Background::Color(theme::input_surface(t))),
                border: iced::Border {
                    color: theme::border_color(t),
                    width: 1.0,
                    radius: theme::RADIUS_MD.into(),
                },
                ..Default::default()
            })
            .into()
    };

    let mut card_items = column![
        column![
            title_row,
            text(tr(lang, desc_key))
                .size(theme::TYPE_CAPTION)
                .color(text_muted),
        ]
        .spacing(4),
        row![input, add_btn]
            .spacing(crate::ui::SP_8)
            .align_y(Alignment::Center),
    ]
    .spacing(crate::ui::SP_12);

    if let Some(err_key) = validation_err {
        card_items = card_items.push(
            text(tr(lang, err_key))
                .size(theme::TYPE_CAPTION)
                .color(theme::danger_text(theme)),
        );
    }
    card_items = card_items.push(list);

    let mut card = container(card_items)
        .padding(theme::CARD_PAD)
        .width(if is_compact {
            Length::Fill
        } else {
            Length::FillPortion(1)
        })
        .style(theme::card_bg);
    if stretch {
        card = card.height(Length::Fill);
    }
    card.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::GuiConfig;

    #[test]
    fn rules_render_handles_empty_and_populated_lists() {
        let mut cfg = GuiConfig::default();
        let _ = render(&cfg, "", "", &iced::Theme::Light);
        cfg.language = Language::Zh;
        cfg.custom_bypass_domains
            .push("very-long-domain.example.internal".into());
        cfg.custom_proxy_domains
            .push("another-extremely-long-subdomain.example.org".into());
        cfg.custom_proxy_ips.push("1.1.1.1/32".into());
        let _ = render(&cfg, "example.com", "invalid/ip", &iced::Theme::Dark);
        let _ = render(&cfg, "10.0.0.0/8", "bad host!", &iced::Theme::Light);
    }
}
