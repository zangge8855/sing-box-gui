//! Design tokens and shared chrome styles for sing-box-gui.
//!
//! "Celadon & Ink": a quiet, paper-and-ink neutral ramp with a single jade
//! accent and a warm ochre counterpart. Light mode separates layers with a
//! whisper of shadow; dark mode relies on hairline borders and tonal steps.
//! Single source of visual truth for the shell and every tab.

use iced::widget::{button, container, pick_list as iced_pick_list, text_input};
use iced::{Background, Border, Color, Shadow};

#[cfg(target_os = "windows")]
pub const UI_FONT_FAMILY: iced::font::Family = iced::font::Family::Name("Segoe UI");
// Helvetica Neue ships with macOS; the separately downloadable "SF Pro"
// family does not. Missing named fonts break even ASCII punctuation in Iced.
#[cfg(target_os = "macos")]
pub const UI_FONT_FAMILY: iced::font::Family = iced::font::Family::Name("Helvetica Neue");
// Respect Fontconfig on GNOME, KDE and other desktops instead of requiring
// Ubuntu's font on Fedora/Arch. The bundled Noto collection supplies CJK.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const UI_FONT_FAMILY: iced::font::Family = iced::font::Family::SansSerif;

pub fn ui_font(weight: iced::font::Weight) -> iced::Font {
    iced::Font {
        family: UI_FONT_FAMILY,
        weight,
        ..Default::default()
    }
}

// ── Radius language ──────────────────────────────────────────────────────────
// One rhythm on every OS: soft 12px surfaces, 8px controls, 6px chips.
pub const RADIUS_LG: f32 = 12.0;
pub const RADIUS_MD: f32 = 8.0;
pub const RADIUS_SM: f32 = 6.0;
pub const RADIUS_XS: f32 = 4.0;
pub const RADIUS_MICRO: f32 = 3.0;

/// Font for numeric metrics (latency, speeds).
///
/// Prefer system **SansSerif** over `Family::Monospace`: iced on Windows often
/// fails to resolve a mono face, so digits and "ms" render as tofu blocks (□).
/// Medium weight keeps metrics slightly emphasized without needing a mono font.
pub fn metric_font() -> iced::Font {
    ui_font(iced::font::Weight::Medium)
}

/// Light-weight face for large display numbers (dashboard speeds).
pub fn display_font() -> iced::Font {
    ui_font(iced::font::Weight::Light)
}

/// Font for logs and configuration previews.
///
/// Iced's automatic shaping uses the bundled CJK fallback for non-ASCII text.
/// cosmic-text maps the generic monospace family to "Noto Sans Mono", which
/// Windows and macOS lack, so there it lands on Courier New. Name a face each
/// OS always ships instead.
pub fn mono_font() -> iced::Font {
    let family = if cfg!(target_os = "windows") {
        iced::font::Family::Name("Consolas")
    } else if cfg!(target_os = "macos") {
        iced::font::Family::Name("Menlo")
    } else {
        iced::font::Family::Monospace
    };
    iced::Font {
        family,
        ..Default::default()
    }
}

// ── Type scale (f32 for iced::Pixels) ────────────────────────────────────────
pub const TYPE_DISPLAY: f32 = 30.0; // hero node name, large speeds
pub const TYPE_TITLE: f32 = 24.0; // page titles
pub const TYPE_HEADING: f32 = 15.0; // card titles — semibold primary
pub const TYPE_BODY: f32 = 14.0;
pub const TYPE_SECTION: f32 = 14.0; // labels inside cards
pub const TYPE_BTN_LG: f32 = 14.0;
pub const TYPE_BTN_MD: f32 = 14.0;
pub const TYPE_BTN_SM: f32 = 13.0;
pub const TYPE_MONO: f32 = 13.0; // latency, mono captions
pub const TYPE_CAPTION: f32 = 12.0;
pub const TYPE_MICRO: f32 = 11.0; // badges only

// ── Spacing / padding presets ────────────────────────────────────────────────
#[cfg(target_os = "macos")]
pub const CARD_PAD: f32 = 22.0;
#[cfg(target_os = "macos")]
pub const GRID_GAP: f32 = 18.0;

#[cfg(not(target_os = "macos"))]
pub const CARD_PAD: f32 = 20.0;
#[cfg(not(target_os = "macos"))]
pub const GRID_GAP: f32 = 16.0;

pub const CARD_PAD_DENSE: f32 = 16.0;
pub const BTN_PAD_SM: [u16; 2] = [7, 12];
pub const BTN_PAD_MD: [u16; 2] = [9, 16];
pub const BTN_PAD_LG: [u16; 2] = [11, 20];
/// Buttons beside a search field or picker: same 34px height as the inputs.
pub const BTN_PAD_TOOLBAR: [u16; 2] = [8, 14];
/// Shared width for page header search inputs.
pub const SEARCH_WIDTH: f32 = 260.0;
pub const SP_8: f32 = 8.0;
pub const SP_12: f32 = 12.0;

pub const SP_40: f32 = 40.0;

// ── Dark mode (ink: green-black with a celadon undertone) ────────────────────
pub const BG_DARK: Color = Color::from_rgb8(0x0F, 0x15, 0x13);
pub const SIDEBAR_BG: Color = Color::from_rgb8(0x0B, 0x10, 0x0F);
pub const CARD_DARK: Color = Color::from_rgb8(0x15, 0x1C, 0x1A);
pub const CARD_ELEVATED_DARK: Color = Color::from_rgb8(0x1B, 0x24, 0x21);
pub const CARD_HOVER: Color = Color::from_rgb8(0x1F, 0x29, 0x26);
pub const BORDER_DARK: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.08);
pub const BORDER_STRONG_DARK: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.14);
pub const TEXT_PRIMARY_DARK: Color = Color::from_rgb8(0xE7, 0xEE, 0xEB);
pub const TEXT_MUTED_DARK: Color = Color::from_rgb8(0x9D, 0xAD, 0xA8);
pub const TEXT_TERTIARY_DARK: Color = Color::from_rgb8(0x89, 0x98, 0x93);
pub const INPUT_BG_DARK: Color = Color::from_rgb8(0x10, 0x17, 0x15);
pub const CONSOLE_BG_DARK: Color = Color::from_rgb8(0x0B, 0x10, 0x0F);

