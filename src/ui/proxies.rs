use crate::message::Message;
use crate::state::ProxyNode;
use crate::ui::theme;
use crate::ui::{PAGE_COMPACT_W, empty_state, page_header};
use iced::widget::{
    Column, Row, Space, button, column, container, responsive, row, scrollable, text, text_input,
};
use iced::{Alignment, Element, Length};

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
    let protocol_count = nodes
        .iter()
        .map(|node| node.node_type.to_ascii_lowercase())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    // name → (type, latency, selectable, server). Server comes only from the
    // parsed profile; API-known groups without a profile node carry None.
    type NodeMeta = (String, Option<u64>, bool, Option<String>);
    let mut node_meta_index: std::collections::HashMap<String, NodeMeta> = nodes
        .iter()
        .map(|node| {
            (
                node.name.clone(),
                (
                    node.node_type.clone(),
                    node.latency,
                    node.selectable,
                    Some(node.server.clone()),
                ),
            )
        })
        .collect();
    for (name, info) in proxy_groups {
        let api_latency = info
            .history
            .as_ref()
            .and_then(|history| history.last())
            .and_then(|sample| sample.get("delay"))
            .and_then(|delay| delay.as_u64())
            .filter(|delay| *delay < 9999);
        let entry = node_meta_index
            .entry(name.clone())
            .or_insert_with(|| (info.proxy_type.clone(), None, true, None));
        entry.0 = info.proxy_type.clone();
        if api_latency.is_some() {
            entry.1 = api_latency;
        }
    }

    let make_header_actions = move |search_query: &str, is_compact: bool| -> Element<'a, Message> {
        // Disabled (no on_press) when core is stopped or already testing
        let speed_test_btn = if latency_testing {
            button(crate::ui::loading_row(tr(lang, "testing_latency"), theme))
                .padding(theme::BTN_PAD_TOOLBAR)
                .style(theme::button_secondary)
        } else if !core_running {
            button(text(tr(lang, "test_latency")).size(theme::TYPE_BTN_MD))
                .padding(theme::BTN_PAD_TOOLBAR)
                .style(theme::button_secondary)
        } else {
            button(text(tr(lang, "test_latency")).size(theme::TYPE_BTN_MD))
                .padding(theme::BTN_PAD_TOOLBAR)
                .style(theme::button_primary)
                .on_press(Message::StartLatencyTest)
        };

        let search_input = text_input(tr(lang, "search_nodes_placeholder"), search_query)
            .on_input(Message::NodeSearchChanged)
            .padding(8)
            .width(if is_compact {
                Length::Fill
            } else {
                Length::Fixed(theme::SEARCH_WIDTH)
            })
            .size(theme::TYPE_BODY)
            .style(theme::input_field);

        let sort_options = vec![
            ProxySortOption {
                sort: crate::state::ProxySort::Latency,
                label: tr(lang, "sort_latency").to_string(),
            },
            ProxySortOption {
                sort: crate::state::ProxySort::Name,
                label: tr(lang, "sort_name").to_string(),
            },
            ProxySortOption {
                sort: crate::state::ProxySort::Original,
                label: tr(lang, "sort_original").to_string(),
            },
        ];

        let selected_sort_opt = sort_options
            .iter()
            .find(|o| o.sort == proxy_sort)
            .cloned()
            .unwrap_or_else(|| sort_options[0].clone());

        let sort_picker =
            iced::widget::pick_list(sort_options, Some(selected_sort_opt), move |opt| {
                Message::SetProxySort(opt.sort)
            })
            .padding(8)
            .text_size(theme::TYPE_BODY)
            .style(theme::pick_list)
            .menu_style(theme::pick_list_menu);

        if is_compact {
            column![
                search_input.width(Length::Fill),
                row![
                    text(format!("{}:", tr(lang, "sort_nodes_by")))
                        .size(theme::TYPE_CAPTION)
                        .color(text_muted),
                    sort_picker.width(Length::Fill),
                    speed_test_btn
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill)
            ]
            .spacing(8)
            .width(Length::Fill)
            .into()
        } else {
            row![
                search_input,
                text(format!("{}:", tr(lang, "sort_nodes_by")))
                    .size(theme::TYPE_CAPTION)
                    .color(text_muted),
                sort_picker.width(Length::Fixed(120.0)),
                speed_test_btn
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .into()
        }
    };

    // Check if we have active groups from Clash API
    let mut groups: Vec<&crate::api::ProxyInfo> =
        proxy_groups.values().filter(|p| p.all.is_some()).collect();

    if !groups.is_empty() {
        // Sort groups (e.g. Proxy first, then alphabetical)
        groups.sort_by(|a, b| {
            (b.name == "Proxy")
                .cmp(&(a.name == "Proxy"))
                .then_with(|| a.name.cmp(&b.name))
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
        let node_meta_index_moved = node_meta_index;
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
                            text(g.name.clone())
                                .size(theme::TYPE_BTN_SM)
                                .font(theme::ui_font(iced::font::Weight::Bold))
                                .color(if is_active {
                                    theme::accent_text(theme)
                                } else {
                                    text_primary
                                }),
                            text(crate::ui::util::truncate_chars(active_node, 16))
                                .size(theme::TYPE_CAPTION)
                                .color(if is_active {
                                    theme::accent_text(theme)
                                } else {
                                    theme::text_muted(theme)
                                })
                        ]
                        .spacing(2),
                    )
                    .padding(theme::BTN_PAD_SM)
                    .style(theme::button_tab(is_active))
                    .on_press(Message::SelectGroup(g.name.clone()));

                    groups_row = groups_row.push(g_btn);
                }

                container(
                    scrollable(groups_row)
                        .style(theme::scrollbar_style)
                        .direction(scrollable::Direction::Horizontal(Default::default())),
                )
                .width(Length::Fill)
            } else {
                let mut groups_col = Column::new().spacing(10).padding(iced::Padding {
                    top: 0.0,
                    right: 10.0,
                    bottom: 0.0,
                    left: 0.0,
                });
                for g in &groups_moved {
                    let is_active = g.name == selected_group_moved;
                    let active_node = g.now.as_deref().unwrap_or("-");

                    let g_btn = button(
                        column![
                            text(g.name.clone())
                                .size(theme::TYPE_SECTION)
                                .font(theme::ui_font(iced::font::Weight::Bold))
                                .color(if is_active {
                                    theme::accent_text(theme)
                                } else {
                                    text_primary
                                }),
                            text(crate::ui::util::truncate_chars(active_node, 20))
                                .size(theme::TYPE_CAPTION)
                                .color(if is_active {
                                    theme::accent_text(theme)
                                } else {
                                    theme::text_muted(theme)
                                })
                        ]
                        .spacing(3),
                    )
                    .padding(theme::BTN_PAD_MD)
                    .width(Length::Fill)
                    .style(theme::button_tab(is_active))
                    .on_press(Message::SelectGroup(g.name.clone()));

                    groups_col = groups_col.push(g_btn);
                }

                container(
                    scrollable(groups_col)
                        .style(theme::scrollbar_style)
                        .height(Length::Fill),
                )
                .width(Length::Fixed(180.0))
                .height(Length::Fill)
            };

            let grid_width = crate::ui::page_content_width(size.width, is_compact)
                - if is_compact { 0.0 } else { 221.0 };
            let cols = card_columns(grid_width);
            let card_w = card_width(grid_width, cols);

            let right_pane_content: Element<'a, Message> = if let Some(ref sub_nodes) =
                group_info_moved.all
            {
                let is_selector = group_info_moved.proxy_type.to_lowercase() == "selector";
                let mut filtered_sub_nodes: Vec<&String> = if search_query_moved.trim().is_empty() {
                    sub_nodes.iter().collect()
                } else {
                    let q = search_query_moved.trim().to_lowercase();
                    sub_nodes
                        .iter()
                        .filter(|n| {
                            // Match server too — the placeholder promises
                            // "by name or server", like the flat list below.
                            n.to_lowercase().contains(&q)
                                || node_meta_index_moved
                                    .get(n.as_str())
                                    .and_then(|meta| meta.3.as_deref())
                                    .is_some_and(|server| server.to_lowercase().contains(&q))
                        })
                        .collect()
                };

                match proxy_sort {
                    crate::state::ProxySort::Latency => {
                        filtered_sub_nodes.sort_by(|a, b| {
                            let lat = |name: &str| {
                                node_meta_index_moved
                                    .get(name)
                                    .and_then(|metadata| metadata.1.filter(|ms| *ms < 9999))
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

                        let (node_type, latency, selectable, _server) = node_meta_index_moved
                            .get(node_name)
                            .cloned()
                            .unwrap_or_else(|| ("unknown".to_string(), None, true, None));

                        let card = render_proxy_card(
                            node_name,
                            &node_type,
                            None,
                            None,
                            latency,
                            active,
                            is_selector,
                            selectable,
                            Some(group_name.to_string()),
                            card_w,
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
            let summary = render_proxy_summary(
                nodes_moved.len(),
                protocol_count,
                groups_moved.len(),
                core_running,
                theme,
                lang,
            );
            let heading = column![header, summary].spacing(crate::ui::SP_8);

            let body: Element<'a, Message> = if is_compact {
                column![heading, group_selector, right_pane_content]
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
                    column![right_pane_content]
                        .spacing(20)
                        .width(Length::Fill)
                        .height(Length::Fill)
                ]
                .spacing(20)
                .height(Length::Fill)
                .width(Length::Fill);

                column![heading, col_content]
                    .spacing(crate::ui::SP_20)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            };

            crate::ui::page_body_fixed_with_pad(body, is_compact)
        });

        main_content.into()
    } else {
        // Fallback: simple flat node list when core is not running / no groups
        // Empty states follow the window like the populated page, so the
        // title and insets don't jump when nodes arrive.
        if nodes.is_empty() {
            return responsive(move |size| {
                let is_compact = size.width < PAGE_COMPACT_W;
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
                    Some(make_header_actions(search_query, is_compact)),
                    theme,
                    is_compact,
                );
                crate::ui::page_shell_fixed_with_pad(header, content, is_compact)
            })
            .into();
        }

        let mut filtered_nodes: Vec<&ProxyNode> = if search_query.trim().is_empty() {
            nodes.iter().collect()
        } else {
            let q = search_query.trim().to_lowercase();
            nodes
                .iter()
                .filter(|n| {
                    n.name.to_lowercase().contains(&q) || n.server.to_lowercase().contains(&q)
                })
                .collect()
        };

        match proxy_sort {
            crate::state::ProxySort::Latency => filtered_nodes.sort_by(|a, b| {
                match (
                    a.latency.filter(|&ms| ms < 9999),
                    b.latency.filter(|&ms| ms < 9999),
                ) {
                    (Some(la), Some(lb)) => la.cmp(&lb).then_with(|| a.name.cmp(&b.name)),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.name.cmp(&b.name),
                }
            }),
            crate::state::ProxySort::Name => filtered_nodes.sort_by(|a, b| a.name.cmp(&b.name)),
            crate::state::ProxySort::Original => {}
        }

        if filtered_nodes.is_empty() {
            return responsive(move |size| {
                let is_compact = size.width < PAGE_COMPACT_W;
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
                    Some(make_header_actions(search_query, is_compact)),
                    theme,
                    is_compact,
                );
                crate::ui::page_shell_fixed_with_pad(header, content, is_compact)
            })
            .into();
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

            let grid_width = crate::ui::page_content_width(size.width, is_compact);
            let cols = card_columns(grid_width);
            let card_w = card_width(grid_width, cols);
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
                    node.selectable,
                    None,
                    card_w,
                    theme,
                    lang,
                );
                card_elements.push(card);
            }

            let grid_content: Element<'_, Message> =
                build_card_grid(card_elements, cols, total_nodes, max_render, theme, lang);

            let header = page_header("proxy_nodes", lang, Some(header_actions), theme, is_compact);
            let summary =
                render_proxy_summary(total_nodes, protocol_count, 0, core_running, theme, lang);

            let heading = column![header, summary].spacing(crate::ui::SP_8);
            let col = column![heading, grid_content]
                .spacing(crate::ui::SP_20)
                .width(Length::Fill)
                .height(Length::Fill);

            crate::ui::page_body_fixed_with_pad(col.into(), is_compact)
        });

        content.into()
    }
}

