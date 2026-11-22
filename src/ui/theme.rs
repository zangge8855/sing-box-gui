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

// ── Radius language (adapted per platform HIG) ───────────────────────────────
// Windows (WinUI 3 / Fluent): crisp 8px surfaces, 6px controls, 5px chips.
#[cfg(target_os = "windows")]
pub const RADIUS_LG: f32 = 8.0;
#[cfg(target_os = "windows")]
pub const RADIUS_MD: f32 = 6.0;
#[cfg(target_os = "windows")]
pub const RADIUS_SM: f32 = 5.0;
#[cfg(target_os = "windows")]
pub const RADIUS_XS: f32 = 4.0;
#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub const RADIUS_MICRO: f32 = 3.0;

// macOS (Aqua / Sequoia HIG): generous 14px surfaces, 10px controls, 7px chips.
#[cfg(target_os = "macos")]
pub const RADIUS_LG: f32 = 14.0;
#[cfg(target_os = "macos")]
pub const RADIUS_MD: f32 = 10.0;
#[cfg(target_os = "macos")]
pub const RADIUS_SM: f32 = 7.0;
#[cfg(target_os = "macos")]
pub const RADIUS_XS: f32 = 5.0;
#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub const RADIUS_MICRO: f32 = 3.0;

// Linux (GNOME / Libadwaita): 12px cards, 8px controls, 6px chips.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const RADIUS_LG: f32 = 12.0;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const RADIUS_MD: f32 = 8.0;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const RADIUS_SM: f32 = 6.0;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const RADIUS_XS: f32 = 4.0;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[allow(dead_code)]
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

pub fn mono_font_bold() -> iced::Font {
    iced::Font {
        weight: iced::font::Weight::Bold,
        ..mono_font()
    }
}

// ── Type scale (f32 for iced::Pixels) ────────────────────────────────────────
pub const TYPE_DISPLAY: f32 = 30.0; // hero node name, large speeds
pub const TYPE_TITLE: f32 = 24.0; // page titles
pub const TYPE_TITLE_SM: f32 = 20.0; // hero headline step-down for long names
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

#[allow(dead_code)]
pub const SP_40: f32 = 40.0;

// ── Dark mode (platform-native neutral surfaces) ─────────────────────────────
#[cfg(target_os = "windows")]
pub const BG_DARK: Color = Color::from_rgb8(0x1F, 0x20, 0x22);
#[cfg(target_os = "windows")]
pub const SIDEBAR_BG: Color = Color::from_rgb8(0x19, 0x1A, 0x1C);
#[cfg(target_os = "windows")]
pub const CARD_DARK: Color = Color::from_rgb8(0x29, 0x2A, 0x2D);
#[cfg(target_os = "windows")]
pub const CARD_ELEVATED_DARK: Color = Color::from_rgb8(0x31, 0x32, 0x36);
#[cfg(target_os = "windows")]
pub const CARD_HOVER: Color = Color::from_rgb8(0x37, 0x38, 0x3C);
#[cfg(target_os = "windows")]
pub const TEXT_PRIMARY_DARK: Color = Color::from_rgb8(0xF3, 0xF3, 0xF3);
#[cfg(target_os = "windows")]
pub const TEXT_MUTED_DARK: Color = Color::from_rgb8(0xB3, 0xB6, 0xBA);
#[cfg(target_os = "windows")]
pub const TEXT_TERTIARY_DARK: Color = Color::from_rgb8(0x9D, 0xA1, 0xA7);
#[cfg(target_os = "windows")]
pub const INPUT_BG_DARK: Color = Color::from_rgb8(0x23, 0x24, 0x27);
#[cfg(target_os = "windows")]
pub const CONSOLE_BG_DARK: Color = Color::from_rgb8(0x17, 0x18, 0x1A);