// ── Light mode (paper: cool off-white with a faint green cast) ───────────────
pub const BG_LIGHT: Color = Color::from_rgb8(0xF3, 0xF5, 0xF4);
pub const SIDEBAR_BG_LIGHT: Color = Color::from_rgb8(0xEA, 0xEF, 0xED);
pub const CARD_LIGHT_BG: Color = Color::from_rgb8(0xFF, 0xFF, 0xFF);
pub const CARD_SELECTED_LIGHT: Color = Color::from_rgb8(0xEC, 0xF5, 0xF1);
pub const CARD_HOVER_LIGHT: Color = Color::from_rgb8(0xF7, 0xFA, 0xF9);
pub const BORDER_LIGHT: Color = Color::from_rgba(0.082, 0.125, 0.114, 0.10);
pub const BORDER_STRONG_LIGHT: Color = Color::from_rgba(0.082, 0.125, 0.114, 0.17);
pub const TEXT_PRIMARY_LIGHT: Color = Color::from_rgb8(0x15, 0x20, 0x1D);
pub const TEXT_MUTED_LIGHT: Color = Color::from_rgb8(0x4A, 0x58, 0x54);
pub const TEXT_TERTIARY_LIGHT: Color = Color::from_rgb8(0x5E, 0x6C, 0x68);
pub const INPUT_BG_LIGHT: Color = Color::from_rgb8(0xF6, 0xF8, 0xF7);
pub const CONSOLE_BG_LIGHT: Color = Color::from_rgb8(0xF7, 0xF9, 0xF8);

// ── Accent + semantic ────────────────────────────────────────────────────────
/// Celadon jade — the one brand color. Carries white text on light surfaces.
pub const ACCENT: Color = Color::from_rgb8(0x2E, 0x7D, 0x6B);
pub const ACCENT_HOVER: Color = Color::from_rgb8(0x28, 0x70, 0x60);
pub const ACCENT_PRESSED: Color = Color::from_rgb8(0x21, 0x5E, 0x51);
/// Luminous jade for fills on dark surfaces; carries ink text.
pub const ACCENT_BRIGHT: Color = Color::from_rgb8(0x6C, 0xC0, 0xAA);
pub const ACCENT_BRIGHT_HOVER: Color = Color::from_rgb8(0x7E, 0xCB, 0xB7);
pub const ACCENT_BRIGHT_PRESSED: Color = Color::from_rgb8(0x5A, 0xAE, 0x98);
/// Ink used for text placed on `ACCENT_BRIGHT`.
pub const ACCENT_INK: Color = Color::from_rgb8(0x0B, 0x1A, 0x16);
/// Slate blue for neutral information.
pub const INFO: Color = Color::from_rgb8(0x3D, 0x7E, 0xB0);
pub const SUCCESS: Color = Color::from_rgb8(0x2F, 0xA0, 0x6A);
pub const WARNING: Color = Color::from_rgb8(0xD3, 0x95, 0x2A);
/// Cinnabar — reserved for destructive actions and failures.
pub const DANGER: Color = Color::from_rgb8(0xB4, 0x3A, 0x2E);
pub const DANGER_HOVER: Color = Color::from_rgb8(0xA6, 0x33, 0x28);
pub const DANGER_PRESSED: Color = Color::from_rgb8(0x93, 0x2C, 0x23);
pub const DANGER_DISABLED: Color = Color::from_rgb8(0x6A, 0x3B, 0x36);

/// Map proxy protocol to a distinct, muted hue for visual scanning. Tones sit
/// at mid luminance so the same hue reads on paper and ink once tinted.
pub fn protocol_color(node_type: &str) -> Color {
    match node_type.trim().to_ascii_lowercase().as_str() {
        "hysteria" | "hysteria2" | "hy2" => Color::from_rgb8(0x7B, 0x66, 0xC4),
        "vless" => Color::from_rgb8(0x2F, 0x8E, 0xA2),
        "vmess" => Color::from_rgb8(0xC0, 0x7B, 0x2E),
        "shadowsocks" | "ss" => Color::from_rgb8(0x3A, 0x98, 0x6C),
        "trojan" => Color::from_rgb8(0x4C, 0x76, 0xC0),
        "tuic" => Color::from_rgb8(0xC0, 0x5A, 0x86),
        "wireguard" | "wg" => Color::from_rgb8(0x93, 0x5A, 0xB2),
        "direct" => Color::from_rgb8(0x5E, 0x96, 0x58),
        "block" | "reject" => Color::from_rgb8(0xC0, 0x4A, 0x3E),
        _ => ACCENT,
    }
}

/// Region badges use letters on every OS: Windows does not render flag
/// emoji reliably. Short country codes must be tokens, not substrings ("us"
/// in "business", "de" in "node", or "in" in "direct" are not locations).
pub fn detect_region_code(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    let tokens: Vec<_> = lower
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|s| !s.is_empty())
        .collect();
    if lower.contains("自动")
        || lower.contains("直连")
        || tokens.iter().any(|t| matches!(*t, "auto" | "direct"))
    {
        return None;
    }
    for (code, flag, keywords) in [
        ("HK", "🇭🇰", &["香港", "hongkong", "hong kong"][..]),
        ("JP", "🇯🇵", &["日本", "japan", "tokyo", "osaka"][..]),
        (
            "US",
            "🇺🇸",
            &[
                "美国",
                "united states",
                "america",
                "los angeles",
                "san jose",
            ][..],
        ),
        ("SG", "🇸🇬", &["新加坡", "singapore", "狮城"][..]),
        ("TW", "🇹🇼", &["台湾", "taiwan", "台北"][..]),
        ("KR", "🇰🇷", &["韩国", "korea", "首尔", "seoul"][..]),
        ("DE", "🇩🇪", &["德国", "germany", "frankfurt"][..]),
        (
            "GB",
            "🇬🇧",
            &["英国", "britain", "united kingdom", "london"][..],
        ),
        ("CA", "🇨🇦", &["加拿大", "canada", "toronto"][..]),
        ("AU", "🇦🇺", &["澳大利亚", "australia", "sydney"][..]),
        ("RU", "🇷🇺", &["俄罗斯", "russia", "moscow"][..]),
        ("FR", "🇫🇷", &["法国", "france", "paris"][..]),
        ("NL", "🇳🇱", &["荷兰", "netherlands", "amsterdam"][..]),
        ("TR", "🇹🇷", &["土耳其", "turkey", "istanbul"][..]),
        ("IN", "🇮🇳", &["印度", "india"][..]),
    ] {
        if name.contains(flag)
            || keywords.iter().any(|keyword| lower.contains(keyword))
            || tokens
                .iter()
                .any(|token| token.eq_ignore_ascii_case(code) || (code == "GB" && *token == "uk"))
        {
            return Some(code);
        }
    }
    None
}

// ── Token accessors ──────────────────────────────────────────────────────────

