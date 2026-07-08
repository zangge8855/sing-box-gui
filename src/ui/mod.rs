pub mod theme;
pub mod dashboard;
pub mod proxies;
pub mod profiles;
pub mod logs;
pub mod settings;
pub mod i18n;
pub mod connections;
pub mod rules;

use iced::widget::{column, container, row, text, Space};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::ui::theme as ui_theme;

// Unified page padding: top 25, right 24, bottom 30, left 24
pub fn page_padding() -> iced::Padding {
    iced::Padding {
        top: 25.0,
        right: 24.0,
        bottom: 30.0,
        left: 24.0,
    }
}

// Unified page header: title (size 24, primary color) on the left, optional
// actions on the right, separated by a fill space.
pub fn page_header<'a>(
    title_key: &'static str,
    lang: crate::state::Language,
    actions: Option<Element<'a, Message>>,
    theme: &iced::Theme,
    is_compact: bool,
) -> Element<'a, Message> {
    let text_primary = ui_theme::text_primary(theme);
    let title = text(crate::ui::i18n::tr(lang, title_key))
        .size(24)
        .color(text_primary);

    let content: Element<'a, Message> = if is_compact {
        if let Some(actions_el) = actions {
            column![
                title,
                actions_el
            ]
            .spacing(10)
            .width(Length::Fill)
            .into()
        } else {
            row![title].into()
        }
    } else {
        let mut header_row = row![title].align_y(Alignment::Center).width(Length::Fill);

        if let Some(actions_el) = actions {
            header_row = header_row.push(Space::new().width(Length::Fill));
            header_row = header_row.push(actions_el);
        }
        header_row.into()
    };

    container(content)
        .width(Length::Fill)
        .padding(iced::Padding {
            top: 0.0,
            right: 0.0,
            bottom: 12.0,
            left: 0.0,
        })
        .into()
}

// Unified page shell with scrollable outer container. Most tabs use this.

// Non-scrolling variant for pages that manage their own inner scroll area
// (e.g. Logs terminal that needs the full height for log scrolling).
pub fn page_shell_fixed<'a>(
    header: Element<'a, Message>,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    let col = column![header, content]
        .spacing(20)
        .width(Length::Fill)
        .height(Length::Fill);

    container(col)
        .width(Length::Fill)
        .max_width(1200.0)
        .center_x(Length::Fill)
        .padding(page_padding())
        .into()
}