#[cfg(target_os = "macos")]
pub const BG_DARK: Color = Color::from_rgb8(0x1E, 0x1E, 0x20);
#[cfg(target_os = "macos")]
pub const SIDEBAR_BG: Color = Color::from_rgb8(0x17, 0x17, 0x19);
#[cfg(target_os = "macos")]
pub const CARD_DARK: Color = Color::from_rgb8(0x27, 0x27, 0x2A);
#[cfg(target_os = "macos")]
pub const CARD_ELEVATED_DARK: Color = Color::from_rgb8(0x30, 0x30, 0x34);
#[cfg(target_os = "macos")]
pub const CARD_HOVER: Color = Color::from_rgb8(0x38, 0x38, 0x3D);
#[cfg(target_os = "macos")]
pub const TEXT_PRIMARY_DARK: Color = Color::from_rgb8(0xF2, 0xF2, 0xF7);
#[cfg(target_os = "macos")]
pub const TEXT_MUTED_DARK: Color = Color::from_rgb8(0xB4, 0xB4, 0xB9);
#[cfg(target_os = "macos")]
pub const TEXT_TERTIARY_DARK: Color = Color::from_rgb8(0x9E, 0x9E, 0xA4);
#[cfg(target_os = "macos")]
pub const INPUT_BG_DARK: Color = Color::from_rgb8(0x21, 0x21, 0x24);
#[cfg(target_os = "macos")]
pub const CONSOLE_BG_DARK: Color = Color::from_rgb8(0x15, 0x15, 0x17);

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const BG_DARK: Color = Color::from_rgb8(0x1F, 0x1F, 0x1F);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const SIDEBAR_BG: Color = Color::from_rgb8(0x18, 0x18, 0x18);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const CARD_DARK: Color = Color::from_rgb8(0x2A, 0x2A, 0x2A);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const CARD_ELEVATED_DARK: Color = Color::from_rgb8(0x33, 0x33, 0x33);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const CARD_HOVER: Color = Color::from_rgb8(0x3B, 0x3B, 0x3B);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const TEXT_PRIMARY_DARK: Color = Color::from_rgb8(0xF5, 0xF5, 0xF5);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const TEXT_MUTED_DARK: Color = Color::from_rgb8(0xB8, 0xB8, 0xB8);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const TEXT_TERTIARY_DARK: Color = Color::from_rgb8(0xA0, 0xA0, 0xA0);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const INPUT_BG_DARK: Color = Color::from_rgb8(0x24, 0x24, 0x24);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const CONSOLE_BG_DARK: Color = Color::from_rgb8(0x16, 0x16, 0x16);

pub const BORDER_DARK: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.08);
pub const BORDER_STRONG_DARK: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.14);

// ── Light mode (platform-native neutral surfaces) ────────────────────────────
#[cfg(target_os = "windows")]
pub const BG_LIGHT: Color = Color::from_rgb8(0xF3, 0xF3, 0xF3);
#[cfg(target_os = "windows")]
pub const SIDEBAR_BG_LIGHT: Color = Color::from_rgb8(0xEA, 0xEA, 0xEA);
#[cfg(target_os = "windows")]
pub const CARD_LIGHT_BG: Color = Color::from_rgb8(0xFF, 0xFF, 0xFF);
#[cfg(target_os = "windows")]
pub const CARD_SELECTED_LIGHT: Color = Color::from_rgb8(0xEE, 0xF6, 0xF3);
#[cfg(target_os = "windows")]
pub const CARD_HOVER_LIGHT: Color = Color::from_rgb8(0xF9, 0xF9, 0xF9);
#[cfg(target_os = "windows")]
pub const TEXT_PRIMARY_LIGHT: Color = Color::from_rgb8(0x1A, 0x1A, 0x1A);
#[cfg(target_os = "windows")]
pub const TEXT_MUTED_LIGHT: Color = Color::from_rgb8(0x4E, 0x4E, 0x4E);
#[cfg(target_os = "windows")]
pub const TEXT_TERTIARY_LIGHT: Color = Color::from_rgb8(0x5E, 0x5E, 0x5E);
#[cfg(target_os = "windows")]
pub const INPUT_BG_LIGHT: Color = Color::from_rgb8(0xFB, 0xFB, 0xFB);
#[cfg(target_os = "windows")]
pub const CONSOLE_BG_LIGHT: Color = Color::from_rgb8(0xF6, 0xF6, 0xF6);

