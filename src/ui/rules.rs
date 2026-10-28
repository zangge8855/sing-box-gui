use crate::message::Message;
use crate::state::{GuiConfig, Language, RuleField, RuleTarget};
use crate::ui::i18n::tr;
use crate::ui::page_header;
use crate::ui::theme;
use iced::widget::{Column, button, column, container, responsive, row, text, text_input};
use iced::{Alignment, Element, Length};

/// Every list row has the same height, so two cards can be matched by count.
const RULE_ROW_H: f32 = 30.0;
const RULE_ROW_GAP: f32 = 2.0;

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
            lang,
            theme,
        );
        let proxy = rule_card(
            RuleTarget::Proxy,
            proxy_input,
            proxy_lists,
            !is_compact && proxy_rows < direct_rows,
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
fn rule_card<'a>(
    target: RuleTarget,
    input_value: &'a str,
    lists: [(RuleField, &'a [String]); 2],
    stretch: bool,
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

    let title_row = row![
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

    let input = text_input(tr(lang, "placeholder_rule"), input_value)
        .on_input(move |value| Message::RulesInputChanged { target, value })
        .on_submit(Message::AddRule { target })
        .padding([8, 10])
        .size(theme::TYPE_BTN_SM)
        .width(Length::Fill)
        .style(theme::input_field);
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
    .style(theme::button_secondary);
    if !input_value.trim().is_empty() {
        add_btn = add_btn.on_press(Message::AddRule { target });
    }

    let mut list = Column::new().spacing(RULE_ROW_GAP).width(Length::Fill);
    for (field, items) in lists {
        let tag = tr(
            lang,
            if field.is_ip() {
                "rule_tag_ip"
            } else {
                "rule_tag_domain"
            },
        );
        for (index, item) in items.iter().enumerate() {
            // Quiet until hovered, like the connection table's close glyph.
            let delete = button(
                text(crate::ui::icons::ICON_DELETE)
                    .font(iced::Font::with_name("Material Icons"))
                    .size(crate::ui::ICON_SIZE),
            )
            .style(theme::button_dismiss)
            .padding(3)
            .on_press(Message::RemoveRule { field, index });
            let line = row![
                container(
                    text(item)
                        .size(theme::TYPE_BTN_SM)
                        .font(theme::mono_font())
                        .color(text_primary)
                        .wrapping(text::Wrapping::None),
                )
                .width(Length::Fill)
                .clip(true),
                text(tag).size(theme::TYPE_MICRO).color(text_tertiary),
                delete,
            ]
            .spacing(crate::ui::SP_12)
            .align_y(Alignment::Center);
            list = list.push(
                container(line)
                    .padding(iced::Padding {
                        left: 10.0,
                        right: 4.0,
                        ..Default::default()
                    })
                    .height(RULE_ROW_H)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                    .style(|t| theme::list_item_style(t, false, false)),
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
        .height(RULE_ROW_H * 2.0 + RULE_ROW_GAP)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into()
    } else {
        list.into()
    };

    let mut card = container(
        column![
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
            list,
        ]
        .spacing(crate::ui::SP_12),
    )
    .padding(theme::CARD_PAD)
    .width(Length::FillPortion(1))
    .style(theme::card_bg);
    if stretch {
        card = card.height(Length::Fill);
    }
    card.into()
}
