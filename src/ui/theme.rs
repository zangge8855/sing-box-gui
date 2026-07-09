//! Premium design tokens and shared chrome styles for sing-box-gui.
//!
//! Single source of visual truth: dark/light layered surfaces, borders,
//! type hierarchy, accent, and semantic colors — consumed by shell + tabs.

use iced::{Background, Border, Color, Shadow};
use iced::widget::{container, button, text_input, pick_list as iced_pick_list};

// ── Radius language ──────────────────────────────────────────────────────────
pub const RADIUS_LG: f32 = 14.0;
pub const RADIUS_MD: f32 = 10.0;
pub const RADIUS_SM: f32 = 8.0;
pub const RADIUS_XS: f32 = 6.0;

// ── Type scale (f32 for iced::Pixels) ────────────────────────────────────────
pub const TYPE_TITLE: f32 = 22.0;
pub const TYPE_SECTION: f32 = 13.0; // group labels — medium muted
pub const TYPE_HEADING: f32 = 14.0; // content titles — semibold primary
pub const TYPE_BODY: f32 = 14.0;
pub const TYPE_CAPTION: f32 = 11.0;
pub const TYPE_MICRO: f32 = 10.0; // badges only
pub const TYPE_BTN_SM: f32 = 12.0;
pub const TYPE_BTN_MD: f32 = 13.0;
#[allow(dead_code)]
pub const TYPE_BTN_LG: f32 = 14.0;
#[allow(dead_code)]
pub const TYPE_METRIC: f32 = 22.0; // dashboard speed numbers
pub const TYPE_MONO: f32 = 12.0; // latency, mono captions

// ── Spacing / padding presets ────────────────────────────────────────────────
pub const CARD_PAD: f32 = 20.0;
pub const CARD_PAD_DENSE: f32 = 16.0;
pub const BTN_PAD_SM: [u16; 2] = [6, 12];
pub const BTN_PAD_MD: [u16; 2] = [8, 16];
#[allow(dead_code)]
pub const BTN_PAD_LG: [u16; 2] = [12, 20];
/// Shared width for page header search inputs.
pub const SEARCH_WIDTH: f32 = 260.0;
/// Grid / card list gap.
pub const GRID_GAP: f32 = 16.0;

// ── Dark mode (cool near-black with subtle blue undertone) ───────────────────
pub const BG_DARK: Color = Color::from_rgb(0.055, 0.057, 0.070); // #0e0f12
pub const SIDEBAR_BG: Color = Color::from_rgb(0.040, 0.042, 0.055); // #0a0b0e
pub const CARD_DARK: Color = Color::from_rgb(0.100, 0.105, 0.128); // #1a1b21
pub const CARD_ELEVATED_DARK: Color = Color::from_rgb(0.125, 0.132, 0.160); // #202229
#[allow(dead_code)]
pub const CARD_HOVER: Color = Color::from_rgb(0.145, 0.152, 0.185);
/// Elevated / selected dark surface alias.
#[allow(dead_code)]
pub const CARD_LIGHT: Color = CARD_ELEVATED_DARK;
pub const BORDER_DARK: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.10);
pub const BORDER_STRONG_DARK: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.16);
pub const TEXT_PRIMARY: Color = Color::from_rgb(0.96, 0.97, 0.99);
pub const TEXT_MUTED: Color = Color::from_rgb(0.62, 0.65, 0.72);
pub const TEXT_TERTIARY: Color = Color::from_rgb(0.42, 0.45, 0.52);
pub const INPUT_BG_DARK: Color = Color::from_rgb(0.045, 0.048, 0.060);
pub const CONSOLE_BG_DARK: Color = Color::from_rgb(0.028, 0.032, 0.045);

