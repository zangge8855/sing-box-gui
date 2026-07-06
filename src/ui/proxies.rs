use iced::widget::{button, column, container, row, scrollable, text, text_input, Column, Row};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::state::ProxyNode;
use crate::ui::theme;

pub fn render<'a>(
    gui_config: &'a crate::state::GuiConfig,
    nodes: &'a [ProxyNode],
    selected_node: Option<&str>,
    latency_testing: bool,
    search_query: &'a str,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    use crate::ui::i18n::tr;
    
    let text_primary = theme::text_primary(theme);
    let text_muted = theme::text_muted(theme);
    
    // Header controls
    let speed_test_btn = if latency_testing {
        button(text(tr(lang, "testing_latency")).size(14))
            .padding([8, 16])
            .style(theme::button_secondary)
    } else {
        button(text(tr(lang, "test_latency")).size(14))
            .padding([8, 16])
            .style(theme::button_primary)
            .on_press(Message::StartLatencyTest)
    };
    
    let search_input = text_input(tr(lang, "search_nodes_placeholder"), search_query)
        .on_input(Message::NodeSearchChanged)
        .padding(8)
        .width(220)
        .style(theme::input_field);
        
    let header = row![
        text(tr(lang, "proxy_nodes")).size(24).color(text_primary).width(Length::Fill),
        search_input,
        speed_test_btn
    ]
    .spacing(20)
    .align_y(Alignment::Center);
    
    // If no nodes in profile at all, display fallback text
    if nodes.is_empty() {
        return container(
            column![
                header,
                container(
                    text(tr(lang, "no_nodes"))
                        .color(text_muted)
                        .size(15)
                )
                .padding(40)
                .width(Length::Fill)
                .style(theme::card_bg)
            ]
            .spacing(20)
        )
        .padding(20)
        .into();
    }
    
    // Filter nodes by search query (case-insensitive)
    let filtered_nodes: Vec<&ProxyNode> = if search_query.trim().is_empty() {
        nodes.iter().collect()
    } else {
        let q = search_query.to_lowercase();
        nodes.iter()
            .filter(|n| n.name.to_lowercase().contains(&q) || n.server.to_lowercase().contains(&q))
            .collect()
    };
    
    if filtered_nodes.is_empty() {
        return container(
            column![
                header,
                container(
                    text(tr(lang, "no_matching_nodes"))
                        .color(text_muted)
                        .size(15)
                )
                .padding(40)
                .width(Length::Fill)
                .style(theme::card_bg)
            ]
            .spacing(20)
        )
        .padding(20)
        .into();
    }
    
    // Build node cards list
    let mut card_elements: Vec<Element<'a, Message>> = Vec::new();
    
    for node in &filtered_nodes {
        let is_selected = Some(node.name.as_str()) == selected_node;
        
        // Latency text & color
        let latency_text = match node.latency {
            Some(ms) => {
                if ms >= 9999 {
                    text("Timeout").color(theme::DANGER).size(12)
                } else {
                    text(format!("{} ms", ms))
                        .color(if ms < 150 {
                            theme::SUCCESS
                        } else if ms < 300 {
                            theme::WARNING
                        } else {
                            theme::DANGER
                        })
                        .size(12)
                }
            }
            None => text("-").color(text_muted).size(12),
        };
        
        // Custom card drawing as a button content
        let card_content = container(
            column![
                row![
                    text(&node.name).color(text_primary).size(14).width(Length::Fill),
                    latency_text
                ]
                .align_y(Alignment::Center),
                row![
                    text(node.node_type.to_uppercase()).color(text_muted).size(11),
                    text(format!(" {}:{}", node.server, node.port))
                        .color(text_muted)
                        .size(11)
                        .width(Length::Fill)
                ]
                .spacing(5)
            ]
            .spacing(8)
        )
        .padding(15)
        .width(Length::Fill)
        .style(move |theme| {
            if is_selected {
                theme::card_selected(theme)
            } else {
                theme::card_bg(theme)
            }
        });
        
        let card_btn = button(card_content)
            .padding(0)
            .style(move |_theme, status| {
                // Button style matches card styling but reacts to hover
                let base = if is_selected {
                    theme::card_selected(_theme)
                } else {
                    theme::card_bg(_theme)
                };
                
                let border_color = match status {
                    button::Status::Hovered => theme::ACCENT_PURPLE,
                    _ => base.border.color,
                };
                
                let text_color = if theme::is_dark(_theme) { theme::TEXT_PRIMARY } else { theme::TEXT_PRIMARY_LIGHT };
                
                button::Style {
                    background: base.background,
                    text_color,
                    border: iced::Border {
                        color: border_color,
                        width: base.border.width,
                        radius: base.border.radius,
                    },
                    shadow: base.shadow,
                    ..Default::default()
                }
            })
            .on_press(Message::SelectNode(node.name.clone()))
            .width(Length::Fill);
            
        card_elements.push(card_btn.into());
    }
    
    // Group cards into rows of 3 to create a pseudo-grid
    let mut grid_rows = Column::new().spacing(15);
    let mut current_row = Row::new().spacing(15);
    
    for (i, card) in card_elements.into_iter().enumerate() {
        current_row = current_row.push(container(card).width(Length::FillPortion(1)));
        if (i + 1) % 3 == 0 {
            grid_rows = grid_rows.push(current_row);
            current_row = Row::new().spacing(15);
        }
    }
    // Push remaining cards in last row
    let remaining_elements = filtered_nodes.len() % 3;
    if remaining_elements > 0 {
        // Pad the row with empty placeholders to keep grid aligned
        for _ in remaining_elements..3 {
            current_row = current_row.push(container(text("")).width(Length::FillPortion(1)));
        }
        grid_rows = grid_rows.push(current_row);
    }
    
    let scroll_content = scrollable(grid_rows)
        .height(Length::Fill);
        
    container(
        column![
            header,
            scroll_content
        ]
        .spacing(20)
    )
    .padding(20)
    .into()
}
