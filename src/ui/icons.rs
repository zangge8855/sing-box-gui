pub const ICON_PROXIES: char = '\u{E63E}';
pub const ICON_DOWN: char = '\u{E5DB}';
pub const ICON_UP: char = '\u{E5D8}';
pub const ICON_SETTINGS: char = '\u{E8B8}';
pub const ICON_WARNING: char = '\u{E002}';
pub const ICON_DELETE: &str = "\u{E5CD}";
pub const ICON_ADD: &str = "\u{E145}";
pub const ICON_CHECK: char = '\u{E5CA}';
pub const ICON_POWER: char = '\u{E8AC}';
pub const ICON_CHECK_CIRCLE: char = '\u{E86C}';
pub const ICON_ERROR: char = '\u{E000}';
pub const ICON_INFO: char = '\u{E88E}';
pub const ICON_EXPAND_MORE: char = '\u{E5CF}';
pub const ICON_EXPAND_LESS: char = '\u{E5CE}';

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_font_covers_used_icons() {
        let font =
            ttf_parser::Face::parse(include_bytes!("../../assets/material-icons.ttf"), 0).unwrap();
        for icon in [
            ICON_PROXIES,
            ICON_DOWN,
            ICON_UP,
            ICON_SETTINGS,
            ICON_WARNING,
            ICON_CHECK,
            ICON_POWER,
            ICON_CHECK_CIRCLE,
            ICON_ERROR,
            ICON_INFO,
        ]
        .into_iter()
        .chain(ICON_DELETE.chars())
        .chain(ICON_ADD.chars())
        {
            assert!(
                font.glyph_index(icon).is_some(),
                "Missing icon U+{:04X}",
                icon as u32
            );
        }
    }
}
