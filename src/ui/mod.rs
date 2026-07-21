pub mod connections;
pub mod dashboard;
pub mod i18n;
pub mod icons;
pub mod logs;
pub mod profiles;
pub mod proxies;
pub mod rules;
pub mod settings;
pub mod theme;
pub mod toast;
pub mod util;

use crate::message::Message;
use crate::ui::theme as ui_theme;
use iced::widget::{Space, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

// ── Responsive breakpoints (content / window width in px) ────────────────────
/// Minimum window width — must stay below SHELL_COMPACT_W so icon sidebar is reachable.
pub const WINDOW_MIN_W: f32 = 720.0;
pub const WINDOW_MIN_H: f32 = 600.0;
/// Shell sidebar collapses to icon-only below this window width.
pub const SHELL_COMPACT_W: f32 = 820.0;
/// Page header actions stack vertically below this content width.
pub const PAGE_COMPACT_W: f32 = 750.0;
/// Single-column card layouts (settings/rules).
pub const PAGE_NARROW_W: f32 = 800.0;
/// Connections: card list below; mid table above this.
pub const CONNECTIONS_TABLE_W: f32 = 850.0;
/// Connections: full 7-column table above this.
pub const CONNECTIONS_WIDE_W: f32 = 1100.0;
/// Settings 2-column layout above this; 3-col above SETTINGS_3COL_W.
pub const SETTINGS_2COL_W: f32 = 800.0;
pub const SETTINGS_3COL_W: f32 = 1150.0;
/// Dashboard stacks status cards below this.
pub const DASHBOARD_COMPACT_W: f32 = 900.0;
/// Content max width for page shells.
pub const PAGE_MAX_WIDTH: f32 = 1200.0;

#[cfg(test)]
#[allow(clippy::items_after_test_module, clippy::assertions_on_constants)]
mod tests {
    use super::*;

    #[test]
    fn window_min_allows_shell_compact() {
        // Criterion 1: user can resize below shell compact breakpoint.
        assert!(
            WINDOW_MIN_W < SHELL_COMPACT_W,
            "WINDOW_MIN_W ({}) must be < SHELL_COMPACT_W ({})",
            WINDOW_MIN_W,
            SHELL_COMPACT_W
        );
    }

    #[test]
    fn connection_tiers_are_ordered() {
        assert!(CONNECTIONS_TABLE_W < CONNECTIONS_WIDE_W);
        assert!(PAGE_COMPACT_W <= PAGE_NARROW_W);
        assert!(SETTINGS_2COL_W < SETTINGS_3COL_W);
    }

    #[test]
    fn page_max_width_above_settings_3col() {
        assert!(PAGE_MAX_WIDTH >= SETTINGS_3COL_W);
    }

    #[test]
    fn spacing_scale_is_ordered() {
        assert!(SP_8 < SP_12);
        assert!(SP_12 < SP_16);
        assert!(SP_16 < SP_20);
        assert!(SP_20 < SP_24);
        // Grid gap aligns with spacing language
        #[cfg(target_os = "macos")]
        assert_eq!(ui_theme::GRID_GAP, 18.0);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(ui_theme::GRID_GAP, SP_16);
        assert_eq!(ICON_SIZE, 16.0);
        assert!(ICON_SIZE_LG > ICON_SIZE);
    }

    #[test]
    fn material_icons_font_asset_is_present() {
        // Same bytes path the iced app builder registers at startup.
        let bytes = include_bytes!("../../assets/material-icons.ttf");
        assert!(
            bytes.len() > 1024,
            "material-icons.ttf should be a real font file, got {} bytes",
            bytes.len()
        );
        // TrueType / OpenType magic: 0x00010000 or 'OTTO' / 'true'
        let is_ttf = bytes.len() >= 4
            && ((bytes[0] == 0x00 && bytes[1] == 0x01 && bytes[2] == 0x00 && bytes[3] == 0x00)
                || &bytes[0..4] == b"OTTO"
                || &bytes[0..4] == b"true"
                || &bytes[0..4] == b"ttcf");
        assert!(
            is_ttf,
            "material-icons.ttf does not look like a font header"
        );
    }

    #[test]
    fn bundled_cjk_font_asset_is_present() {
        let bytes = include_bytes!("../../assets/NotoSansCJK-Regular.ttc");
        assert!(
            bytes.len() > 1_000_000,
            "bundled CJK font is unexpectedly small"
        );
        assert_eq!(
            &bytes[0..4],
            b"ttcf",
            "bundled CJK font must be a TTC collection"
        );
    }

    #[test]
    fn check_icon_loading() {
        let bytes = include_bytes!("../../assets/app-icon.png");
        let icon = iced::window::icon::from_file_data(bytes, None);
        assert!(icon.is_ok(), "Failed to load icon: {:?}", icon.err());
    }
}

// ── Spacing scale ────────────────────────────────────────────────────────────
pub const SP_8: f32 = 8.0;
pub const SP_12: f32 = 12.0;
/// Aligns with `theme::GRID_GAP` — keep for spacing-scale completeness.
#[allow(dead_code)]
pub const SP_16: f32 = 16.0;
pub const SP_20: f32 = 20.0;
#[allow(dead_code)]
pub const SP_24: f32 = 24.0;

// Unified page padding — generous breathing room for premium layout
pub fn page_padding() -> iced::Padding {
    iced::Padding {
        top: 24.0,
        right: 24.0,
        bottom: 28.0,
        left: 24.0,
    }
}

/// Compact page padding for narrow layouts.
pub fn page_padding_compact() -> iced::Padding {
    iced::Padding {
        top: 20.0,
        right: 16.0,
        bottom: 24.0,
        left: 16.0,
    }
}

pub fn page_pad(is_compact: bool) -> iced::Padding {
    if is_compact {
        page_padding_compact()
    } else {
        page_padding()
    }
}

// Unified page header: title (size 22, semibold primary) on the left, optional
// actions on the right, separated by a fill space.
pub fn page_header<'a>(
    title_key: &'static str,
    lang: crate::state::Language,
    actions: Option<Element<'a, Message>>,
    theme: &iced::Theme,
    is_compact: bool,
) -> Element<'a, Message> {
    let text_primary = ui_theme::text_primary(theme);
    let title = text(crate::ui::i18n::tr(lang, title_key))
        .size(ui_theme::TYPE_TITLE)
        .font(ui_theme::ui_font(iced::font::Weight::Semibold))
        .color(text_primary);

    let content: Element<'a, Message> = if is_compact {
        if let Some(actions_el) = actions {
            column![title, actions_el]
                .spacing(SP_12)
                .width(Length::Fill)
                .into()
        } else {
            row![title].into()
        }
    } else {
        let mut header_row = row![title].align_y(Alignment::Center).width(Length::Fill);

        if let Some(actions_el) = actions {
            header_row = header_row.push(Space::new().width(Length::Fill));
            header_row = header_row.push(actions_el);
        }
        header_row.into()
    };

    let divider = container(Space::new())
        .height(1)
        .width(Length::Fill)
        .style(|t| container::Style {
            background: Some(iced::Background::Color(ui_theme::border_color(t))),
            ..Default::default()
        });

    column![content, divider]
        .spacing(SP_12)
        .width(Length::Fill)
        .into()
}

