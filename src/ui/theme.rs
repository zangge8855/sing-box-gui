use iced::{Background, Border, Color, Shadow};
use iced::widget::{container, button, text_input};

pub const BG_DARK: Color = Color::from_rgb(0.04, 0.06, 0.10);      // #0b0f19
pub const SIDEBAR_BG: Color = Color::from_rgb(0.05, 0.06, 0.10);   // #0d0f1a
pub const CARD_DARK: Color = Color::from_rgb(0.09, 0.11, 0.14);    // #161b24
pub const CARD_LIGHT: Color = Color::from_rgb(0.12, 0.15, 0.19);   // #1f2630
#[allow(dead_code)]
pub const CARD_HOVER: Color = Color::from_rgb(0.14, 0.17, 0.22);   // #242b38
pub const ACCENT_PURPLE: Color = Color::from_rgb(0.55, 0.36, 0.96); // #8b5cf6
pub const ACCENT_BLUE: Color = Color::from_rgb(0.23, 0.51, 0.96);   // #3b82f6
pub const ACCENT_GREEN: Color = Color::from_rgb(0.16, 0.78, 0.56);  // #29c78f
pub const TEXT_PRIMARY: Color = Color::from_rgb(0.95, 0.96, 0.98);  // #f3f4f9
pub const TEXT_MUTED: Color = Color::from_rgb(0.61, 0.64, 0.69);    // #9ca3af
pub const BORDER_DARK: Color = Color::from_rgb(0.18, 0.22, 0.28);   // #2d3748
pub const SUCCESS: Color = Color::from_rgb(0.06, 0.73, 0.51);       // #10b981
pub const WARNING: Color = Color::from_rgb(0.96, 0.62, 0.04);       // #f59e0b
pub const DANGER: Color = Color::from_rgb(0.94, 0.27, 0.27);        // #ef4444

// Main window background styling
pub fn main_bg(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_DARK)),
        text_color: Some(TEXT_PRIMARY),
        ..Default::default()
    }
}

// Sidebar background styling
pub fn sidebar_bg(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SIDEBAR_BG)),
        border: Border {
            color: BORDER_DARK,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

// Card styling
pub fn card_bg(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(CARD_DARK)),
        border: Border {
            color: BORDER_DARK,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.15),
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        ..Default::default()
    }
}

// Selected Card styling (e.g. active proxy node)
pub fn card_selected(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(CARD_LIGHT)),
        border: Border {
            color: ACCENT_PURPLE,
            width: 1.5,
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.55, 0.36, 0.96, 0.25),
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 15.0,
        },
        ..Default::default()
    }
}

// Active connection or speed card
pub fn status_card(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb(0.06, 0.08, 0.12))),
        border: Border {
            color: BORDER_DARK,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

// Primary Action Button styling
pub fn button_primary(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let base_color = ACCENT_PURPLE;
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.60, 0.43, 0.98),
        button::Status::Pressed => Color::from_rgb(0.50, 0.30, 0.90),
        button::Status::Disabled => Color::from_rgb(0.25, 0.20, 0.35),
        button::Status::Active => base_color,
    };
    
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT_PRIMARY,
        border: Border {
            radius: 6.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

// Secondary Button styling
pub fn button_secondary(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.16, 0.20, 0.27),
        button::Status::Pressed => Color::from_rgb(0.10, 0.12, 0.17),
        button::Status::Disabled => Color::from_rgb(0.05, 0.07, 0.10),
        button::Status::Active => CARD_DARK,
    };
    
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT_PRIMARY,
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: BORDER_DARK,
        },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

// Danger Button styling
pub fn button_danger(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.98, 0.35, 0.35),
        button::Status::Pressed => Color::from_rgb(0.85, 0.20, 0.20),
        button::Status::Disabled => Color::from_rgb(0.35, 0.15, 0.15),
        button::Status::Active => DANGER,
    };
    
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT_PRIMARY,
        border: Border {
            radius: 6.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

// Tab navigation button (sidebar buttons)
pub fn button_tab(is_active: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme: &iced::Theme, status: button::Status| {
        let bg = if is_active {
            Color::from_rgba(0.55, 0.36, 0.96, 0.12) // subtle purple overlay
        } else {
            match status {
                button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.04),
                _ => Color::TRANSPARENT,
            }
        };
        
        let text_color = if is_active {
            ACCENT_PURPLE
        } else {
            TEXT_MUTED
        };
        
        button::Style {
            background: Some(Background::Color(bg)),
            text_color,
            border: Border {
                radius: 6.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            shadow: Shadow::default(),
            ..Default::default()
        }
    }
}

// Monospace Console box background
pub fn console_bg(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb(0.02, 0.03, 0.05))), // #05070c
        border: Border {
            color: BORDER_DARK,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

// Text Inputs
pub fn input_field(_theme: &iced::Theme, status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused { .. } => ACCENT_PURPLE,
        text_input::Status::Hovered => Color::from_rgb(0.30, 0.36, 0.45),
        _ => BORDER_DARK,
    };
    
    text_input::Style {
        background: Background::Color(CARD_DARK),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: border_color,
        },
        value: TEXT_PRIMARY,
        placeholder: TEXT_MUTED,
        selection: Color::from_rgba(0.55, 0.36, 0.96, 0.3),
        icon: TEXT_MUTED,
    }
}
