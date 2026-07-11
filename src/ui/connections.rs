use iced::widget::{button, column, container, row, scrollable, text, text_input, Space, responsive};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::ui::theme;
use crate::api::Connection;
use crate::ui::{page_header, CONNECTIONS_TABLE_W, CONNECTIONS_WIDE_W};
use crate::ui::util::{format_size, truncate_chars};

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

pub fn render<'a>(
    gui_config: &'a crate::state::GuiConfig,
    active_connections: &'a [Connection],
    search_query: &'a str,
    connections_sort: crate::state::ConnectionSort,
    connections_sort_desc: bool,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    use crate::ui::i18n::tr;
    
    let theme_cloned = theme.clone();
    let query_str = search_query.to_string();
    
    let main_content = responsive(move |size| {
        let theme = &theme_cloned;
        let text_primary = theme::text_primary(theme);
        let text_muted = theme::text_muted(theme);
        let is_compact = size.width < CONNECTIONS_TABLE_W;
        let is_wide = size.width >= CONNECTIONS_WIDE_W;
        
        // Filter connections
        let mut filtered_connections: Vec<&Connection> = if query_str.trim().is_empty() {
            active_connections.iter().collect()
        } else {
            let q = query_str.to_lowercase();
            active_connections.iter()
                .filter(|conn| {
                    let host_text = if !conn.metadata.host.is_empty() {
                        &conn.metadata.host
                    } else {
                        &conn.metadata.destination_ip
                    };
                    let process = conn
                        .metadata
                        .process_display()
                        .unwrap_or_default()
                        .to_lowercase();
                    host_text.to_lowercase().contains(&q)
                        || conn.metadata.destination_ip.to_lowercase().contains(&q)
                        || conn.chains.iter().any(|c| c.to_lowercase().contains(&q))
                        || conn.rule.to_lowercase().contains(&q)
                        || conn.metadata.network.to_lowercase().contains(&q)
                        || process.contains(&q)
                })
                .collect()
        };
        
        // Sort connections
        filtered_connections.sort_by(|a, b| {
            let ord = match connections_sort {
                crate::state::ConnectionSort::None => std::cmp::Ordering::Equal,
                crate::state::ConnectionSort::Host => {
                    let ha = if !a.metadata.host.is_empty() { &a.metadata.host } else { &a.metadata.destination_ip };
                    let hb = if !b.metadata.host.is_empty() { &b.metadata.host } else { &b.metadata.destination_ip };
                    ha.to_lowercase().cmp(&hb.to_lowercase())
                }
                crate::state::ConnectionSort::Process => {
                    let pa = a.metadata.process_display().unwrap_or_default();
                    let pb = b.metadata.process_display().unwrap_or_default();
                    pa.to_lowercase().cmp(&pb.to_lowercase())
                }
                crate::state::ConnectionSort::Network => {
                    a.metadata.network.to_lowercase().cmp(&b.metadata.network.to_lowercase())
                }
                crate::state::ConnectionSort::Chains => {
                    let ca = a.chains.join(" ➔ ");
                    let cb = b.chains.join(" ➔ ");
                    ca.to_lowercase().cmp(&cb.to_lowercase())
                }
                crate::state::ConnectionSort::Rule => {
                    a.rule.to_lowercase().cmp(&b.rule.to_lowercase())
                }
                crate::state::ConnectionSort::Download => {
                    a.download.cmp(&b.download)
                }
                crate::state::ConnectionSort::Upload => {
                    a.upload.cmp(&b.upload)
                }
            };
            if connections_sort_desc { ord.reverse() } else { ord }
        });
        
        let search_input = text_input(tr(lang, "placeholder_connections_search"), &query_str)
            .on_input(Message::ConnectionsSearchChanged)
            .padding(8)
            .width(if is_compact { Length::Fill } else { Length::Fixed(theme::SEARCH_WIDTH) })
            .style(theme::input_field);

        let close_all_btn = button(text(tr(lang, "close_all_conn")).size(theme::TYPE_BTN_MD))
            .padding(theme::BTN_PAD_MD)
            .style(theme::button_danger)
            .on_press(Message::CloseAllConnections);

        let sort_options = vec![
            ConnectionSortOption { sort: crate::state::ConnectionSort::None, label: tr(lang, "sort_original").to_string() },
            ConnectionSortOption { sort: crate::state::ConnectionSort::Host, label: tr(lang, "sort_host").to_string() },
            ConnectionSortOption { sort: crate::state::ConnectionSort::Process, label: tr(lang, "sort_process").to_string() },
            ConnectionSortOption { sort: crate::state::ConnectionSort::Network, label: tr(lang, "sort_network").to_string() },
            ConnectionSortOption { sort: crate::state::ConnectionSort::Rule, label: tr(lang, "sort_rule").to_string() },
            ConnectionSortOption { sort: crate::state::ConnectionSort::Download, label: tr(lang, "sort_download").to_string() },
            ConnectionSortOption { sort: crate::state::ConnectionSort::Upload, label: tr(lang, "sort_upload").to_string() },
        ];

        let selected_sort_opt = sort_options.iter()
            .find(|o| o.sort == connections_sort)
            .cloned()
            .unwrap_or_else(|| sort_options[0].clone());

        let sort_picker = iced::widget::pick_list(
            sort_options,
            Some(selected_sort_opt),
            move |opt| Message::SortConnections(opt.sort)
        )
        .padding(8)
        .style(theme::pick_list);

        let direction_btn = button(
            text(if connections_sort_desc { "▼" } else { "▲" }).size(theme::TYPE_BTN_MD)
        )
        .padding([8, 12])
        .style(theme::button_secondary)
        .on_press(Message::SortConnections(connections_sort));

        let header_actions: Element<'_, Message> = if is_compact {
            column![
                search_input.width(Length::Fill),
                row![
                    text(format!("{}:", tr(lang, "sort_connections_by"))).size(theme::TYPE_CAPTION).color(text_muted),
                    sort_picker.width(Length::Fill),
                    direction_btn
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill),
                close_all_btn.width(Length::Fill)
            ]
            .spacing(8)
            .width(Length::Fill)
            .into()
        } else {
            row![search_input, close_all_btn]
                .spacing(12)
                .align_y(Alignment::Center)
                .into()
        };

        let page_title = page_header("tab_connections", lang, Some(header_actions), theme, is_compact);

        if is_compact {
            // Mobile / Compact Layout: List of clean Connection Cards
            let mut list = column!().spacing(12);
            
            if filtered_connections.is_empty() {
                let empty_msg = if query_str.trim().is_empty() {
                    tr(lang, "no_active_connections")
                } else {
                    tr(lang, "no_matching_connections")
                };
                let cta = if query_str.trim().is_empty() {
                    button(text(tr(lang, "btn_start_core_short")).size(theme::TYPE_BTN_MD))
                        .padding(theme::BTN_PAD_MD)
                        .style(theme::button_primary)
                        .on_press(Message::ToggleCore)
                        .into()
                } else {
                    button(text(tr(lang, "btn_clear_search")).size(theme::TYPE_BTN_MD))
                        .padding(theme::BTN_PAD_MD)
                        .style(theme::button_secondary)
                        .on_press(Message::ConnectionsSearchChanged(String::new()))
                        .into()
                };
                let hint = if query_str.trim().is_empty() {
                    Some(tr(lang, "empty_connections_hint"))
                } else {
                    None
                };
                list = list.push(crate::ui::empty_state(empty_msg, hint, Some(cta), theme));
            } else {
                for conn in filtered_connections {
                    let host_full = if !conn.metadata.host.is_empty() {
                        conn.metadata.host.clone()
                    } else {
                        conn.metadata.destination_ip.clone()
                    };
                    
                    let chains_text = if conn.chains.is_empty() {
                        tr(lang, "direct_chain").to_string()
                    } else {
                        conn.chains.join(" ➔ ")
                    };

                    let process_label = conn
                        .metadata
                        .process_display()
                        .unwrap_or_else(|| tr(lang, "process_unknown").to_string());
                    
                    let dl_text = format_size(conn.download);
                    let ul_text = format_size(conn.upload);
                    
                    let close_btn = button(
                        text(tr(lang, "close_conn")).size(theme::TYPE_CAPTION)
                    )
                    .style(theme::button_danger)
                    .padding([4, 8])
                    .on_press(Message::CloseConnection(conn.id.clone()));
                    
                    let card = container(
                        column![
                            row![
                                text(truncate_chars(&host_full, 48))
                                    .color(text_primary)
                                    .size(theme::TYPE_SECTION)
                                    .font(iced::Font {
                                        weight: iced::font::Weight::Bold,
                                        ..Default::default()
                                    })
                                    .width(Length::Fill),
                                close_btn
                            ]
                            .align_y(Alignment::Center)
                            .width(Length::Fill),
                            
                            row![
                                container(text(&conn.metadata.network).size(theme::TYPE_MICRO).color(theme::SUCCESS))
                                    .padding([2, 6])
                                    .style(theme::badge_bg),
                                container(text(truncate_chars(&conn.rule, 24)).size(theme::TYPE_MICRO).color(theme::ACCENT_PURPLE))
                                    .padding([2, 6])
                                    .style(theme::badge_bg),
                            ]
                            .spacing(crate::ui::SP_8)
                            .align_y(Alignment::Center),

                            text(format!("{}: {}", tr(lang, "col_process"), truncate_chars(&process_label, 36)))
                                .color(text_muted)
                                .size(theme::TYPE_CAPTION),
                            
                            text(format!("{}: {}", tr(lang, "chains"), truncate_chars(&chains_text, 40)))
                                .color(text_muted)
                                .size(theme::TYPE_CAPTION),
                                
                            row![
                                text(format!("↓ {}", dl_text)).color(theme::ACCENT_BLUE).size(theme::TYPE_CAPTION),
                                Space::new().width(12),
                                text(format!("↑ {}", ul_text)).color(theme::ACCENT_PURPLE).size(theme::TYPE_CAPTION)
                            ]
                            .align_y(Alignment::Center)
                        ]
                        .spacing(crate::ui::SP_8)
                    )
                    .padding(theme::CARD_PAD_DENSE)
                    .width(Length::Fill)
                    .style(theme::card_bg);
                    
                    list = list.push(card);
                }
            }
            let col = column![page_title, scrollable(list).style(theme::scrollbar_style).height(Length::Fill)]
                .spacing(20)
                .width(Length::Fill)
                .height(Length::Fill);
            crate::ui::page_body_fixed_with_pad(col.into(), is_compact)
        } else {
            // Desktop: mid-width 5-col (host, net+rule, chains, traffic, close)
            //          wide 7-col table
            let make_hdr_btn = |sort_col: crate::state::ConnectionSort, s: &'static str, portion: u16| {
                let is_sorted = connections_sort == sort_col;
                let label = if is_sorted {
                    format!("{} {}", tr(lang, s), if connections_sort_desc { "▼" } else { "▲" })
                } else {
                    tr(lang, s).to_string()
                };
                button(
                    text(label)
                        .size(theme::TYPE_SECTION)
                        .font(iced::Font {
                            weight: if is_sorted { iced::font::Weight::Bold } else { iced::font::Weight::Semibold },
                            ..Default::default()
                        })
                )
                .style(theme::button_header)
                .padding([4, 6])
                .on_press(Message::SortConnections(sort_col))
                .width(Length::FillPortion(portion))
            };

            let make_hdr_btn_custom = |sort_col: crate::state::ConnectionSort, label_str: String, portion: u16| {
                let is_sorted = connections_sort == sort_col;
                let final_label = if is_sorted {
                    format!("{} {}", label_str, if connections_sort_desc { "▼" } else { "▲" })
                } else {
                    label_str
                };
                button(
                    text(final_label)
                        .size(theme::TYPE_SECTION)
                        .font(iced::Font {
                            weight: if is_sorted { iced::font::Weight::Bold } else { iced::font::Weight::Semibold },
                            ..Default::default()
                        })
                )
                .style(theme::button_header)
                .padding([4, 6])
                .on_press(Message::SortConnections(sort_col))
                .width(Length::FillPortion(portion))
            };

            let header: Element<'_, Message> = if is_wide {
                row![
                    make_hdr_btn(crate::state::ConnectionSort::Host, "host", 3),
                    make_hdr_btn(crate::state::ConnectionSort::Process, "col_process", 2),
                    make_hdr_btn(crate::state::ConnectionSort::Network, "network", 1),
                    make_hdr_btn(crate::state::ConnectionSort::Chains, "chains", 2),
                    make_hdr_btn(crate::state::ConnectionSort::Rule, "rule", 1),
                    make_hdr_btn(crate::state::ConnectionSort::Download, "download", 1),
                    make_hdr_btn(crate::state::ConnectionSort::Upload, "upload", 1),
                    Space::new().width(Length::FillPortion(1))
                ]
                .spacing(crate::ui::SP_12)
                .padding([0, 10])
                .into()
            } else {
                let net_rule_label = format!("{} / {}", tr(lang, "network"), tr(lang, "rule"));
                row![
                    make_hdr_btn(crate::state::ConnectionSort::Host, "host", 3),
                    make_hdr_btn(crate::state::ConnectionSort::Process, "col_process", 2),
                    make_hdr_btn_custom(crate::state::ConnectionSort::Network, net_rule_label, 2),
                    make_hdr_btn(crate::state::ConnectionSort::Chains, "chains", 2),
                    make_hdr_btn_custom(crate::state::ConnectionSort::Download, "↓ / ↑".to_string(), 2),
                    Space::new().width(Length::FillPortion(1))
                ]
                .spacing(crate::ui::SP_12)
                .padding([0, 10])
                .into()
            };
            
            let mut list = column!().spacing(0);
            if filtered_connections.is_empty() {
                let empty_msg = if query_str.trim().is_empty() {
                    tr(lang, "no_active_connections")
                } else {
                    tr(lang, "no_matching_connections")
                };
                let cta = if query_str.trim().is_empty() {
                    button(text(tr(lang, "btn_start_core_short")).size(theme::TYPE_BTN_MD))
                        .padding(theme::BTN_PAD_MD)
                        .style(theme::button_primary)
                        .on_press(Message::ToggleCore)
                        .into()
                } else {
                    button(text(tr(lang, "btn_clear_search")).size(theme::TYPE_BTN_MD))
                        .padding(theme::BTN_PAD_MD)
                        .style(theme::button_secondary)
                        .on_press(Message::ConnectionsSearchChanged(String::new()))
                        .into()
                };
                let hint = if query_str.trim().is_empty() {
                    Some(tr(lang, "empty_connections_hint"))
                } else {
                    None
                };
                list = list.push(crate::ui::empty_state(empty_msg, hint, Some(cta), theme));
            } else {
                let len = filtered_connections.len();
                for (idx, conn) in filtered_connections.into_iter().enumerate() {
                    let host_full = if !conn.metadata.host.is_empty() {
                        conn.metadata.host.clone()
                    } else {
                        conn.metadata.destination_ip.clone()
                    };
                    let host_text = truncate_chars(&host_full, if is_wide { 48 } else { 32 });
                    let process_label = conn
                        .metadata
                        .process_display()
                        .unwrap_or_else(|| tr(lang, "process_unknown").to_string());
                    let process_text = truncate_chars(&process_label, if is_wide { 28 } else { 18 });
                    
                    let chains_text = if conn.chains.is_empty() {
                        tr(lang, "direct_chain").to_string()
                    } else {
                        conn.chains.join(" ➔ ")
                    };
                    let chains_text = truncate_chars(&chains_text, if is_wide { 36 } else { 24 });
                    
                    let dl_text = format_size(conn.download);
                    let ul_text = format_size(conn.upload);
                    
                    let close_btn = button(
                        text(tr(lang, "close_conn")).size(theme::TYPE_BTN_SM)
                    )
                    .style(theme::button_danger)
                    .padding([4, 8])
                    .on_press(Message::CloseConnection(conn.id.clone()));
                    
                    let row_content: Element<'_, Message> = if is_wide {
                        row![
                            text(host_text).width(Length::FillPortion(3)).size(theme::TYPE_SECTION).color(text_primary),
                            text(process_text).width(Length::FillPortion(2)).size(theme::TYPE_BTN_SM).color(text_muted),
                            text(&conn.metadata.network).width(Length::FillPortion(1)).size(theme::TYPE_SECTION).color(theme::SUCCESS),
                            text(chains_text).width(Length::FillPortion(2)).size(theme::TYPE_SECTION).color(text_muted),
                            text(truncate_chars(&conn.rule, 16)).width(Length::FillPortion(1)).size(theme::TYPE_SECTION).color(text_primary),
                            text(dl_text).width(Length::FillPortion(1)).size(theme::TYPE_SECTION).color(text_primary),
                            text(ul_text).width(Length::FillPortion(1)).size(theme::TYPE_SECTION).color(text_primary),
                            container(close_btn).width(Length::FillPortion(1)).center_x(Length::FillPortion(1))
                        ]
                        .align_y(Alignment::Center)
                        .spacing(crate::ui::SP_12)
                        .padding(10)
                        .into()
                    } else {
                        let net_rule = format!("{} · {}", conn.metadata.network, truncate_chars(&conn.rule, 14));
                        let traffic = format!("↓{} ↑{}", dl_text, ul_text);
                        row![
                            text(host_text).width(Length::FillPortion(3)).size(theme::TYPE_SECTION).color(text_primary),
                            text(process_text).width(Length::FillPortion(2)).size(theme::TYPE_BTN_SM).color(text_muted),
                            text(net_rule).width(Length::FillPortion(2)).size(theme::TYPE_BTN_SM).color(theme::SUCCESS),
                            text(chains_text).width(Length::FillPortion(2)).size(theme::TYPE_BTN_SM).color(text_muted),
                            text(traffic).width(Length::FillPortion(2)).size(theme::TYPE_BTN_SM).color(text_primary),
                            container(close_btn).width(Length::FillPortion(1)).center_x(Length::FillPortion(1))
                        ]
                        .align_y(Alignment::Center)
                        .spacing(crate::ui::SP_12)
                        .padding(10)
                        .into()
                    };
                    
                    // Hover-friendly row surface with zebra pattern
                    list = list.push(
                        container(row_content)
                            .width(Length::Fill)
                            .style(move |t| theme::list_item_zebra(t, false, false, idx % 2 == 1)),
                    );
                    
                    if idx + 1 < len {
                        let separator = container(Space::new())
                            .height(1)
                            .width(Length::Fill)
                            .style(|t| container::Style {
                                background: Some(iced::Background::Color(theme::border_color(t))),
                                ..Default::default()
                            });
                        list = list.push(separator);
                    }
                }
            }
            
            let header_styled = container(header)
                .padding([12, 10])
                .style(|t| container::Style {
                    background: Some(iced::Background::Color(theme::elevated_surface(t))),
                    border: iced::Border {
                        color: theme::border_color(t),
                        width: 1.0,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                });
 
            let list_content: Element<'_, Message> = container(
                column![
                    header_styled,
                    scrollable(list).style(theme::scrollbar_style).height(Length::Fill)
                ]
            )
            .style(theme::card_bg)
            .height(Length::Fill)
            .width(Length::Fill)
            .into();
            
            let col = column![page_title, list_content]
                .spacing(crate::ui::SP_20)
                .width(Length::Fill)
                .height(Length::Fill);

            crate::ui::page_body_fixed_with_pad(col.into(), false)
        }
    });

    main_content.into()
}
