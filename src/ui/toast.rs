use crate::message::Message;
use crate::state::{Toast, ToastKind};
use crate::ui::theme;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Font, Length};

pub fn render_stack<'a>(toasts: &'a [Toast], theme: &iced::Theme) -> Element<'a, Message> {
    let items: Vec<Element<'a, Message>> = toasts
        .iter()
        .enumerate()
        .map(|(idx, toast)| render_toast(toast, Some(idx), theme))
        .collect();
    column(items)
        .spacing(crate::ui::SP_8)
        .align_x(Alignment::End)
        .into()
}

fn render_toast<'a>(
    toast: &'a Toast,
    index: Option<usize>,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    // Semantic colour lives in the glyph only; the surface stays neutral so
    // a stream of toasts never turns the corner of the window into a rainbow.
    let (glyph, glyph_color) = match toast.kind {
        ToastKind::Success => (
            crate::ui::icons::ICON_CHECK_CIRCLE,
            theme::success_text(theme),
        ),
        ToastKind::Error => (crate::ui::icons::ICON_ERROR, theme::danger_text(theme)),
        ToastKind::Info => (crate::ui::icons::ICON_INFO, theme::info_text(theme)),
    };

    let icon = container(
        text(glyph.to_string())
            .font(Font::with_name("Material Icons"))
            .size(crate::ui::ICON_SIZE)
            .color(glyph_color),
    )
    .width(20)
    .height(20)
    .align_x(Alignment::Center)
    .align_y(Alignment::Center);

    let dismiss_msg = match index {
        Some(i) => Message::DismissToastAt(i),
        None => Message::DismissToast,
    };

    let dismiss = button(
        container(
            text(crate::ui::icons::ICON_CLOSE)
                .font(Font::with_name("Material Icons"))
                .size(14),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center),
    )
    .width(24)
    .height(24)
    .padding(0)
    .style(theme::button_dismiss)
    .on_press(dismiss_msg);

    let msg_scroll = container(
        scrollable(
            text(&toast.message)
                .size(theme::TYPE_BTN_SM)
                .color(theme::text_primary(theme))
                .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
        )
        .width(Length::Shrink)
        .height(Length::Shrink)
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new()
                .width(4)
                .scroller_width(4)
                .margin(2),
        ))
        .style(theme::scrollbar_style),
    )
    .width(Length::Shrink)
    .max_width(340.0)
    .max_height(120.0);

    let body = row![icon, msg_scroll, dismiss]
        .spacing(10)
        .align_y(Alignment::Center)
        .padding(iced::Padding {
            top: 10.0,
            right: 10.0,
            bottom: 10.0,
            left: 12.0,
        });

    container(body)
        .width(Length::Shrink)
        .max_width(440.0)
        .style(theme::popover_bg)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toast_stack_renders_empty_and_long_unbroken_messages() {
        let _ = render_stack(&[], &iced::Theme::Light);
        let toasts = vec![
            Toast::success("Done"),
            Toast::error(format!("https://example.com/{}", "a".repeat(200))),
            Toast::info("中文提示信息".repeat(30)),
        ];
        let _ = render_stack(&toasts, &iced::Theme::Dark);
        let _ = render_stack(&toasts, &iced::Theme::Light);
    }
}
