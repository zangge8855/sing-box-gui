use iced::widget::{button, container, row, text, Space};
use iced::{Alignment, Element, Length};
use crate::message::Message;
use crate::state::{Toast, ToastKind};
use crate::ui::theme;

pub fn render<'a>(toast: &'a Toast, theme: &iced::Theme) -> Element<'a, Message> {
    let accent = match toast.kind {
        ToastKind::Success => theme::SUCCESS,
        ToastKind::Error => theme::DANGER,
        ToastKind::Info => theme::ACCENT_BLUE,
    };

    let dismiss = button(text("✕").size(theme::TYPE_BTN_SM))
        .padding([4, 8])
        .style(theme::button_secondary)
        .on_press(Message::DismissToast);

    let body = row![
        container(Space::new())
            .width(4)
            .height(28)
            .style(move |_t| container::Style {
                background: Some(iced::Background::Color(accent)),
                border: iced::Border {
                    radius: (theme::RADIUS_XS / 3.0).into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        text(&toast.message)
            .size(theme::TYPE_SECTION)
            .color(theme::text_primary(theme))
            .width(Length::Fill),
        dismiss,
    ]
    .spacing(crate::ui::SP_12)
    .align_y(Alignment::Center)
    .padding([12, 16]);

    container(body)
        .width(Length::Shrink)
        .max_width(480.0)
        .style(move |t| {
            let mut s = theme::tinted_banner(t, accent);
            // Solid elevated surface for readability, keep accent border
            s.background = Some(iced::Background::Color(theme::card_surface(t)));
            s.border.color = accent;
            s.border.width = 1.5;
            s.border.radius = theme::RADIUS_MD.into();
            s.shadow = iced::Shadow {
                color: theme::with_alpha(accent, 0.22),
                offset: iced::Vector::new(0.0, 6.0),
                blur_radius: 16.0,
            };
            s
        })
        .into()
}
