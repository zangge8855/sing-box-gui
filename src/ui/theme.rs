use iced::{Background, Border, Color, Shadow};
use iced::widget::{container, button, text_input};

// Dark mode palette
pub const BG_DARK: Color = Color::from_rgb(0.04, 0.06, 0.10);      // #0b0f19
pub const SIDEBAR_BG: Color = Color::from_rgb(0.05, 0.06, 0.10);   // #0d0f1a
pub const CARD_DARK: Color = Color::from_rgb(0.09, 0.11, 0.14);    // #161b24
pub const CARD_LIGHT: Color = Color::from_rgb(0.12, 0.15, 0.19);   // #1f2630
#[allow(dead_code)]
pub const CARD_HOVER: Color = Color::from_rgb(0.14, 0.17, 0.22);   // #242b38
pub const BORDER_DARK: Color = Color::from_rgb(0.18, 0.22, 0.28);   // #2d3748
pub const TEXT_PRIMARY: Color = Color::from_rgb(0.95, 0.96, 0.98);  // #f3f4f9
pub const TEXT_MUTED: Color = Color::from_rgb(0.61, 0.64, 0.69);    // #9ca3af

// Light mode palette
pub const BG_LIGHT: Color = Color::from_rgb(0.95, 0.96, 0.98);      // #f3f4f8
pub const SIDEBAR_BG_LIGHT: Color = Color::from_rgb(0.91, 0.92, 0.95); // #e8ebf2
pub const CARD_LIGHT_BG: Color = Color::from_rgb(1.0, 1.0, 1.0);    // #ffffff
pub const CARD_SELECTED_LIGHT: Color = Color::from_rgb(0.93, 0.95, 0.99); // #edf2fc
pub const BORDER_LIGHT: Color = Color::from_rgb(0.88, 0.90, 0.94);   // #e2e5eb
pub const TEXT_PRIMARY_LIGHT: Color = Color::from_rgb(0.09, 0.11, 0.14); // #161b24
pub const TEXT_MUTED_LIGHT: Color = Color::from_rgb(0.45, 0.48, 0.52);   // #737a85

// Accent colors (work well in both light & dark)
pub const ACCENT_PURPLE: Color = Color::from_rgb(0.55, 0.36, 0.96); // #8b5cf6
pub const ACCENT_BLUE: Color = Color::from_rgb(0.23, 0.51, 0.96);   // #3b82f6
pub const ACCENT_GREEN: Color = Color::from_rgb(0.16, 0.78, 0.56);  // #29c78f
pub const SUCCESS: Color = Color::from_rgb(0.06, 0.73, 0.51);       // #10b981
pub const WARNING: Color = Color::from_rgb(0.96, 0.62, 0.04);       // #f59e0b
pub const DANGER: Color = Color::from_rgb(0.94, 0.27, 0.27);        // #ef4444

pub fn is_dark(theme: &iced::Theme) -> bool {
    match theme {
        iced::Theme::Light | iced::Theme::SolarizedLight | iced::Theme::GruvboxLight | iced::Theme::TokyoNightLight | iced::Theme::KanagawaLotus => false,
        _ => true,
    }
}

// Main window background styling
pub fn main_bg(theme: &iced::Theme) -> container::Style {
    let bg = if is_dark(theme) { BG_DARK } else { BG_LIGHT };
    let text = if is_dark(theme) { TEXT_PRIMARY } else { TEXT_PRIMARY_LIGHT };
    container::Style {
        background: Some(Background::Color(bg)),
        text_color: Some(text),
        ..Default::default()
    }
}

// Sidebar background styling
pub fn sidebar_bg(theme: &iced::Theme) -> container::Style {
    let bg = if is_dark(theme) { SIDEBAR_BG } else { SIDEBAR_BG_LIGHT };
    let border = if is_dark(theme) { BORDER_DARK } else { BORDER_LIGHT };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

// Card styling
pub fn card_bg(theme: &iced::Theme) -> container::Style {
    let bg = if is_dark(theme) { CARD_DARK } else { CARD_LIGHT_BG };
    let border = if is_dark(theme) { BORDER_DARK } else { BORDER_LIGHT };
    let shadow_color = if is_dark(theme) {
        Color::from_rgba(0.0, 0.0, 0.0, 0.15)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.06)
    };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: shadow_color,
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        ..Default::default()
    }
}

// Selected Card styling (e.g. active proxy node)
pub fn card_selected(theme: &iced::Theme) -> container::Style {
    let bg = if is_dark(theme) { CARD_LIGHT } else { CARD_SELECTED_LIGHT };
    let shadow_color = if is_dark(theme) {
        Color::from_rgba(0.55, 0.36, 0.96, 0.25)
    } else {
        Color::from_rgba(0.55, 0.36, 0.96, 0.15)
    };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: ACCENT_PURPLE,
            width: 1.5,
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: shadow_color,
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 15.0,
        },
        ..Default::default()
    }
}

