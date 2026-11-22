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
pub const WINDOW_MIN_W: f32 = 640.0;
pub const WINDOW_MIN_H: f32 = 480.0;
/// Compact icon-only sidebar width.
pub const SIDEBAR_COMPACT_W: f32 = 64.0;
/// Expanded sidebar width.
pub const SIDEBAR_W: f32 = 200.0;
/// Shell sidebar collapses to icon-only below this window width.
pub const SHELL_COMPACT_W: f32 = 900.0;
/// Page header actions stack vertically below this content width.
/// Kept below `SHELL_COMPACT_W - SIDEBAR_W - 1.0` so expanding the sidebar never
/// pushes content width back down into compact mode.
pub const PAGE_COMPACT_W: f32 = 680.0;
/// Single-column card layouts (settings/rules/profiles/proxies).
pub const PAGE_NARROW_W: f32 = 680.0;
/// Connections: card list below; mid table above this.
pub const CONNECTIONS_TABLE_W: f32 = 740.0;
/// Connections: full 7-column table above this.
pub const CONNECTIONS_WIDE_W: f32 = 1100.0;
/// Settings splits into two columns above this, sized so the default 1080px
/// window (879px content alongside SIDEBAR_W) shows the 2-column layout.
pub const SETTINGS_2COL_W: f32 = 820.0;
/// Dashboard stacks status cards below this.
pub const DASHBOARD_COMPACT_W: f32 = 680.0;
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
    fn shell_breakpoint_never_inverts_page_compact() {
        let min_expanded_content = SHELL_COMPACT_W - SIDEBAR_W - 1.0;
        assert!(
            min_expanded_content >= PAGE_COMPACT_W,
            "Content width at SHELL_COMPACT_W ({}) must stay >= PAGE_COMPACT_W ({})",
            min_expanded_content,
            PAGE_COMPACT_W
        );
        assert!(
            min_expanded_content >= DASHBOARD_COMPACT_W,
            "Content width at SHELL_COMPACT_W ({}) must stay >= DASHBOARD_COMPACT_W ({})",
            min_expanded_content,
            DASHBOARD_COMPACT_W
        );
    }

    #[test]
    fn connection_tiers_are_ordered() {
        assert!(CONNECTIONS_TABLE_W < CONNECTIONS_WIDE_W);
        assert!(PAGE_COMPACT_W <= PAGE_NARROW_W);
        assert!(PAGE_NARROW_W < SETTINGS_2COL_W);
    }

    #[test]
    fn page_max_width_above_settings_2col() {
        assert!(PAGE_MAX_WIDTH >= SETTINGS_2COL_W);
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

    #[test]
    fn usage_bar_portions_cover_all_edge_ratios() {
        assert_eq!(usage_bar_portions(0.0), (0, 1000));
        assert_eq!(usage_bar_portions(-0.5), (0, 1000));
        assert_eq!(usage_bar_portions(f32::NAN), (0, 1000));
        assert_eq!(usage_bar_portions(0.5), (500, 500));
        assert_eq!(usage_bar_portions(0.0001), (1, 999));
        assert_eq!(usage_bar_portions(0.9999), (999, 1));
        assert_eq!(usage_bar_portions(1.0), (1000, 0));
        assert_eq!(usage_bar_portions(2.0), (1000, 0));
    }

    #[test]
    fn page_content_width_stays_non_negative_on_narrow_and_capped_widths() {
        assert_eq!(page_content_width(0.0, true), 0.0);
        assert_eq!(page_content_width(575.0, true), 543.0);
        assert_eq!(page_content_width(2000.0, false), PAGE_MAX_WIDTH - 48.0);
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

/// Page insets switch at one width for every tab. Pages pick their own
/// breakpoints for layout (stacked toolbars, table vs cards), but if each
/// also chose its padding the content edge would jump when changing tabs.
pub fn is_pad_compact(width: f32) -> bool {
    width < PAGE_COMPACT_W
}

pub fn page_pad(is_compact: bool) -> iced::Padding {
    if is_compact {
        page_padding_compact()
    } else {
        page_padding()
    }
}

/// Space available to cards after the shared page insets, in logical pixels.
pub fn page_content_width(width: f32, is_compact: bool) -> f32 {
    let padding = page_pad(is_compact);
    (width.min(PAGE_MAX_WIDTH) - padding.left - padding.right).max(0.0)
}

// Unified page header: title (semibold primary) on the left, optional actions
// on the right, separated by a fill space. Whitespace, not a rule, divides it
// from the body.
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

    // The zero-width strut keeps the title on the same baseline on every tab,
    // stacked or not, with or without actions, without clipping taller rows.
    let title_row = row![Space::new().height(40), title].align_y(Alignment::Center);
    if is_compact {
        if let Some(actions_el) = actions {
            column![title_row, actions_el]
                .spacing(SP_12)
                .width(Length::Fill)
                .into()
        } else {
            title_row.into()
        }
    } else {
        let mut header_row = title_row.width(Length::Fill);

        if let Some(actions_el) = actions {
            header_row = header_row.push(Space::new().width(Length::Fill));
            header_row = header_row.push(actions_el);
        }
        header_row.into()
    }
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
    // Keep the title/actions reachable when editing a long settings page.
    // The floating scrollbar lives in the page's right inset: content gets
    // a matching right pad, so the bar never paints over card edges and the
    // cards still line up with the header actions.
    let body = scrollable(container(content).padding(iced::Padding {
        top: 1.0,
        right: SCROLL_GUTTER,
        bottom: 1.0,
        left: 1.0,
    }))
    .id(iced::widget::Id::new("page-body"))
    .direction(iced::widget::scrollable::Direction::Vertical(
        iced::widget::scrollable::Scrollbar::new()
            .width(6)
            .scroller_width(6)
            .margin(3),
    ))
    .style(ui_theme::scrollbar_style)
    .height(Length::Fill)
    .width(Length::Fill);

    let pad = page_pad(is_compact);
    let col = column![
        container(header).padding(iced::Padding {
            right: SCROLL_GUTTER,
            ..Default::default()
        }),
        body
    ]
    .spacing(SP_20)
    .width(Length::Fill)
    .height(Length::Fill);

    container(col)
        .width(Length::Fill)
        .max_width(PAGE_MAX_WIDTH)
        .center_x(Length::Fill)
        .height(Length::Fill)
        .padding(iced::Padding {
            right: pad.right - SCROLL_GUTTER,
            ..pad
        })
        .into()
}

/// Width of the page scrollbar lane (6px bar + 3px margin each side), carved
/// out of the page's right inset.
pub const SCROLL_GUTTER: f32 = 12.0;

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

/// Fixed shell for pages that scroll an inner pane (the Proxies grid). Like
/// `page_shell_with_pad`, the pane's scrollbar lane lives in the page's right
/// inset, so its cards end on the same edge as every other tab's cards.
/// Rows that do not scroll go through [`lane_aligned`].
pub fn page_body_lane_with_pad<'a>(
    body: Element<'a, Message>,
    is_compact: bool,
) -> Element<'a, Message> {
    let pad = page_pad(is_compact);
    container(body)
        .width(Length::Fill)
        .max_width(PAGE_MAX_WIDTH)
        .center_x(Length::Fill)
        .height(Length::Fill)
        .padding(iced::Padding {
            right: pad.right - SCROLL_GUTTER,
            ..pad
        })
        .into()
}

