use iced::widget::{button, column, container, row, scrollable, text, text_input, Column, Row, responsive};
use iced::{Alignment, Element, Length, Color};
use crate::message::Message;
use crate::state::ProxyNode;
use crate::ui::theme;
use crate::ui::{page_header, PAGE_COMPACT_W};
use crate::ui::util::truncate_chars;

pub fn render<'a>(
    gui_config: &'a crate::state::GuiConfig,
    nodes: &'a [ProxyNode],
    selected_node: Option<&'a str>,
    latency_testing: bool,
    search_query: &'a str,
    proxy_groups: &'a std::collections::HashMap<String, crate::api::ProxyInfo>,
    selected_group: &'a str,
    core_running: bool,
    theme: &'a iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    use crate::ui::i18n::tr;
    
    let text_primary = theme::text_primary(theme);
    let text_muted = theme::text_muted(theme);
    
    let make_header_actions = move |search_query: &str, is_compact: bool| -> Element<'a, Message> {
        // Disabled (no on_press) when core is stopped or already testing
        let speed_test_btn = if latency_testing {
            button(
                crate::ui::loading_row(tr(lang, "testing_latency"), theme)
            )
                .padding(theme::BTN_PAD_MD)
                .style(theme::button_secondary)
        } else if !core_running {
            button(text(tr(lang, "test_latency")).size(14))
                .padding(theme::BTN_PAD_MD)
                .style(theme::button_secondary)
        } else {
            button(text(tr(lang, "test_latency")).size(14))
                .padding(theme::BTN_PAD_MD)
                .style(theme::button_primary)
                .on_press(Message::StartLatencyTest)
        };
        
        let search_input = text_input(tr(lang, "search_nodes_placeholder"), search_query)
            .on_input(Message::NodeSearchChanged)
            .padding(8)
            .width(if is_compact { Length::Fill } else { Length::Fixed(theme::SEARCH_WIDTH) })
            .style(theme::input_field);
            
        row![
            search_input,
            speed_test_btn
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .into()
    };
    
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
        
        let groups_cloned = groups.clone();
        let selected_group_cloned = group_name.to_string();
        let group_info_cloned = group_info.clone();
        let nodes_cloned = nodes.to_vec();
        let proxy_groups_cloned = proxy_groups.clone();
        let search_query_cloned = search_query.to_string();
        let theme_cloned = theme.clone();
        
        let main_content = responsive(move |size| {
            let theme = &theme_cloned;
            let text_primary = theme::text_primary(theme);
            let text_muted = theme::text_muted(theme);
            let is_compact = size.width < PAGE_COMPACT_W;
            
            let header_actions = make_header_actions(&search_query_cloned, is_compact);
            let group_selector = if is_compact {
                let mut groups_row = Row::new().spacing(8);
                for g in &groups_cloned {
                    let is_active = g.name == selected_group_cloned;
                    let active_node = g.now.as_deref().unwrap_or("-");
                    
                    let g_btn = button(
                        column![
                            text(g.name.clone()).size(12).font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            }).color(if is_active { Color::WHITE } else { text_primary }),
                            text(crate::ui::util::truncate_chars(active_node, 16))
                                .size(theme::TYPE_CAPTION)
                                .color(if is_active { Color::WHITE } else { theme::ACCENT_BLUE })
                        ]
                        .spacing(2)
                    )
                    .padding(theme::BTN_PAD_SM)
                    .style(move |t, s| {
                        if is_active {
                            theme::button_primary(t, s)
                        } else {
                            theme::button_secondary(t, s)
                        }
                    })
                    .on_press(Message::SelectGroup(g.name.clone()));
                    
                    groups_row = groups_row.push(g_btn);
                }
                
                container(
                    scrollable(groups_row).style(theme::scrollbar_style).direction(scrollable::Direction::Horizontal(Default::default()))
                )
                .width(Length::Fill)
            } else {
                let mut groups_col = Column::new().spacing(10).padding(iced::Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 0.0 });
                for g in &groups_cloned {
                    let is_active = g.name == selected_group_cloned;
                    let active_node = g.now.as_deref().unwrap_or("-");
                    
                    let g_btn = button(
                        column![
                            text(g.name.clone()).size(13).font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            }).color(if is_active { Color::WHITE } else { text_primary }),
                            text(crate::ui::util::truncate_chars(active_node, 20))
                                .size(theme::TYPE_CAPTION)
                                .color(if is_active { Color::WHITE } else { theme::ACCENT_BLUE })
                        ]
                        .spacing(3)
                    )
                    .padding(theme::BTN_PAD_MD)
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
                
                container(
                    scrollable(groups_col).style(theme::scrollbar_style).height(Length::Fill)
                )
                .width(Length::Fixed(180.0))
                .height(Length::Fill)
            };
            
            let grid_width = if is_compact { size.width } else { size.width - 220.0 };
            let cols = if grid_width < 450.0 {
                1
            } else if grid_width < 700.0 {
                2
            } else if grid_width < 950.0 {
                3
            } else {
                4
            };
            
            let right_pane_content: Element<'a, Message> = if let Some(ref sub_nodes) = group_info_cloned.all {
                let is_selector = group_info_cloned.proxy_type.to_lowercase() == "selector";
                let mut filtered_sub_nodes: Vec<&String> = if search_query_cloned.trim().is_empty() {
                    sub_nodes.iter().collect()
                } else {
                    let q = search_query_cloned.to_lowercase();
                    sub_nodes.iter()
                        .filter(|n| n.to_lowercase().contains(&q))
                        .collect()
                };

                // Sort by latency ascending; missing/timeout last, then name
                filtered_sub_nodes.sort_by(|a, b| {
                    let lat = |name: &str| -> Option<u64> {
                        if let Some(n_info) = proxy_groups_cloned.get(name) {
                            if let Some(ref hist) = n_info.history {
                                if let Some(last) = hist.last() {
                                    if let Some(d) = last.get("delay").and_then(|d| d.as_u64()) {
                                        if d < 9999 {
                                            return Some(d);
                                        }
                                    }
                                }
                            }
                        }
                        nodes_cloned.iter().find(|n| n.name == name).and_then(|n| {
                            n.latency.filter(|&ms| ms < 9999)
                        })
                    };
                    match (lat(a), lat(b)) {
                        (Some(la), Some(lb)) => la.cmp(&lb).then_with(|| a.cmp(b)),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.cmp(b),
                    }
                });

                if filtered_sub_nodes.is_empty() {
                    let cta = button(text(tr(lang, "btn_clear_search")).size(theme::TYPE_BTN_MD))
                        .padding(theme::BTN_PAD_MD)
                        .style(theme::button_secondary)
                        .on_press(Message::NodeSearchChanged(String::new()));
                    crate::ui::empty_state(
                        tr(lang, "no_matching_nodes"),
                        None,
                        Some(cta.into()),
                        theme,
                    )
                } else {
                    let mut card_elements: Vec<Element<'a, Message>> = Vec::new();
                    for node_name in filtered_sub_nodes {
                        let active = Some(node_name.as_str()) == group_info_cloned.now.as_deref();
                        
                        let mut latency = None;
                        let mut node_type = "unknown".to_string();
                        
                        if let Some(n_info) = proxy_groups_cloned.get(node_name) {
                            node_type = n_info.proxy_type.clone();
                            if let Some(ref hist) = n_info.history {
                                if let Some(last) = hist.last() {
                                    if let Some(d) = last.get("delay").and_then(|d| d.as_u64()) {
                                        latency = Some(d);
                                    }
                                }
                            }
                        } else if let Some(n) = nodes_cloned.iter().find(|n| n.name == *node_name) {
                            node_type = n.node_type.clone();
                        }
                        
                        if latency.is_none() {
                            if let Some(n) = nodes_cloned.iter().find(|n| n.name == *node_name) {
                                latency = n.latency;
                            }
                        }
                        
                        let latency_font = iced::Font {
                            family: iced::font::Family::Monospace,
                            weight: iced::font::Weight::Medium,
                            ..Default::default()
                        };
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
                                    text(tr(lang, "latency_timeout"))
                                        .color(theme::DANGER)
                                        .size(theme::TYPE_MONO)
                                        .font(latency_font)
                                } else {
                                    text(format!("{} ms", ms))
                                        .color(col)
                                        .size(theme::TYPE_MONO)
                                        .font(latency_font)
                                }
                            }
                            None => text("-")
                                .color(text_muted)
                                .size(theme::TYPE_MONO)
                                .font(latency_font),
                        };
                        
                        let type_tag = container(
                            text(node_type.to_uppercase())
                                .size(theme::TYPE_MICRO)
                                .color(text_muted)
                                .font(iced::Font {
                                    weight: iced::font::Weight::Bold,
                                    ..Default::default()
                                })
                        )
                        .padding([2, 6])
                        .style(theme::badge_bg);
                        
                        // Inner content: padding only — chrome lives on the outer button
                        let card_content = column![
                            row![
                                text(truncate_chars(node_name, 28))
                                    .color(text_primary)
                                    .size(theme::TYPE_BODY)
                                    .font(iced::Font {
                                        weight: iced::font::Weight::Medium,
                                        ..Default::default()
                                    })
                                    .width(Length::Fill),
                                latency_text
                            ]
                            .align_y(Alignment::Center)
                            .spacing(8),
                            row![type_tag]
                        ]
                        .spacing(8)
                        .padding(theme::CARD_PAD);
                        
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

                    let mut grid_rows = Column::new().spacing(theme::GRID_GAP);
                    let mut current_row = Row::new().spacing(theme::GRID_GAP);
                    let total_cards = card_elements.len();
                    
                    for (i, card) in card_elements.into_iter().enumerate() {
                        current_row = current_row.push(container(card).width(Length::FillPortion(1)));
                        if (i + 1) % cols == 0 {
                            grid_rows = grid_rows.push(current_row);
                            current_row = Row::new().spacing(15);
                        }
                    }
                    let remaining_elements = total_cards % cols;
                    if remaining_elements > 0 {
                        for _ in remaining_elements..cols {
                            current_row = current_row.push(container(text("")).width(Length::FillPortion(1)));
                        }
                        grid_rows = grid_rows.push(current_row);
                    }
                    scrollable(grid_rows).style(theme::scrollbar_style).height(Length::Fill).into()
                }
            } else {
                container(
                    text(tr(lang, "no_matching_nodes"))
                        .color(text_muted)
                        .size(15)
                )
                .padding(40)
                .width(Length::Fill)
                .style(theme::card_bg)
                .into()
            };
            
            let header = page_header("proxy_nodes", lang, Some(header_actions), theme, is_compact);
            
            let body: Element<'a, Message> = if is_compact {
                column![
                    header,
                    group_selector,
                    right_pane_content
                ]
                .spacing(15)
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
            } else {
                let divider = container(iced::widget::Space::new())
                    .width(1)
                    .height(Length::Fill)
                    .style(|t| container::Style {
                        background: Some(iced::Background::Color(theme::border_color(t))),
                        ..Default::default()
                    });
                    
                let col_content = row![
                    group_selector,
                    divider,
                    column![right_pane_content].spacing(20).width(Length::Fill).height(Length::Fill)
                ]
                .spacing(20)
                .height(Length::Fill)
                .width(Length::Fill);

                column![header, col_content]
                    .spacing(20)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            };

            crate::ui::page_body_fixed_with_pad(body, is_compact)
        });
        
        main_content.into()
        
    } else {
        // Fallback: simple flat node list when core is not running / no groups
        if nodes.is_empty() {
            let cta = if !core_running {
                Some(
                    button(text(tr(lang, "btn_start_core_short")).size(theme::TYPE_BTN_MD))
                        .padding(theme::BTN_PAD_MD)
                        .style(theme::button_primary)
                        .on_press(Message::ToggleCore)
                        .into(),
                )
            } else {
                None
            };
            let content = crate::ui::empty_state(
                tr(lang, "no_nodes"),
                Some(if !core_running {
                    tr(lang, "core_not_running_hint")
                } else {
                    tr(lang, "no_proxy_groups")
                }),
                cta,
                theme,
            );
            let header = page_header(
                "proxy_nodes",
                lang,
                Some(make_header_actions(search_query, true)),
                theme,
                true,
            );
            return crate::ui::page_shell_fixed_with_pad(header, content, true);
        }
        
        let mut filtered_nodes: Vec<&ProxyNode> = if search_query.trim().is_empty() {
            nodes.iter().collect()
        } else {
            let q = search_query.to_lowercase();
            nodes.iter()
                .filter(|n| n.name.to_lowercase().contains(&q) || n.server.to_lowercase().contains(&q))
                .collect()
        };

        filtered_nodes.sort_by(|a, b| {
            match (a.latency.filter(|&ms| ms < 9999), b.latency.filter(|&ms| ms < 9999)) {
                (Some(la), Some(lb)) => la.cmp(&lb).then_with(|| a.name.cmp(&b.name)),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.name.cmp(&b.name),
            }
        });
        
        if filtered_nodes.is_empty() {
            let content: Element<'a, Message> = container(
                text(tr(lang, "no_matching_nodes"))
                    .color(text_muted)
                    .size(15)
            )
            .padding(40)
            .width(Length::Fill)
            .style(theme::card_bg)
            .into();
            let header = page_header(
                "proxy_nodes",
                lang,
                Some(make_header_actions(search_query, true)),
                theme,
                true,
            );
            return crate::ui::page_shell_fixed_with_pad(header, content, true);
        }
        
        let content = responsive(move |size| {
            let is_compact = size.width < PAGE_COMPACT_W;
            
            let header_actions = make_header_actions(search_query, is_compact);
            
            let mut card_elements: Vec<Element<'_, Message>> = Vec::new();
            
            for node in &filtered_nodes {
                let is_selected = Some(node.name.as_str()) == selected_node;
                
                let latency_font = iced::Font {
                    family: iced::font::Family::Monospace,
                    weight: iced::font::Weight::Medium,
                    ..Default::default()
                };
                let latency_text = match node.latency {
                    Some(ms) => {
                        if ms >= 9999 {
                            text(tr(lang, "latency_timeout")).color(theme::DANGER).size(theme::TYPE_MONO).font(latency_font)
                        } else {
                            text(format!("{} ms", ms))
                                .color(if ms < 150 {
                                    theme::SUCCESS
                                } else if ms < 300 {
                                    theme::WARNING
                                } else {
                                    theme::DANGER
                                  })
                                .size(theme::TYPE_MONO)
                                .font(latency_font)
                        }
                    }
                    None => text("-").color(text_muted).size(theme::TYPE_MONO).font(latency_font),
                };
                
                let card_content = column![
                    row![
                        text(truncate_chars(&node.name, 28)).color(text_primary).size(theme::TYPE_BODY).width(Length::Fill),
                        latency_text
                    ]
                    .align_y(Alignment::Center)
                    .spacing(8),
                    row![
                        text(node.node_type.to_uppercase()).color(text_muted).size(theme::TYPE_CAPTION),
                        text(format!(" {}:{}", node.server, node.port))
                            .color(text_muted)
                            .size(theme::TYPE_CAPTION)
                            .width(Length::Fill)
                    ]
                    .spacing(5)
                ]
                .spacing(10)
                .padding(theme::CARD_PAD);
                
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

            // Calculate columns based on width
            let cols = if size.width < 500.0 {
                1
            } else if size.width < 750.0 {
                2
            } else if size.width < 1000.0 {
                3
            } else {
                4
            };
            
            let mut grid_rows = Column::new().spacing(theme::GRID_GAP);
            let mut current_row = Row::new().spacing(theme::GRID_GAP);
            let total_cards = card_elements.len();
            
            for (i, card) in card_elements.into_iter().enumerate() {
                current_row = current_row.push(container(card).width(Length::FillPortion(1)));
                if (i + 1) % cols == 0 {
                    grid_rows = grid_rows.push(current_row);
                    current_row = Row::new().spacing(theme::GRID_GAP);
                }
            }
            let remaining_elements = total_cards % cols;
            if remaining_elements > 0 {
                for _ in remaining_elements..cols {
                    current_row = current_row.push(container(text("")).width(Length::FillPortion(1)));
                }
                grid_rows = grid_rows.push(current_row);
            }
            
            let grid_content: Element<'_, Message> = scrollable(grid_rows).style(theme::scrollbar_style).height(Length::Fill).into();
            
            let header = page_header("proxy_nodes", lang, Some(header_actions), theme, is_compact);
            
            let col = column![header, grid_content]
                .spacing(20)
                .width(Length::Fill)
                .height(Length::Fill);

            crate::ui::page_body_fixed_with_pad(col.into(), is_compact)
        });
            
        content.into()
    }
}
