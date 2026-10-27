use crate::message::Message;
use crate::state::LogFilter;
use crate::ui::page_header;
use crate::ui::theme;
use iced::widget::text::Span;
use iced::widget::{
    Column, Space, button, column, container, responsive, rich_text, row, scrollable, text,
    text_input, toggler,
};
use iced::{Alignment, Element, Length};
use std::ops::Range;
use std::sync::OnceLock;

pub fn get_logs_scrollable_id() -> &'static iced::widget::Id {
    static ID: OnceLock<iced::widget::Id> = OnceLock::new();
    ID.get_or_init(iced::widget::Id::unique)
}

/// Below this page width the toolbar stacks under the title.
const LOGS_TOOLBAR_INLINE_W: f32 = 1100.0;
/// Below this the second toolbar row wraps instead of pushing the actions right.
const LOGS_TOOLBAR_SPREAD_W: f32 = 700.0;

/// Locates the severity word sing-box prints near the start of a line
/// (`+0800 2026-10-27 14:30:00 INFO [..] ...`) so the line can be set as
/// timestamp / level / message rather than painted in a single colour.
fn level_token(line: &str) -> Option<(Range<usize>, u8)> {
    let mut offset = 0;
    for word in line.split(' ').take(5) {
        let severity = match word {
            "ERROR" | "FATAL" | "PANIC" => Some(3),
            "WARN" | "WARNING" => Some(2),
            "INFO" => Some(1),
            "DEBUG" | "TRACE" => Some(0),
            _ => None,
        };
        if let Some(severity) = severity {
            return Some((offset..offset + word.len(), severity));
        }
        offset += word.len() + 1;
    }
    None
}

/// Lines without a level tag (GUI notices, panics) fall back to keywords.
fn keyword_severity(uppercase: &str) -> u8 {
    if uppercase.contains("ERROR") || uppercase.contains("FATAL") || uppercase.contains("FAILED") {
        3
    } else if uppercase.contains("WARN") {
        2
    } else if uppercase.contains("INFO") {
        1
    } else {
        0
    }
}

