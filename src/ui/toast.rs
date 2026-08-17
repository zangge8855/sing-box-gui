use crate::message::Message;
use crate::state::{Toast, ToastKind};
use crate::ui::theme;
use iced::widget::{button, container, row, text};
use iced::{Alignment, Color, Element, Length};


pub fn render<'a>(toast: &'a Toast, theme: &iced::Theme) -> Element<'a, Message> {
    let (accent, icon_str) = match toast.kind {
        ToastKind::Success => (theme::SUCCESS, "✓"),
        ToastKind::Error => (theme::DANGER, "✕"),
        ToastKind::Info => (theme::ACCENT_BLUE, "ℹ"),
    };

    let icon_badge = container(
        text(icon_str)
            .size(theme::TYPE_MICRO)
            .color(Color::WHITE)
            .font(theme::ui_font(iced::font::Weight::Bold)),
    )
    .width(18)
    .height(18)
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .style(move |_t| container::Style {
        background: Some(iced::Background::Color(accent)),
        border: iced::Border {
            radius: 9.0.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    let dismiss = button(text("✕").size(theme::TYPE_MICRO))
        .padding([4, 6])
        .style(theme::button_ghost)
        .on_press(Message::DismissToast);

    let body = row![
        icon_badge,
        text(&toast.message)
            .size(theme::TYPE_BTN_SM)
            .color(theme::text_primary(theme))
            .width(Length::Fill),
        dismiss,
    ]
    .spacing(crate::ui::SP_12)
    .align_y(Alignment::Center)
    .padding([10, 14]);

    container(body)
        .width(Length::Shrink)
        .max_width(480.0)
        .style(move |t| {
            let mut s = theme::card_bg(t);
            s.background = Some(iced::Background::Color(theme::elevated_surface(t)));
            s.border.color = theme::with_alpha(accent, if theme::is_dark(t) { 0.50 } else { 0.40 });
            s.border.width = 1.0;
            s.border.radius = theme::RADIUS_MD.into();
            s.shadow = iced::Shadow {
                color: theme::with_alpha(accent, if theme::is_dark(t) { 0.25 } else { 0.15 }),
                offset: iced::Vector::new(0.0, 6.0),
                blur_radius: 16.0,
            };
            s
        })
        .into()
}