fn render_proxy_summary<'a>(
    node_count: usize,
    protocol_count: usize,
    group_count: usize,
    core_running: bool,
    theme: &iced::Theme,
    lang: crate::state::Language,
) -> Element<'a, Message> {
    use crate::ui::i18n::tr;

    let status_color = if core_running {
        theme::success_text(theme)
    } else {
        theme::text_tertiary(theme)
    };
    let status = crate::ui::status_dot(
        status_color,
        core_running,
        if core_running {
            tr(lang, "status_running")
        } else {
            tr(lang, "status_stopped")
        },
        status_color,
        theme::TYPE_CAPTION,
    );

    // Figures in ink, units in grey — quieter than a "·"-joined string.
    let figure = |value: usize, unit_key: &'static str| {
        row![
            text(value.to_string())
                .size(theme::TYPE_CAPTION)
                .font(theme::metric_font())
                .color(theme::text_primary(theme)),
            text(tr(lang, unit_key))
                .size(theme::TYPE_CAPTION)
                .color(theme::text_muted(theme)),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
    };

    let mut summary = row![
        status,
        figure(node_count, "nodes_unit"),
        figure(protocol_count, "protocols_unit"),
    ]
    .spacing(crate::ui::SP_20)
    .align_y(Alignment::Center);
    if group_count > 0 {
        summary = summary.push(figure(group_count, "groups_unit"));
    }
    summary.into()
}

