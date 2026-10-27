use crate::message::Message;
use crate::state::{Toast, ToastKind};
use crate::ui::theme;
use iced::widget::{button, container, row, text};
use iced::{Alignment, Element, Font, Length};

pub fn render<'a>(toast: &'a Toast, theme: &iced::Theme) -> Element<'a, Message> {
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

    let icon = text(glyph.to_string())
        .font(Font::with_name("Material Icons"))
        .size(crate::ui::ICON_SIZE_LG)
        .color(glyph_color);

    let dismiss = button(
        text(crate::ui::icons::ICON_DELETE)
            .font(Font::with_name("Material Icons"))
            .size(14),
    )
    .padding(4)
    .style(theme::button_ghost)
    .on_press(Message::DismissToast);

    let body = row![
        icon,
        text(&toast.message)
            .size(theme::TYPE_BTN_SM)
            .color(theme::text_primary(theme))
            .width(Length::Fill),
        dismiss,
    ]
    .spacing(crate::ui::SP_12)
    .align_y(Alignment::Center)
    .padding(iced::Padding {
        top: 10.0,
        right: 8.0,
        bottom: 10.0,
        left: 14.0,
    });

    container(body)
        .width(Length::Shrink)
        .max_width(480.0)
        .style(theme::popover_bg)
        .into()
}