pub fn render<'a>(
    gui_config: &'a crate::state::GuiConfig,
    log_lines: &'a std::collections::VecDeque<String>,
    log_filter: LogFilter,
    log_search: &'a str,
    logs_follow: bool,
    core_running: bool,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let lang = gui_config.language;
    use crate::ui::i18n::tr;

    let theme_cloned = theme.clone();
    let search_cloned = log_search.to_string();
    let total_lines = log_lines.len();
    // Precompute case-folded text and severity once per view rebuild. The
    // responsive closure may be evaluated repeatedly while resizing, so it
    // should only assemble widgets, not re-scan every log line.
    let q_upper = log_search.to_uppercase();
    const MAX_VISIBLE_LOG_LINES: usize = 400;
    let mut filtered_lines: Vec<(&str, u8, Option<Range<usize>>)> = log_lines
        .iter()
        .filter_map(|line| {
            let uppercase = line.to_uppercase();
            if !log_filter.matches_upper(&uppercase) {
                return None;
            }
            if !q_upper.is_empty() && !uppercase.contains(&q_upper) {
                return None;
            }
            // The level tag wins over keywords, so an INFO line that merely
            // mentions "error" is not painted as a failure.
            Some(match level_token(line) {
                Some((range, severity)) => (line.as_str(), severity, Some(range)),
                None => (line.as_str(), keyword_severity(&uppercase), None),
            })
        })
        .collect();
    if filtered_lines.len() > MAX_VISIBLE_LOG_LINES {
        filtered_lines.drain(..filtered_lines.len() - MAX_VISIBLE_LOG_LINES);
    }

    let main_content = responsive(move |size| {
        let theme = &theme_cloned;
        let stacked = size.width < LOGS_TOOLBAR_INLINE_W;
        let text_primary = theme::text_primary(theme);
        let text_muted = theme::text_muted(theme);
        let text_tertiary = theme::text_tertiary(theme);

        // Level filter as one segmented control, matching the dashboard's
        // routing switch, instead of four loose buttons.
        let segment =
            |f: LogFilter, key: &'static str| {
                let active = log_filter == f;
                let mut b = button(text(tr(lang, key)).size(theme::TYPE_BTN_SM).font(
                    theme::ui_font(if active {
                        iced::font::Weight::Semibold
                    } else {
                        iced::font::Weight::Normal
                    }),
                ))
                .padding([6, 12])
                .style(theme::button_tab(active));
                if !active {
                    b = b.on_press(Message::LogFilterChanged(f));
                }
                b
            };
        let filters = container(
            row![
                segment(LogFilter::All, "log_filter_all"),
                segment(LogFilter::Info, "log_filter_info"),
                segment(LogFilter::Warn, "log_filter_warn"),
                segment(LogFilter::Error, "log_filter_error"),
            ]
            .spacing(3),
        )
        .padding(3)
        .style(theme::segmented_track);

        let search_input = text_input(tr(lang, "log_search_placeholder"), &search_cloned)
            .on_input(Message::LogSearchChanged)
            .padding(8)
            .width(if stacked {
                Length::Fill
            } else {
                Length::Fixed(theme::SEARCH_WIDTH)
            })
            .size(theme::TYPE_BODY)
            .style(theme::input_field);

        // Pause/resume tailing: without this, the per-second snap-to-end makes
        // scrolling back through history impossible. It is a state, so it
        // reads as a switch rather than a button that looks pressed.
        let follow_switch = toggler(logs_follow)
            .label(tr(lang, "logs_follow"))
            .text_size(theme::TYPE_BTN_MD)
            .size(18)
            .spacing(8)
            .on_toggle(|_| Message::ToggleLogsFollow)
            .style(theme::toggler_style);

        // Same height as the search field and the segmented track.
        let action = |key: &'static str, msg: Message| {
            button(text(tr(lang, key)).size(theme::TYPE_BTN_MD))
                .padding(theme::BTN_PAD_TOOLBAR)
                .style(theme::button_secondary)
                .on_press(msg)
        };
        let actions_group = row![
            follow_switch,
            action("clear_logs", Message::ClearLogs),
            action("export_logs", Message::ExportLogs),
        ]
        .spacing(theme::SP_12)
        .align_y(Alignment::Center);

        let actions: Element<'_, Message> = if !stacked {
            row![search_input, filters, actions_group]
                .spacing(theme::SP_12)
                .align_y(Alignment::Center)
                .into()
        } else if size.width >= LOGS_TOOLBAR_SPREAD_W {
            column![
                search_input,
                row![filters, Space::new().width(Length::Fill), actions_group]
                    .align_y(Alignment::Center)
            ]
            .spacing(theme::SP_12)
            .width(Length::Fill)
            .into()
        } else {
            column![
                search_input,
                row![filters, actions_group]
                    .spacing(theme::SP_12)
                    .align_y(Alignment::Center)
                    .wrap()
                    .vertical_spacing(theme::SP_8)
            ]
            .spacing(theme::SP_12)
            .width(Length::Fill)
            .into()
        };

        let mut logs_col = Column::new().spacing(3);
        let shown = filtered_lines.len();

        for (line, severity, level) in &filtered_lines {
            let level_color = match severity {
                3 => theme::danger_text(theme),
                2 => theme::warning_text(theme),
                1 => theme::info_text(theme),
                _ => text_tertiary,
            };
            // Only the level tag carries colour; the message stays in text
            // ink so a full screen reads calmly. Errors keep their tint so
            // they still jump out while scrolling.
            let message_color = match severity {
                3 => theme::danger_text(theme),
                0 => text_muted,
                _ => text_primary,
            };
            let spans: Vec<Span<'_, (), iced::Font>> = match level {
                Some(range) => vec![
                    Span::new(&line[..range.start]).color(text_tertiary),
                    Span::new(&line[range.clone()])
                        .color(level_color)
                        .font(theme::mono_font()),
                    Span::new(&line[range.end..]).color(message_color),
                ],
                None => vec![Span::new(*line).color(match severity {
                    3 | 2 => level_color,
                    _ => text_muted,
                })],
            };

            logs_col = logs_col.push(
                rich_text(spans)
                    .font(theme::mono_font())
                    .size(theme::TYPE_MONO)
                    .width(Length::Fill),
            );
        }

        let log_terminal = if total_lines == 0 {
            // ToggleCore *stops* a running core — only offer "Start Core"
            // when it is actually stopped (e.g. right after Clear it isn't).
            let cta: Option<Element<'_, Message>> = if core_running {
                None
            } else {
                Some(
                    button(text(tr(lang, "btn_start_core_short")).size(theme::TYPE_BTN_MD))
                        .padding(theme::BTN_PAD_MD)
                        .style(theme::button_primary)
                        .on_press(Message::ToggleCore)
                        .into(),
                )
            };
            container(crate::ui::empty_state(
                tr(lang, "no_logs"),
                Some(tr(lang, "empty_logs_start_hint")),
                cta,
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
            // Embedded thin scrollbar: once the log overflows it takes its own
            // lane, so it never paints over the ends of wrapped lines.
            container(
                scrollable(logs_col)
                    .id(get_logs_scrollable_id().clone())
                    .direction(scrollable::Direction::Vertical(
                        scrollable::Scrollbar::new()
                            .width(6)
                            .scroller_width(6)
                            .margin(0)
                            .spacing(8),
                    ))
                    .style(theme::scrollbar_style)
                    .height(Length::Fill)
                    .width(Length::Fill),
            )
            .padding(iced::Padding {
                top: theme::CARD_PAD_DENSE,
                right: 10.0,
                bottom: theme::CARD_PAD_DENSE,
                left: theme::CARD_PAD_DENSE,
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::console_bg)
        };

        let header = page_header("tab_logs", lang, Some(actions), theme, stacked);

        let col = column![header, log_terminal]
            .spacing(crate::ui::SP_20)
            .width(Length::Fill)
            .height(Length::Fill);

        crate::ui::page_body_fixed_with_pad(col.into(), crate::ui::is_pad_compact(size.width))
    });

    main_content.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_token_splits_sing_box_lines() {
        let line = "+0800 2026-10-27 14:30:00 WARN [12 0ms] dns: timeout";
        let (range, severity) = level_token(line).unwrap();
        assert_eq!(&line[range.clone()], "WARN");
        assert_eq!(severity, 2);
        assert_eq!(&line[..range.start], "+0800 2026-10-27 14:30:00 ");

        // Untimestamped output and multibyte text before the message.
        let (range, severity) = level_token("ERROR 香港: refused").unwrap();
        assert_eq!(range, 0..5);
        assert_eq!(severity, 3);

        // The level tag beats message keywords, and GUI notices have none.
        assert_eq!(level_token("14:30 INFO retry after error").unwrap().1, 1);
        assert!(level_token("[GUI] Failed to remove staged update").is_none());
    }
}
