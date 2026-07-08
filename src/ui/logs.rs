use iced::widget::{button, container, scrollable, text, Column, responsive, column};
use iced::{Element, Length, Alignment};
use crate::message::Message;
use crate::ui::theme;
use crate::ui::{page_header, page_padding};
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
    
    let theme_cloned = theme.clone();
    let log_lines_cloned = log_lines.to_vec();
    
    let main_content = responsive(move |size| {
        let theme = &theme_cloned;
        let is_compact = size.width < 750.0;
        let text_muted = theme::text_muted(theme);
        
        let clear_logs_btn = button(text(tr(lang, "clear_logs")).size(14))
            .padding([8, 16])
            .style(theme::button_secondary)
            .on_press(Message::ClearLogs);
        
        let mut logs_col = Column::new().spacing(4);
        
        for line in &log_lines_cloned {
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
                text(line.clone())
                    .color(line_color)
                    .size(12)
                    .width(Length::Fill)
            );
        }
        
        let log_terminal = if log_lines_cloned.is_empty() {
            container(
                text(tr(lang, "no_logs"))
                    .color(text_muted)
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
        
        let header = page_header("tab_logs", lang, Some(clear_logs_btn.into()), theme, is_compact);
        
        let col = column![header, log_terminal]
            .spacing(20)
            .width(Length::Fill)
            .height(Length::Fill);

        container(col)
            .width(Length::Fill)
            .max_width(1200.0)
            .center_x(Length::Fill)
            .padding(page_padding())
            .into()
    });
    
    main_content.into()
}