#[cfg(target_os = "macos")]
pub const BG_LIGHT: Color = Color::from_rgb8(0xF5, 0xF5, 0xF7);
#[cfg(target_os = "macos")]
pub const SIDEBAR_BG_LIGHT: Color = Color::from_rgb8(0xEC, 0xEC, 0xEE);
#[cfg(target_os = "macos")]
pub const CARD_LIGHT_BG: Color = Color::from_rgb8(0xFF, 0xFF, 0xFF);
#[cfg(target_os = "macos")]
pub const CARD_SELECTED_LIGHT: Color = Color::from_rgb8(0xEC, 0xF5, 0xF2);
#[cfg(target_os = "macos")]
pub const CARD_HOVER_LIGHT: Color = Color::from_rgb8(0xF8, 0xF8, 0xFA);
#[cfg(target_os = "macos")]
pub const TEXT_PRIMARY_LIGHT: Color = Color::from_rgb8(0x1D, 0x1D, 0x1F);
#[cfg(target_os = "macos")]
pub const TEXT_MUTED_LIGHT: Color = Color::from_rgb8(0x4C, 0x4C, 0x50);
#[cfg(target_os = "macos")]
pub const TEXT_TERTIARY_LIGHT: Color = Color::from_rgb8(0x5C, 0x5C, 0x61);
#[cfg(target_os = "macos")]
pub const INPUT_BG_LIGHT: Color = Color::from_rgb8(0xF9, 0xF9, 0xFB);
#[cfg(target_os = "macos")]
pub const CONSOLE_BG_LIGHT: Color = Color::from_rgb8(0xF6, 0xF6, 0xF8);

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const BG_LIGHT: Color = Color::from_rgb8(0xFA, 0xFA, 0xFA);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const SIDEBAR_BG_LIGHT: Color = Color::from_rgb8(0xEB, 0xEB, 0xEB);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const CARD_LIGHT_BG: Color = Color::from_rgb8(0xFF, 0xFF, 0xFF);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const CARD_SELECTED_LIGHT: Color = Color::from_rgb8(0xEC, 0xF5, 0xF2);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const CARD_HOVER_LIGHT: Color = Color::from_rgb8(0xF5, 0xF5, 0xF5);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const TEXT_PRIMARY_LIGHT: Color = Color::from_rgb8(0x1C, 0x1C, 0x1C);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const TEXT_MUTED_LIGHT: Color = Color::from_rgb8(0x4C, 0x4C, 0x4C);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const TEXT_TERTIARY_LIGHT: Color = Color::from_rgb8(0x5B, 0x5B, 0x5B);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const INPUT_BG_LIGHT: Color = Color::from_rgb8(0xF4, 0xF4, 0xF4);
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const CONSOLE_BG_LIGHT: Color = Color::from_rgb8(0xF6, 0xF6, 0xF6);

pub const BORDER_LIGHT: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.085);
pub const BORDER_STRONG_LIGHT: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.145);

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
pub const DANGER_DISABLED: Color = Color::from_rgb8(0x5A, 0x2F, 0x2B);

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

fn contains_keyword(haystack_lower: &str, keyword: &str) -> bool {
    if !keyword.is_ascii() {
        return haystack_lower.contains(keyword);
    }
    let bytes = haystack_lower.as_bytes();
    let mut offset = 0;
    while let Some(rel) = haystack_lower[offset..].find(keyword) {
        let start = offset + rel;
        let end = start + keyword.len();
        let left_ok = start == 0 || !bytes[start - 1].is_ascii_alphabetic();
        let right_ok = end == bytes.len() || !bytes[end].is_ascii_alphabetic();
        if left_ok && right_ok {
            return true;
        }
        offset = start + 1;
    }
    false
}

