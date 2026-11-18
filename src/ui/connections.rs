use crate::api::Connection;
use crate::message::Message;
use crate::ui::icons::{ICON_CLOSE, ICON_DOWN, ICON_UP};
use crate::ui::theme;
use crate::ui::util::{SEMIBOLD_WIDTH, fit_width, format_size, labelled, with_colon};
use crate::ui::{CONNECTIONS_TABLE_W, CONNECTIONS_WIDE_W, page_header};
use iced::widget::text::Wrapping;
use iced::widget::{
    Space, button, column, container, responsive, row, scrollable, text, text_input, tooltip,
};
use iced::{Alignment, Color, Element, Font, Length};

/// Below this width the Connections header search/sort/close-all toolbar
/// stacks under the title so controls never clip at 740–779px.
const CONNECTIONS_TOOLBAR_INLINE_W: f32 = 780.0;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConnectionSortOption {
    sort: crate::state::ConnectionSort,
    label: String,
}

impl std::fmt::Display for ConnectionSortOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

struct PreparedConnection<'a> {
    connection: &'a Connection,
    host_key: String,
    process_label: String,
    process_key: String,
    network_key: String,
    chains_label: String,
    chains_key: String,
    rule_key: String,
    search_key: String,
}

pub fn render<'a>(
    gui_config: &'a crate::state::GuiConfig,
    active_connections: &'a [Connection],
    core_running: bool,
    search_query: &'a str,
    connections_sort: crate::state::ConnectionSort,
    connections_sort_desc: bool,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    use crate::ui::i18n::tr;

    let theme_cloned = theme.clone();
    let query_str = search_query.to_string();

    // Build display/search/sort keys once per snapshot. The responsive view can
    // be evaluated more than once during a resize; doing the string work here
    // avoids repeated lowercase allocations and path parsing in each pass.
    let query_lower = query_str.trim().to_lowercase();
    let mut prepared_connections: Vec<PreparedConnection<'a>> = active_connections
        .iter()
        .map(|connection| {
            let host = if !connection.metadata.host.is_empty() {
                &connection.metadata.host
            } else {
                &connection.metadata.destination_ip
            };
            let process_label = connection.metadata.process_display().unwrap_or_default();
            let chains_label = if connection.chains.is_empty() {
                String::new()
            } else {
                connection.chains.join(" → ")
            };
            let host_key = host.to_lowercase();
            let process_key = process_label.to_lowercase();
            let network_key = connection.metadata.network.to_lowercase();
            let chains_key = chains_label.to_lowercase();
            let rule_key = connection.rule.to_lowercase();
            let search_key = if query_str.trim().is_empty() {
                String::new()
            } else {
                format!(
                    "{} {} {} {} {} {} {}",
                    host_key,
                    connection.metadata.destination_ip.to_lowercase(),
                    connection.metadata.destination_port,
                    process_key,
                    chains_key,
                    rule_key,
                    network_key
                )
            };
            PreparedConnection {
                connection,
                host_key,
                process_label,
                process_key,
                network_key,
                chains_label,
                chains_key,
                rule_key,
                search_key,
            }
        })
        .filter(|prepared| {
            query_lower.trim().is_empty() || prepared.search_key.contains(&query_lower)
        })
        .collect();

    if connections_sort != crate::state::ConnectionSort::None {
        prepared_connections
            .sort_by(|a, b| compare_connections(a, b, connections_sort, connections_sort_desc));
    }
    const MAX_VISIBLE_CONNECTIONS: usize = 100;
    let hidden_connections = prepared_connections
        .len()
        .saturating_sub(MAX_VISIBLE_CONNECTIONS);
    prepared_connections.truncate(MAX_VISIBLE_CONNECTIONS);

    let main_content = responsive(move |size| {
        let theme = &theme_cloned;
        let text_primary = theme::text_primary(theme);
        let text_muted = theme::text_muted(theme);
        let is_compact = size.width < CONNECTIONS_TABLE_W;
        let header_stacked = size.width < CONNECTIONS_TOOLBAR_INLINE_W;
        let is_wide = size.width >= CONNECTIONS_WIDE_W;
        let (down_color, up_color) = theme::chart_series(theme);

        let filtered_connections = &prepared_connections;

        let search_input = text_input(tr(lang, "placeholder_connections_search"), &query_str)
            .on_input(Message::ConnectionsSearchChanged)
            .padding(8)
            .width(if header_stacked {
                Length::Fill
            } else {
                Length::Fixed(theme::SEARCH_WIDTH)
            })
            .size(theme::TYPE_BODY)
            .style(theme::input_field);

        let close_all_btn = button(text(tr(lang, "close_all_conn")).size(theme::TYPE_BTN_MD))
            .padding(theme::BTN_PAD_TOOLBAR)
            .style(theme::button_danger_soft);
        let close_all_btn = if core_running && !active_connections.is_empty() {
            close_all_btn.on_press(Message::CloseAllConnections)
        } else {
            close_all_btn
        };

        let sort_options = vec![
            ConnectionSortOption {
                sort: crate::state::ConnectionSort::None,
                label: tr(lang, "sort_original").to_string(),
            },
            ConnectionSortOption {
                sort: crate::state::ConnectionSort::Host,
                label: tr(lang, "sort_host").to_string(),
            },
            ConnectionSortOption {
                sort: crate::state::ConnectionSort::Process,
                label: tr(lang, "sort_process").to_string(),
            },
            ConnectionSortOption {
                sort: crate::state::ConnectionSort::Network,
                label: tr(lang, "sort_network").to_string(),
            },
            ConnectionSortOption {
                sort: crate::state::ConnectionSort::Chains,
                label: tr(lang, "chains").to_string(),
            },
            ConnectionSortOption {
                sort: crate::state::ConnectionSort::Rule,
                label: tr(lang, "sort_rule").to_string(),
            },
            ConnectionSortOption {
                sort: crate::state::ConnectionSort::Download,
                label: tr(lang, "sort_download").to_string(),
            },
            ConnectionSortOption {
                sort: crate::state::ConnectionSort::Upload,
                label: tr(lang, "sort_upload").to_string(),
            },
        ];

        let selected_sort_opt = sort_options
            .iter()
            .find(|o| o.sort == connections_sort)
            .cloned()
            .unwrap_or_else(|| sort_options[0].clone());

        let sort_picker =
            iced::widget::pick_list(sort_options, Some(selected_sort_opt), move |opt| {
                Message::SortConnections(opt.sort)
            })
            .padding(8)
            .text_size(theme::TYPE_BODY)
            .style(theme::pick_list)
            .menu_style(theme::pick_list_menu);

        let direction_btn = button(
            text(
                if connections_sort_desc {
                    ICON_DOWN
                } else {
                    ICON_UP
                }
                .to_string(),
            )
            .font(Font::with_name("Material Icons"))
            .size(16),
        )
        .padding([8, 10])
        .style(theme::button_secondary)
        .on_press(Message::SortConnections(connections_sort));
        // Reversing "original order" is a no-op, so the toggle only appears
        // once a real sort key is chosen.
        let direction_btn: Option<Element<'_, Message>> =
            (connections_sort != crate::state::ConnectionSort::None).then(|| direction_btn.into());

        let header_actions: Element<'_, Message> = if header_stacked {
            column![
                search_input.width(Length::Fill),
                row![
                    text(with_colon(lang, tr(lang, "sort_connections_by")))
                        .size(theme::TYPE_CAPTION)
                        .color(text_muted),
                    sort_picker.width(Length::Fill),
                ]
                .push(direction_btn)
                // Shares the sort row: a full-width red bar would be the
                // loudest thing on a narrow page and cost a row of height.
                .push(close_all_btn)
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill)
            ]
            .spacing(8)
            .width(Length::Fill)
            .into()
        } else {
            // The desktop column headers cover only a subset of sorts (the
            // mid-width table has no Upload column, and no header returns to
            // original order) — keep the full picker reachable here too.
            row![search_input, sort_picker.width(Length::Fixed(140.0))]
                .push(direction_btn)
                .push(close_all_btn)
                .spacing(12)
                .align_y(Alignment::Center)
                .into()
        };

        let page_title = page_header(
            "tab_connections",
            lang,
            Some(header_actions),
            theme,
            header_stacked,
        );

        if is_compact {
            // Mobile / Compact Layout: List of clean Connection Cards
            let mut list = column!().spacing(12);
            // Card text runs from the card's inner left edge to its close
            // button (28px plus a gap), inside the scroll lane.
            let card_text_w =
                crate::ui::page_content_width(size.width, crate::ui::is_pad_compact(size.width))
                    - crate::ui::SCROLL_GUTTER
                    - theme::CARD_PAD_DENSE * 2.0;

            if filtered_connections.is_empty() {
                let empty_msg = if query_str.trim().is_empty() {
                    tr(lang, "no_active_connections")
                } else {
                    tr(lang, "no_matching_connections")
                };
                let cta: Option<Element<'_, Message>> = if !query_str.trim().is_empty() {
                    Some(
                        button(text(tr(lang, "btn_clear_search")).size(theme::TYPE_BTN_MD))
                            .padding(theme::BTN_PAD_MD)
                            .style(theme::button_secondary)
                            .on_press(Message::ConnectionsSearchChanged(String::new()))
                            .into(),
                    )
                } else if gui_config.active_profile_id.is_none() {
                    Some(
                        button(text(tr(lang, "btn_goto_profiles")).size(theme::TYPE_BTN_MD))
                            .padding(theme::BTN_PAD_MD)
                            .style(theme::button_primary)
                            .on_press(Message::TabChanged(crate::state::Tab::Profiles))
                            .into(),
                    )
                } else if !core_running {
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
                let hint = if query_str.trim().is_empty() && !core_running {
                    Some(tr(lang, "empty_connections_hint"))
                } else {
                    None
                };
                list = list.push(crate::ui::empty_state(empty_msg, hint, cta, theme));
            } else {
                for prepared in filtered_connections.iter() {
                    let conn = prepared.connection;
                    let host_full = if !conn.metadata.host.is_empty() {
                        conn.metadata.host.as_str()
                    } else {
                        conn.metadata.destination_ip.as_str()
                    };
                    let chains_text = if prepared.chains_label.is_empty() {
                        tr(lang, "direct_chain")
                    } else {
                        prepared.chains_label.as_str()
                    };
                    let process_label = if prepared.process_label.is_empty() {
                        tr(lang, "process_unknown")
                    } else {
                        prepared.process_label.as_str()
                    };

                    let close_btn = close_button(conn.id.clone(), lang, theme);

                    let host_short = fit_width(
                        host_full,
                        card_text_w - 40.0,
                        theme::TYPE_SECTION * SEMIBOLD_WIDTH,
                    );
                    let host_el: Element<'_, Message> = if host_short != host_full {
                        tooltip(
                            text(host_short)
                                .color(text_primary)
                                .size(theme::TYPE_SECTION)
                                .font(theme::ui_font(iced::font::Weight::Bold)),
                            container(text(host_full).size(theme::TYPE_BTN_SM).color(text_primary))
                                .padding([6, 10])
                                .style(theme::popover_bg),
                            tooltip::Position::Top,
                        )
                        .gap(4)
                        .into()
                    } else {
                        text(host_short)
                            .color(text_primary)
                            .size(theme::TYPE_SECTION)
                            .font(theme::ui_font(iced::font::Weight::Bold))
                            .into()
                    };

                    let mut badges = row![].spacing(crate::ui::SP_8).align_y(Alignment::Center);
                    if !conn.metadata.network.trim().is_empty() {
                        badges = badges.push(
                            container(
                                text(conn.metadata.network.to_uppercase())
                                    .size(theme::TYPE_MICRO)
                                    .font(theme::ui_font(iced::font::Weight::Semibold))
                                    .color(text_muted),
                            )
                            .padding([2, 6])
                            .style(theme::badge_bg),
                        );
                    }
                    if !conn.rule.trim().is_empty() {
                        let rule_short = fit_width(&conn.rule, 180.0, theme::TYPE_MICRO);
                        let rule_trunc = rule_short != conn.rule;
                        let rule_badge: Element<'_, Message> =
                            container(text(rule_short).size(theme::TYPE_MICRO).color(text_muted))
                                .padding([2, 6])
                                .style(theme::badge_bg)
                                .into();
                        badges =
                            badges.push(tip_wrap(rule_badge, &conn.rule, rule_trunc, text_primary));
                    }

                    let process_full = labelled(lang, tr(lang, "col_process"), process_label);
                    let process_short = fit_width(&process_full, card_text_w, theme::TYPE_CAPTION);
                    let process_trunc = process_short != process_full;
                    let process_el = tip_wrap(
                        text(process_short)
                            .color(text_muted)
                            .size(theme::TYPE_CAPTION)
                            .wrapping(Wrapping::None)
                            .into(),
                        &process_full,
                        process_trunc,
                        text_primary,
                    );

                    let chains_full = labelled(lang, tr(lang, "chains"), chains_text);
                    let chains_short = fit_width(&chains_full, card_text_w, theme::TYPE_CAPTION);
                    let chains_trunc = chains_short != chains_full;
                    let chains_el = tip_wrap(
                        text(chains_short)
                            .color(text_muted)
                            .size(theme::TYPE_CAPTION)
                            .wrapping(Wrapping::None)
                            .into(),
                        &chains_full,
                        chains_trunc,
                        text_primary,
                    );

                    let mut card_col = column![
                        row![container(host_el).width(Length::Fill), close_btn]
                            .align_y(Alignment::Center)
                            .width(Length::Fill),
                    ]
                    .spacing(crate::ui::SP_8);
                    if !conn.metadata.network.trim().is_empty() || !conn.rule.trim().is_empty() {
                        card_col = card_col.push(badges);
                    }
                    card_col = card_col.push(process_el).push(chains_el).push(
                        row![
                            rate_label(ICON_DOWN, down_color, conn.download, theme),
                            rate_label(ICON_UP, up_color, conn.upload, theme),
                        ]
                        .spacing(crate::ui::SP_12)
                        .align_y(Alignment::Center),
                    );

                    let card = container(card_col)
                        // The 28px close button sits in the top-right corner: pull
                        // the top and right insets in by its 6px padding so the
                        // glyph, not the hit area, lines up with the card margin.
                        .padding(iced::Padding {
                            top: theme::CARD_PAD_DENSE - 6.0,
                            right: theme::CARD_PAD_DENSE - 6.0,
                            bottom: theme::CARD_PAD_DENSE,
                            left: theme::CARD_PAD_DENSE,
                        })
                        .width(Length::Fill)
                        .style(theme::card_bg);

                    list = list.push(card);
                }
            }
            if hidden_connections > 0 {
                list = list.push(
                    container(
                        text(format!(
                            "{} {}",
                            hidden_connections,
                            tr(lang, "connections_hidden_hint")
                        ))
                        .size(theme::TYPE_CAPTION)
                        .color(text_muted),
                    )
                    .width(Length::Fill)
                    .padding(12)
                    .center_x(Length::Fill),
                );
            }
            let list_gutter = if filtered_connections.is_empty() {
                0.0
            } else {
                crate::ui::SCROLL_GUTTER
            };
            let col = column![
                page_title,
                scrollable(container(list).padding(iced::Padding {
                    right: list_gutter,
                    ..Default::default()
                }))
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new()
                        .width(6)
                        .scroller_width(6)
                        .margin(3),
                ))
                .style(theme::scrollbar_style)
                .height(Length::Fill)
            ]
            .spacing(20)
            .width(Length::Fill)
            .height(Length::Fill);
            crate::ui::page_body_fixed_with_pad(col.into(), crate::ui::is_pad_compact(size.width))
        } else {
            // Desktop: mid-width 5-col (host, net+rule, chains, traffic, close)
            //          wide 7-col table
            let make_hdr_btn =
                |sort_col: crate::state::ConnectionSort, s: &'static str, portion: u16| {
                    let is_sorted = connections_sort == sort_col;
                    let label = if is_sorted {
                        format!(
                            "{} {}",
                            tr(lang, s),
                            if connections_sort_desc { "▼" } else { "▲" }
                        )
                    } else {
                        tr(lang, s).to_string()
                    };
                    button(text(label).size(theme::TYPE_SECTION).font(theme::ui_font(
                        if is_sorted {
                            iced::font::Weight::Bold
                        } else {
                            iced::font::Weight::Semibold
                        },
                    )))
                    .style(theme::button_header)
                    .padding([4, 6])
                    .on_press(Message::SortConnections(sort_col))
                    .width(Length::FillPortion(portion))
                };

            let make_hdr_btn_custom =
                |sort_col: crate::state::ConnectionSort, label_str: String, portion: u16| {
                    let is_sorted = connections_sort == sort_col;
                    let final_label = if is_sorted {
                        format!(
                            "{} {}",
                            label_str,
                            if connections_sort_desc { "▼" } else { "▲" }
                        )
                    } else {
                        label_str
                    };
                    button(
                        text(final_label)
                            .size(theme::TYPE_SECTION)
                            .font(theme::ui_font(if is_sorted {
                                iced::font::Weight::Bold
                            } else {
                                iced::font::Weight::Semibold
                            })),
                    )
                    .style(theme::button_header)
                    .padding([4, 6])
                    .on_press(Message::SortConnections(sort_col))
                    .width(Length::FillPortion(portion))
                };

            let header: Element<'_, Message> = if is_wide {
                row![
                    make_hdr_btn(crate::state::ConnectionSort::Host, "host", 6),
                    make_hdr_btn(crate::state::ConnectionSort::Process, "col_process", 4),
                    make_hdr_btn(crate::state::ConnectionSort::Network, "network", 2),
                    make_hdr_btn(crate::state::ConnectionSort::Chains, "chains", 4),
                    make_hdr_btn(crate::state::ConnectionSort::Rule, "rule", 3),
                    make_hdr_btn(crate::state::ConnectionSort::Download, "download", 2),
                    make_hdr_btn(crate::state::ConnectionSort::Upload, "upload", 2),
                    Space::new().width(Length::Fixed(CLOSE_COL_W))
                ]
                .spacing(crate::ui::SP_12)
                .padding([0, 4])
                .into()
            } else {
                let net_rule_label = format!("{} / {}", tr(lang, "network"), tr(lang, "rule"));
                row![
                    make_hdr_btn(crate::state::ConnectionSort::Host, "host", 6),
                    make_hdr_btn(crate::state::ConnectionSort::Process, "col_process", 3),
                    make_hdr_btn_custom(crate::state::ConnectionSort::Network, net_rule_label, 5),
                    make_hdr_btn(crate::state::ConnectionSort::Chains, "chains", 4),
                    make_hdr_btn_custom(
                        crate::state::ConnectionSort::Download,
                        "↓ / ↑".to_string(),
                        6
                    ),
                    Space::new().width(Length::Fixed(CLOSE_COL_W))
                ]
                .spacing(crate::ui::SP_12)
                .padding([0, 4])
                .into()
            };

            let list_content: Element<'_, Message> = if filtered_connections.is_empty() {
                let empty_msg = if query_str.trim().is_empty() {
                    tr(lang, "no_active_connections")
                } else {
                    tr(lang, "no_matching_connections")
                };
                let cta: Option<Element<'_, Message>> = if !query_str.trim().is_empty() {
                    Some(
                        button(text(tr(lang, "btn_clear_search")).size(theme::TYPE_BTN_MD))
                            .padding(theme::BTN_PAD_MD)
                            .style(theme::button_secondary)
                            .on_press(Message::ConnectionsSearchChanged(String::new()))
                            .into(),
                    )
                } else if gui_config.active_profile_id.is_none() {
                    Some(
                        button(text(tr(lang, "btn_goto_profiles")).size(theme::TYPE_BTN_MD))
                            .padding(theme::BTN_PAD_MD)
                            .style(theme::button_primary)
                            .on_press(Message::TabChanged(crate::state::Tab::Profiles))
                            .into(),
                    )
                } else if !core_running {
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
                let hint = if query_str.trim().is_empty() && !core_running {
                    Some(tr(lang, "empty_connections_hint"))
                } else {
                    None
                };
                crate::ui::empty_state(empty_msg, hint, cta, theme)
            } else {
                let mut list = column!().spacing(0);
                let len = filtered_connections.len();
                // Cells are single-line: budget each column from its share of
                // the table so text ends in an ellipsis instead of wrapping the
                // row. Fill portions: wide 6+4+2+4+3+2+2, mid 6+3+5+4+6; the
                // close column is fixed.
                let (portions, gutters) = if is_wide { (23.0, 7.0) } else { (24.0, 5.0) };
                let table_w = crate::ui::page_content_width(size.width, false);
                let fixed =
                    4.0 + 8.0 + TABLE_SCROLL_LANE + CLOSE_COL_W + gutters * crate::ui::SP_12;
                let unit = ((table_w - fixed) / portions).max(1.0);
                // A cell's text room: its share of the row less the 6px insets.
                let room = |portion: f32| (unit * portion - 12.0).max(24.0);
                for (idx, prepared) in filtered_connections.iter().enumerate() {
                    let conn = prepared.connection;
                    let host_full = if !conn.metadata.host.is_empty() {
                        conn.metadata.host.as_str()
                    } else {
                        conn.metadata.destination_ip.as_str()
                    };
                    let process_label = if prepared.process_label.is_empty() {
                        tr(lang, "process_unknown")
                    } else {
                        prepared.process_label.as_str()
                    };
                    let chains_label = if prepared.chains_label.is_empty() {
                        tr(lang, "direct_chain")
                    } else {
                        prepared.chains_label.as_str()
                    };

                    let host_short = fit_width(host_full, room(6.0), theme::TYPE_SECTION);
                    let host_trunc = host_short != host_full;
                    let host = tip_wrap(
                        text(host_short)
                            .size(theme::TYPE_SECTION)
                            .color(text_primary)
                            .wrapping(Wrapping::None)
                            .into(),
                        host_full,
                        host_trunc,
                        text_primary,
                    );

                    let proc_short = fit_width(
                        process_label,
                        room(if is_wide { 4.0 } else { 3.0 }),
                        theme::TYPE_BTN_SM,
                    );
                    let proc_trunc = proc_short != process_label;
                    let process = tip_wrap(
                        text(proc_short)
                            .size(theme::TYPE_BTN_SM)
                            .color(text_muted)
                            .wrapping(Wrapping::None)
                            .into(),
                        process_label,
                        proc_trunc,
                        text_primary,
                    );

                    let chains_short = fit_width(chains_label, room(4.0), theme::TYPE_BTN_SM);
                    let chains_trunc = chains_short != chains_label;
                    let chains = tip_wrap(
                        text(chains_short)
                            .size(theme::TYPE_BTN_SM)
                            .color(text_muted)
                            .wrapping(Wrapping::None)
                            .into(),
                        chains_label,
                        chains_trunc,
                        text_primary,
                    );

                    let is_udp = conn.metadata.network.to_lowercase().contains("udp");
                    let proto_badge = container(
                        text(conn.metadata.network.to_uppercase())
                            .size(theme::TYPE_MICRO)
                            .font(theme::ui_font(iced::font::Weight::Semibold))
                            .color(text_muted)
                            .wrapping(Wrapping::None),
                    )
                    .padding([1, 5])
                    .style(move |t| {
                        if is_udp {
                            theme::badge_colored(theme::INFO, t)
                        } else {
                            theme::badge_bg(t)
                        }
                    });

                    let row_content: Element<'_, Message> = if is_wide {
                        let rule_short = fit_width(&conn.rule, room(3.0), theme::TYPE_BTN_SM);
                        let rule_trunc = rule_short != conn.rule;
                        let rule_el = tip_wrap(
                            text(rule_short)
                                .size(theme::TYPE_BTN_SM)
                                .color(text_muted)
                                .wrapping(Wrapping::None)
                                .into(),
                            &conn.rule,
                            rule_trunc,
                            text_primary,
                        );
                        row![
                            cell(host, 6),
                            cell(process, 4),
                            cell(proto_badge, 2),
                            cell(chains, 4),
                            cell(rule_el, 3),
                            cell(rate_label(ICON_DOWN, down_color, conn.download, theme), 2),
                            cell(rate_label(ICON_UP, up_color, conn.upload, theme), 2),
                            container(close_button(conn.id.clone(), lang, theme))
                                .width(Length::Fixed(CLOSE_COL_W))
                                .align_x(Alignment::End),
                        ]
                        .align_y(Alignment::Center)
                        .spacing(crate::ui::SP_12)
                        .padding(ROW_PAD)
                        .into()
                    } else {
                        // The protocol badge (about 40px) and its gap share the cell.
                        let rule_short =
                            fit_width(&conn.rule, (room(5.0) - 46.0).max(20.0), theme::TYPE_BTN_SM);
                        let rule_trunc = rule_short != conn.rule;
                        let rule_el = tip_wrap(
                            text(rule_short)
                                .size(theme::TYPE_BTN_SM)
                                .color(text_muted)
                                .wrapping(Wrapping::None)
                                .into(),
                            &conn.rule,
                            rule_trunc,
                            text_primary,
                        );
                        let net_rule = row![proto_badge, rule_el]
                            .spacing(6)
                            .align_y(Alignment::Center);
                        let traffic = row![
                            rate_label(ICON_DOWN, down_color, conn.download, theme),
                            rate_label(ICON_UP, up_color, conn.upload, theme),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center);

                        row![
                            cell(host, 6),
                            cell(process, 3),
                            cell(net_rule, 5),
                            cell(chains, 4),
                            cell(traffic, 6),
                            container(close_button(conn.id.clone(), lang, theme))
                                .width(Length::Fixed(CLOSE_COL_W))
                                .align_x(Alignment::End),
                        ]
                        .align_y(Alignment::Center)
                        .spacing(crate::ui::SP_12)
                        .padding(ROW_PAD)
                        .into()
                    };

                    list = list.push(
                        container(row_content)
                            .width(Length::Fill)
                            .style(move |t| theme::table_row_bg(t, idx % 2 == 1)),
                    );

                    if idx + 1 < len {
                        list = list.push(
                            container(Space::new())
                                .height(1)
                                .width(Length::Fill)
                                .style(theme::hairline),
                        );
                    }
                }

                // The header band follows the card's rounded top and is split
                // from the rows by a hairline, inset 1px so the card's own
                // outline stays unbroken.
                let header_styled = column![
                    container(header)
                        .padding(iced::Padding {
                            top: 10.0,
                            right: TABLE_SCROLL_LANE,
                            bottom: 10.0,
                            left: 0.0,
                        })
                        .width(Length::Fill)
                        .style(|t| container::Style {
                            background: Some(iced::Background::Color(theme::elevated_surface(t))),
                            border: iced::Border {
                                radius: iced::border::Radius {
                                    top_left: theme::RADIUS_LG - 1.0,
                                    top_right: theme::RADIUS_LG - 1.0,
                                    bottom_right: 0.0,
                                    bottom_left: 0.0,
                                },
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    container(Space::new())
                        .height(1)
                        .width(Length::Fill)
                        .style(theme::hairline),
                ];

                container(column![
                    header_styled,
                    scrollable(container(if hidden_connections > 0 {
                        list.push(
                            container(
                                text(format!(
                                    "{} {}",
                                    hidden_connections,
                                    tr(lang, "connections_hidden_hint")
                                ))
                                .size(theme::TYPE_CAPTION)
                                .color(text_muted),
                            )
                            .width(Length::Fill)
                            .padding(12)
                            .center_x(Length::Fill),
                        )
                    } else {
                        list
                    }))
                    .direction(scrollable::Direction::Vertical(
                        scrollable::Scrollbar::new()
                            .width(6)
                            .scroller_width(6)
                            .margin(3),
                    ))
                    .style(theme::scrollbar_style)
                    .height(Length::Fill)
                ])
                .padding(1)
                .style(theme::card_bg)
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
            };

            let col = column![page_title, list_content]
                .spacing(crate::ui::SP_20)
                .width(Length::Fill)
                .height(Length::Fill);

            crate::ui::page_body_fixed_with_pad(col.into(), crate::ui::is_pad_compact(size.width))
        }
    });

    main_content.into()
}