// ── Light mode (warm off-white, readable borders) ────────────────────────────
pub const BG_LIGHT: Color = Color::from_rgb(0.955, 0.958, 0.970); // #f4f4f7
pub const SIDEBAR_BG_LIGHT: Color = Color::from_rgb(0.930, 0.933, 0.950);
pub const CARD_LIGHT_BG: Color = Color::from_rgb(1.0, 1.0, 1.0);
pub const CARD_SELECTED_LIGHT: Color = Color::from_rgb(0.965, 0.960, 1.0);
pub const BORDER_LIGHT: Color = Color::from_rgba(0.10, 0.12, 0.18, 0.10);
pub const BORDER_STRONG_LIGHT: Color = Color::from_rgba(0.10, 0.12, 0.18, 0.16);
pub const TEXT_PRIMARY_LIGHT: Color = Color::from_rgb(0.09, 0.10, 0.14);
pub const TEXT_MUTED_LIGHT: Color = Color::from_rgb(0.32, 0.35, 0.42); // darker for WCAG-ish contrast on white
pub const TEXT_TERTIARY_LIGHT: Color = Color::from_rgb(0.46, 0.49, 0.56);
pub const INPUT_BG_LIGHT: Color = Color::from_rgb(0.945, 0.948, 0.960);
pub const CONSOLE_BG_LIGHT: Color = Color::from_rgb(0.965, 0.968, 0.978);

// ── Accent + semantic (shared across modes) ──────────────────────────────────
/// Primary brand violet — slightly richer for premium feel.
pub const ACCENT_PURPLE: Color = Color::from_rgb(0.52, 0.38, 0.98); // ~#8561fa
pub const ACCENT_PURPLE_HOVER: Color = Color::from_rgb(0.62, 0.48, 1.0);
pub const ACCENT_PURPLE_PRESSED: Color = Color::from_rgb(0.44, 0.30, 0.88);
pub const ACCENT_PURPLE_DISABLED: Color = Color::from_rgb(0.28, 0.24, 0.40);
pub const ACCENT_BLUE: Color = Color::from_rgb(0.28, 0.52, 0.96); // #4785f5
/// Prefer SUCCESS for health/status; kept for decorative accents if needed.
#[allow(dead_code)]
pub const ACCENT_GREEN: Color = Color::from_rgb(0.18, 0.76, 0.58);
pub const SUCCESS: Color = Color::from_rgb(0.12, 0.72, 0.52); // #1fb885
pub const WARNING: Color = Color::from_rgb(0.94, 0.60, 0.12);
pub const DANGER: Color = Color::from_rgb(0.92, 0.30, 0.32);
pub const DANGER_HOVER: Color = Color::from_rgb(0.96, 0.38, 0.38);
pub const DANGER_PRESSED: Color = Color::from_rgb(0.80, 0.22, 0.24);
pub const DANGER_DISABLED: Color = Color::from_rgb(0.40, 0.20, 0.22);

// ── Token accessors ──────────────────────────────────────────────────────────

pub fn is_dark(theme: &iced::Theme) -> bool {
    match theme {
        iced::Theme::Light
        | iced::Theme::SolarizedLight
        | iced::Theme::GruvboxLight
        | iced::Theme::TokyoNightLight
        | iced::Theme::KanagawaLotus => false,
        _ => true,
    }
}

pub fn bg(theme: &iced::Theme) -> Color {
    if is_dark(theme) { BG_DARK } else { BG_LIGHT }
}

pub fn sidebar_surface(theme: &iced::Theme) -> Color {
    if is_dark(theme) { SIDEBAR_BG } else { SIDEBAR_BG_LIGHT }
}

pub fn card_surface(theme: &iced::Theme) -> Color {
    if is_dark(theme) { CARD_DARK } else { CARD_LIGHT_BG }
}

pub fn elevated_surface(theme: &iced::Theme) -> Color {
    if is_dark(theme) { CARD_ELEVATED_DARK } else { CARD_SELECTED_LIGHT }
}

pub fn border_color(theme: &iced::Theme) -> Color {
    if is_dark(theme) { BORDER_DARK } else { BORDER_LIGHT }
}

#[allow(dead_code)]
pub fn border_strong(theme: &iced::Theme) -> Color {
    if is_dark(theme) { BORDER_STRONG_DARK } else { BORDER_STRONG_LIGHT }
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

pub fn input_surface(theme: &iced::Theme) -> Color {
    if is_dark(theme) { INPUT_BG_DARK } else { INPUT_BG_LIGHT }
}

pub fn console_surface(theme: &iced::Theme) -> Color {
    if is_dark(theme) { CONSOLE_BG_DARK } else { CONSOLE_BG_LIGHT }
}

/// Soft tint of a solid color (for badges / banners).
pub fn with_alpha(color: Color, alpha: f32) -> Color {
    Color {
        r: color.r,
        g: color.g,
        b: color.b,
        a: alpha,
    }
}

fn card_shadow(theme: &iced::Theme) -> Shadow {
    if is_dark(theme) {
        Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.38),
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 20.0,
        }
    } else {
        Shadow {
            color: Color::from_rgba(0.08, 0.10, 0.16, 0.10),
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        }
    }
}