/// Insets a non-scrolling row by the scrollbar lane, aligning it with the
/// cards of a pane laid out by [`page_body_lane_with_pad`].
pub fn lane_aligned<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .padding(iced::Padding {
            right: SCROLL_GUTTER,
            ..Default::default()
        })
        .into()
}

/// Shared empty-state block: title + description + optional primary CTA.
#[allow(dead_code)]
pub fn empty_state<'a>(
    title: &'a str,
    description: Option<&'a str>,
    cta: Option<Element<'a, Message>>,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    empty_state_with_icon(None, title, description, cta, theme)
}

/// Shared empty-state block with an optional leading circular icon badge.
pub fn empty_state_with_icon<'a>(
    icon: Option<char>,
    title: &'a str,
    description: Option<&'a str>,
    cta: Option<Element<'a, Message>>,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    container(empty_state_inner(icon, title, description, cta, theme))
        .padding([24, 28])
        .width(Length::Fill)
        .center_x(Length::Fill)
        .style(ui_theme::status_card)
        .into()
}

/// Frameless empty-state block for embedding inside an existing surface (e.g. log console).
pub fn empty_state_plain<'a>(
    icon: Option<char>,
    title: &'a str,
    description: Option<&'a str>,
    cta: Option<Element<'a, Message>>,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    container(empty_state_inner(icon, title, description, cta, theme))
        .padding([16, 24])
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
}