/// Table row insets. The right side also clears the scrollbar lane, so the
/// zebra fill still runs to the card edge beneath the bar.
const ROW_PAD: iced::Padding = iced::Padding {
    top: 7.0,
    right: 4.0 + TABLE_SCROLL_LANE,
    bottom: 7.0,
    left: 4.0,
};

/// Close-button column: a 28px glyph button plus breathing room.
const CLOSE_COL_W: f32 = 32.0;

/// Right-hand lane the table's floating scrollbar occupies (6px bar plus
/// 3px margins), kept clear of row content.
const TABLE_SCROLL_LANE: f32 = 12.0;

/// Single-line table cell. The caller truncates to the column budget; the
/// clip is only a backstop so a stray wide glyph can never wrap the row.
fn cell<'a>(content: impl Into<Element<'a, Message>>, portion: u16) -> Element<'a, Message> {
    // Same 6px side inset as the header buttons, so text sits under its title.
    container(content)
        .padding([0, 6])
        .width(Length::FillPortion(portion))
        .clip(true)
        .into()
}

/// Transfer total with a series-coloured arrow; the figure itself stays in
/// text ink so the column reads as numbers, not as two colours of noise.
fn rate_label<'a>(
    icon: char,
    color: Color,
    bytes: u64,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    row![
        text(icon.to_string())
            .font(Font::with_name("Material Icons"))
            .size(12)
            .color(color),
        text(format_size(bytes))
            .size(theme::TYPE_CAPTION)
            .font(theme::metric_font())
            .color(theme::text_primary(theme))
            .wrapping(Wrapping::None),
    ]
    .spacing(2)
    .align_y(Alignment::Center)
    .into()
}

