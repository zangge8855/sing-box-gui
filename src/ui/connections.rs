use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::ui::theme;
use crate::api::Connection;

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn render<'a>(
    gui_config: &'a crate::state::GuiConfig,
    active_connections: &'a [Connection],
    search_query: &'a str,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    use crate::ui::i18n::tr;
    
    let text_primary = theme::text_primary(theme);
    let text_muted = theme::text_muted(theme);
    
    // Build Header
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

    // Filter connections
    let filtered_connections: Vec<&Connection> = if search_query.trim().is_empty() {
        active_connections.iter().collect()
    } else {
        let q = search_query.to_lowercase();
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

    // Build List
    let mut list = column!().spacing(0);
    if filtered_connections.is_empty() {
        let empty_msg = if search_query.trim().is_empty() {
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
            let host_text = if !conn.metadata.host.is_empty() {
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
            
            // Add subtle divider except for the last item
            if idx + 1 < len {
                let separator = container(Space::new())
                    .height(1)
                    .width(Length::Fill)
                    .style(|theme| container::Style {
                        background: Some(iced::Background::Color(if theme::is_dark(theme) {
                            theme::BORDER_DARK
                        } else {
                            theme::BORDER_LIGHT
                        })),
                        ..Default::default()
                    });
                list = list.push(separator);
            }
        }
    }
    
    // Header styled background inside the card
    let header_styled = container(header)
        .padding([12, 10])
        .style(|theme| container::Style {
            background: Some(iced::Background::Color(if theme::is_dark(theme) {
                theme::CARD_LIGHT
            } else {
                theme::SIDEBAR_BG_LIGHT
            })),
            border: iced::Border {
                color: if theme::is_dark(theme) { theme::BORDER_DARK } else { theme::BORDER_LIGHT },
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        });

    let table_container = container(
        column![
            header_styled,
            scrollable(list).height(Length::Fill)
        ]
    )
    .style(theme::card_bg)
    .height(Length::Fill)
    .width(Length::Fill);

    let search_input = text_input(tr(lang, "placeholder_connections_search"), search_query)
        .on_input(Message::ConnectionsSearchChanged)
        .padding(8)
        .width(280)
        .style(theme::input_field);

    let title_row = row![
        text(tr(lang, "tab_connections")).size(24).color(text_primary).width(Length::Fill),
        search_input
    ]
    .align_y(Alignment::Center)
    .spacing(20);

    let content = column![
        title_row,
        table_container
    ]
    .spacing(20);
    
    container(content).padding(20).into()
}