pub fn is_dark(theme: &iced::Theme) -> bool {
    !matches!(
        theme,
        iced::Theme::Light
            | iced::Theme::SolarizedLight
            | iced::Theme::GruvboxLight
            | iced::Theme::TokyoNightLight
            | iced::Theme::KanagawaLotus
    )
}

pub fn bg(theme: &iced::Theme) -> Color {
    if is_dark(theme) { BG_DARK } else { BG_LIGHT }
}

pub fn sidebar_surface(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        SIDEBAR_BG
    } else {
        SIDEBAR_BG_LIGHT
    }
}

pub fn card_surface(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        CARD_DARK
    } else {
        CARD_LIGHT_BG
    }
}

pub fn elevated_surface(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        CARD_ELEVATED_DARK
    } else {
        CARD_HOVER_LIGHT
    }
}

pub fn border_color(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        BORDER_DARK
    } else {
        BORDER_LIGHT
    }
}

pub fn border_strong(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        BORDER_STRONG_DARK
    } else {
        BORDER_STRONG_LIGHT
    }
}

pub fn text_primary(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        TEXT_PRIMARY_DARK
    } else {
        TEXT_PRIMARY_LIGHT
    }
}

pub fn text_muted(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        TEXT_MUTED_DARK
    } else {
        TEXT_MUTED_LIGHT
    }
}

pub fn text_tertiary(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        TEXT_TERTIARY_DARK
    } else {
        TEXT_TERTIARY_LIGHT
    }
}

/// Accent fill for marks, focus rings, toggles and chart strokes.
pub fn accent(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        ACCENT_BRIGHT
    } else {
        ACCENT
    }
}

/// Foreground placed on top of an `accent()` fill.
pub fn on_accent(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        ACCENT_INK
    } else {
        Color::WHITE
    }
}

/// Semantic foregrounds need darker variants on light surfaces. The bright
/// chart/badge fills above are decorative colors, not small-text colors.
pub fn success_text(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        Color::from_rgb8(0x62, 0xC9, 0x91)
    } else {
        Color::from_rgb8(0x1E, 0x7A, 0x4C)
    }
}

pub fn warning_text(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        Color::from_rgb8(0xE3, 0xB3, 0x5A)
    } else {
        Color::from_rgb8(0x8A, 0x5A, 0x0E)
    }
}

pub fn danger_text(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        Color::from_rgb8(0xF0, 0x8C, 0x80)
    } else {
        Color::from_rgb8(0xA8, 0x32, 0x2A)
    }
}

pub fn info_text(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        Color::from_rgb8(0x86, 0xBA, 0xE0)
    } else {
        Color::from_rgb8(0x2B, 0x63, 0x90)
    }
}

pub fn accent_text(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        Color::from_rgb8(0x7C, 0xCB, 0xB7)
    } else {
        Color::from_rgb8(0x22, 0x69, 0x5A)
    }
}

/// Traffic chart series (download, upload). Chart marks need more chroma
/// than UI accents and ≥3:1 against the card in each mode; this pair passes
/// the lightness band, chroma floor and colour-blind separation checks.
pub fn chart_series(theme: &iced::Theme) -> (Color, Color) {
    if is_dark(theme) {
        (
            Color::from_rgb8(0x30, 0xA3, 0x83),
            Color::from_rgb8(0xBC, 0x82, 0x26),
        )
    } else {
        (
            Color::from_rgb8(0x17, 0x8F, 0x72),
            Color::from_rgb8(0xB2, 0x7A, 0x20),
        )
    }
}

pub fn input_surface(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        INPUT_BG_DARK
    } else {
        INPUT_BG_LIGHT
    }
}

pub fn console_surface(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        CONSOLE_BG_DARK
    } else {
        CONSOLE_BG_LIGHT
    }
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

/// Opaque blend of `over` onto `base` (`amount` = share of `over`).
pub fn mix(base: Color, over: Color, amount: f32) -> Color {
    let t = amount.clamp(0.0, 1.0);
    Color {
        r: base.r + (over.r - base.r) * t,
        g: base.g + (over.g - base.g) * t,
        b: base.b + (over.b - base.b) * t,
        a: 1.0,
    }
}

/// Faint hover / pressed wash that works over any surface.
pub fn hover_wash(theme: &iced::Theme, pressed: bool) -> Color {
    let dark = is_dark(theme);
    match (dark, pressed) {
        (true, false) => Color::from_rgba(1.0, 1.0, 1.0, 0.05),
        (true, true) => Color::from_rgba(1.0, 1.0, 1.0, 0.08),
        (false, false) => Color::from_rgba(0.08, 0.13, 0.11, 0.045),
        (false, true) => Color::from_rgba(0.08, 0.13, 0.11, 0.075),
    }
}

/// Paper-like lift for light surfaces; dark mode separates with borders only.
fn surface_shadow(theme: &iced::Theme) -> Shadow {
    if is_dark(theme) {
        Shadow::default()
    } else {
        Shadow {
            color: Color::from_rgba(0.06, 0.14, 0.11, 0.06),
            offset: iced::Vector::new(0.0, 1.0),
            blur_radius: 6.0,
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
    container::Style {
        background: Some(Background::Color(sidebar_surface(theme))),
        text_color: Some(text_primary(theme)),
        ..Default::default()
    }
}

/// One-pixel rule (separators between sidebar/content, list sections).
pub fn hairline(theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(border_color(theme))),
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
        shadow: surface_shadow(theme),
        text_color: Some(text_primary(theme)),
        ..Default::default()
    }
}

/// Floating surfaces (tooltips, toasts, menus): raised and always outlined.
pub fn popover_bg(theme: &iced::Theme) -> container::Style {
    let dark = is_dark(theme);
    container::Style {
        background: Some(Background::Color(if dark {
            CARD_ELEVATED_DARK
        } else {
            CARD_LIGHT_BG
        })),
        border: Border {
            color: border_strong(theme),
            width: 1.0,
            radius: RADIUS_MD.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, if dark { 0.40 } else { 0.10 }),
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        text_color: Some(text_primary(theme)),
        ..Default::default()
    }
}

/// The dashboard hero: a card washed with a breath of jade while the core
/// runs, fading diagonally into the plain surface.
pub fn hero_panel(active: bool) -> impl Fn(&iced::Theme) -> container::Style {
    move |theme: &iced::Theme| {
        let dark = is_dark(theme);
        let base = card_surface(theme);
        let mut style = card_bg(theme);
        if active {
            let wash = mix(base, accent(theme), if dark { 0.10 } else { 0.07 });
            style.background = Some(Background::Gradient(iced::Gradient::Linear(
                iced::gradient::Linear::new(iced::Degrees(120.0))
                    .add_stop(0.0, wash)
                    .add_stop(0.65, base),
            )));
            style.border.color = with_alpha(accent(theme), if dark { 0.22 } else { 0.20 });
        }
        style
    }
}

