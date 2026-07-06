use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::{Alignment, Element, Font, Length};
use crate::message::Message;
use crate::ui::theme;

pub fn render<'a>(
    log_lines: &'a [String],
) -> Element<'a, Message> {
    
    let header = row![
        text("Core Logs").size(24).color(theme::TEXT_PRIMARY).width(Length::Fill),
        button(text("Clear Logs").size(14))
            .padding([8, 16])
            .style(theme::button_secondary)
            .on_press(Message::NewLogLine("CLEAR_LOG_BUFFER".to_string())) // Clear logs message trigger
    ]
    .spacing(20)
    .align_y(Alignment::Center);
    
    let mut logs_col = Column::new().spacing(4);
    
    if log_lines.is_empty() {
        logs_col = logs_col.push(
            text("Log buffer is empty. Start the sing-box core to view live logs.")
                .color(theme::TEXT_MUTED)
                .font(Font::MONOSPACE)
                .size(12)
        );
    } else {
        for line in log_lines {
            // Determine log line color based on levels
            let line_color = if line.contains("ERROR") || line.contains("FATAL") || line.contains("failed") {
                theme::DANGER
            } else if line.contains("WARN") || line.contains("warning") {
                theme::WARNING
            } else if line.contains("INFO") {
                theme::SUCCESS
            } else {
                theme::TEXT_MUTED
            };
            
            logs_col = logs_col.push(
                text(line)
                    .color(line_color)
                    .font(Font::MONOSPACE)
                    .size(11)
            );
        }
    }
    
    let log_terminal = container(
        scrollable(logs_col)
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
