use iced::widget::{button, column, container, row, scrollable, text, text_input, Space, responsive};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::ui::theme;
use crate::api::Connection;
use crate::ui::page_header;
use crate::ui::util::format_size;

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
        let is_compact = size.width < 850.0;
        
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
                    host_text.to_lowercase().contains(&q)
                        || conn.metadata.destination_ip.to_lowercase().contains(&q)
                        || conn.chains.iter().any(|c| c.to_lowercase().contains(&q))
                        || conn.rule.to_lowercase().contains(&q)
                        || conn.metadata.network.to_lowercase().contains(&q)
                })
                .collect()
        };
        
        let search_input = text_input(tr(lang, "placeholder_connections_search"), &query_str)
            .on_input(Message::ConnectionsSearchChanged)
            .padding(8)
            .width(if is_compact { Length::Fill } else { Length::Fixed(280.0) })
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
                list = list.push(
                    container(text(empty_msg).color(text_muted).size(13))
                        .width(Length::Fill)
                        .center_x(Length::Fill)
                        .padding(40)
                );
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
                                text(host_full)
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
                                container(text(&conn.metadata.network).size(9).color(theme::ACCENT_GREEN))
                                    .padding([2, 6])
                                    .style(|t| theme::tinted_banner(t, theme::ACCENT_GREEN)),
                                container(text(&conn.rule).size(9).color(theme::ACCENT_PURPLE))
                                    .padding([2, 6])
                                    .style(|t| theme::tinted_banner(t, theme::ACCENT_PURPLE)),
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                            
                            text(format!("{}: {}", tr(lang, "chains"), chains_text))
                                .color(text_muted)
                                .size(11),
                                
                            row![
                                text(format!("↓ {}", dl_text)).color(theme::ACCENT_BLUE).size(11),
                                Space::new().width(12),
                                text(format!("↑ {}", ul_text)).color(theme::ACCENT_PURPLE).size(11)
                            ]
                            .align_y(Alignment::Center)
                        ]
                        .spacing(8)
                    )
                    .padding(14)
                    .width(Length::Fill)
                    .style(theme::card_bg);
                    
                    list = list.push(card);
                }
            }
            let col = column![page_title, scrollable(list).height(Length::Fill)]
                .spacing(20)
                .width(Length::Fill)
                .height(Length::Fill);
            container(col)
                .width(Length::Fill)
                .max_width(1200.0)
                .center_x(Length::Fill)
                .padding(crate::ui::page_padding())
                .into()
        } else {
            // Desktop Layout: 7-Column clean aligned table
            let header = row![
                text(tr(lang, "host")).width(Length::FillPortion(3)).color(text_muted).size(14),
                text(tr(lang, "network")).width(Length::FillPortion(1)).color(text_muted).size(14),
                text(tr(lang, "chains")).width(Length::FillPortion(2)).color(text_muted).size(14),
                text(tr(lang, "rule")).width(Length::FillPortion(1)).color(text_muted).size(14),
                text(tr(lang, "download")).width(Length::FillPortion(1)).color(text_muted).size(14),
                text(tr(lang, "upload")).width(Length::FillPortion(1)).color(text_muted).size(14),
                Space::new().width(Length::FillPortion(1))
            ]
            .spacing(10)
            .padding([0, 10]);
            
            let mut list = column!().spacing(0);
            if filtered_connections.is_empty() {
                let empty_msg = if query_str.trim().is_empty() {
                    tr(lang, "no_active_connections")
                } else {
                    tr(lang, "no_matching_connections")
                };
                list = list.push(
                    container(text(empty_msg).color(text_muted))
                        .width(Length::Fill)
                        .center_x(Length::Fill)
                        .padding(40)
                );
            } else {
                let len = filtered_connections.len();
                for (idx, conn) in filtered_connections.into_iter().enumerate() {
                    let host_full = if !conn.metadata.host.is_empty() {
                        conn.metadata.host.clone()
                    } else {
                        conn.metadata.destination_ip.clone()
                    };
                    let host_text = if host_full.chars().count() > 48 {
                        let head: String = host_full.chars().take(45).collect();
                        format!("{}...", head)
                    } else {
                        host_full
                    };
                    
                    let chains_text = if conn.chains.is_empty() {
                        tr(lang, "direct_chain").to_string()
                    } else {
                        conn.chains.join(" ➔ ")
                    };
                    
                    let dl_text = format_size(conn.download);
                    let ul_text = format_size(conn.upload);
                    
                    let close_btn = button(
                        text(tr(lang, "close_conn")).size(12)
                    )
                    .style(theme::button_danger)
                    .padding([4, 8])
                    .on_press(Message::CloseConnection(conn.id.clone()));
                    
                    let row_content = row![
                        text(host_text).width(Length::FillPortion(3)).size(13).color(text_primary),
                        text(&conn.metadata.network).width(Length::FillPortion(1)).size(13).color(theme::ACCENT_GREEN),
                        text(chains_text).width(Length::FillPortion(2)).size(13).color(text_muted),
                        text(&conn.rule).width(Length::FillPortion(1)).size(13).color(text_primary),
                        text(dl_text).width(Length::FillPortion(1)).size(13).color(text_primary),
                        text(ul_text).width(Length::FillPortion(1)).size(13).color(text_primary),
                        container(close_btn).width(Length::FillPortion(1)).center_x(Length::FillPortion(1))
                    ]
                    .align_y(Alignment::Center)
                    .spacing(10)
                    .padding(10);
                    
                    list = list.push(container(row_content));
                    
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
                    scrollable(list).height(Length::Fill)
                ]
            )
            .style(theme::card_bg)
            .height(Length::Fill)
            .width(Length::Fill)
            .into();
            
            let col = column![page_title, list_content].spacing(20).width(Length::Fill).height(Length::Fill);

            container(col)
                .width(Length::Fill)
                .max_width(1200.0)
                .center_x(Length::Fill)
                .padding(crate::ui::page_padding())
                .into()
        }
    });

    main_content.into()
}
