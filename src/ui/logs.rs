use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::{Alignment, Element, Font, Length};
use crate::message::Message;
use crate::ui::theme;
use std::sync::OnceLock;

pub fn get_logs_scrollable_id() -> &'static iced::widget::Id {
    static ID: OnceLock<iced::widget::Id> = OnceLock::new();
    ID.get_or_init(iced::widget::Id::unique)
}

pub fn render<'a>(
    gui_config: &'a crate::state::GuiConfig,
    log_lines: &'a [String],
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    use crate::ui::i18n::tr;
    
    let text_primary = theme::text_primary(theme);
    let text_muted = theme::text_muted(theme);
    
    let header = row![
        text(tr(lang, "console_logs")).size(24).color(text_primary).width(Length::Fill),
        button(text(tr(lang, "clear_logs")).size(14))
            .padding([8, 16])
            .style(theme::button_secondary)
            .on_press(Message::ClearLogs)
    ]
    .spacing(20)
    .align_y(Alignment::Center);
    
    let mut logs_col = Column::new().spacing(4);
    
    if log_lines.is_empty() {
        logs_col = logs_col.push(
            text(tr(lang, "no_logs"))
                .color(text_muted)
                .font(Font::MONOSPACE)
                .size(13)
        );
    } else {
        for line in log_lines {
            let line_upper = line.to_uppercase();
            let line_color = if line_upper.contains("ERROR") || line_upper.contains("FATAL") || line_upper.contains("FAILED") {
                theme::DANGER
            } else if line_upper.contains("WARN") || line_upper.contains("WARNING") {
                theme::WARNING
            } else if line_upper.contains("INFO") {
                theme::SUCCESS
            } else {
                text_muted
            };
            
            logs_col = logs_col.push(
                text(line)
                    .color(line_color)
                    .font(Font::MONOSPACE)
                    .size(12)
            );
        }
    }
    
    let log_terminal = container(
        scrollable(logs_col)
            .id(get_logs_scrollable_id().clone())
            .height(Length::Fill)
            .width(Length::Fill)
    )
    .padding(15)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(theme::console_bg);
    
    container(
        column![
            header,
            log_terminal
        ]
        .spacing(20)
        .height(Length::Fill)
    )
    .padding(20)
    .height(Length::Fill)
    .into()
}
