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

    let dismiss = button(text("✕").size(12).color(theme::text_muted(theme)))
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
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        text(&toast.message)
            .size(13)
            .color(theme::text_primary(theme))
            .width(Length::Fill),
        dismiss,
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding([12, 16]);

    container(body)
        .width(Length::Fixed(360.0))
        .style(move |t| {
            let dark = theme::is_dark(t);
            let bg = match toast.kind {
                ToastKind::Success => {
                    if dark {
                        iced::Color::from_rgba(0.06, 0.73, 0.51, 0.22)
                    } else {
                        iced::Color::from_rgba(0.06, 0.73, 0.51, 0.14)
                    }
                }
                ToastKind::Error => {
                    if dark {
                        iced::Color::from_rgba(0.94, 0.27, 0.27, 0.22)
                    } else {
                        iced::Color::from_rgba(0.94, 0.27, 0.27, 0.12)
                    }
                }
                ToastKind::Info => {
                    if dark {
                        iced::Color::from_rgba(0.23, 0.51, 0.96, 0.22)
                    } else {
                        iced::Color::from_rgba(0.23, 0.51, 0.96, 0.12)
                    }
                }
            };
            let mut s = theme::card_bg(t);
            s.background = Some(iced::Background::Color(if dark {
                theme::CARD_DARK
            } else {
                theme::CARD_LIGHT_BG
            }));
            // overlay tint via border accent; solid card for readability
            let _ = bg;
            s.border.color = accent;
            s.border.width = 1.5;
            s.border.radius = 10.0.into();
            s
        })
        .into()
}