fn soft_shadow(theme: &iced::Theme) -> Shadow {
    if is_dark(theme) {
        Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.28),
            offset: iced::Vector::new(0.0, 3.0),
            blur_radius: 12.0,
        }
    } else {
        Shadow {
            color: Color::from_rgba(0.08, 0.10, 0.16, 0.07),
            offset: iced::Vector::new(0.0, 3.0),
            blur_radius: 10.0,
        }
    }
}

// ── Chrome styles ────────────────────────────────────────────────────────────

pub fn main_bg(theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(bg(theme))),
        text_color: Some(text_primary(theme)),
        ..Default::default()
    }
}

pub fn sidebar_bg(theme: &iced::Theme) -> container::Style {
    let dark = is_dark(theme);
    container::Style {
        background: Some(Background::Color(sidebar_surface(theme))),
        border: Border {
            // Right-edge separator into main content
            color: if dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.08)
            } else {
                Color::from_rgba(0.10, 0.12, 0.18, 0.12)
            },
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn card_bg(theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(card_surface(theme))),
        border: Border {
            color: border_color(theme),
            width: 1.0,
            radius: RADIUS_LG.into(),
        },
        shadow: card_shadow(theme),
        text_color: Some(text_primary(theme)),
        ..Default::default()
    }
}

pub fn card_selected(theme: &iced::Theme) -> container::Style {
    let dark = is_dark(theme);
    let bg = if dark {
        Color {
            r: ACCENT_PURPLE.r * 0.18 + CARD_DARK.r * 0.82,
            g: ACCENT_PURPLE.g * 0.18 + CARD_DARK.g * 0.82,
            b: ACCENT_PURPLE.b * 0.18 + CARD_DARK.b * 0.82,
            a: 1.0,
        }
    } else {
        CARD_SELECTED_LIGHT
    };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: ACCENT_PURPLE,
            width: 1.5,
            radius: RADIUS_LG.into(),
        },
        shadow: Shadow {
            color: with_alpha(ACCENT_PURPLE, if dark { 0.28 } else { 0.18 }),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        text_color: Some(text_primary(theme)),
        ..Default::default()
    }
}

pub fn status_card(theme: &iced::Theme) -> container::Style {
    let dark = is_dark(theme);
    let bg = if dark {
        Color::from_rgb(0.075, 0.080, 0.098)
    } else {
        Color::from_rgb(0.985, 0.986, 0.992)
    };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border_color(theme),
            width: 1.0,
            radius: RADIUS_LG.into(),
        },
        shadow: soft_shadow(theme),
        text_color: Some(text_primary(theme)),
        ..Default::default()
    }
}

/// Pill / type badge surface (e.g. SS, VMess tags).
pub fn badge_bg(theme: &iced::Theme) -> container::Style {
    let dark = is_dark(theme);
    container::Style {
        background: Some(Background::Color(if dark {
            Color::from_rgb(0.14, 0.16, 0.22)
        } else {
            Color::from_rgb(0.91, 0.92, 0.96)
        })),
        border: Border {
            color: border_color(theme),
            width: 1.0,
            radius: RADIUS_XS.into(),
        },
        ..Default::default()
    }
}

/// Solid success pill (e.g. "Active" profile badge).
pub fn badge_success(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SUCCESS)),
        border: Border {
            color: SUCCESS,
            width: 0.0,
            radius: RADIUS_XS.into(),
        },
        text_color: Some(Color::WHITE),
        ..Default::default()
    }
}

