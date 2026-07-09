use iced::widget::{button, column, container, row, scrollable, text, text_input, Space, responsive};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::ui::theme;
use crate::api::Connection;
use crate::ui::{page_header, CONNECTIONS_TABLE_W, CONNECTIONS_WIDE_W};
use crate::ui::util::{format_size, truncate_chars};

pub fn render<'a>(
    gui_config: &'a crate::state::GuiConfig,
    active_connections: &'a [Connection],
    search_query: &'a str,
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
        let filtered_connections: Vec<&Connection> = if query_str.trim().is_empty() {
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
        
        let search_input = text_input(tr(lang, "placeholder_connections_search"), &query_str)
            .on_input(Message::ConnectionsSearchChanged)
            .padding(8)
            .width(if is_compact { Length::Fill } else { Length::Fixed(theme::SEARCH_WIDTH) })
            .style(theme::input_field);

        let close_all_btn = button(text(tr(lang, "close_all_conn")).size(13))
            .padding([8, 14])
            .style(theme::button_danger)
            .on_press(Message::CloseAllConnections);

        let header_actions: Element<'_, Message> = if is_compact {
            column![
                search_input.width(Length::Fill),
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
                        text(tr(lang, "close_conn")).size(11)
                    )
                    .style(theme::button_danger)
                    .padding([4, 8])
                    .on_press(Message::CloseConnection(conn.id.clone()));
                    
                    let card = container(
                        column![
                            row![
                                text(truncate_chars(&host_full, 48))
                                    .color(text_primary)
                                    .size(13)
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
                            .spacing(8)
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
                        .spacing(8)
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
            let make_hdr_text = |s: &'static str| {
                text(tr(lang, s))
                    .color(text_muted)
                    .size(13)
                    .font(iced::Font {
                        weight: iced::font::Weight::Semibold,
                        ..Default::default()
                    })
            };

            let header: Element<'_, Message> = if is_wide {
                row![
                    make_hdr_text("host").width(Length::FillPortion(3)),
                    make_hdr_text("col_process").width(Length::FillPortion(2)),
                    make_hdr_text("network").width(Length::FillPortion(1)),
                    make_hdr_text("chains").width(Length::FillPortion(2)),
                    make_hdr_text("rule").width(Length::FillPortion(1)),
                    make_hdr_text("download").width(Length::FillPortion(1)),
                    make_hdr_text("upload").width(Length::FillPortion(1)),
                    Space::new().width(Length::FillPortion(1))
                ]
                .spacing(10)
                .padding([0, 10])
                .into()
            } else {
                row![
                    make_hdr_text("host").width(Length::FillPortion(3)),
                    make_hdr_text("col_process").width(Length::FillPortion(2)),
                    make_hdr_text("network").width(Length::FillPortion(2)),
                    make_hdr_text("chains").width(Length::FillPortion(2)),
                    text("↓ / ↑")
                        .color(text_muted)
                        .size(13)
                        .font(iced::Font {
                            weight: iced::font::Weight::Semibold,
                            ..Default::default()
                        })
                        .width(Length::FillPortion(2)),
                    Space::new().width(Length::FillPortion(1))
                ]
                .spacing(10)
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
                        text(tr(lang, "close_conn")).size(12)
                    )
                    .style(theme::button_danger)
                    .padding([4, 8])
                    .on_press(Message::CloseConnection(conn.id.clone()));
                    
                    let row_content: Element<'_, Message> = if is_wide {
                        row![
                            text(host_text).width(Length::FillPortion(3)).size(13).color(text_primary),
                            text(process_text).width(Length::FillPortion(2)).size(12).color(text_muted),
                            text(&conn.metadata.network).width(Length::FillPortion(1)).size(13).color(theme::SUCCESS),
                            text(chains_text).width(Length::FillPortion(2)).size(13).color(text_muted),
                            text(truncate_chars(&conn.rule, 16)).width(Length::FillPortion(1)).size(13).color(text_primary),
                            text(dl_text).width(Length::FillPortion(1)).size(13).color(text_primary),
                            text(ul_text).width(Length::FillPortion(1)).size(13).color(text_primary),
                            container(close_btn).width(Length::FillPortion(1)).center_x(Length::FillPortion(1))
                        ]
                        .align_y(Alignment::Center)
                        .spacing(10)
                        .padding(10)
                        .into()
                    } else {
                        let net_rule = format!("{} · {}", conn.metadata.network, truncate_chars(&conn.rule, 14));
                        let traffic = format!("↓{} ↑{}", dl_text, ul_text);
                        row![
                            text(host_text).width(Length::FillPortion(3)).size(13).color(text_primary),
                            text(process_text).width(Length::FillPortion(2)).size(12).color(text_muted),
                            text(net_rule).width(Length::FillPortion(2)).size(12).color(theme::SUCCESS),
                            text(chains_text).width(Length::FillPortion(2)).size(12).color(text_muted),
                            text(traffic).width(Length::FillPortion(2)).size(12).color(text_primary),
                            container(close_btn).width(Length::FillPortion(1)).center_x(Length::FillPortion(1))
                        ]
                        .align_y(Alignment::Center)
                        .spacing(10)
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
                .spacing(20)
                .width(Length::Fill)
                .height(Length::Fill);

            crate::ui::page_body_fixed_with_pad(col.into(), false)
        }
    });

    main_content.into()
}
