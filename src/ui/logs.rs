use iced::widget::{button, container, scrollable, text, Column};
use iced::{Element, Font, Length, Alignment};
use crate::message::Message;
use crate::ui::theme;
use crate::ui::{page_header, page_shell_fixed};
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
    
    let text_muted = theme::text_muted(theme);
    
    let clear_logs_btn = button(text(tr(lang, "clear_logs")).size(14))
        .padding([8, 16])
        .style(theme::button_secondary)
        .on_press(Message::ClearLogs);
    
    let mut logs_col = Column::new().spacing(4);
    
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
    
    let log_terminal = if log_lines.is_empty() {
        container(
            text(tr(lang, "no_logs"))
                .color(text_muted)
                .font(Font::MONOSPACE)
                .size(13)
        )
        .padding(15)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(theme::console_bg)
    } else {
        container(
            scrollable(logs_col)
                .id(get_logs_scrollable_id().clone())
                .height(Length::Fill)
                .width(Length::Fill)
        )
        .padding(15)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::console_bg)
    };
    
    let header = page_header("tab_logs", lang, Some(clear_logs_btn.into()), theme);
    page_shell_fixed(header, log_terminal.into())
}
