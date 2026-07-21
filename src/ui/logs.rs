use crate::message::Message;
use crate::state::LogFilter;
use crate::ui::page_header;
use crate::ui::theme;
use iced::widget::{
    Column, button, column, container, responsive, row, scrollable, text, text_input,
};
use iced::{Alignment, Element, Length};
use std::sync::OnceLock;

pub fn get_logs_scrollable_id() -> &'static iced::widget::Id {
    static ID: OnceLock<iced::widget::Id> = OnceLock::new();
    ID.get_or_init(iced::widget::Id::unique)
}

pub fn render<'a>(
    gui_config: &'a crate::state::GuiConfig,
    log_lines: &'a std::collections::VecDeque<String>,
    log_filter: LogFilter,
    log_search: &'a str,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    use crate::ui::i18n::tr;

    let theme_cloned = theme.clone();
    let search_cloned = log_search.to_string();
    let total_lines = log_lines.len();
    // Filter once per view rebuild (not full buffer clone when search/filter reduces size).
    let q_lower = log_search.to_lowercase();
    const MAX_VISIBLE_LOG_LINES: usize = 400;
    let mut filtered_lines: Vec<&String> = log_lines
        .iter()
        .filter(|line| log_filter.matches(line))
        .filter(|line| q_lower.is_empty() || line.to_lowercase().contains(&q_lower))
        .collect();
    if filtered_lines.len() > MAX_VISIBLE_LOG_LINES {
        filtered_lines.drain(..filtered_lines.len() - MAX_VISIBLE_LOG_LINES);
    }

    let main_content = responsive(move |size| {
        let theme = &theme_cloned;
        let is_compact = size.width < crate::ui::PAGE_COMPACT_W;
        let text_muted = theme::text_muted(theme);

        let filter_btn = |f: LogFilter, key: &'static str| {
            let active = log_filter == f;
            let mut b = button(text(tr(lang, key)).size(theme::TYPE_BTN_SM))
                .padding(theme::BTN_PAD_SM)
                .style(if active {
                    theme::button_primary
                } else {
                    theme::button_secondary
                });
            if !active {
                b = b.on_press(Message::LogFilterChanged(f));
            }
            b
        };

        let search_input = text_input(tr(lang, "log_search_placeholder"), &search_cloned)
            .on_input(Message::LogSearchChanged)
            .padding(8)
            .width(if is_compact {
                Length::Fill
            } else {
                Length::Fixed(theme::SEARCH_WIDTH)
            })
            .style(theme::input_field);

        let clear_logs_btn = button(text(tr(lang, "clear_logs")).size(theme::TYPE_BTN_MD))
            .padding(theme::BTN_PAD_MD)
            .style(theme::button_secondary)
            .on_press(Message::ClearLogs);

        let export_btn = button(text(tr(lang, "export_logs")).size(theme::TYPE_BTN_MD))
            .padding(theme::BTN_PAD_MD)
            .style(theme::button_secondary)
            .on_press(Message::ExportLogs);

        let filters = row![
            filter_btn(LogFilter::All, "log_filter_all"),
            filter_btn(LogFilter::Info, "log_filter_info"),
            filter_btn(LogFilter::Warn, "log_filter_warn"),
            filter_btn(LogFilter::Error, "log_filter_error"),
        ]
        .spacing(theme::SP_8);

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
                .spacing(theme::SP_12)
                .align_y(Alignment::Center)
                .into()
        };

        let mut logs_col = Column::new().spacing(4);
        let shown = filtered_lines.len();

        for line in &filtered_lines {
            let line_upper = line.to_uppercase();
            let line_color = if line_upper.contains("ERROR")
                || line_upper.contains("FATAL")
                || line_upper.contains("FAILED")
            {
                theme::DANGER
            } else if line_upper.contains("WARN") || line_upper.contains("WARNING") {
                theme::WARNING
            } else if line_upper.contains("INFO") {
                // Info is informational, not a success outcome
                theme::ACCENT_BLUE
            } else {
                text_muted
            };

            logs_col = logs_col.push(
                text(line.as_str())
                    .font(theme::mono_font())
                    .color(line_color)
                    .size(theme::TYPE_MONO)
                    .width(Length::Fill),
            );
        }

        let log_terminal = if total_lines == 0 {
            let cta = button(text(tr(lang, "btn_start_core_short")).size(theme::TYPE_BTN_MD))
                .padding(theme::BTN_PAD_MD)
                .style(theme::button_primary)
                .on_press(Message::ToggleCore);
            container(crate::ui::empty_state(
                tr(lang, "no_logs"),
                Some(tr(lang, "empty_logs_start_hint")),
                Some(cta.into()),
                theme,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(theme::console_bg)
        } else if shown == 0 {
            let cta = button(text(tr(lang, "btn_clear_search")).size(theme::TYPE_BTN_MD))
                .padding(theme::BTN_PAD_MD)
                .style(theme::button_secondary)
                .on_press(Message::LogSearchChanged(String::new()));
            container(crate::ui::empty_state(
                tr(lang, "no_matching_logs"),
                None,
                Some(cta.into()),
                theme,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(theme::console_bg)
        } else {
            container(
                scrollable(logs_col)
                    .id(get_logs_scrollable_id().clone())
                    .style(theme::scrollbar_style)
                    .height(Length::Fill)
                    .width(Length::Fill),
            )
            .padding(theme::CARD_PAD_DENSE)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::console_bg)
        };

        let header = page_header("tab_logs", lang, Some(actions), theme, is_compact);

        let col = column![header, log_terminal]
            .spacing(crate::ui::SP_20)
            .width(Length::Fill)
            .height(Length::Fill);

        crate::ui::page_body_fixed_with_pad(col.into(), is_compact)
    });

    main_content.into()
}