/// Region badges use letters on every OS: Windows does not render flag
/// emoji reliably. Short country codes must be tokens, not substrings ("us"
/// in "business", "de" in "node", or "in" in "direct" are not locations).
pub fn detect_region_code(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    let tokens: Vec<(usize, &str)> = {
        let mut out = Vec::new();
        let bytes = name.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i].is_ascii_alphabetic() {
                let start = i;
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                out.push((start, &name[start..i]));
            } else {
                i += 1;
            }
        }
        out
    };
    if lower.contains("自动")
        || lower.contains("直连")
        || tokens
            .iter()
            .any(|(_, t)| t.eq_ignore_ascii_case("auto") || t.eq_ignore_ascii_case("direct"))
    {
        return None;
    }

    const REGIONS: &[(&str, &str, &[&str])] = &[
        ("HK", "🇭🇰", &["香港", "hongkong", "hong kong"]),
        (
            "JP",
            "🇯🇵",
            &["日本", "japan", "tokyo", "東京", "osaka", "大阪", "名古屋"],
        ),
        (
            "US",
            "🇺🇸",
            &[
                "美国",
                "美國",
                "united states",
                "america",
                "los angeles",
                "洛杉矶",
                "洛杉磯",
                "san jose",
                "圣何塞",
                "聖何塞",
                "new york",
                "纽约",
                "紐約",
                "seattle",
                "西雅图",
                "西雅圖",
                "san francisco",
                "silicon valley",
                "chicago",
                "dallas",
                "usa",
            ],
        ),
        (
            "SG",
            "🇸🇬",
            &["新加坡", "singapore", "狮城", "獅城", "坡县", "坡縣"],
        ),
        (
            "TW",
            "🇹🇼",
            &["台湾", "台灣", "taiwan", "台北", "臺北", "新北", "高雄"],
        ),
        (
            "KR",
            "🇰🇷",
            &["韩国", "韓國", "korea", "首尔", "首爾", "seoul"],
        ),
        (
            "DE",
            "🇩🇪",
            &[
                "德国",
                "德國",
                "germany",
                "frankfurt",
                "法兰克福",
                "法蘭克福",
            ],
        ),
        (
            "GB",
            "🇬🇧",
            &[
                "英国",
                "英國",
                "britain",
                "united kingdom",
                "london",
                "伦敦",
                "倫敦",
            ],
        ),
        (
            "CA",
            "🇨🇦",
            &[
                "加拿大",
                "canada",
                "toronto",
                "多伦多",
                "多倫多",
                "vancouver",
                "温哥华",
                "溫哥華",
            ],
        ),
        (
            "AU",
            "🇦🇺",
            &[
                "澳大利亚",
                "澳大利亞",
                "澳洲",
                "australia",
                "sydney",
                "悉尼",
                "雪梨",
                "melbourne",
                "墨尔本",
                "墨爾本",
            ],
        ),
        (
            "RU",
            "🇷🇺",
            &["俄罗斯", "俄羅斯", "russia", "moscow", "莫斯科"],
        ),
        ("FR", "🇫🇷", &["法国", "法國", "france", "paris", "巴黎"]),
        (
            "NL",
            "🇳🇱",
            &["荷兰", "荷蘭", "netherlands", "amsterdam", "阿姆斯特丹"],
        ),
        (
            "TR",
            "🇹🇷",
            &["土耳其", "turkey", "istanbul", "伊斯坦布尔", "伊斯坦堡"],
        ),
        (
            "ID",
            "🇮🇩",
            &["印度尼西亚", "印度尼西亞", "印尼", "indonesia", "jakarta"],
        ),
        (
            "IN",
            "🇮🇳",
            &["印度", "india", "mumbai", "bangalore", "new delhi"],
        ),
    ];

    // Explicit flag emoji takes priority over secondary keywords in relay names.
    for &(code, flag, _) in REGIONS {
        if name.contains(flag) {
            return Some(code);
        }
    }

    for &(code, _, keywords) in REGIONS {
        if keywords
            .iter()
            .any(|keyword| contains_keyword(&lower, keyword))
        {
            return Some(code);
        }
        // Avoid false positives: "ID" is too ambiguous in English ("ID"), "in"
        // is an English preposition unless uppercase "IN", "100GB" / "100 GB"
        // is a data cap rather than Great Britain, and lowercase "us" in
        // English prose ("Contact us") is not United States.
        if code == "ID" {
            continue;
        }
        let bytes = name.as_bytes();
        let token_hit = tokens.iter().any(|&(start, token)| {
            if code == "IN" {
                return token == "IN";
            }
            if code == "GB"
                && bytes[..start]
                    .iter()
                    .rev()
                    .find(|&&b| b != b' ')
                    .is_some_and(|b| b.is_ascii_digit())
            {
                return false;
            }
            if code == "US" && token.eq_ignore_ascii_case("us") {
                if token == "US" {
                    return true;
                }
                let end = start + token.len();
                let delim = |b: u8| {
                    matches!(b, b'-' | b'_' | b'/' | b'.' | b'[' | b']' | b'(' | b')')
                        || b.is_ascii_digit()
                };
                return start == 0
                    || (start > 0 && delim(bytes[start - 1]))
                    || (end < bytes.len() && delim(bytes[end]));
            }
            token.eq_ignore_ascii_case(code) || (code == "GB" && token.eq_ignore_ascii_case("uk"))
        });
        if token_hit {
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
            | iced::Theme::CatppuccinLatte
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
        Color::from_rgb8(0x1A, 0x6E, 0x44)
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

/// Platform-native elevation shadow:
/// - Windows (Fluent): subtle crisp lift in light mode; border-only in dark mode.
/// - macOS (Aqua): soft ambient depth in light mode; border-only in dark mode.
/// - Linux (Adwaita): flat elevation relying on 1px borders and tonal steps.
fn surface_shadow(theme: &iced::Theme) -> Shadow {
    if is_dark(theme) {
        return Shadow::default();
    }
    #[cfg(target_os = "windows")]
    {
        Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.045),
            offset: iced::Vector::new(0.0, 1.5),
            blur_radius: 4.0,
        }
    }
    #[cfg(target_os = "macos")]
    {
        Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.06),
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 10.0,
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.03),
            offset: iced::Vector::new(0.0, 1.0),
            blur_radius: 3.0,
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
        shadow: surface_shadow(theme),
        text_color: Some(text_primary(theme)),
        ..Default::default()
    }
}