/// Soft ring around a status indicator (running core).
pub fn status_ring(fill: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(with_alpha(fill, 0.22))),
        border: Border {
            color: with_alpha(fill, 0.35),
            width: 0.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

/// Soft semantic banner (errors, hints) with tinted fill.
pub fn tinted_banner(theme: &iced::Theme, accent: Color) -> container::Style {
    let dark = is_dark(theme);
    container::Style {
        background: Some(Background::Color(with_alpha(accent, if dark { 0.14 } else { 0.10 }))),
        border: Border {
            color: with_alpha(accent, if dark { 0.55 } else { 0.45 }),
            width: 1.0,
            radius: RADIUS_MD.into(),
        },
        text_color: Some(text_primary(theme)),
        ..Default::default()
    }
}

pub fn button_primary(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let disabled = matches!(status, button::Status::Disabled);
    let (bg, shadow) = match status {
        button::Status::Hovered => (
            ACCENT_PURPLE_HOVER,
            Shadow {
                color: with_alpha(ACCENT_PURPLE, 0.45),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 14.0,
            },
        ),
        button::Status::Pressed => (ACCENT_PURPLE_PRESSED, Shadow::default()),
        button::Status::Disabled => (
            Color {
                r: ACCENT_PURPLE_DISABLED.r * 0.85 + 0.12,
                g: ACCENT_PURPLE_DISABLED.g * 0.85 + 0.10,
                b: ACCENT_PURPLE_DISABLED.b * 0.85 + 0.14,
                a: 1.0,
            },
            Shadow::default(),
        ),
        button::Status::Active => (
            ACCENT_PURPLE,
            Shadow {
                color: with_alpha(ACCENT_PURPLE, 0.28),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            },
        ),
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: if disabled {
            with_alpha(Color::WHITE, 0.55)
        } else {
            Color::WHITE
        },
        border: Border {
            radius: RADIUS_SM.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow,
        ..Default::default()
    }
}

pub fn button_secondary(theme: &iced::Theme, status: button::Status) -> button::Style {
    let dark = is_dark(theme);
    let disabled = matches!(status, button::Status::Disabled);
    let (bg, border_c, text_c) = if dark {
        let b = match status {
            button::Status::Hovered => Color::from_rgb(0.16, 0.18, 0.24),
            button::Status::Pressed => Color::from_rgb(0.11, 0.12, 0.16),
            button::Status::Disabled => Color::from_rgb(0.08, 0.09, 0.12),
            button::Status::Active => CARD_DARK,
        };
        let border = if disabled {
            with_alpha(BORDER_STRONG_DARK, 0.45)
        } else {
            BORDER_STRONG_DARK
        };
        let text = if disabled { TEXT_TERTIARY } else { TEXT_PRIMARY };
        (b, border, text)
    } else {
        let b = match status {
            button::Status::Hovered => Color::from_rgb(0.94, 0.945, 0.96),
            button::Status::Pressed => Color::from_rgb(0.90, 0.91, 0.94),
            button::Status::Disabled => Color::from_rgb(0.96, 0.96, 0.97),
            button::Status::Active => CARD_LIGHT_BG,
        };
        let border = if disabled {
            with_alpha(BORDER_STRONG_LIGHT, 0.45)
        } else {
            BORDER_STRONG_LIGHT
        };
        let text = if disabled {
            TEXT_TERTIARY_LIGHT
        } else {
            TEXT_PRIMARY_LIGHT
        };
        (b, border, text)
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: text_c,
        border: Border {
            radius: RADIUS_SM.into(),
            width: 1.0,
            color: border_c,
        },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

pub fn button_danger(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => DANGER_HOVER,
        button::Status::Pressed => DANGER_PRESSED,
        button::Status::Disabled => DANGER_DISABLED,
        button::Status::Active => DANGER,
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            radius: RADIUS_SM.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: if matches!(status, button::Status::Active | button::Status::Hovered) {
            Shadow {
                color: with_alpha(DANGER, 0.30),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            }
        } else {
            Shadow::default()
        },
        ..Default::default()
    }
}

pub fn button_tab(is_active: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |theme: &iced::Theme, status: button::Status| {
        let dark = is_dark(theme);
        let bg = if is_active {
            with_alpha(ACCENT_PURPLE, if dark { 0.16 } else { 0.12 })
        } else {
            match status {
                button::Status::Hovered => {
                    if dark {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.055)
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.045)
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
                radius: RADIUS_SM.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            shadow: Shadow::default(),
            ..Default::default()
        }
    }
}

pub fn list_item_style(theme: &iced::Theme, is_selected: bool, is_hovered: bool) -> container::Style {
    let dark = is_dark(theme);
    let base_bg = if dark {
        Color::from_rgb(0.07, 0.08, 0.10)
    } else {
        Color::from_rgb(0.97, 0.975, 0.985)
    };
    let hover_bg = if dark {
        Color::from_rgb(0.10, 0.11, 0.14)
    } else {
        Color::from_rgb(0.94, 0.945, 0.96)
    };
    let selected_bg = if dark {
        Color {
            r: ACCENT_PURPLE.r * 0.12 + base_bg.r * 0.88,
            g: ACCENT_PURPLE.g * 0.12 + base_bg.g * 0.88,
            b: ACCENT_PURPLE.b * 0.12 + base_bg.b * 0.88,
            a: 1.0,
        }
    } else {
        CARD_SELECTED_LIGHT
    };

    let fill = if is_selected {
        selected_bg
    } else if is_hovered {
        hover_bg
    } else {
        base_bg
    };

    container::Style {
        background: Some(Background::Color(fill)),
        border: Border {
            color: if is_selected {
                ACCENT_PURPLE
            } else {
                border_color(theme)
            },
            width: if is_selected { 1.5 } else { 1.0 },
            radius: RADIUS_SM.into(),
        },
        ..Default::default()
    }
}

pub fn console_bg(theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(console_surface(theme))),
        border: Border {
            color: border_color(theme),
            width: 1.0,
            radius: RADIUS_MD.into(),
        },
        text_color: Some(text_primary(theme)),
        ..Default::default()
    }
}

pub fn input_field(theme: &iced::Theme, status: text_input::Status) -> text_input::Style {
    let dark = is_dark(theme);
    let border_default = if dark {
        BORDER_STRONG_DARK
    } else {
        BORDER_STRONG_LIGHT
    };
    let border_color = match status {
        text_input::Status::Focused { .. } => ACCENT_PURPLE,
        text_input::Status::Hovered => {
            if dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.22)
            } else {
                Color::from_rgba(0.10, 0.12, 0.18, 0.22)
            }
        }
        _ => border_default,
    };

    text_input::Style {
        background: Background::Color(input_surface(theme)),
        border: Border {
            radius: RADIUS_SM.into(),
            width: 1.0,
            color: border_color,
        },
        value: text_primary(theme),
        placeholder: text_muted(theme),
        selection: with_alpha(ACCENT_PURPLE, 0.32),
        icon: text_muted(theme),
    }
}