pub fn card_selected(theme: &iced::Theme) -> container::Style {
    let dark = is_dark(theme);
    let bg = if dark {
        mix(CARD_DARK, ACCENT_BRIGHT, 0.08)
    } else {
        CARD_SELECTED_LIGHT
    };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: accent(theme),
            width: 1.5,
            radius: RADIUS_LG.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(text_primary(theme)),
        ..Default::default()
    }
}

/// Recessed, borderless panel used for empty states and quiet groupings.
pub fn status_card(theme: &iced::Theme) -> container::Style {
    let dark = is_dark(theme);
    let bg = if dark {
        Color::from_rgb8(0x12, 0x19, 0x17)
    } else {
        Color::from_rgb8(0xEE, 0xF2, 0xF0)
    };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border_color(theme),
            width: 1.0,
            radius: RADIUS_LG.into(),
        },
        shadow: Shadow::default(),
        text_color: Some(text_primary(theme)),
        ..Default::default()
    }
}

/// Neutral chip surface (region codes, counters).
pub fn badge_bg(theme: &iced::Theme) -> container::Style {
    let dark = is_dark(theme);
    container::Style {
        background: Some(Background::Color(if dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.06)
        } else {
            Color::from_rgba(0.08, 0.13, 0.11, 0.06)
        })),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: RADIUS_XS.into(),
        },
        text_color: Some(text_muted(theme)),
        ..Default::default()
    }
}

/// Soft success chip (e.g. "Active" profile badge).
pub fn badge_success(theme: &iced::Theme) -> container::Style {
    badge_colored(SUCCESS, theme).with_text(success_text(theme))
}

/// Chip tinted with `color`; text uses a theme-adjusted shade of the same hue.
pub fn badge_colored(color: Color, theme: &iced::Theme) -> container::Style {
    let dark = is_dark(theme);
    container::Style {
        background: Some(Background::Color(with_alpha(
            color,
            if dark { 0.16 } else { 0.11 },
        ))),
        border: Border {
            color: with_alpha(color, if dark { 0.28 } else { 0.22 }),
            width: 1.0,
            radius: RADIUS_XS.into(),
        },
        text_color: Some(badge_text(color, theme)),
        ..Default::default()
    }
}

/// Readable foreground for a hue placed on its own soft tint.
pub fn badge_text(color: Color, theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        mix(color, Color::WHITE, 0.38)
    } else {
        mix(color, Color::BLACK, 0.32)
    }
}

trait WithText {
    fn with_text(self, color: Color) -> Self;
}

impl WithText for container::Style {
    fn with_text(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }
}

/// Soft ring around a status indicator (running core).
pub fn status_ring(fill: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(with_alpha(fill, 0.20))),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

/// Soft semantic banner (errors, hints) with tinted fill and no hard frame.
pub fn tinted_banner(theme: &iced::Theme, accent: Color) -> container::Style {
    let dark = is_dark(theme);
    container::Style {
        background: Some(Background::Color(with_alpha(
            accent,
            if dark { 0.12 } else { 0.09 },
        ))),
        border: Border {
            color: with_alpha(accent, if dark { 0.30 } else { 0.26 }),
            width: 1.0,
            radius: RADIUS_MD.into(),
        },
        text_color: Some(text_primary(theme)),
        ..Default::default()
    }
}