/// Recessed panel used for empty states and quiet groupings.
pub fn status_card(theme: &iced::Theme) -> container::Style {
    let fill = mix(bg(theme), card_surface(theme), 0.45);
    container::Style {
        background: Some(Background::Color(fill)),
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
        mix(color, Color::BLACK, 0.36)
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
                with_alpha(TEXT_PRIMARY_DARK, 0.65),
            ),
            button::Status::Active => (ACCENT_BRIGHT, ACCENT_INK),
        }
    } else {
        match status {
            button::Status::Hovered => (ACCENT_HOVER, Color::WHITE),
            button::Status::Pressed => (ACCENT_PRESSED, Color::WHITE),
            button::Status::Disabled => (
                mix(CARD_LIGHT_BG, ACCENT, 0.14),
                with_alpha(TEXT_MUTED_LIGHT, 0.92),
            ),
            button::Status::Active => (ACCENT, Color::WHITE),
        }
    };

    let disabled = matches!(status, button::Status::Disabled);
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: text,
        border: Border {
            radius: RADIUS_MD.into(),
            width: if disabled { 1.0 } else { 0.0 },
            color: if disabled {
                border_color(theme)
            } else {
                Color::TRANSPARENT
            },
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
        shadow: if !dark && matches!(status, button::Status::Active | button::Status::Hovered) {
            Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.035),
                offset: iced::Vector::new(0.0, 1.0),
                blur_radius: 1.5,
            }
        } else {
            Shadow::default()
        },
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
            with_alpha(Color::WHITE, 0.72)
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
        button::Status::Disabled => (None, text_tertiary(theme)),
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
    let fill = if is_dark(theme) {
        mix(bg(theme), sidebar_surface(theme), 0.4)
    } else {
        mix(bg(theme), sidebar_surface(theme), 0.55)
    };
    container::Style {
        background: Some(Background::Color(fill)),
        border: Border {
            color: border_color(theme),
            width: 1.0,
            radius: (RADIUS_MD + 3.0).into(),
        },
        ..Default::default()
    }
}

