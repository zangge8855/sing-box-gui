use iced::widget::{button, column, container, row, scrollable, text, text_input, Column, responsive};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::state::{GuiConfig, RuleField};
use crate::ui::theme;
use crate::ui::page_header;

pub fn render<'a>(
    gui_config: &'a GuiConfig,
    bypass_domain_input: &'a str,
    proxy_domain_input: &'a str,
    bypass_ip_input: &'a str,
    proxy_ip_input: &'a str,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    use crate::ui::i18n::tr;
    
    let theme_cloned = theme.clone();

    let main_content = responsive(move |size| {
        let is_compact = size.width < crate::ui::PAGE_NARROW_W;
        let theme = &theme_cloned;
        let text_primary = theme::text_primary(theme);
        let text_muted = theme::text_muted(theme);

        let builtin_banner = container(
            column![
                text(tr(lang, "rules_builtin_title")).color(text_primary).size(14).font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                }),
                text(tr(lang, "rules_builtin_desc")).color(text_muted).size(12),
            ]
            .spacing(6)
        )
        .padding(16)
        .width(Length::Fill)
        .style(theme::status_card);

        let make_rule_section = |
            title_key: &'static str,
            input_value: &'a str,
            field: RuleField,
            items: &'a [String],
        | {
            let mut list_col = Column::new().spacing(6);
            
            for (idx, item) in items.iter().enumerate() {
                let del_btn = button(
                    text("\u{E5CD}".to_string())
                        .font(iced::Font::with_name("Material Icons"))
                        .size(16),
                )
                    .style(theme::button_secondary)
                    .padding([4, 8])
                    .on_press(Message::RemoveRule {
                        field,
                        index: idx,
                    });
                    
                let item_row = row![
                    text(item).color(text_primary).size(13).width(Length::Fill),
                    del_btn
                ]
                .align_y(Alignment::Center)
                .spacing(10)
                .padding([4, 8]);
                
                list_col = list_col.push(container(item_row).style(move |t| theme::list_item_style(t, false, false)));
            }
            
            let list_content: Element<'_, Message> = if items.is_empty() {
                container(
                    text(tr(lang, "rules_desc"))
                        .color(theme::text_tertiary(theme))
                        .size(12)
                        .align_x(Alignment::Center)
                )
                .width(Length::Fill)
                .height(180.0)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into()
            } else {
                scrollable(list_col)
                    .style(theme::scrollbar_style)
                    .height(180.0)
                    .into()
            };
            
            let placeholder = if field.is_ip() {
                tr(lang, "placeholder_ip")
            } else {
                tr(lang, "placeholder_domain")
            };
            
            let input_box = text_input(placeholder, input_value)
                .on_input(move |s| Message::RulesInputChanged {
                    field,
                    value: s,
                })
                .on_submit(Message::AddRule { field })
                .padding(10)
                .style(theme::input_field);
                
            let add_btn = button(
                text("\u{E145}".to_string())
                    .font(iced::Font::with_name("Material Icons"))
                    .size(18),
            )
                .style(theme::button_primary)
                .padding([9, 14])
                .on_press(Message::AddRule { field });
                
            container(
                column![
                    text(tr(lang, title_key)).color(text_primary).size(theme::TYPE_HEADING).font(iced::Font {
                        weight: iced::font::Weight::Semibold,
                        ..Default::default()
                    }),
                    row![input_box, add_btn].spacing(10).align_y(Alignment::Center),
                    list_content
                ]
                .spacing(12)
            )
            .padding(16)
            .width(Length::Fill)
            .style(theme::card_bg)
        };
        
        let left_column = column![
            make_rule_section(
                "rules_bypass_domains",
                bypass_domain_input,
                RuleField::BypassDomains,
                &gui_config.custom_bypass_domains,
            ),
            make_rule_section(
                "rules_bypass_ips",
                bypass_ip_input,
                RuleField::BypassIps,
                &gui_config.custom_bypass_ips,
            )
        ]
        .spacing(20)
        .width(if is_compact { Length::Fill } else { Length::FillPortion(1) });
        
        let right_column = column![
            make_rule_section(
                "rules_proxy_domains",
                proxy_domain_input,
                RuleField::ProxyDomains,
                &gui_config.custom_proxy_domains,
            ),
            make_rule_section(
                "rules_proxy_ips",
                proxy_ip_input,
                RuleField::ProxyIps,
                &gui_config.custom_proxy_ips,
            )
        ]
        .spacing(20)
        .width(if is_compact { Length::Fill } else { Length::FillPortion(1) });
        
        let rules_layout: Element<'_, Message> = if is_compact {
            column![left_column, right_column].spacing(20).width(Length::Fill).into()
        } else {
            row![left_column, right_column].spacing(20).width(Length::Fill).into()
        };
        
        let header = page_header("tab_rules", lang, None, theme, is_compact);
        
        let content = column![builtin_banner, rules_layout]
            .spacing(20)
            .width(Length::Fill);

        crate::ui::page_shell_with_pad(header, content.into(), is_compact)
    });
    
    main_content.into()
}