pub fn button_primary(theme: &iced::Theme, status: button::Status) -> button::Style {
    let dark = is_dark(theme);
    let (bg, text) = if dark {
        match status {
            button::Status::Hovered => (ACCENT_BRIGHT_HOVER, ACCENT_INK),
            button::Status::Pressed => (ACCENT_BRIGHT_PRESSED, ACCENT_INK),
            button::Status::Disabled => (
                mix(CARD_DARK, ACCENT_BRIGHT, 0.22),
                with_alpha(TEXT_PRIMARY_DARK, 0.45),
            ),
            button::Status::Active => (ACCENT_BRIGHT, ACCENT_INK),
        }
    } else {
        match status {
            button::Status::Hovered => (ACCENT_HOVER, Color::WHITE),
            button::Status::Pressed => (ACCENT_PRESSED, Color::WHITE),
            button::Status::Disabled => (
                mix(CARD_LIGHT_BG, ACCENT, 0.35),
                with_alpha(Color::WHITE, 0.85),
            ),
            button::Status::Active => (ACCENT, Color::WHITE),
        }
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: text,
        border: Border {
            radius: RADIUS_MD.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: if !dark && matches!(status, button::Status::Active | button::Status::Hovered) {
            Shadow {
                color: with_alpha(ACCENT_PRESSED, 0.22),
                offset: iced::Vector::new(0.0, 1.0),
                blur_radius: 3.0,
            }
        } else {
            Shadow::default()
        },
        ..Default::default()
    }
}

pub fn button_secondary(theme: &iced::Theme, status: button::Status) -> button::Style {
    let dark = is_dark(theme);
    let disabled = matches!(status, button::Status::Disabled);
    let bg = match status {
        button::Status::Hovered => {
            if dark {
                CARD_HOVER
            } else {
                CARD_HOVER_LIGHT
            }
        }
        button::Status::Pressed => {
            if dark {
                CARD_ELEVATED_DARK
            } else {
                Color::from_rgb8(0xEE, 0xF2, 0xF0)
            }
        }
        button::Status::Disabled | button::Status::Active => card_surface(theme),
    };
    let strong = border_strong(theme);
    let border_c = if disabled {
        // Fade the enabled border; `with_alpha` sets (not multiplies) alpha.
        with_alpha(strong, strong.a * 0.5)
    } else {
        strong
    };
    let text_c = if disabled {
        text_tertiary(theme)
    } else {
        text_primary(theme)
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: text_c,
        border: Border {
            radius: RADIUS_MD.into(),
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
        text_color: if matches!(status, button::Status::Disabled) {
            with_alpha(Color::WHITE, 0.6)
        } else {
            Color::WHITE
        },
        border: Border {
            radius: RADIUS_MD.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

/// Destructive action that isn't the page's main event (e.g. "close all"):
/// a danger-tinted wash with danger ink, filling to solid only on press.
pub fn button_danger_soft(theme: &iced::Theme, status: button::Status) -> button::Style {
    let (bg, fg) = match status {
        button::Status::Active => (with_alpha(DANGER, 0.10), danger_text(theme)),
        button::Status::Hovered => (with_alpha(DANGER, 0.16), danger_text(theme)),
        button::Status::Pressed => (DANGER_PRESSED, Color::WHITE),
        button::Status::Disabled => (with_alpha(DANGER, 0.05), text_tertiary(theme)),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: fg,
        border: Border {
            radius: RADIUS_MD.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

/// Quiet dismiss/close glyph: tertiary ink at rest, danger-tinted under the
/// pointer so the destructive intent shows only when it's about to happen.
pub fn button_dismiss(theme: &iced::Theme, status: button::Status) -> button::Style {
    let (bg, fg) = match status {
        button::Status::Active => (None, text_tertiary(theme)),
        button::Status::Hovered => (Some(with_alpha(DANGER, 0.12)), danger_text(theme)),
        button::Status::Pressed => (Some(with_alpha(DANGER, 0.20)), danger_text(theme)),
        button::Status::Disabled => (None, with_alpha(text_tertiary(theme), 0.5)),
    };
    button::Style {
        background: bg.map(Background::Color),
        text_color: fg,
        border: Border {
            radius: RADIUS_MD.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

/// Segmented-control / side-list item. The active item is lifted onto the
/// card surface; inactive items stay flat and muted.
pub fn button_tab(is_active: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |theme: &iced::Theme, status: button::Status| {
        let dark = is_dark(theme);
        if is_active {
            button::Style {
                background: Some(Background::Color(if dark {
                    CARD_ELEVATED_DARK
                } else {
                    CARD_LIGHT_BG
                })),
                text_color: text_primary(theme),
                border: Border {
                    radius: RADIUS_MD.into(),
                    width: 1.0,
                    color: border_strong(theme),
                },
                shadow: surface_shadow(theme),
                ..Default::default()
            }
        } else {
            let bg = match status {
                button::Status::Hovered => Some(Background::Color(hover_wash(theme, false))),
                // Deepen (not drop) the tint while held.
                button::Status::Pressed => Some(Background::Color(hover_wash(theme, true))),
                _ => None,
            };
            button::Style {
                background: bg,
                text_color: text_muted(theme),
                border: Border {
                    radius: RADIUS_MD.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                shadow: Shadow::default(),
                ..Default::default()
            }
        }
    }
}

/// Track that hosts `button_tab` segments.
pub fn segmented_track(theme: &iced::Theme) -> container::Style {
    let dark = is_dark(theme);
    container::Style {
        background: Some(Background::Color(if dark {
            Color::from_rgb8(0x10, 0x16, 0x14)
        } else {
            Color::from_rgb8(0xEC, 0xF0, 0xEE)
        })),
        border: Border {
            color: border_color(theme),
            width: 1.0,
            radius: (RADIUS_MD + 3.0).into(),
        },
        ..Default::default()
    }
}

/// Sidebar navigation item. Active: soft jade wash + jade label.
pub fn nav_item(is_active: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |theme: &iced::Theme, status: button::Status| {
        let dark = is_dark(theme);
        let bg = if is_active {
            Some(Background::Color(with_alpha(
                accent(theme),
                if dark { 0.12 } else { 0.10 },
            )))
        } else {
            match status {
                button::Status::Hovered => Some(Background::Color(hover_wash(theme, false))),
                button::Status::Pressed => Some(Background::Color(hover_wash(theme, true))),
                _ => None,
            }
        };
        button::Style {
            background: bg,
            text_color: if is_active {
                accent_text(theme)
            } else {
                text_muted(theme)
            },
            border: Border {
                radius: RADIUS_MD.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            shadow: Shadow::default(),
            ..Default::default()
        }
    }
}

pub fn button_header(theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(Background::Color(hover_wash(theme, false))),
        button::Status::Pressed => Some(Background::Color(hover_wash(theme, true))),
        _ => None,
    };
    button::Style {
        background: bg,
        text_color: text_muted(theme),
        border: Border {
            radius: RADIUS_SM.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

/// Large round power control on the dashboard hero.
pub fn power_button(running: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |theme: &iced::Theme, status: button::Status| {
        let dark = is_dark(theme);
        let hovered = matches!(status, button::Status::Hovered);
        let pressed = matches!(status, button::Status::Pressed);
        let disabled = matches!(status, button::Status::Disabled);
        let radius = 999.0;
        if running {
            let fill = if pressed {
                if dark {
                    ACCENT_BRIGHT_PRESSED
                } else {
                    ACCENT_PRESSED
                }
            } else if hovered {
                if dark {
                    ACCENT_BRIGHT_HOVER
                } else {
                    ACCENT_HOVER
                }
            } else {
                accent(theme)
            };
            button::Style {
                background: Some(Background::Color(fill)),
                text_color: on_accent(theme),
                border: Border {
                    radius: radius.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                // The halo: the memorable glow of a live connection.
                shadow: Shadow {
                    color: with_alpha(accent(theme), if dark { 0.38 } else { 0.32 }),
                    offset: iced::Vector::new(0.0, 0.0),
                    blur_radius: if hovered { 26.0 } else { 20.0 },
                },
                ..Default::default()
            }
        } else {
            let bg = if pressed {
                mix(card_surface(theme), accent(theme), 0.14)
            } else if hovered {
                mix(card_surface(theme), accent(theme), 0.07)
            } else {
                card_surface(theme)
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: if disabled {
                    text_tertiary(theme)
                } else if hovered || pressed {
                    accent_text(theme)
                } else {
                    text_muted(theme)
                },
                border: Border {
                    radius: radius.into(),
                    width: 1.5,
                    color: if hovered || pressed {
                        accent(theme)
                    } else {
                        border_strong(theme)
                    },
                },
                shadow: surface_shadow(theme),
                ..Default::default()
            }
        }
    }
}

pub fn list_item_style(
    theme: &iced::Theme,
    is_selected: bool,
    is_hovered: bool,
) -> container::Style {
    let dark = is_dark(theme);
    let base_bg = card_surface(theme);
    let fill = if is_selected {
        if dark {
            mix(base_bg, ACCENT_BRIGHT, 0.08)
        } else {
            CARD_SELECTED_LIGHT
        }
    } else if is_hovered {
        elevated_surface(theme)
    } else {
        base_bg
    };

    container::Style {
        background: Some(Background::Color(fill)),
        border: Border {
            color: if is_selected {
                accent(theme)
            } else {
                border_color(theme)
            },
            width: if is_selected { 1.5 } else { 1.0 },
            radius: RADIUS_MD.into(),
        },
        ..Default::default()
    }
}

pub fn list_item_zebra(
    theme: &iced::Theme,
    is_selected: bool,
    is_hovered: bool,
    is_zebra: bool,
) -> container::Style {
    let mut style = list_item_style(theme, is_selected, is_hovered);
    if !is_selected && !is_hovered && is_zebra {
        let dark = is_dark(theme);
        let bg = if dark {
            Color::from_rgb8(0x18, 0x20, 0x1D)
        } else {
            Color::from_rgb8(0xF8, 0xFA, 0xF9)
        };
        style.background = Some(Background::Color(bg));
    }
    style.border.width = 0.0;
    style.border.radius = 0.0.into();
    style
}

pub fn console_bg(theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(console_surface(theme))),
        border: Border {
            color: border_color(theme),
            width: 1.0,
            radius: RADIUS_LG.into(),
        },
        text_color: Some(text_primary(theme)),
        ..Default::default()
    }
}

pub fn input_field_error(theme: &iced::Theme, status: text_input::Status) -> text_input::Style {
    let mut style = input_field(theme, status);
    style.border = iced::Border {
        color: danger_text(theme),
        width: 1.0,
        radius: RADIUS_MD.into(),
    };
    style
}

pub fn input_field(theme: &iced::Theme, status: text_input::Status) -> text_input::Style {
    let dark = is_dark(theme);
    let (border_color, width) = match status {
        text_input::Status::Focused { .. } => (accent(theme), 1.5),
        text_input::Status::Hovered => (
            if dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.22)
            } else {
                Color::from_rgba(0.08, 0.13, 0.11, 0.26)
            },
            1.0,
        ),
        _ => (border_strong(theme), 1.0),
    };

    text_input::Style {
        background: Background::Color(match status {
            text_input::Status::Focused { .. } => card_surface(theme),
            text_input::Status::Disabled => bg(theme),
            _ => input_surface(theme),
        }),
        border: Border {
            radius: RADIUS_MD.into(),
            width,
            color: border_color,
        },
        value: text_primary(theme),
        placeholder: text_tertiary(theme),
        selection: with_alpha(accent(theme), 0.28),
        icon: text_muted(theme),
    }
}

pub fn pick_list(theme: &iced::Theme, status: iced_pick_list::Status) -> iced_pick_list::Style {
    let dark = is_dark(theme);
    let border_c = match status {
        iced_pick_list::Status::Opened { .. } => accent(theme),
        iced_pick_list::Status::Hovered => {
            if dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.22)
            } else {
                Color::from_rgba(0.08, 0.13, 0.11, 0.26)
            }
        }
        _ => border_strong(theme),
    };

    iced_pick_list::Style {
        background: Background::Color(input_surface(theme)),
        border: Border {
            radius: RADIUS_MD.into(),
            width: 1.0,
            color: border_c,
        },
        text_color: text_primary(theme),
        placeholder_color: text_tertiary(theme),
        handle_color: text_muted(theme),
    }
}

/// Drop-down menu of a `pick_list`, matched to the popover surface.
pub fn pick_list_menu(theme: &iced::Theme) -> iced::overlay::menu::Style {
    let pop = popover_bg(theme);
    iced::overlay::menu::Style {
        background: pop
            .background
            .unwrap_or(Background::Color(card_surface(theme))),
        border: pop.border,
        text_color: text_primary(theme),
        selected_text_color: accent_text(theme),
        selected_background: Background::Color(with_alpha(
            accent(theme),
            if is_dark(theme) { 0.14 } else { 0.10 },
        )),
        shadow: pop.shadow,
    }
}

pub fn scrollbar_style(
    theme: &iced::Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    use iced::widget::scrollable::{Rail, Scroller, Status};
    let dark = is_dark(theme);
    let (v_active, h_active) = match status {
        Status::Hovered {
            is_vertical_scrollbar_hovered,
            is_horizontal_scrollbar_hovered,
            ..
        } => (
            is_vertical_scrollbar_hovered,
            is_horizontal_scrollbar_hovered,
        ),
        Status::Dragged {
            is_vertical_scrollbar_dragged,
            is_horizontal_scrollbar_dragged,
            ..
        } => (
            is_vertical_scrollbar_dragged,
            is_horizontal_scrollbar_dragged,
        ),
        Status::Active { .. } => (false, false),
    };
    let scroller = |active: bool| {
        let alpha = if active { 0.34 } else { 0.18 };
        Scroller {
            background: Background::Color(if dark {
                Color::from_rgba(1.0, 1.0, 1.0, alpha)
            } else {
                Color::from_rgba(0.08, 0.13, 0.11, alpha)
            }),
            border: Border {
                radius: 4.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
        }
    };
    let rail = |active: bool| Rail {
        background: None,
        border: Border::default(),
        scroller: scroller(active),
    };
    iced::widget::scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail(v_active),
        horizontal_rail: rail(h_active),
        gap: None,
        auto_scroll: iced::widget::scrollable::AutoScroll {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            shadow: Shadow::default(),
            icon: Color::TRANSPARENT,
        },
    }
}

pub fn toggler_style(
    theme: &iced::Theme,
    status: iced::widget::toggler::Status,
) -> iced::widget::toggler::Style {
    use iced::widget::toggler::Status;
    let dark = is_dark(theme);
    let (is_toggled, hovered, disabled) = match status {
        Status::Active { is_toggled } => (is_toggled, false, false),
        Status::Hovered { is_toggled } => (is_toggled, true, false),
        Status::Disabled { is_toggled } => (is_toggled, false, true),
    };
    let track = if is_toggled {
        let on = if hovered {
            if dark {
                ACCENT_BRIGHT_HOVER
            } else {
                ACCENT_HOVER
            }
        } else {
            accent(theme)
        };
        if disabled {
            mix(card_surface(theme), on, 0.45)
        } else {
            on
        }
    } else if dark {
        if hovered {
            Color::from_rgb8(0x34, 0x40, 0x3C)
        } else {
            Color::from_rgb8(0x2A, 0x35, 0x31)
        }
    } else if hovered {
        Color::from_rgb8(0xC4, 0xCE, 0xCA)
    } else {
        Color::from_rgb8(0xD3, 0xDB, 0xD8)
    };
    let knob = if is_toggled && dark {
        ACCENT_INK
    } else {
        Color::WHITE
    };
    iced::widget::toggler::Style {
        background: Background::Color(track),
        background_border_color: Color::TRANSPARENT,
        background_border_width: 0.0,
        foreground: Background::Color(knob),
        foreground_border_color: Color::TRANSPARENT,
        foreground_border_width: 0.0,
        text_color: Some(text_primary(theme)),
        // `None` = fully rounded track and knob.
        border_radius: None,
        // Fraction of the track height left around the knob. Values ≥ 0.5
        // collapse the knob to nothing (it was 1.0, rendering knob-less pills).
        padding_ratio: 0.12,
    }
}

pub fn button_ghost(theme: &iced::Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Some(Background::Color(hover_wash(theme, false))),
        button::Status::Pressed => Some(Background::Color(hover_wash(theme, true))),
        _ => None,
    };
    button::Style {
        background: bg,
        text_color: if matches!(status, button::Status::Disabled) {
            text_tertiary(theme)
        } else {
            text_primary(theme)
        },
        border: Border {
            radius: RADIUS_MD.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

// ── Contrast / palette unit tests ────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::assertions_on_constants)]
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
    fn semantic_text_is_readable_in_both_themes() {
        for theme in [iced::Theme::Light, iced::Theme::Dark] {
            for status in [
                button::Status::Active,
                button::Status::Hovered,
                button::Status::Pressed,
            ] {
                for style in [
                    button_primary(&theme, status),
                    button_danger(&theme, status),
                    button_secondary(&theme, status),
                ] {
                    if let Some(Background::Color(fill)) = style.background {
                        assert!(
                            contrast_ratio(style.text_color, fill) >= 4.5,
                            "button text {:?} on {fill:?}",
                            style.text_color
                        );
                    }
                }
                let power = power_button(true)(&theme, status);
                if let Some(Background::Color(fill)) = power.background {
                    assert!(contrast_ratio(power.text_color, fill) >= 4.5);
                }
            }
            for color in [
                text_primary(&theme),
                text_muted(&theme),
                text_tertiary(&theme),
                success_text(&theme),
                warning_text(&theme),
                danger_text(&theme),
                info_text(&theme),
                accent_text(&theme),
            ] {
                for surface in [
                    bg(&theme),
                    card_surface(&theme),
                    console_surface(&theme),
                    sidebar_surface(&theme),
                    input_surface(&theme),
                ] {
                    assert!(
                        contrast_ratio(color, surface) >= 4.5,
                        "Unreadable {color:?} on {surface:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn chart_series_marks_stand_off_the_card() {
        for theme in [iced::Theme::Light, iced::Theme::Dark] {
            let (down, up) = chart_series(&theme);
            let card = card_surface(&theme);
            assert!(contrast_ratio(down, card) >= 3.0, "{theme:?} down");
            assert!(contrast_ratio(up, card) >= 3.0, "{theme:?} up");
        }
    }

    #[test]
    fn protocol_badges_stay_readable_on_their_tint() {
        for theme in [iced::Theme::Light, iced::Theme::Dark] {
            for proto in [
                "hysteria2",
                "vless",
                "vmess",
                "shadowsocks",
                "trojan",
                "tuic",
                "wireguard",
                "direct",
                "block",
                "other",
            ] {
                let hue = protocol_color(proto);
                let fill = mix(
                    card_surface(&theme),
                    hue,
                    if is_dark(&theme) { 0.16 } else { 0.11 },
                );
                assert!(
                    contrast_ratio(badge_text(hue, &theme), fill) >= 4.5,
                    "{proto} badge unreadable"
                );
            }
        }
    }

    #[test]
    fn region_codes_do_not_match_fragments_of_unrelated_words() {
        for name in [
            "node-01",
            "business",
            "Auto",
            "自动选择",
            "DIRECT",
            "direct connection",
            "premium",
            "reliable",
        ] {
            assert_eq!(detect_region_code(name), None, "{name}");
        }
        assert_eq!(detect_region_code("HK-01"), Some("HK"));
        assert_eq!(detect_region_code("US 02 / Premium"), Some("US"));
        assert_eq!(detect_region_code("🇯🇵 東京 JP"), Some("JP"));
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
        assert!(BORDER_STRONG_DARK.a > BORDER_DARK.a);
        assert!(BORDER_STRONG_LIGHT.a > BORDER_LIGHT.a);

        // Text hierarchy: primary brighter/darker than muted in each mode
        assert!(relative_luminance(TEXT_PRIMARY_DARK) > relative_luminance(TEXT_MUTED_DARK));
        assert!(relative_luminance(TEXT_MUTED_DARK) > relative_luminance(TEXT_TERTIARY_DARK));
        assert!(relative_luminance(TEXT_PRIMARY_LIGHT) < relative_luminance(TEXT_MUTED_LIGHT));
        assert!(relative_luminance(TEXT_MUTED_LIGHT) < relative_luminance(TEXT_TERTIARY_LIGHT));

        // Dark surfaces step upward: sidebar < canvas < card < elevated
        assert!(relative_luminance(SIDEBAR_BG) < relative_luminance(BG_DARK));
        assert!(relative_luminance(BG_DARK) < relative_luminance(CARD_DARK));
        assert!(relative_luminance(CARD_DARK) < relative_luminance(CARD_ELEVATED_DARK));
    }

    #[test]
    fn primary_text_contrasts_against_surfaces() {
        assert!(contrast_ratio(TEXT_PRIMARY_DARK, BG_DARK) >= 7.0);
        assert!(contrast_ratio(TEXT_PRIMARY_DARK, CARD_DARK) >= 7.0);
        assert!(contrast_ratio(TEXT_PRIMARY_LIGHT, BG_LIGHT) >= 7.0);
        assert!(contrast_ratio(TEXT_PRIMARY_LIGHT, CARD_LIGHT_BG) >= 7.0);
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
        assert_eq!(text_primary(&dark), TEXT_PRIMARY_DARK);
        assert_eq!(text_primary(&light), TEXT_PRIMARY_LIGHT);
        assert_eq!(border_color(&dark), BORDER_DARK);
        assert_eq!(border_color(&light), BORDER_LIGHT);
        assert_eq!(accent(&dark), ACCENT_BRIGHT);
        assert_eq!(accent(&light), ACCENT);
        // Semantics are solid fills
        assert_eq!(ACCENT.a, 1.0);
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
        // Dark relies on borders, light on a whisper of shadow.
        assert_eq!(card_d.shadow, Shadow::default());
        assert_ne!(card_bg(&light).shadow, Shadow::default());

        let card_l = card_bg(&light);
        assert_ne!(card_d.background.unwrap(), card_l.background.unwrap());

        assert_eq!(card_selected(&dark).border.color, ACCENT_BRIGHT);
        assert_eq!(card_selected(&light).border.color, ACCENT);

        let primary_l = button_primary(&light, button::Status::Active);
        assert_eq!(primary_l.text_color, Color::WHITE);
        assert!(matches!(primary_l.background, Some(Background::Color(c)) if c == ACCENT));
        let primary_d = button_primary(&dark, button::Status::Active);
        assert_eq!(primary_d.text_color, ACCENT_INK);
        assert!(matches!(primary_d.background, Some(Background::Color(c)) if c == ACCENT_BRIGHT));
        let primary_dis = button_primary(&dark, button::Status::Disabled);
        assert!(primary_dis.text_color.a < 1.0);
        let secondary_dis = button_secondary(&dark, button::Status::Disabled);
        assert_eq!(secondary_dis.text_color, TEXT_TERTIARY_DARK);

        let secondary_l = button_secondary(&light, button::Status::Active);
        assert_eq!(secondary_l.text_color, TEXT_PRIMARY_LIGHT);
        assert!(secondary_l.border.width >= 1.0);
        // Light secondary border must not be near-invisible
        assert!(secondary_l.border.color.a >= 0.08);

        let input_l = input_field(&light, text_input::Status::Active);
        assert!(input_l.border.color.a >= 0.08);
        let focused = input_field(&dark, text_input::Status::Focused { is_hovered: false });
        assert_eq!(focused.border.color, ACCENT_BRIGHT);

        // Active segment is lifted; inactive ones stay flat.
        let seg_on = button_tab(true)(&light, button::Status::Active);
        let seg_off = button_tab(false)(&light, button::Status::Active);
        assert!(seg_on.background.is_some());
        assert!(seg_off.background.is_none());
        assert_eq!(seg_on.text_color, TEXT_PRIMARY_LIGHT);
    }

    #[test]
    fn toggler_knob_is_visible() {
        use iced::widget::toggler::Status;
        for theme in [iced::Theme::Light, iced::Theme::Dark] {
            for is_toggled in [false, true] {
                let style = toggler_style(&theme, Status::Active { is_toggled });
                // iced sizes the knob as `height - 2 * padding_ratio * height`.
                assert!(style.padding_ratio > 0.0 && style.padding_ratio < 0.25);
                assert_ne!(style.foreground, style.background);
            }
        }
    }

    #[test]
    fn with_alpha_preserves_rgb() {
        let c = with_alpha(SUCCESS, 0.2);
        assert!((c.r - SUCCESS.r).abs() < 0.001);
        assert!((c.a - 0.2).abs() < 0.001);
    }

    #[test]
    fn mix_blends_toward_target() {
        let half = mix(Color::BLACK, Color::WHITE, 0.5);
        assert!((half.r - 0.5).abs() < 0.001);
        assert_eq!(mix(ACCENT, Color::WHITE, 0.0), ACCENT);
        assert_eq!(mix(ACCENT, Color::WHITE, 2.0), Color::WHITE);
    }

    #[test]
    fn metric_font_uses_bundled_cjk_face() {
        let f = metric_font();
        assert_eq!(f.family, UI_FONT_FAMILY);
        assert_eq!(f.weight, iced::font::Weight::Medium);
    }

    #[test]
    fn log_font_is_a_monospace_face() {
        let f = mono_font();
        assert_ne!(f.family, UI_FONT_FAMILY);
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        assert_eq!(f.family, iced::font::Family::Monospace);
    }

    #[test]
    fn type_scale_is_ordered() {
        assert!(TYPE_DISPLAY > TYPE_TITLE);
        assert!(TYPE_TITLE > TYPE_HEADING);
        assert!(TYPE_HEADING > TYPE_BODY);
        assert!(TYPE_BODY >= TYPE_SECTION);
        assert!(TYPE_SECTION >= TYPE_BTN_MD);
        assert!(TYPE_BTN_MD > TYPE_BTN_SM);
        assert!(TYPE_BTN_SM >= TYPE_MONO);
        assert!(TYPE_MONO > TYPE_CAPTION);
        assert!(TYPE_CAPTION > TYPE_MICRO);
        assert!(TYPE_MICRO >= 11.0);
    }

    #[test]
    fn radius_scale_is_ordered() {
        assert!(RADIUS_LG > RADIUS_MD);
        assert!(RADIUS_MD > RADIUS_SM);
        assert!(RADIUS_SM > RADIUS_XS);
        assert!(RADIUS_XS > RADIUS_MICRO);
        assert!(RADIUS_MICRO >= 3.0);
    }

    #[test]
    fn spacing_and_chrome_presets_are_positive() {
        assert!(CARD_PAD >= CARD_PAD_DENSE);
        assert!(CARD_PAD_DENSE >= 12.0);
        assert!(GRID_GAP >= 12.0);
        assert!(SEARCH_WIDTH >= 200.0);
        assert!(BTN_PAD_SM[0] < BTN_PAD_MD[0] && BTN_PAD_MD[0] < BTN_PAD_LG[0]);
        assert!(BTN_PAD_SM[1] < BTN_PAD_MD[1] && BTN_PAD_MD[1] < BTN_PAD_LG[1]);
    }

    #[test]
    fn test_platform_native_styling() {
        #[cfg(target_os = "windows")]
        assert_eq!(UI_FONT_FAMILY, iced::font::Family::Name("Segoe UI"));

        #[cfg(target_os = "macos")]
        {
            assert_eq!(UI_FONT_FAMILY, iced::font::Family::Name("Helvetica Neue"));
            assert_eq!(CARD_PAD, 22.0);
            assert_eq!(GRID_GAP, 18.0);
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        assert_eq!(UI_FONT_FAMILY, iced::font::Family::SansSerif);

        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(CARD_PAD, 20.0);
            assert_eq!(GRID_GAP, 16.0);
        }
    }

    #[test]
    fn test_protocol_colors_distinct() {
        let hy = protocol_color("hysteria2");
        let vless = protocol_color("vless");
        let vmess = protocol_color("vmess");
        let ss = protocol_color("shadowsocks");
        assert_ne!(hy, vless);
        assert_ne!(vless, vmess);
        assert_ne!(vmess, ss);
    }

    #[test]
    fn test_detect_region_code() {
        assert_eq!(detect_region_code("香港 BGP 01"), Some("HK"));
        assert_eq!(detect_region_code("Tokyo Premium"), Some("JP"));
        assert_eq!(detect_region_code("US - Los Angeles"), Some("US"));
        assert_eq!(detect_region_code("SG Singapore 02"), Some("SG"));
        assert_eq!(detect_region_code("🇹🇼 台湾 01"), Some("TW"));
    }

    #[test]
    fn test_badge_colored() {
        let dark = iced::Theme::Dark;
        let style = badge_colored(ACCENT, &dark);
        assert!(style.background.is_some());
        assert_eq!(style.text_color, Some(badge_text(ACCENT, &dark)));
    }
}