/// Proxy group selector item on the Proxies page.
pub fn group_tab(is_active: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |theme: &iced::Theme, status: button::Status| {
        let dark = is_dark(theme);
        if is_active {
            let fill = if dark {
                mix(CARD_DARK, ACCENT_BRIGHT, 0.08)
            } else {
                CARD_SELECTED_LIGHT
            };
            button::Style {
                background: Some(Background::Color(fill)),
                text_color: accent_text(theme),
                border: Border {
                    radius: RADIUS_MD.into(),
                    width: 1.0,
                    color: with_alpha(accent(theme), if dark { 0.45 } else { 0.40 }),
                },
                shadow: surface_shadow(theme),
                ..Default::default()
            }
        } else {
            let bg = match status {
                button::Status::Hovered => Some(Background::Color(hover_wash(theme, false))),
                button::Status::Pressed => Some(Background::Color(hover_wash(theme, true))),
                _ => None,
            };
            button::Style {
                background: bg,
                text_color: text_primary(theme),
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

/// Sidebar navigation item:
/// - Windows (WinUI 3 NavigationView): subtle neutral active pill; accent comes
///   from the leading 3px indicator bar.
/// - macOS / Linux: soft celadon wash + celadon label.
pub fn nav_item(is_active: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |theme: &iced::Theme, status: button::Status| {
        let dark = is_dark(theme);
        #[cfg(target_os = "windows")]
        let (bg, fg) = {
            let bg = if is_active {
                Some(Background::Color(if dark {
                    match status {
                        button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.11),
                        button::Status::Pressed => Color::from_rgba(1.0, 1.0, 1.0, 0.06),
                        _ => Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                    }
                } else {
                    match status {
                        button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.90),
                        button::Status::Pressed => Color::from_rgba(1.0, 1.0, 1.0, 0.56),
                        _ => Color::from_rgba(1.0, 1.0, 1.0, 0.72),
                    }
                }))
            } else {
                match status {
                    button::Status::Hovered => Some(Background::Color(hover_wash(theme, false))),
                    button::Status::Pressed => Some(Background::Color(hover_wash(theme, true))),
                    _ => None,
                }
            };
            let fg = if is_active {
                text_primary(theme)
            } else {
                text_muted(theme)
            };
            (bg, fg)
        };
        #[cfg(not(target_os = "windows"))]
        let (bg, fg) = {
            let bg = if is_active {
                Some(Background::Color(with_alpha(
                    accent(theme),
                    if dark { 0.14 } else { 0.11 },
                )))
            } else {
                match status {
                    button::Status::Hovered => Some(Background::Color(hover_wash(theme, false))),
                    button::Status::Pressed => Some(Background::Color(hover_wash(theme, true))),
                    _ => None,
                }
            };
            let fg = if is_active {
                accent_text(theme)
            } else {
                text_muted(theme)
            };
            (bg, fg)
        };
        let _ = dark;
        button::Style {
            background: bg,
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
            let bg = if disabled {
                bg(theme)
            } else if pressed {
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
                    color: if disabled {
                        border_color(theme)
                    } else if hovered || pressed {
                        accent(theme)
                    } else {
                        border_strong(theme)
                    },
                },
                shadow: if disabled {
                    Shadow::default()
                } else {
                    surface_shadow(theme)
                },
                ..Default::default()
            }
        }
    }
}

/// Busy (starting / stopping) state for the round power control on the dashboard hero.
pub fn power_button_busy(theme: &iced::Theme, _status: button::Status) -> button::Style {
    let dark = is_dark(theme);
    button::Style {
        background: Some(Background::Color(mix(
            card_surface(theme),
            WARNING,
            if dark { 0.12 } else { 0.10 },
        ))),
        text_color: warning_text(theme),
        border: Border {
            radius: 999.0.into(),
            width: 1.5,
            color: with_alpha(WARNING, if dark { 0.55 } else { 0.45 }),
        },
        shadow: Shadow {
            color: with_alpha(WARNING, if dark { 0.24 } else { 0.18 }),
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 16.0,
        },
        ..Default::default()
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

#[allow(dead_code)]
pub fn list_item_flat(theme: &iced::Theme) -> container::Style {
    list_item_style(theme, false, false)
}

pub fn table_header_bg(theme: &iced::Theme) -> Color {
    if is_dark(theme) {
        CARD_ELEVATED_DARK
    } else {
        mix(CARD_LIGHT_BG, BG_LIGHT, 0.72)
    }
}

pub fn table_row_bg(theme: &iced::Theme, is_zebra: bool) -> container::Style {
    list_item_zebra(theme, false, false, is_zebra)
}

pub fn list_item_zebra(
    theme: &iced::Theme,
    is_selected: bool,
    is_hovered: bool,
    is_zebra: bool,
) -> container::Style {
    let mut style = list_item_style(theme, is_selected, is_hovered);
    if !is_selected && !is_hovered && is_zebra {
        let bg = if is_dark(theme) {
            mix(card_surface(theme), elevated_surface(theme), 0.42)
        } else {
            mix(card_surface(theme), bg(theme), 0.36)
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
        placeholder: mix(
            input_surface(theme),
            text_tertiary(theme),
            if dark { 0.88 } else { 0.94 },
        ),
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
            Color::from_rgb8(0x45, 0x47, 0x4C)
        } else {
            Color::from_rgb8(0x38, 0x3A, 0x3F)
        }
    } else if hovered {
        Color::from_rgb8(0xC2, 0xC6, 0xCA)
    } else {
        Color::from_rgb8(0xD2, 0xD5, 0xD9)
    };
    let knob = if is_toggled && dark {
        ACCENT_INK
    } else {
        Color::WHITE
    };
    let track_border = if !is_toggled {
        border_strong(theme)
    } else {
        Color::TRANSPARENT
    };
    iced::widget::toggler::Style {
        background: Background::Color(track),
        background_border_color: track_border,
        background_border_width: if !is_toggled && cfg!(target_os = "windows") {
            1.0
        } else {
            0.0
        },
        foreground: Background::Color(knob),
        foreground_border_color: Color::TRANSPARENT,
        foreground_border_width: 0.0,
        text_color: Some(text_primary(theme)),
        // `None` = fully rounded track and knob.
        border_radius: None,
        // Fraction of the track height left around the knob.
        padding_ratio: if cfg!(target_os = "windows") {
            0.16
        } else {
            0.12
        },
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

    fn composite_over(base: Color, fg: Color) -> Color {
        mix(base, fg, fg.a)
    }

    #[test]
    fn semantic_text_is_readable_in_both_themes() {
        for theme in [iced::Theme::Light, iced::Theme::Dark] {
            for status in [
                button::Status::Active,
                button::Status::Hovered,
                button::Status::Pressed,
                button::Status::Disabled,
            ] {
                for style in [
                    button_primary(&theme, status),
                    button_danger(&theme, status),
                    button_secondary(&theme, status),
                ] {
                    if let Some(Background::Color(fill)) = style.background {
                        let effective_fg = composite_over(fill, style.text_color);
                        assert!(
                            contrast_ratio(effective_fg, fill) >= 4.5,
                            "button text {:?} (effective {:?}) on {fill:?} ({status:?})",
                            style.text_color,
                            effective_fg
                        );
                    }
                }
                for running in [true, false] {
                    let power = power_button(running)(&theme, status);
                    if let Some(Background::Color(fill)) = power.background {
                        let effective_fg = composite_over(fill, power.text_color);
                        assert!(
                            contrast_ratio(effective_fg, fill) >= 4.5,
                            "power button ({running}, {status:?}) unreadable"
                        );
                    }
                }
                let power_busy = power_button_busy(&theme, status);
                if let Some(Background::Color(fill)) = power_busy.background {
                    let effective_fg = composite_over(fill, power_busy.text_color);
                    assert!(
                        contrast_ratio(effective_fg, fill) >= 4.5,
                        "power button busy ({status:?}) unreadable"
                    );
                }
            }
            let input_s = input_field(&theme, text_input::Status::Active);
            if let Background::Color(fill) = input_s.background {
                assert!(
                    contrast_ratio(input_s.placeholder, fill) >= 4.5,
                    "input placeholder unreadable on {theme:?}"
                );
            }
            let nav_active = nav_item(true)(&theme, button::Status::Active);
            if let Some(Background::Color(overlay)) = nav_active.background {
                let effective_bg = composite_over(sidebar_surface(&theme), overlay);
                assert!(
                    contrast_ratio(nav_active.text_color, effective_bg) >= 4.5,
                    "active nav item unreadable on {theme:?}"
                );
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
                    elevated_surface(&theme),
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
            "Auto - 香港",
            "DIRECT",
            "direct connection",
            "premium",
            "reliable",
            "Node in Europe",
            "Traffic 100GB",
            "Traffic 100 GB",
            "Relay 50gb Plan",
            "剩余流量 50 GB",
            "User ID Check",
            "Indianapolis Relay",
            "Comparison Node",
            "Traffic Usage Stats",
            "Contact us on Telegram",
        ] {
            assert_eq!(detect_region_code(name), None, "{name}");
        }
        assert_eq!(detect_region_code("HK-01"), Some("HK"));
        assert_eq!(detect_region_code("US 02 / Premium"), Some("US"));
        assert_eq!(detect_region_code("us-west-01"), Some("US"));
        assert_eq!(detect_region_code("node-us"), Some("US"));
        assert_eq!(detect_region_code("🇯🇵 東京 JP"), Some("JP"));
        assert_eq!(detect_region_code("東京 01"), Some("JP"));
        assert_eq!(detect_region_code("台灣 臺北 01"), Some("TW"));
        assert_eq!(detect_region_code("GB-01"), Some("GB"));
        assert_eq!(detect_region_code("UK London"), Some("GB"));
        assert_eq!(detect_region_code("IN-Mumbai-01"), Some("IN"));
        assert_eq!(detect_region_code("印度尼西亚 01"), Some("ID"));
        assert_eq!(detect_region_code("Indonesia Jakarta"), Some("ID"));
        assert_eq!(detect_region_code("🇺🇸 Relay via HK"), Some("US"));
        assert_eq!(detect_region_code("Paris-01"), Some("FR"));
    }

    #[test]
    fn all_buttons_badges_and_banners_meet_wcag_aa_contrast() {
        for theme in [iced::Theme::Light, iced::Theme::Dark] {
            let card = card_surface(&theme);
            let canvas = bg(&theme);
            let track = segmented_track(&theme)
                .background
                .and_then(|b| match b {
                    Background::Color(c) => Some(c),
                    _ => None,
                })
                .unwrap();

            for status in [
                button::Status::Active,
                button::Status::Hovered,
                button::Status::Pressed,
                button::Status::Disabled,
            ] {
                for (name, style, under) in [
                    ("danger_soft", button_danger_soft(&theme, status), card),
                    ("danger_soft_bg", button_danger_soft(&theme, status), canvas),
                    ("dismiss", button_dismiss(&theme, status), card),
                    ("ghost", button_ghost(&theme, status), card),
                    (
                        "header",
                        button_header(&theme, status),
                        elevated_surface(&theme),
                    ),
                    ("tab_active", button_tab(true)(&theme, status), track),
                    ("tab_inactive", button_tab(false)(&theme, status), track),
                    ("group_active", group_tab(true)(&theme, status), canvas),
                    ("group_inactive", group_tab(false)(&theme, status), canvas),
                    (
                        "nav_active",
                        nav_item(true)(&theme, status),
                        sidebar_surface(&theme),
                    ),
                    (
                        "nav_inactive",
                        nav_item(false)(&theme, status),
                        sidebar_surface(&theme),
                    ),
                ] {
                    let effective_bg = match style.background {
                        Some(Background::Color(overlay)) => composite_over(under, overlay),
                        _ => under,
                    };
                    let effective_fg = composite_over(effective_bg, style.text_color);
                    assert!(
                        contrast_ratio(effective_fg, effective_bg) >= 4.5,
                        "{name} ({theme:?}, {status:?}) failed AA contrast: fg={effective_fg:?} bg={effective_bg:?}"
                    );
                }
            }

            // Neutral badge & semantic badges/banners across card and canvas surfaces.
            for under in [card, canvas] {
                if let Some(Background::Color(bg_overlay)) = badge_bg(&theme).background {
                    let eff_bg = composite_over(under, bg_overlay);
                    assert!(
                        contrast_ratio(text_muted(&theme), eff_bg) >= 4.5,
                        "badge_bg unreadable on {theme:?}"
                    );
                }
                for (hue, fg) in [
                    (SUCCESS, success_text(&theme)),
                    (WARNING, warning_text(&theme)),
                    (DANGER, danger_text(&theme)),
                    (INFO, info_text(&theme)),
                ] {
                    if let Some(Background::Color(bg_overlay)) =
                        badge_colored(hue, &theme).background
                    {
                        let eff_bg = composite_over(under, bg_overlay);
                        assert!(
                            contrast_ratio(fg, eff_bg) >= 4.5,
                            "badge_colored semantic text unreadable on {theme:?}"
                        );
                        assert!(
                            contrast_ratio(badge_text(hue, &theme), eff_bg) >= 4.5,
                            "badge_text unreadable on {theme:?}"
                        );
                    }
                    if let Some(Background::Color(bg_overlay)) =
                        tinted_banner(&theme, hue).background
                    {
                        let eff_bg = composite_over(under, bg_overlay);
                        assert!(
                            contrast_ratio(fg, eff_bg) >= 4.5,
                            "tinted_banner unreadable on {theme:?}"
                        );
                    }
                }
            }

            // Menu selected item contrast.
            let menu = pick_list_menu(&theme);
            if let (Background::Color(pop), Background::Color(sel_overlay)) =
                (menu.background, menu.selected_background)
            {
                let eff_bg = composite_over(pop, sel_overlay);
                assert!(
                    contrast_ratio(menu.selected_text_color, eff_bg) >= 4.5,
                    "pick_list_menu selected item unreadable on {theme:?}"
                );
            }
        }
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
        assert!(TYPE_TITLE > TYPE_TITLE_SM);
        assert!(TYPE_TITLE_SM > TYPE_HEADING);
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