/// Outer scrollable page shell — for tabs whose content is taller than the window.
#[allow(dead_code)]
pub fn page_shell<'a>(
    header: Element<'a, Message>,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    page_shell_with_pad(header, content, false)
}

pub fn page_shell_with_pad<'a>(
    header: Element<'a, Message>,
    content: Element<'a, Message>,
    is_compact: bool,
) -> Element<'a, Message> {
    let col = column![header, content].spacing(SP_20).width(Length::Fill);

    let inner = container(col)
        .width(Length::Fill)
        .max_width(PAGE_MAX_WIDTH)
        .center_x(Length::Fill)
        .padding(page_pad(is_compact));

    container(
        scrollable(inner)
            .style(ui_theme::scrollbar_style)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Non-scrolling shell for pages that manage their own inner scroll area
/// (e.g. Logs terminal, Proxies grid).
#[allow(dead_code)]
pub fn page_shell_fixed<'a>(
    header: Element<'a, Message>,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    page_shell_fixed_with_pad(header, content, false)
}

pub fn page_shell_fixed_with_pad<'a>(
    header: Element<'a, Message>,
    content: Element<'a, Message>,
    is_compact: bool,
) -> Element<'a, Message> {
    let col = column![header, content]
        .spacing(SP_20)
        .width(Length::Fill)
        .height(Length::Fill);

    container(col)
        .width(Length::Fill)
        .max_width(PAGE_MAX_WIDTH)
        .center_x(Length::Fill)
        .height(Length::Fill)
        .padding(page_pad(is_compact))
        .into()
}

