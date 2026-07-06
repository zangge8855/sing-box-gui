use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::ui::theme;
use crate::api::Connection;

pub fn render<'a>(
    gui_config: &'a crate::state::GuiConfig,
    active_connections: &'a Vec<Connection>,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    use crate::ui::i18n::tr;
    
    // Build Header
    let header = row![
        text(tr(lang, "host")).width(Length::FillPortion(3)).color(theme::TEXT_MUTED).size(14),
        text(tr(lang, "network")).width(Length::FillPortion(1)).color(theme::TEXT_MUTED).size(14),
        text(tr(lang, "chains")).width(Length::FillPortion(2)).color(theme::TEXT_MUTED).size(14),
        text(tr(lang, "rule")).width(Length::FillPortion(1)).color(theme::TEXT_MUTED).size(14),
        text(tr(lang, "download")).width(Length::FillPortion(1)).color(theme::TEXT_MUTED).size(14),
        text(tr(lang, "upload")).width(Length::FillPortion(1)).color(theme::TEXT_MUTED).size(14),
        Space::new().width(Length::FillPortion(1))
    ]
    .spacing(10)
    .padding([0, 10]);

    // Build List
    let mut list = column!().spacing(8);
    if active_connections.is_empty() {
        list = list.push(
            container(text(tr(lang, "no_active_connections")).color(theme::TEXT_MUTED))
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding(40)
        );
    } else {
        for conn in active_connections {
            let host_text = if !conn.metadata.host.is_empty() {
                conn.metadata.host.clone()
            } else {
                conn.metadata.destination_ip.clone()
            };
            
            let chains_text = if conn.chains.is_empty() {
                "Direct".to_string()
            } else {
                conn.chains.join(" ➔ ")
            };
            
            let dl_text = format!("{:.1} KB", conn.download as f64 / 1024.0);
            let ul_text = format!("{:.1} KB", conn.upload as f64 / 1024.0);
            
            let close_btn = button(
                text(tr(lang, "close_conn")).size(12)
            )
            .style(theme::button_danger)
            .padding([4, 8])
            .on_press(Message::CloseConnection(conn.id.clone()));
            
            let row_content = row![
                text(host_text).width(Length::FillPortion(3)).size(13),
                text(&conn.metadata.network).width(Length::FillPortion(1)).size(13).color(theme::ACCENT_GREEN),
                text(chains_text).width(Length::FillPortion(2)).size(13).color(theme::TEXT_MUTED),
                text(&conn.rule).width(Length::FillPortion(1)).size(13),
                text(dl_text).width(Length::FillPortion(1)).size(13),
                text(ul_text).width(Length::FillPortion(1)).size(13),
                container(close_btn).width(Length::FillPortion(1)).center_x(Length::FillPortion(1))
            ]
            .align_y(Alignment::Center)
            .spacing(10)
            .padding(10);
            
            list = list.push(container(row_content).style(theme::card_bg));
        }
    }
    
    let content = column![
        text(tr(lang, "tab_connections")).size(24).color(theme::TEXT_PRIMARY),
        container(header).padding([10, 0]),
        scrollable(list).height(Length::Fill)
    ]
    .spacing(20);
    
    container(content).padding(20).into()
}