/// Per-row close: a quiet × that only turns red under the pointer, so a
/// full table isn't a column of alarm buttons.
fn close_button<'a>(
    id: String,
    lang: crate::state::Language,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    tooltip(
        button(
            text(ICON_CLOSE)
                .font(Font::with_name("Material Icons"))
                .size(16),
        )
        .padding(6)
        .style(theme::button_dismiss)
        .on_press(Message::CloseConnection(id)),
        container(
            text(crate::ui::i18n::tr(lang, "close_conn"))
                .size(theme::TYPE_CAPTION)
                .color(theme::text_primary(theme)),
        )
        .padding([6, 10])
        .style(theme::popover_bg),
        tooltip::Position::Left,
    )
    .gap(4)
    .into()
}

fn compare_connections(
    a: &PreparedConnection<'_>,
    b: &PreparedConnection<'_>,
    sort: crate::state::ConnectionSort,
    desc: bool,
) -> std::cmp::Ordering {
    if sort == crate::state::ConnectionSort::None {
        return std::cmp::Ordering::Equal;
    }
    let ord = match sort {
        crate::state::ConnectionSort::None => std::cmp::Ordering::Equal,
        crate::state::ConnectionSort::Host => a.host_key.cmp(&b.host_key),
        crate::state::ConnectionSort::Process => a.process_key.cmp(&b.process_key),
        crate::state::ConnectionSort::Network => a.network_key.cmp(&b.network_key),
        crate::state::ConnectionSort::Chains => a.chains_key.cmp(&b.chains_key),
        crate::state::ConnectionSort::Rule => a.rule_key.cmp(&b.rule_key),
        crate::state::ConnectionSort::Download => a.connection.download.cmp(&b.connection.download),
        crate::state::ConnectionSort::Upload => a.connection.upload.cmp(&b.connection.upload),
    };
    let primary = if desc { ord.reverse() } else { ord };
    primary
        .then_with(|| a.host_key.cmp(&b.host_key))
        .then_with(|| a.connection.id.cmp(&b.connection.id))
}