pub fn pick_list(theme: &iced::Theme, status: iced_pick_list::Status) -> iced_pick_list::Style {
    let dark = is_dark(theme);
    let border_default = if dark {
        BORDER_STRONG_DARK
    } else {
        BORDER_STRONG_LIGHT
    };
    let border_c = match status {
        iced_pick_list::Status::Opened { .. } => ACCENT_PURPLE,
        iced_pick_list::Status::Hovered => {
            if dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.22)
            } else {
                Color::from_rgba(0.10, 0.12, 0.18, 0.22)
            }
        }
        _ => border_default,
    };

    iced_pick_list::Style {
        background: Background::Color(input_surface(theme)),
        border: Border {
            radius: RADIUS_SM.into(),
            width: 1.0,
            color: border_c,
        },
        text_color: text_primary(theme),
        placeholder_color: text_muted(theme),
        handle_color: text_muted(theme),
    }
}

// ── Contrast / palette unit tests ────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn channel_l(c: f32) -> f32 {
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    fn relative_luminance(c: Color) -> f32 {
        0.2126 * channel_l(c.r) + 0.7152 * channel_l(c.g) + 0.0722 * channel_l(c.b)
    }

    fn contrast_ratio(a: Color, b: Color) -> f32 {
        let l1 = relative_luminance(a);
        let l2 = relative_luminance(b);
        let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
        (hi + 0.05) / (lo + 0.05)
    }

    #[test]
    fn dark_and_light_palettes_are_distinct_layers() {
        // Backgrounds differ from card surfaces
        assert!(BG_DARK != CARD_DARK);
        assert!(BG_LIGHT != CARD_LIGHT_BG);
        assert!(SIDEBAR_BG != BG_DARK);
        assert!(SIDEBAR_BG_LIGHT != BG_LIGHT);

        // Borders are not fully transparent
        assert!(BORDER_DARK.a >= 0.08);
        assert!(BORDER_LIGHT.a >= 0.08);

        // Light border is visible enough (alpha was historically 0.03)
        assert!(BORDER_LIGHT.a > 0.05);

        // Text hierarchy: primary brighter/darker than muted in each mode
        assert!(relative_luminance(TEXT_PRIMARY) > relative_luminance(TEXT_MUTED));
        assert!(relative_luminance(TEXT_MUTED) > relative_luminance(TEXT_TERTIARY));
        assert!(relative_luminance(TEXT_PRIMARY_LIGHT) < relative_luminance(TEXT_MUTED_LIGHT));
        assert!(relative_luminance(TEXT_MUTED_LIGHT) < relative_luminance(TEXT_TERTIARY_LIGHT));
    }

    #[test]
    fn primary_text_contrasts_against_surfaces() {
        // WCAG-ish floor for large text / UI chrome (~3:1)
        assert!(contrast_ratio(TEXT_PRIMARY, BG_DARK) >= 3.0);
        assert!(contrast_ratio(TEXT_PRIMARY, CARD_DARK) >= 3.0);
        assert!(contrast_ratio(TEXT_PRIMARY_LIGHT, BG_LIGHT) >= 3.0);
        assert!(contrast_ratio(TEXT_PRIMARY_LIGHT, CARD_LIGHT_BG) >= 3.0);
        // Muted should still be readable on card
        assert!(contrast_ratio(TEXT_MUTED, CARD_DARK) >= 2.0);
        assert!(contrast_ratio(TEXT_MUTED_LIGHT, CARD_LIGHT_BG) >= 2.5);
    }

    #[test]
    fn token_accessors_flip_with_theme() {
        let dark = iced::Theme::Dark;
        let light = iced::Theme::Light;
        assert!(is_dark(&dark));
        assert!(!is_dark(&light));
        assert_eq!(bg(&dark), BG_DARK);
        assert_eq!(bg(&light), BG_LIGHT);
        assert_eq!(card_surface(&dark), CARD_DARK);
        assert_eq!(card_surface(&light), CARD_LIGHT_BG);
        assert_eq!(text_primary(&dark), TEXT_PRIMARY);
        assert_eq!(text_primary(&light), TEXT_PRIMARY_LIGHT);
        assert_eq!(border_color(&dark), BORDER_DARK);
        assert_eq!(border_color(&light), BORDER_LIGHT);
        // Accents/semantics are mode-shared
        assert_eq!(ACCENT_PURPLE.a, 1.0);
        assert_eq!(SUCCESS.a, 1.0);
        assert_eq!(WARNING.a, 1.0);
        assert_eq!(DANGER.a, 1.0);
    }

    #[test]
    fn chrome_styles_emit_expected_roles() {
        let dark = iced::Theme::Dark;
        let light = iced::Theme::Light;

        let card_d = card_bg(&dark);
        assert!(card_d.background.is_some());
        assert!(card_d.border.radius.top_left >= RADIUS_LG - 0.1);
        assert!(card_d.border.width >= 1.0);

        let card_l = card_bg(&light);
        assert_ne!(
            card_d.background.unwrap(),
            card_l.background.unwrap()
        );

        let sel = card_selected(&dark);
        assert_eq!(sel.border.color, ACCENT_PURPLE);

        let primary = button_primary(&dark, button::Status::Active);
        assert_eq!(primary.text_color, Color::WHITE);
        let primary_dis = button_primary(&dark, button::Status::Disabled);
        assert!(primary_dis.text_color.a < 1.0);
        let secondary_dis = button_secondary(&dark, button::Status::Disabled);
        assert_eq!(secondary_dis.text_color, TEXT_TERTIARY);
        assert!(matches!(primary.background, Some(Background::Color(c)) if c == ACCENT_PURPLE));

        let secondary_l = button_secondary(&light, button::Status::Active);
        assert_eq!(secondary_l.text_color, TEXT_PRIMARY_LIGHT);
        assert!(secondary_l.border.width >= 1.0);
        // Light secondary border must not be near-invisible
        assert!(secondary_l.border.color.a >= 0.08);

        let input_l = input_field(&light, text_input::Status::Active);
        assert!(input_l.border.color.a >= 0.08);
    }

    #[test]
    fn with_alpha_preserves_rgb() {
        let c = with_alpha(SUCCESS, 0.2);
        assert!((c.r - SUCCESS.r).abs() < 0.001);
        assert!((c.a - 0.2).abs() < 0.001);
    }
}
