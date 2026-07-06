use iced::widget::{button, column, container, row, scrollable, text, text_input, Column};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::state::GuiConfig;
use crate::ui::theme;

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
    
    let text_primary = theme::text_primary(theme);
    let text_muted = theme::text_muted(theme);
    
    let title = text(tr(lang, "tab_rules")).size(24).color(text_primary);
    
    let make_rule_section = |
        title_key: &'static str,
        input_value: &'a str,
        field_name: &'static str,
        items: &'a [String],
    | {
        let mut list_col = Column::new().spacing(6);
        
        if items.is_empty() {
            list_col = list_col.push(
                text(tr(lang, "rules_desc"))
                    .color(text_muted)
                    .size(12)
            );
        } else {
            for (idx, item) in items.iter().enumerate() {
                let del_btn = button(text("✕").size(10))
                    .style(theme::button_danger)
                    .padding([3, 6])
                    .on_press(Message::RemoveRule {
                        field: field_name.to_string(),
                        index: idx,
                    });
                    
                let item_row = row![
                    text(item).color(text_primary).size(13).width(Length::Fill),
                    del_btn
                ]
                .align_y(Alignment::Center)
                .spacing(10)
                .padding([4, 8]);
                
                list_col = list_col.push(container(item_row).style(theme::list_item_bg));
            }
        }
        
        let placeholder = if field_name.contains("ip") {
            tr(lang, "placeholder_ip")
        } else {
            tr(lang, "placeholder_domain")
        };
        
        let input_box = text_input(placeholder, input_value)
            .on_input(move |s| Message::RulesInputChanged {
                field: field_name.to_string(),
                value: s,
            })
            .on_submit(Message::AddRule {
                field: field_name.to_string(),
            })
            .padding(10)
            .style(theme::input_field);
            
        let add_btn = button(text("+").size(16))
            .style(theme::button_primary)
            .padding([9, 14])
            .on_press(Message::AddRule {
                field: field_name.to_string(),
            });
            
        container(
            column![
                text(tr(lang, title_key)).color(text_primary).size(14).font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                }),
                row![input_box, add_btn].spacing(10).align_y(Alignment::Center),
                scrollable(list_col).height(Length::Fixed(180.0))
            ]
            .spacing(12)
        )
        .padding(16)
        .width(Length::FillPortion(1))
        .style(theme::card_bg)
    };
    
    let left_column = column![
        make_rule_section(
            "rules_bypass_domains",
            bypass_domain_input,
            "bypass_domains",
            &gui_config.custom_bypass_domains,
        ),
        make_rule_section(
            "rules_bypass_ips",
            bypass_ip_input,
            "bypass_ips",
            &gui_config.custom_bypass_ips,
        )
    ]
    .spacing(20)
    .width(Length::FillPortion(1));
    
    let right_column = column![
        make_rule_section(
            "rules_proxy_domains",
            proxy_domain_input,
            "proxy_domains",
            &gui_config.custom_proxy_domains,
        ),
        make_rule_section(
            "rules_proxy_ips",
            proxy_ip_input,
            "proxy_ips",
            &gui_config.custom_proxy_ips,
        )
    ]
    .spacing(20)
    .width(Length::FillPortion(1));
    
    let rules_row = row![left_column, right_column].spacing(20);
    
    container(
        column![
            title,
            scrollable(rules_row).height(Length::Fill)
        ]
        .spacing(20)
    )
    .padding(20)
    .height(Length::Fill)
    .into()
}