// --- Helper Functions for Deduplication ---

/// Every card shares one height so grid rows line up regardless of name length.
const PROXY_CARD_HEIGHT: f32 = 84.0;
const PROXY_CARD_MIN_W: f32 = 236.0;

/// Width of one grid card for `cols` columns spread across `grid_width`.
fn card_width(grid_width: f32, cols: usize) -> f32 {
    let cols = cols.max(1) as f32;
    ((grid_width - (cols - 1.0) * theme::GRID_GAP) / cols).max(PROXY_CARD_MIN_W)
}

#[allow(clippy::too_many_arguments)]
fn render_proxy_card<'a>(
    node_name: &str,
    node_type: &str,
    server: Option<&'a str>,
    port: Option<u16>,
    latency: Option<u64>,
    is_selected: bool,
    is_selector: bool,
    direct_selectable: bool,
    group_clone: Option<String>,
    card_w: f32,
    theme: &iced::Theme,
    lang: crate::state::Language,
) -> Element<'a, Message> {
    use crate::ui::i18n::tr;
    use crate::ui::theme;
    use crate::ui::util::truncate_width;
    use iced::widget::{button, column, container, row, text, tooltip};
    use iced::{Alignment, Length};

    let text_primary = theme::text_primary(theme);
    let pad = theme::CARD_PAD_DENSE;

    // Latency: a small semantic dot plus the figure, right-aligned.
    let (latency_label, latency_color) = match latency {
        Some(ms) if ms >= 9999 => (
            tr(lang, "latency_timeout").to_string(),
            theme::danger_text(theme),
        ),
        Some(ms) => (
            format!("{ms} ms"),
            if ms < 100 {
                theme::success_text(theme)
            } else if ms < 250 {
                theme::warning_text(theme)
            } else {
                theme::danger_text(theme)
            },
        ),
        None => ("—".to_string(), theme::text_tertiary(theme)),
    };
    let latency_el = row![
        container(Space::new())
            .width(6)
            .height(6)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(theme::with_alpha(
                    latency_color,
                    if latency.is_some() { 1.0 } else { 0.5 },
                ))),
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        text(latency_label)
            .size(theme::TYPE_CAPTION)
            .font(theme::metric_font())
            .color(latency_color),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let proto_color = theme::protocol_color(node_type);
    let proto_label = node_type.to_uppercase();
    let type_tag = container(
        text(truncate_width(&proto_label, 12))
            .size(theme::TYPE_MICRO)
            .color(theme::badge_text(proto_color, theme))
            .font(theme::ui_font(iced::font::Weight::Semibold)),
    )
    .padding([2, 6])
    .style(move |t| theme::badge_colored(proto_color, t));

    // Meta row: protocol chip, server (when known), latency.
    let mut meta_row = row![type_tag]
        .spacing(crate::ui::SP_8)
        .align_y(Alignment::Center);
    if let (Some(s), Some(p)) = (server, port) {
        let chip_w = 14.0 + proto_label.chars().count().min(12) as f32 * 7.0;
        let server_px = card_w - pad * 2.0 - chip_w - 72.0 - crate::ui::SP_8 * 2.0;
        let server_cols = (server_px / 6.6).max(6.0) as usize;
        meta_row = meta_row.push(
            text(truncate_width(&format!("{s}:{p}"), server_cols))
                .color(theme::text_tertiary(theme))
                .size(theme::TYPE_CAPTION)
                .wrapping(text::Wrapping::None)
                .width(Length::Fill),
        );
    } else {
        meta_row = meta_row.push(Space::new().width(Length::Fill));
    }
    meta_row = meta_row.push(latency_el);

    let clean_name: String = node_name
        .chars()
        .filter(|c| !(('\u{1f1e6}'..='\u{1f1ff}').contains(c)))
        .collect();
    let clean_name = clean_name.trim();
    let flag_opt = theme::detect_region_code(node_name);

    // Budget the name to one line: card width minus padding, region chip and
    // the selection mark, at ~7.4px per Latin column for 14px text.
    let name_px = card_w
        - pad * 2.0
        - if flag_opt.is_some() { 34.0 } else { 0.0 }
        - if is_selected { 26.0 } else { 0.0 };
    let name_cols = (name_px / 7.4).max(8.0) as usize;
    let display_name = truncate_width(clean_name, name_cols);
    let name_truncated = display_name != clean_name;
    let name_text = text(display_name)
        .shaping(text::Shaping::Advanced)
        .wrapping(text::Wrapping::None)
        .color(if is_selected {
            theme::accent_text(theme)
        } else {
            text_primary
        })
        .size(theme::TYPE_SECTION)
        .font(theme::ui_font(if is_selected {
            iced::font::Weight::Semibold
        } else {
            iced::font::Weight::Medium
        }))
        .width(Length::Fill);

    let mut title_row = row![].spacing(8).align_y(Alignment::Center);
    if let Some(code) = flag_opt {
        title_row = title_row.push(
            container(
                text(code)
                    .size(theme::TYPE_MICRO)
                    .font(theme::ui_font(iced::font::Weight::Semibold))
                    .color(theme::text_muted(theme)),
            )
            .padding([2, 5])
            .style(theme::badge_bg),
        );
    }
    title_row = title_row.push(name_text);
    if is_selected {
        title_row = title_row.push(
            container(
                text(crate::ui::icons::ICON_CHECK.to_string())
                    .font(iced::Font::with_name("Material Icons"))
                    .size(12)
                    .color(theme::on_accent(theme)),
            )
            .width(18)
            .height(18)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(|t| container::Style {
                background: Some(iced::Background::Color(theme::accent(t))),
                border: iced::Border {
                    radius: 9.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        );
    }

    let card_content = column![title_row, Space::new().height(Length::Fill), meta_row]
        .padding(pad)
        .height(Length::Fill);

    let node_clone = node_name.to_string();
    let interactive = if group_clone.is_some() {
        is_selector
    } else {
        direct_selectable && server.is_some()
    };

    let mut card_btn = button(card_content)
        .padding(0)
        .height(Length::Fixed(PROXY_CARD_HEIGHT))
        .width(Length::Fill)
        .style(move |t, status| {
            if is_selected {
                let base = theme::card_selected(t);
                return button::Style {
                    background: base.background,
                    text_color: text_primary,
                    border: base.border,
                    shadow: base.shadow,
                    ..Default::default()
                };
            }
            let base = theme::card_bg(t);
            let engaged =
                interactive && matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if engaged {
                    Some(iced::Background::Color(theme::elevated_surface(t)))
                } else {
                    base.background
                },
                text_color: text_primary,
                border: iced::Border {
                    color: if engaged {
                        theme::with_alpha(theme::accent(t), 0.55)
                    } else {
                        base.border.color
                    },
                    width: base.border.width,
                    radius: base.border.radius,
                },
                shadow: base.shadow,
                ..Default::default()
            }
        });

    if let Some(gc) = group_clone {
        if is_selector {
            card_btn = card_btn.on_press(Message::SelectGroupNode {
                group: gc,
                node: node_clone,
            });
        }
    } else if direct_selectable {
        card_btn = card_btn.on_press(Message::SelectNode(node_clone));
    }

    if name_truncated {
        tooltip(
            card_btn,
            container(
                text(node_name.to_string())
                    .size(theme::TYPE_BTN_SM)
                    .color(theme::text_primary(theme)),
            )
            .padding([6, 10])
            .style(theme::popover_bg),
            tooltip::Position::Top,
        )
        .into()
    } else {
        card_btn.into()
    }
}

fn card_columns(width: f32) -> usize {
    ((width + theme::GRID_GAP) / (PROXY_CARD_MIN_W + theme::GRID_GAP))
        .floor()
        .clamp(1.0, 4.0) as usize
}

fn build_card_grid<'a>(
    card_elements: Vec<Element<'a, Message>>,
    cols: usize,
    total_nodes: usize,
    max_render: usize,
    theme: &iced::Theme,
    lang: crate::state::Language,
) -> Element<'a, Message> {
    use crate::ui::i18n::tr;
    use crate::ui::theme;
    use iced::widget::{Column, Row, container, scrollable, text};
    use iced::{Alignment, Length};

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
                    .size(theme::TYPE_CAPTION),
            )
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .padding(12),
        );
    }

    scrollable(grid_rows)
        .style(theme::scrollbar_style)
        .height(Length::Fill)
        .into()
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
