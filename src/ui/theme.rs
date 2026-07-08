use iced::{Background, Border, Color, Shadow};
use iced::widget::{container, button, text_input, pick_list as iced_pick_list};

// Dark mode palette
pub const BG_DARK: Color = Color::from_rgb(0.06, 0.06, 0.07);      // Deep gray/black, cleaner
pub const SIDEBAR_BG: Color = Color::from_rgb(0.04, 0.04, 0.05);   // Slightly darker than main BG for depth
pub const CARD_DARK: Color = Color::from_rgb(0.10, 0.10, 0.12);    // Softer card background
pub const CARD_LIGHT: Color = Color::from_rgb(0.13, 0.13, 0.16);
#[allow(dead_code)]
pub const CARD_HOVER: Color = Color::from_rgb(0.15, 0.15, 0.18);
pub const BORDER_DARK: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.08); // Subtle top-light highlight
pub const TEXT_PRIMARY: Color = Color::from_rgb(0.95, 0.96, 0.98);
pub const TEXT_MUTED: Color = Color::from_rgb(0.55, 0.58, 0.63);
pub const TEXT_TERTIARY: Color = Color::from_rgb(0.35, 0.38, 0.42);

// Light mode palette
pub const BG_LIGHT: Color = Color::from_rgb(0.97, 0.97, 0.98);      // Very light gray/off-white
pub const SIDEBAR_BG_LIGHT: Color = Color::from_rgb(0.93, 0.93, 0.95);
pub const CARD_LIGHT_BG: Color = Color::from_rgb(1.0, 1.0, 1.0);    // Pure white for crispness
pub const CARD_SELECTED_LIGHT: Color = Color::from_rgb(0.95, 0.96, 1.0);
pub const BORDER_LIGHT: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.03); // Extremely subtle border
pub const TEXT_PRIMARY_LIGHT: Color = Color::from_rgb(0.10, 0.11, 0.14);
pub const TEXT_MUTED_LIGHT: Color = Color::from_rgb(0.45, 0.48, 0.52);
pub const TEXT_TERTIARY_LIGHT: Color = Color::from_rgb(0.65, 0.68, 0.72);

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
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0, // Remove right border for seamless look
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
        Color::from_rgba(0.0, 0.0, 0.0, 0.25) // Softer shadow
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.03) // Very light shadow
    };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border,
            width: 1.0,
            radius: 16.0.into(), // Larger radius for high-end look
        },
        shadow: Shadow {
            color: shadow_color,
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 24.0, // Wider, softer shadow
        },
        ..Default::default()
    }
}

// Selected Card styling (e.g. active proxy node)
pub fn card_selected(theme: &iced::Theme) -> container::Style {
    let bg = if is_dark(theme) { CARD_LIGHT } else { CARD_SELECTED_LIGHT };
    let shadow_color = if is_dark(theme) {
        Color::from_rgba(0.55, 0.36, 0.96, 0.20)
    } else {
        Color::from_rgba(0.55, 0.36, 0.96, 0.15)
    };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: ACCENT_PURPLE,
            width: 1.5,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: shadow_color,
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        ..Default::default()
    }
}

// Active connection or speed card
pub fn status_card(theme: &iced::Theme) -> container::Style {
    let bg = if is_dark(theme) {
        Color::from_rgb(0.08, 0.08, 0.10)
    } else {
        Color::from_rgb(0.98, 0.98, 0.99)
    };
    let border = if is_dark(theme) { BORDER_DARK } else { BORDER_LIGHT };
    let shadow_color = if is_dark(theme) {
        Color::from_rgba(0.0, 0.0, 0.0, 0.25)
    } else {
        Color::from_rgba(0.0, 0.0, 0.0, 0.04)
    };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border,
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: shadow_color,
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

// Primary Action Button styling
pub fn button_primary(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let base_color = ACCENT_PURPLE;
    let (bg, shadow) = match status {
        button::Status::Hovered => (
            Color::from_rgb(0.65, 0.48, 1.0), // Brighter hover
            Shadow {
                color: Color::from_rgba(0.55, 0.36, 0.96, 0.5),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            }
        ),
        button::Status::Pressed => (Color::from_rgb(0.50, 0.30, 0.90), Shadow::default()),
        button::Status::Disabled => (Color::from_rgb(0.25, 0.20, 0.35), Shadow::default()),
        button::Status::Active => (base_color, Shadow::default()),
    };
    
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow,
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
            radius: 8.0.into(),
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
            radius: 8.0.into(),
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
                radius: 8.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            shadow: Shadow::default(),
            ..Default::default()
        }
    }
}

