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
    proxy_groups: &'a std::collections::HashMap<String, crate::api::ProxyInfo>,
    selected_group: &'a str,
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
    
    // Check if we have active groups from Clash API
    let mut groups: Vec<&crate::api::ProxyInfo> = proxy_groups.values()
        .filter(|p| p.all.is_some() && p.now.is_some())
        .collect();
        
    if !groups.is_empty() {
        // Sort groups (e.g. Proxy first, then alphabetical)
        groups.sort_by(|a, b| {
            if a.name == "Proxy" {
                std::cmp::Ordering::Less
            } else if b.name == "Proxy" {
                std::cmp::Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        });
        
        // Find selected group
        let group_name = if proxy_groups.contains_key(selected_group) {
            selected_group
        } else {
            &groups[0].name
        };
        
        let group_info = proxy_groups.get(group_name).unwrap();
        
        // Left Column: Groups List
        let mut groups_col = Column::new().spacing(10);
        for g in &groups {
            let is_active = g.name == group_name;
            let active_node = g.now.as_deref().unwrap_or("-");
            
            let g_btn = button(
                column![
                    text(&g.name).size(13).font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    }).color(if is_active { Color::WHITE } else { text_primary }),
                    text(active_node).size(10).color(if is_active { Color::WHITE } else { theme::ACCENT_BLUE })
                ]
                .spacing(3)
            )
            .padding([10, 14])
            .width(Length::Fill)
            .style(move |t, s| {
                if is_active {
                    theme::button_primary(t, s)
                } else {
                    theme::button_secondary(t, s)
                }
            })
            .on_press(Message::SelectGroup(g.name.clone()));
            
            groups_col = groups_col.push(g_btn);
        }
        
        let left_pane = container(
            scrollable(groups_col).height(Length::Fill)
        )
        .width(Length::Fixed(180.0))
        .height(Length::Fill);
        
        // Right Column: Nodes Grid of Selected Group
        let mut card_elements: Vec<Element<'a, Message>> = Vec::new();
        
        if let Some(ref sub_nodes) = group_info.all {
            let is_selector = group_info.proxy_type.to_lowercase() == "selector";
            let filtered_sub_nodes: Vec<&String> = if search_query.trim().is_empty() {
                sub_nodes.iter().collect()
            } else {
                let q = search_query.to_lowercase();
                sub_nodes.iter()
                    .filter(|n| n.to_lowercase().contains(&q))
                    .collect()
            };
            
            for node_name in filtered_sub_nodes {
                let active = Some(node_name.as_str()) == group_info.now.as_deref();
                
                // Find node latency and type
                let mut latency = None;
                let mut node_type = "unknown".to_string();
                
                if let Some(n_info) = proxy_groups.get(node_name) {
                    node_type = n_info.proxy_type.clone();
                    if let Some(ref hist) = n_info.history {
                        if let Some(last) = hist.last() {
                            if let Some(d) = last.get("delay").and_then(|d| d.as_u64()) {
                                latency = Some(d);
                            }
                        }
                    }
                } else if let Some(n) = nodes.iter().find(|n| n.name == *node_name) {
                    node_type = n.node_type.clone();
                }
                
                if latency.is_none() {
                    if let Some(n) = nodes.iter().find(|n| n.name == *node_name) {
                        latency = n.latency;
                    }
                }
                
                let latency_text = match latency {
                    Some(ms) => {
                        let col = if ms < 150 {
                            theme::SUCCESS
                        } else if ms < 300 {
                            theme::WARNING
                        } else {
                            theme::DANGER
                        };
                        
                        if ms >= 9999 {
                            text("Timeout")
                                .color(theme::DANGER)
                                .size(11)
                                .font(iced::Font {
                                    weight: iced::font::Weight::Bold,
                                    ..Default::default()
                                })
                        } else {
                            text(format!("{} ms", ms))
                                .color(col)
                                .size(11)
                                .font(iced::Font {
                                    weight: iced::font::Weight::Bold,
                                    ..Default::default()
                                })
                        }
                    }
                    None => text("-")
                        .color(text_muted)
                        .size(11)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        }),
                };
                
                let type_tag = container(
                    text(node_type.to_uppercase())
                        .size(9)
                        .color(text_muted)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        })
                )
                .padding([2, 6])
                .style(move |t| {
                    let bg_color = if theme::is_dark(t) {
                        Color::from_rgb(0.14, 0.17, 0.22)
                    } else {
                        Color::from_rgb(0.92, 0.94, 0.97)
                    };
                    container::Style {
                        background: Some(iced::Background::Color(bg_color)),
                        border: iced::Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                });
                
                let card_content = container(
                    column![
                        row![
                            text(node_name)
                                .color(text_primary)
                                .size(14)
                                .font(iced::Font {
                                    weight: iced::font::Weight::Medium,
                                    ..Default::default()
                                })
                                .width(Length::Fill),
                            latency_text
                        ]
                        .align_y(Alignment::Center),
                        row![
                            type_tag
                        ]
                    ]
                    .spacing(6)
                )
                .padding(15)
                .width(Length::Fill)
                .style(move |t| {
                    if active {
                        theme::card_selected(t)
                    } else {
                        theme::card_bg(t)
                    }
                });
                
                let group_clone = group_name.to_string();
                let node_clone = node_name.clone();
                
                let mut card_btn = button(card_content)
                    .padding(0)
                    .style(move |t, s| {
                        let base = if active {
                            theme::card_selected(t)
                        } else {
                            theme::card_bg(t)
                        };
                        let border_color = match s {
                            button::Status::Hovered if is_selector => theme::ACCENT_PURPLE,
                            _ => base.border.color,
                        };
                        button::Style {
                            background: base.background,
                            text_color: if theme::is_dark(t) { theme::TEXT_PRIMARY } else { theme::TEXT_PRIMARY_LIGHT },
                            border: iced::Border {
                                color: border_color,
                                width: base.border.width,
                                radius: base.border.radius,
                            },
                            shadow: base.shadow,
                            ..Default::default()
                        }
                    })
                    .width(Length::Fill);
                    
                if is_selector {
                    card_btn = card_btn.on_press(Message::SelectGroupNode {
                        group: group_clone,
                        node: node_clone,
                    });
                }
                    
                card_elements.push(card_btn.into());
            }
        }
        
        let right_grid: Element<'a, Message> = if card_elements.is_empty() {
            container(
                text(tr(lang, "no_matching_nodes"))
                    .color(text_muted)
                    .size(15)
            )
            .padding(40)
            .width(Length::Fill)
            .style(theme::card_bg)
            .into()
        } else {
            let mut grid_rows = Column::new().spacing(15);
            let mut current_row = Row::new().spacing(15);
            let total_cards = card_elements.len();
            
            for (i, card) in card_elements.into_iter().enumerate() {
                current_row = current_row.push(container(card).width(Length::FillPortion(1)));
                if (i + 1) % 3 == 0 {
                    grid_rows = grid_rows.push(current_row);
                    current_row = Row::new().spacing(15);
                }
            }
            let remaining_elements = total_cards % 3;
            if remaining_elements > 0 {
                for _ in remaining_elements..3 {
                    current_row = current_row.push(container(text("")).width(Length::FillPortion(1)));
                }
                grid_rows = grid_rows.push(current_row);
            }
            scrollable(grid_rows).height(Length::Fill).into()
        };
        
        let right_pane = column![
            header,
            right_grid
        ]
        .spacing(20)
        .width(Length::Fill)
        .height(Length::Fill);
        
        let divider = container(iced::widget::Space::new())
            .width(1)
            .height(Length::Fill)
            .style(|theme| container::Style {
                background: Some(iced::Background::Color(if theme::is_dark(theme) {
                    theme::BORDER_DARK
                } else {
                    theme::BORDER_LIGHT
                })),
                ..Default::default()
            });

        container(
            row![
                left_pane,
                divider,
                right_pane
            ]
            .spacing(20)
            .height(Length::Fill)
            .width(Length::Fill)
        )
        .padding(20)
        .into()
        
    } else {
        // Fallback: simple flat node list when core is not running
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
        
        let mut card_elements: Vec<Element<'a, Message>> = Vec::new();
        
        for node in &filtered_nodes {
            let is_selected = Some(node.name.as_str()) == selected_node;
            
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
                        text_color: if theme::is_dark(_theme) { theme::TEXT_PRIMARY } else { theme::TEXT_PRIMARY_LIGHT },
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
        
        let mut grid_rows = Column::new().spacing(15);
        let mut current_row = Row::new().spacing(15);
        
        for (i, card) in card_elements.into_iter().enumerate() {
            current_row = current_row.push(container(card).width(Length::FillPortion(1)));
            if (i + 1) % 3 == 0 {
                grid_rows = grid_rows.push(current_row);
                current_row = Row::new().spacing(15);
            }
        }
        let remaining_elements = filtered_nodes.len() % 3;
        if remaining_elements > 0 {
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
}

// Add a dummy Color type wrapper since we used iced Color
use iced::Color;