/// Wrap arbitrary page body (header already included) in the fixed shell.
#[allow(dead_code)]
pub fn page_body_fixed<'a>(body: Element<'a, Message>) -> Element<'a, Message> {
    page_body_fixed_with_pad(body, false)
}

pub fn page_body_fixed_with_pad<'a>(
    body: Element<'a, Message>,
    is_compact: bool,
) -> Element<'a, Message> {
    container(body)
        .width(Length::Fill)
        .max_width(PAGE_MAX_WIDTH)
        .center_x(Length::Fill)
        .height(Length::Fill)
        .padding(page_pad(is_compact))
        .into()
}

/// Shared empty-state block: title + description + optional primary CTA.
pub fn empty_state<'a>(
    title: &'a str,
    description: Option<&'a str>,
    cta: Option<Element<'a, Message>>,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let mut col = column![
        text(title)
            .size(ui_theme::TYPE_HEADING)
            .color(ui_theme::text_primary(theme))
            .font(ui_theme::ui_font(iced::font::Weight::Medium)),
    ]
    .spacing(SP_8)
    .align_x(Alignment::Center);

    if let Some(desc) = description {
        col = col.push(
            text(desc)
                .size(ui_theme::TYPE_BTN_SM)
                .color(ui_theme::text_muted(theme)),
        );
    }
    if let Some(btn) = cta {
        col = col.push(btn);
    }

    container(col)
        .padding(theme::SP_40)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .style(ui_theme::status_card)
        .into()
}

/// Busy / loading row: accent bar + label (download, latency, update check).
pub fn loading_row<'a>(label: &'a str, theme: &iced::Theme) -> Element<'a, Message> {
    let bar = container(Space::new())
        .width(Length::Fixed(72.0))
        .height(Length::Fixed(4.0))
        .style(|_t| container::Style {
            background: Some(iced::Background::Color(ui_theme::ACCENT_PURPLE)),
            border: iced::Border {
                radius: 2.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });
    row![
        bar,
        text(label)
            .size(ui_theme::TYPE_CAPTION)
            .color(ui_theme::text_muted(theme)),
    ]
    .spacing(SP_12)
    .align_y(Alignment::Center)
    .into()
}

/// Default Material Icons glyph size used across shell + pages.
pub const ICON_SIZE: f32 = 16.0;
/// Slightly larger icons for primary actions / compact sidebar.
pub const ICON_SIZE_LG: f32 = 18.0;

/// Material Icons glyph as text (requires material-icons font loaded).
#[allow(dead_code)]
pub fn material_icon(unicode: char) -> text::Text<'static> {
    text(unicode.to_string())
        .font(iced::Font::with_name("Material Icons"))
        .size(ICON_SIZE)
}

/// Shared status dot component (used in dashboard and sidebar)
pub fn status_dot<'a, Message: Clone + 'a>(
    color: iced::Color,
    is_active: bool,
    label: &'a str,
    text_color: iced::Color,
    text_size: f32,
) -> Element<'a, Message> {
    let dot = container(Space::new())
        .width(8)
        .height(8)
        .style(move |_t| container::Style {
            background: Some(iced::Background::Color(color)),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let dot_wrapper = if is_active {
        container(dot)
            .padding(4)
            .style(move |_t| ui_theme::status_ring(color))
    } else {
        container(dot).padding(4)
    };

    row![dot_wrapper, text(label).color(text_color).size(text_size)]
        .spacing(theme::SP_8)
        .align_y(Alignment::Center)
        .into()
}