// Simple list item styling with interactive states
pub fn list_item_style(theme: &iced::Theme, is_selected: bool, is_hovered: bool) -> container::Style {
    let dark = is_dark(theme);
    
    let base_bg = if dark { Color::from_rgb(0.06, 0.08, 0.12) } else { Color::from_rgb(0.97, 0.98, 0.99) };
    let hover_bg = if dark { Color::from_rgb(0.09, 0.11, 0.15) } else { Color::from_rgb(0.93, 0.94, 0.96) };
    let selected_bg = if dark { Color::from_rgb(0.12, 0.14, 0.18) } else { Color::from_rgb(0.89, 0.91, 0.94) };

    let bg = if is_selected {
        selected_bg
    } else if is_hovered {
        hover_bg
    } else {
        base_bg
    };

    let border = if dark { BORDER_DARK } else { BORDER_LIGHT };
    
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: if is_selected { ACCENT_PURPLE } else { border },
            width: if is_selected { 1.5 } else { 1.0 },
            radius: 6.0.into(),
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
    let border_default = if dark { Color::from_rgb(0.22, 0.26, 0.32) } else { BORDER_LIGHT };
    let border_color = match status {
        text_input::Status::Focused { .. } => ACCENT_PURPLE,
        text_input::Status::Hovered => {
            if dark {
                Color::from_rgb(0.35, 0.40, 0.48)
            } else {
                Color::from_rgb(0.60, 0.65, 0.72)
            }
        }
        _ => border_default,
    };
    
    // Use the main background color for inputs to make them sink into the card
    let bg = if dark { BG_DARK } else { BG_LIGHT };
    let text = if dark { TEXT_PRIMARY } else { TEXT_PRIMARY_LIGHT };
    let placeholder = if dark { TEXT_MUTED } else { TEXT_MUTED_LIGHT };
    
    text_input::Style {
        background: Background::Color(bg),
        border: Border {
            radius: 8.0.into(),
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

pub fn text_tertiary(theme: &iced::Theme) -> Color {
    if is_dark(theme) { TEXT_TERTIARY } else { TEXT_TERTIARY_LIGHT }
}

pub fn pick_list(theme: &iced::Theme, status: iced_pick_list::Status) -> iced_pick_list::Style {
    let dark = is_dark(theme);
    let border_default = if dark { Color::from_rgb(0.22, 0.26, 0.32) } else { BORDER_LIGHT };
    let border_color = match status {
        iced_pick_list::Status::Opened { .. } => ACCENT_PURPLE,
        iced_pick_list::Status::Hovered => {
            if dark {
                Color::from_rgb(0.35, 0.40, 0.48)
            } else {
                Color::from_rgb(0.60, 0.65, 0.72)
            }
        }
        _ => border_default,
    };
    
    let bg = if dark { BG_DARK } else { BG_LIGHT };
    let text_color = if dark { TEXT_PRIMARY } else { TEXT_PRIMARY_LIGHT };
    let placeholder = if dark { TEXT_MUTED } else { TEXT_MUTED_LIGHT };
    let handle = if dark { TEXT_MUTED } else { TEXT_MUTED_LIGHT };
    
    iced_pick_list::Style {
        background: Background::Color(bg),
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: border_color,
        },
        text_color,
        placeholder_color: placeholder,
        handle_color: handle,
    }
}