fn tip_wrap<'a>(
    el: Element<'a, Message>,
    full: &str,
    truncated: bool,
    text_primary: Color,
) -> Element<'a, Message> {
    if truncated {
        tooltip(
            el,
            container(
                text(full.to_string())
                    .size(theme::TYPE_BTN_SM)
                    .color(text_primary),
            )
            .padding([6, 10])
            .max_width(420)
            .style(theme::popover_bg),
            tooltip::Position::Top,
        )
        .gap(4)
        .into()
    } else {
        el
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Connection, ConnectionMetadata};
    use crate::state::ConnectionSort;

    fn make_prepared<'a>(
        conn: &'a Connection,
        host: &str,
        network: &str,
        rule: &str,
    ) -> PreparedConnection<'a> {
        PreparedConnection {
            connection: conn,
            host_key: host.to_lowercase(),
            process_label: String::new(),
            process_key: String::new(),
            network_key: network.to_lowercase(),
            chains_label: String::new(),
            chains_key: String::new(),
            rule_key: rule.to_lowercase(),
            search_key: String::new(),
        }
    }

    #[test]
    fn connection_sort_tiebreaks_deterministically_by_host_and_id() {
        let c1 = Connection {
            id: "2".into(),
            metadata: ConnectionMetadata::default(),
            upload: 100,
            download: 500,
            start: String::new(),
            chains: vec![],
            rule: "RuleA".into(),
        };
        let c2 = Connection {
            id: "1".into(),
            metadata: ConnectionMetadata::default(),
            upload: 200,
            download: 500,
            start: String::new(),
            chains: vec![],
            rule: "RuleA".into(),
        };
        let p_b = make_prepared(&c1, "beta.example", "tcp", "RuleA");
        let p_a = make_prepared(&c2, "alpha.example", "tcp", "RuleA");
        let mut items = [p_b, p_a];
        items.sort_by(|a, b| compare_connections(a, b, ConnectionSort::Download, true));
        assert_eq!(items[0].host_key, "alpha.example");
        assert_eq!(items[1].host_key, "beta.example");

        items.sort_by(|a, b| compare_connections(a, b, ConnectionSort::Upload, true));
        assert_eq!(items[0].connection.id, "1");
        assert_eq!(items[1].connection.id, "2");
    }

    #[test]
    fn connection_sort_is_total_order_for_all_variants_and_directions() {
        let conns = [
            Connection {
                id: "c1".into(),
                metadata: ConnectionMetadata::default(),
                upload: 100,
                download: 200,
                start: String::new(),
                chains: vec![],
                rule: "RuleA".into(),
            },
            Connection {
                id: "c2".into(),
                metadata: ConnectionMetadata::default(),
                upload: 100,
                download: 300,
                start: String::new(),
                chains: vec!["Proxy".into()],
                rule: "RuleB".into(),
            },
            Connection {
                id: "c3".into(),
                metadata: ConnectionMetadata::default(),
                upload: 50,
                download: 200,
                start: String::new(),
                chains: vec!["Proxy".into()],
                rule: "rulea".into(),
            },
        ];
        let prepared = [
            make_prepared(&conns[0], "a.example", "tcp", "RuleA"),
            make_prepared(&conns[1], "b.example", "udp", "RuleB"),
            make_prepared(&conns[2], "a.example", "tcp", "rulea"),
        ];
        for sort in [
            ConnectionSort::None,
            ConnectionSort::Host,
            ConnectionSort::Process,
            ConnectionSort::Network,
            ConnectionSort::Chains,
            ConnectionSort::Rule,
            ConnectionSort::Download,
            ConnectionSort::Upload,
        ] {
            for desc in [false, true] {
                for a in &prepared {
                    assert_eq!(
                        compare_connections(a, a, sort, desc),
                        std::cmp::Ordering::Equal
                    );
                    for b in &prepared {
                        let ab = compare_connections(a, b, sort, desc);
                        let ba = compare_connections(b, a, sort, desc);
                        assert_eq!(ab, ba.reverse());
                        for c in &prepared {
                            let bc = compare_connections(b, c, sort, desc);
                            let ac = compare_connections(a, c, sort, desc);
                            if ab == bc {
                                assert_eq!(ac, ab);
                            }
                        }
                    }
                }
            }
        }
    }
}