fn empty_state_inner<'a>(
    icon: Option<char>,
    title: &'a str,
    description: Option<&'a str>,
    cta: Option<Element<'a, Message>>,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let mut col = column![].spacing(SP_8).align_x(Alignment::Center);

    if let Some(glyph) = icon {
        let badge = container(
            text(glyph.to_string())
                .font(iced::Font::with_name("Material Icons"))
                .size(22.0)
                .color(ui_theme::text_muted(theme)),
        )
        .width(44)
        .height(44)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(|t| {
            let mut s = ui_theme::badge_bg(t);
            s.border.radius = 22.0.into();
            s
        });
        col = col.push(badge).push(Space::new().height(2));
    }

    col = col.push(
        text(title)
            .size(ui_theme::TYPE_HEADING)
            .color(ui_theme::text_primary(theme))
            .font(ui_theme::ui_font(iced::font::Weight::Medium))
            .align_x(Alignment::Center),
    );

    if let Some(desc) = description {
        col = col.push(
            text(desc)
                .size(ui_theme::TYPE_BTN_SM)
                .color(ui_theme::text_muted(theme))
                .align_x(Alignment::Center),
        );
    }
    if let Some(btn) = cta {
        col = col.push(Space::new().height(4)).push(btn);
    }

    col.max_width(440).into()
}

/// Animated dual-tone busy indicator; `phase` advances on UI ticks.
pub fn loading_row_animated<'a>(
    label: &'a str,
    phase: u8,
    theme: &iced::Theme,
) -> Element<'a, Message> {
    let (lead, thumb, trail): (u16, u16, u16) = match phase % 4 {
        0 => (0, 3, 5),
        1 => (2, 4, 2),
        2 => (5, 3, 0),
        _ => (2, 4, 2),
    };
    let thumb_pill = container(Space::new())
        .width(Length::FillPortion(thumb))
        .height(Length::Fixed(5.0))
        .style(|t| container::Style {
            background: Some(iced::Background::Color(ui_theme::accent(t))),
            border: iced::Border {
                radius: 2.5.into(),
                ..Default::default()
            },
            ..Default::default()
        });
    let mut track_row = row![].height(Length::Fixed(5.0)).width(Length::Fill);
    if lead > 0 {
        track_row = track_row.push(Space::new().width(Length::FillPortion(lead)));
    }
    track_row = track_row.push(thumb_pill);
    if trail > 0 {
        track_row = track_row.push(Space::new().width(Length::FillPortion(trail)));
    }
    let bar = container(track_row)
        .width(Length::Fixed(76.0))
        .height(Length::Fixed(5.0))
        .style(|t| container::Style {
            background: Some(iced::Background::Color(ui_theme::with_alpha(
                ui_theme::accent(t),
                if ui_theme::is_dark(t) { 0.16 } else { 0.14 },
            ))),
            border: iced::Border {
                radius: 2.5.into(),
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

fn usage_bar_portions(ratio: f32) -> (u16, u16) {
    let clamped = if ratio.is_finite() {
        ratio.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if clamped <= 0.0 {
        (0, 1000)
    } else if clamped >= 1.0 {
        (1000, 0)
    } else {
        let filled = ((clamped * 1000.0).round() as u16).clamp(1, 999);
        (filled, 1000 - filled)
    }
}

/// Thin subscription usage meter: jade until 75%, amber to 90%, then red.
pub fn usage_bar<'a>(ratio: f32, theme: &iced::Theme) -> Element<'a, Message> {
    let ratio = if ratio.is_finite() {
        ratio.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let fill = if ratio >= 0.9 {
        ui_theme::DANGER
    } else if ratio >= 0.75 {
        ui_theme::WARNING
    } else {
        ui_theme::accent(theme)
    };
    let segment = |portion: u16, color: Option<iced::Color>| {
        container(Space::new())
            .width(Length::FillPortion(portion))
            .height(5.0)
            .style(move |_| container::Style {
                background: color.map(iced::Background::Color),
                border: iced::Border {
                    radius: 2.5.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
    };
    let (filled, rest) = usage_bar_portions(ratio);
    let mut track = row![].width(Length::Fill);
    if filled > 0 {
        track = track.push(segment(filled, Some(fill)));
    }
    if rest > 0 {
        track = track.push(segment(rest, None));
    }
    container(track)
        .width(Length::Fill)
        .style(|t| container::Style {
            background: Some(iced::Background::Color(if ui_theme::is_dark(t) {
                iced::Color::from_rgba(1.0, 1.0, 1.0, 0.10)
            } else {
                iced::Color::from_rgba(0.0, 0.0, 0.0, 0.09)
            })),
            border: iced::Border {
                radius: 2.5.into(),
                ..Default::default()
            },
            ..Default::default()
        })
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

    row![
        dot_wrapper,
        text(label)
            .color(text_color)
            .size(text_size)
            .wrapping(text::Wrapping::None)
    ]
    .spacing(theme::SP_8)
    .align_y(Alignment::Center)
    .into()
}
