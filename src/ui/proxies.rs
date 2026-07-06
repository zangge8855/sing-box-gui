use iced::widget::{button, column, container, row, scrollable, text, Column, Row};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::state::ProxyNode;
use crate::ui::theme;

pub fn render<'a>(
    nodes: &'a [ProxyNode],
    selected_node: Option<&str>,
    latency_testing: bool,
) -> Element<'a, Message> {
    
    // Header controls
    let speed_test_btn = if latency_testing {
        button(text("Testing Latency...").size(14))
            .padding([8, 16])
            .style(theme::button_secondary)
    } else {
        button(text("Test All Latency").size(14))
            .padding([8, 16])
            .style(theme::button_primary)
            .on_press(Message::StartLatencyTest)
    };
    
    let header = row![
        text("Proxy Nodes").size(24).color(theme::TEXT_PRIMARY),
        speed_test_btn
    ]
    .spacing(20)
    .align_y(Alignment::Center);
    
    // If no nodes, display fallback text
    if nodes.is_empty() {
        return container(
            column![
                header,
                container(
                    text("No nodes found in the active profile. Please select or update a profile.")
                        .color(theme::TEXT_MUTED)
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
    
    for node in nodes {
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
            None => text("-").color(theme::TEXT_MUTED).size(12),
        };
        
        // Custom card drawing as a button content
        let card_content = container(
            column![
                row![
                    text(&node.name).color(theme::TEXT_PRIMARY).size(14).width(Length::Fill),
                    latency_text
                ]
                .align_y(Alignment::Center),
                row![
                    text(node.node_type.to_uppercase()).color(theme::TEXT_MUTED).size(11),
                    text(format!(" {}:{}", node.server, node.port))
                        .color(theme::TEXT_MUTED)
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
                
                button::Style {
                    background: base.background,
                    text_color: theme::TEXT_PRIMARY,
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
    let remaining_elements = nodes.len() % 3;
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
