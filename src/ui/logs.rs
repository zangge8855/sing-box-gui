use iced::widget::{button, container, scrollable, text, text_input, Column, responsive, column, row};
use iced::{Element, Length, Alignment};
use crate::message::Message;
use crate::state::LogFilter;
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
    log_filter: LogFilter,
    log_search: &'a str,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    use crate::ui::i18n::tr;
    
    let theme_cloned = theme.clone();
    let log_lines_cloned = log_lines.to_vec();
    let search_cloned = log_search.to_string();
    
    let main_content = responsive(move |size| {
        let theme = &theme_cloned;
        let is_compact = size.width < 750.0;
        let text_muted = theme::text_muted(theme);

        let filter_btn = |f: LogFilter, key: &'static str| {
            let active = log_filter == f;
            let mut b = button(text(tr(lang, key)).size(12))
                .padding([6, 12])
                .style(if active { theme::button_primary } else { theme::button_secondary });
            if !active {
                b = b.on_press(Message::LogFilterChanged(f));
            }
            b
        };

        let search_input = text_input(tr(lang, "log_search_placeholder"), &search_cloned)
            .on_input(Message::LogSearchChanged)
            .padding(8)
            .width(if is_compact { Length::Fill } else { Length::Fixed(220.0) })
            .style(theme::input_field);

        let clear_logs_btn = button(text(tr(lang, "clear_logs")).size(13))
            .padding([8, 14])
            .style(theme::button_secondary)
            .on_press(Message::ClearLogs);

        let export_btn = button(text(tr(lang, "export_logs")).size(13))
            .padding([8, 14])
            .style(theme::button_secondary)
            .on_press(Message::ExportLogs);

        let filters = row![
            filter_btn(LogFilter::All, "log_filter_all"),
            filter_btn(LogFilter::Info, "log_filter_info"),
            filter_btn(LogFilter::Warn, "log_filter_warn"),
            filter_btn(LogFilter::Error, "log_filter_error"),
        ]
        .spacing(6);

        let actions: Element<'_, Message> = if is_compact {
            column![
                search_input,
                filters,
                row![clear_logs_btn, export_btn].spacing(8)
            ]
            .spacing(8)
            .width(Length::Fill)
            .into()
        } else {
            row![search_input, filters, clear_logs_btn, export_btn]
                .spacing(10)
                .align_y(Alignment::Center)
                .into()
        };
        
        let mut logs_col = Column::new().spacing(4);
        let q = search_cloned.to_lowercase();
        let mut shown = 0usize;
        
        for line in &log_lines_cloned {
            if !log_filter.matches(line) {
                continue;
            }
            if !q.is_empty() && !line.to_lowercase().contains(&q) {
                continue;
            }
            shown += 1;
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
                column![
                    text(tr(lang, "no_logs")).color(text_muted).size(14),
                    text(tr(lang, "empty_logs_hint")).color(theme::text_tertiary(theme)).size(12),
                ]
                .spacing(8)
                .align_x(Alignment::Center)
            )
            .padding(15)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(theme::console_bg)
        } else if shown == 0 {
            container(
                text(tr(lang, "no_matching_logs"))
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
        
        let header = page_header("tab_logs", lang, Some(actions), theme, is_compact);
        
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