// Active connection or speed card
pub fn status_card(theme: &iced::Theme) -> container::Style {
    let bg = if is_dark(theme) {
        Color::from_rgb(0.06, 0.08, 0.12)
    } else {
        Color::from_rgb(0.97, 0.98, 0.99)
    };
    let border = if is_dark(theme) { BORDER_DARK } else { BORDER_LIGHT };
    let shadow_color = if is_dark(theme) {
        Color::from_rgba(0.0, 0.0, 0.0, 0.15)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.06)
    };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: shadow_color,
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 8.0,
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
        text_color: Color::WHITE,
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
pub fn button_secondary(theme: &iced::Theme, status: button::Status) -> button::Style {
    let dark = is_dark(theme);
    let (bg, border_color, text_color) = if dark {
        let b = match status {
            button::Status::Hovered => Color::from_rgb(0.16, 0.20, 0.27),
            button::Status::Pressed => Color::from_rgb(0.10, 0.12, 0.17),
            button::Status::Disabled => Color::from_rgb(0.05, 0.07, 0.10),
            button::Status::Active => CARD_DARK,
        };
        (b, BORDER_DARK, TEXT_PRIMARY)
    } else {
        let b = match status {
            button::Status::Hovered => Color::from_rgb(0.90, 0.92, 0.96),
            button::Status::Pressed => Color::from_rgb(0.85, 0.87, 0.91),
            button::Status::Disabled => Color::from_rgb(0.95, 0.96, 0.97),
            button::Status::Active => CARD_LIGHT_BG,
        };
        (b, BORDER_LIGHT, TEXT_PRIMARY_LIGHT)
    };
    
    button::Style {
        background: Some(Background::Color(bg)),
        text_color,
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: border_color,
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
        text_color: Color::WHITE,
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
    move |theme: &iced::Theme, status: button::Status| {
        let dark = is_dark(theme);
        let bg = if is_active {
            Color::from_rgba(0.55, 0.36, 0.96, 0.12) // subtle purple overlay
        } else {
            match status {
                button::Status::Hovered => {
                    if dark {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.04)
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.04)
                    }
                }
                _ => Color::TRANSPARENT,
            }
        };
        
        let text_color = if is_active {
            ACCENT_PURPLE
        } else if dark {
            TEXT_MUTED
        } else {
            TEXT_MUTED_LIGHT
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

// Simple list item styling (flat, no shadow)
pub fn list_item_bg(theme: &iced::Theme) -> container::Style {
    let bg = if is_dark(theme) {
        Color::from_rgb(0.06, 0.08, 0.12)
    } else {
        Color::from_rgb(0.97, 0.98, 0.99)
    };
    let border = if is_dark(theme) { BORDER_DARK } else { BORDER_LIGHT };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

// Monospace Console box background
pub fn console_bg(theme: &iced::Theme) -> container::Style {
    let bg = if is_dark(theme) {
        Color::from_rgb(0.02, 0.03, 0.05) // #05070c
    } else {
        Color::from_rgb(0.97, 0.97, 0.98) // light grey console
    };
    let border = if is_dark(theme) { BORDER_DARK } else { BORDER_LIGHT };
    let text = if is_dark(theme) { TEXT_PRIMARY } else { TEXT_PRIMARY_LIGHT };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border,
            width: 1.0,
            radius: 6.0.into(),
        },
        text_color: Some(text),
        ..Default::default()
    }
}

// Text Inputs
pub fn input_field(theme: &iced::Theme, status: text_input::Status) -> text_input::Style {
    let dark = is_dark(theme);
    let border_default = if dark { BORDER_DARK } else { BORDER_LIGHT };
    let border_color = match status {
        text_input::Status::Focused { .. } => ACCENT_PURPLE,
        text_input::Status::Hovered => {
            if dark {
                Color::from_rgb(0.30, 0.36, 0.45)
            } else {
                Color::from_rgb(0.70, 0.75, 0.82)
            }
        }
        _ => border_default,
    };
    
    let bg = if dark { CARD_DARK } else { CARD_LIGHT_BG };
    let text = if dark { TEXT_PRIMARY } else { TEXT_PRIMARY_LIGHT };
    let placeholder = if dark { TEXT_MUTED } else { TEXT_MUTED_LIGHT };
    
    text_input::Style {
        background: Background::Color(bg),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: border_color,
        },
        value: text,
        placeholder,
        selection: Color::from_rgba(0.55, 0.36, 0.96, 0.3),
        icon: placeholder,
    }
}

pub fn text_primary(theme: &iced::Theme) -> Color {
    if is_dark(theme) { TEXT_PRIMARY } else { TEXT_PRIMARY_LIGHT }
}

pub fn text_muted(theme: &iced::Theme) -> Color {
    if is_dark(theme) { TEXT_MUTED } else { TEXT_MUTED_LIGHT }
}
