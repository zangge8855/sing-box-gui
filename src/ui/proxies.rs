use iced::widget::{button, column, container, row, scrollable, text, text_input, Column, Row, responsive};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::state::ProxyNode;
use crate::ui::theme;
use crate::ui::{empty_state, page_header, PAGE_COMPACT_W};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProxySortOption {
    sort: crate::state::ProxySort,
    label: String,
}

impl std::fmt::Display for ProxySortOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render<'a>(
    gui_config: &'a crate::state::GuiConfig,
    nodes: &'a [ProxyNode],
    selected_node: Option<&'a str>,
    latency_testing: bool,
    search_query: &'a str,
    proxy_groups: &'a std::collections::HashMap<String, crate::api::ProxyInfo>,
    selected_group: &'a str,
    core_running: bool,
    proxy_sort: crate::state::ProxySort,
    theme: &'a iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    use crate::ui::i18n::tr;
    
    let _text_primary = theme::text_primary(theme);
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
            button(text(tr(lang, "test_latency")).size(theme::TYPE_BTN_MD))
                .padding(theme::BTN_PAD_MD)
                .style(theme::button_secondary)
        } else {
            button(text(tr(lang, "test_latency")).size(theme::TYPE_BTN_MD))
                .padding(theme::BTN_PAD_MD)
                .style(theme::button_primary)
                .on_press(Message::StartLatencyTest)
        };
        
        let search_input = text_input(tr(lang, "search_nodes_placeholder"), search_query)
            .on_input(Message::NodeSearchChanged)
            .padding(8)
            .width(if is_compact { Length::Fill } else { Length::Fixed(theme::SEARCH_WIDTH) })
            .style(theme::input_field);
            
        let sort_options = vec![
            ProxySortOption { sort: crate::state::ProxySort::Latency, label: tr(lang, "sort_latency").to_string() },
            ProxySortOption { sort: crate::state::ProxySort::Name, label: tr(lang, "sort_name").to_string() },
            ProxySortOption { sort: crate::state::ProxySort::Original, label: tr(lang, "sort_original").to_string() },
        ];

        let selected_sort_opt = sort_options.iter()
            .find(|o| o.sort == proxy_sort)
            .cloned()
            .unwrap_or_else(|| sort_options[0].clone());

        let sort_picker = iced::widget::pick_list(
            sort_options,
            Some(selected_sort_opt),
            move |opt| Message::SetProxySort(opt.sort)
        )
        .padding(8)
        .style(theme::pick_list);

        if is_compact {
            column![
                search_input.width(Length::Fill),
                row![
                    text(format!("{}:", tr(lang, "sort_nodes_by"))).size(theme::TYPE_CAPTION).color(text_muted),
                    sort_picker.width(Length::Fill)
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill),
                speed_test_btn.width(Length::Fill)
            ]
            .spacing(8)
            .width(Length::Fill)
            .into()
        } else {
            row![
                search_input,
                text(format!("{}:", tr(lang, "sort_nodes_by"))).size(theme::TYPE_CAPTION).color(text_muted),
                sort_picker.width(Length::Fixed(120.0)),
                speed_test_btn
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .into()
        }
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
        
        let Some(group_info) = proxy_groups.get(group_name) else {
            return empty_state(
                tr(lang, "no_proxy_groups"),
                Some(tr(lang, "core_not_running_hint")),
                None,
                theme,
            );
        };
        
        let groups_moved = groups;
        let selected_group_moved = group_name;
        let group_info_moved = group_info;
        let nodes_moved = nodes;
        let proxy_groups_moved = proxy_groups;
        let search_query_moved = search_query;
        let theme_moved = theme;
        
        let main_content = responsive(move |size| {
            let theme = theme_moved;
            let text_primary = theme::text_primary(theme);
            let _text_muted = theme::text_muted(theme);
            let is_compact = size.width < PAGE_COMPACT_W;
            
            let header_actions = make_header_actions(search_query_moved, is_compact);
            let group_selector = if is_compact {
                let mut groups_row = Row::new().spacing(8);
                for g in &groups_moved {
                    let is_active = g.name == selected_group_moved;
                    let active_node = g.now.as_deref().unwrap_or("-");
                    
                    let g_btn = button(
                        column![
                            text(g.name.clone()).size(theme::TYPE_BTN_SM).font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            }).color(if is_active { theme::ACCENT_PURPLE } else { text_primary }),
                            text(crate::ui::util::truncate_chars(active_node, 16))
                                .size(theme::TYPE_CAPTION)
                                .color(if is_active { theme::ACCENT_PURPLE } else { theme::text_muted(theme) })
                        ]
                        .spacing(2)
                    )
                    .padding(theme::BTN_PAD_SM)
                    .style(theme::button_tab(is_active))
                    .on_press(Message::SelectGroup(g.name.clone()));
                    
                    groups_row = groups_row.push(g_btn);
                }
                
                container(
                    scrollable(groups_row).style(theme::scrollbar_style).direction(scrollable::Direction::Horizontal(Default::default()))
                )
                .width(Length::Fill)
            } else {
                let mut groups_col = Column::new().spacing(10).padding(iced::Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 0.0 });
                for g in &groups_moved {
                    let is_active = g.name == selected_group_moved;
                    let active_node = g.now.as_deref().unwrap_or("-");
                    
                    let g_btn = button(
                        column![
                            text(g.name.clone()).size(theme::TYPE_SECTION).font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            }).color(if is_active { theme::ACCENT_PURPLE } else { text_primary }),
                            text(crate::ui::util::truncate_chars(active_node, 20))
                                .size(theme::TYPE_CAPTION)
                                .color(if is_active { theme::ACCENT_PURPLE } else { theme::text_muted(theme) })
                        ]
                        .spacing(3)
                    )
                    .padding(theme::BTN_PAD_MD)
                    .width(Length::Fill)
                    .style(theme::button_tab(is_active))
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
            
            let right_pane_content: Element<'a, Message> = if let Some(ref sub_nodes) = group_info_moved.all {
                let is_selector = group_info_moved.proxy_type.to_lowercase() == "selector";
                let mut filtered_sub_nodes: Vec<&String> = if search_query_moved.trim().is_empty() {
                    sub_nodes.iter().collect()
                } else {
                    let q = search_query_moved.to_lowercase();
                    sub_nodes.iter()
                        .filter(|n| n.to_lowercase().contains(&q))
                        .collect()
                };

                match proxy_sort {
                    crate::state::ProxySort::Latency => {
                        filtered_sub_nodes.sort_by(|a, b| {
                            let lat = |name: &str| -> Option<u64> {
                                if let Some(n_info) = proxy_groups_moved.get(name)
                                    && let Some(ref hist) = n_info.history
                                        && let Some(last) = hist.last()
                                            && let Some(d) = last.get("delay").and_then(|d| d.as_u64())
                                                && d < 9999 {
                                                    return Some(d);
                                                }
                                nodes_moved.iter().find(|n| n.name == name).and_then(|n| {
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
                    }
                    crate::state::ProxySort::Name => {
                        filtered_sub_nodes.sort();
                    }
                    crate::state::ProxySort::Original => {
                        // Keep original order
                    }
                }

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
                    let total_nodes = filtered_sub_nodes.len();
                    let max_render = 120;
                    let display_nodes = if total_nodes > max_render {
                        &filtered_sub_nodes[..max_render]
                    } else {
                        &filtered_sub_nodes[..]
                    };

                    let mut card_elements: Vec<Element<'a, Message>> = Vec::new();
                    for &node_name in display_nodes {
                        let active = Some(node_name.as_str()) == group_info_moved.now.as_deref();
                        
                        let mut latency = None;
                        let mut node_type = "unknown".to_string();
                        
                        if let Some(n_info) = proxy_groups_moved.get(node_name) {
                            node_type = n_info.proxy_type.clone();
                            if let Some(ref hist) = n_info.history
                                && let Some(last) = hist.last()
                                    && let Some(d) = last.get("delay").and_then(|d| d.as_u64()) {
                                        latency = Some(d);
                                    }
                        } else if let Some(n) = nodes_moved.iter().find(|n| n.name == *node_name) {
                            node_type = n.node_type.clone();
                        }
                        
                        if latency.is_none()
                            && let Some(n) = nodes_moved.iter().find(|n| n.name == *node_name) {
                                latency = n.latency;
                            }
                        
                        let card = render_proxy_card(
                            node_name,
                            &node_type,
                            None, None,
                            latency,
                            active,
                            is_selector,
                            Some(group_name.to_string()),
                            theme,
                            lang,
                        );
                        card_elements.push(card);
                    }

                
                build_card_grid(card_elements, cols, total_nodes, max_render, theme, lang)
            }
            } else {
                // Group has no `all` node list (not a failed search filter).
                crate::ui::empty_state(
                    tr(lang, "no_nodes"),
                    Some(tr(lang, "no_proxy_groups")),
                    None,
                    theme,
                )
            };
            
            let header = page_header("proxy_nodes", lang, Some(header_actions), theme, is_compact);
            
            let body: Element<'a, Message> = if is_compact {
                column![
                    header,
                    group_selector,
                    right_pane_content
                ]
                .spacing(crate::ui::SP_20)
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
            let cta = button(text(tr(lang, "btn_clear_search")).size(theme::TYPE_BTN_MD))
                .padding(theme::BTN_PAD_MD)
                .style(theme::button_secondary)
                .on_press(Message::NodeSearchChanged(String::new()));
            let content = crate::ui::empty_state(
                tr(lang, "no_matching_nodes"),
                None,
                Some(cta.into()),
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
        
        let content = responsive(move |size| {
            let is_compact = size.width < PAGE_COMPACT_W;
            
            let header_actions = make_header_actions(search_query, is_compact);
            
            let total_nodes = filtered_nodes.len();
            let max_render = 120;
            let display_nodes = if total_nodes > max_render {
                &filtered_nodes[..max_render]
            } else {
                &filtered_nodes[..]
            };

            let mut card_elements: Vec<Element<'_, Message>> = Vec::new();
            
            for &node in display_nodes {
                let is_selected = Some(node.name.as_str()) == selected_node;
                
                let card = render_proxy_card(
                    &node.name,
                    &node.node_type,
                    Some(&node.server),
                    Some(node.port),
                    node.latency,
                    is_selected,
                    false,
                    None,
                    theme,
                    lang,
                );
                card_elements.push(card);
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
            
            let grid_content: Element<'_, Message> = build_card_grid(card_elements, cols, total_nodes, max_render, theme, lang);
            
            let header = page_header("proxy_nodes", lang, Some(header_actions), theme, is_compact);
            
            let col = column![header, grid_content]
                .spacing(crate::ui::SP_20)
                .width(Length::Fill)
                .height(Length::Fill);

            crate::ui::page_body_fixed_with_pad(col.into(), is_compact)
        });
            
        content.into()
    }
}


// --- Helper Functions for Deduplication ---

#[allow(clippy::too_many_arguments)]
fn render_proxy_card<'a>(
    node_name: &str,
    node_type: &str,
    server: Option<&'a str>,
    port: Option<u16>,
    latency: Option<u64>,
    is_selected: bool,
    is_selector: bool,
    group_clone: Option<String>,
    theme: &iced::Theme,
    lang: crate::state::Language,
) -> Element<'a, Message> {
    use crate::ui::i18n::tr;
    use iced::widget::{button, row, column, text, container};
    use iced::{Alignment, Length};
    use crate::ui::theme;
    
    let text_primary = if theme::is_dark(theme) { theme::TEXT_PRIMARY_DARK } else { theme::TEXT_PRIMARY_LIGHT };
    let text_muted = theme::text_muted(theme);
    
    let latency_font = theme::metric_font();
    let latency_text = match latency {
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
    
    let mut bottom_row = row![].spacing(crate::ui::SP_8);
    if let (Some(s), Some(p)) = (server, port) {
        bottom_row = bottom_row.push(text(node_type.to_uppercase()).color(text_muted).size(theme::TYPE_CAPTION));
        bottom_row = bottom_row.push(text(format!(" {}:{}", s, p)).color(text_muted).size(theme::TYPE_CAPTION).width(Length::Fill));
    } else {
        let type_tag = container(
            text(node_type.to_uppercase())
                .size(theme::TYPE_MICRO)
                .color(text_muted)
                .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() })
        )
        .padding([2, 6])
        .style(theme::badge_bg);
        bottom_row = bottom_row.push(type_tag);
    }
    
    let card_content = column![
        row![
            text(crate::ui::util::truncate_chars(node_name, 28)).color(text_primary).size(theme::TYPE_BODY).width(Length::Fill),
            latency_text
        ]
        .align_y(Alignment::Center)
        .spacing(8),
        bottom_row
    ]
    .spacing(if server.is_some() { crate::ui::SP_12 } else { crate::ui::SP_8 })
    .padding(theme::CARD_PAD);
    
    let node_clone = node_name.to_string();
    
    let mut card_btn = button(card_content)
        .padding(0)
        .style(move |_theme, status| {
            let base = if is_selected {
                theme::card_selected(_theme)
            } else {
                theme::card_bg(_theme)
            };
            let border_color = match status {
                button::Status::Hovered if is_selector => theme::ACCENT_PURPLE,
                button::Status::Hovered if !is_selector && server.is_some() => theme::ACCENT_PURPLE,
                _ => base.border.color,
            };
            button::Style {
                background: base.background,
                text_color: text_primary,
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
        
    if let Some(gc) = group_clone {
        if is_selector {
            card_btn = card_btn.on_press(Message::SelectGroupNode {
                group: gc,
                node: node_clone,
            });
        }
    } else {
        card_btn = card_btn.on_press(Message::SelectNode(node_clone));
    }
        
    card_btn.into()
}

fn build_card_grid<'a>(
    card_elements: Vec<Element<'a, Message>>,
    cols: usize,
    total_nodes: usize,
    max_render: usize,
    theme: &iced::Theme,
    lang: crate::state::Language,
) -> Element<'a, Message> {
    use iced::widget::{container, text, scrollable, Column, Row};
    use iced::{Alignment, Length};
    use crate::ui::theme;
    use crate::ui::i18n::tr;
    
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
    
    if total_nodes > max_render {
        let more_count = total_nodes - max_render;
        let hint_str = tr(lang, "more_nodes_hint").replace("{}", &more_count.to_string());
        grid_rows = grid_rows.push(
            container(
                text(hint_str)
                    .color(theme::text_muted(theme))
                    .size(theme::TYPE_CAPTION)
            )
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .padding(12)
        );
    }
    
    scrollable(grid_rows).style(theme::scrollbar_style).height(Length::Fill).into()
}

#[cfg(test)]
mod tests {
    /// When a group has no `all` list, the empty-state must not use search-filter framing.
    #[test]
    fn empty_group_all_none_uses_no_nodes_not_search_copy() {
        let src = include_str!("proxies.rs");
        let code = src
            .split("#[cfg(test)]")
            .next()
            .expect("proxies.rs production source");
        // Locate the branch that handles missing `all` (after filtered grid).
        assert!(
            code.contains("Group has no `all` node list"),
            "expected documented empty-all branch"
        );
        // The no-all branch must title with no_nodes (or no_proxy_groups), never only search framing.
        let marker = "Group has no `all` node list";
        let idx = code.find(marker).expect("marker");
        let window = &code[idx..idx.saturating_add(280).min(code.len())];
        assert!(
            window.contains("\"no_nodes\"") || window.contains("\"no_proxy_groups\""),
            "empty-all branch must use no_nodes/no_proxy_groups; window={window}"
        );
        assert!(
            !window.contains("\"no_matching_nodes\""),
            "empty-all branch must not use search framing no_matching_nodes; window={window}"
        );
    }
}
