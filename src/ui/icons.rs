pub const ICON_DASHBOARD: char = '\u{E871}';
pub const ICON_PROXIES: char = '\u{E63E}';
pub const ICON_PROFILES: char = '\u{E2C7}';
pub const ICON_RULES: char = '\u{E41E}';
pub const ICON_CONNECTIONS: char = '\u{E894}';
pub const ICON_LOGS: char = '\u{E85D}';
pub const ICON_DOWN: char = '\u{E5DB}';
pub const ICON_UP: char = '\u{E5D8}';
pub const ICON_SETTINGS: char = '\u{E8B8}';
pub const ICON_WARNING: char = '\u{E002}';
pub const ICON_CLOSE: &str = "\u{E5CD}";
pub const ICON_ADD: &str = "\u{E145}";
pub const ICON_COPY: &str = "\u{E14D}";
pub const ICON_FOLDER_OPEN: &str = "\u{E2C8}";
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
            ICON_DASHBOARD,
            ICON_PROXIES,
            ICON_PROFILES,
            ICON_RULES,
            ICON_CONNECTIONS,
            ICON_LOGS,
            ICON_DOWN,
            ICON_UP,
            ICON_SETTINGS,
            ICON_WARNING,
            ICON_CHECK,
            ICON_POWER,
            ICON_CHECK_CIRCLE,
            ICON_ERROR,
            ICON_INFO,
            ICON_EXPAND_MORE,
            ICON_EXPAND_LESS,
        ]
        .into_iter()
        .chain(ICON_CLOSE.chars())
        .chain(ICON_ADD.chars())
        .chain(ICON_COPY.chars())
        .chain(ICON_FOLDER_OPEN.chars())
        {
            assert!(
                font.glyph_index(icon).is_some(),
                "Missing icon U+{:04X}",
                icon as u32
            );
        }
    }
}
