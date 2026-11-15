//! Shared human-readable byte / speed formatters used across UI pages.

use crate::state::Language;
use crate::ui::i18n::tr;

const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

/// Binary-unit size with three significant digits, the way Explorer and
/// Finder print sizes: "512 B", "6.41 MB", "12.3 GB", "376 KB". A value
/// that would round up to four digits moves to the next unit instead.
pub fn format_size(bytes: u64) -> String {
    if bytes < 1000 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while unit + 1 < UNITS.len() && value >= 999.5 {
        value /= 1024.0;
        unit += 1;
    }
    let decimals = if value < 9.995 {
        2
    } else if value < 99.95 {
        1
    } else {
        0
    };
    format!("{value:.decimals$} {}", UNITS[unit])
}

pub fn format_speed(bytes: u64) -> String {
    format!("{}/s", format_size(bytes))
}

/// Format subscription traffic line: used/total (remaining).
#[allow(dead_code)]
pub fn format_traffic_usage(upload: u64, download: u64, total: u64) -> String {
    format_traffic_usage_lang(Language::En, upload, download, total)
}

/// Localized subscription traffic line.
pub fn format_traffic_usage_lang(lang: Language, upload: u64, download: u64, total: u64) -> String {
    let used = upload.saturating_add(download);
    if total == 0 {
        format!("{} {}", format_size(used), tr(lang, "traffic_used"))
    } else {
        let remain = total.saturating_sub(used);
        format!(
            "{} / {} ({} {})",
            format_size(used),
            format_size(total),
            format_size(remain),
            tr(lang, "traffic_left")
        )
    }
}

/// "剩余 1 GB" vs "1 GB left": the label leads in Chinese and trails in English.
pub fn labelled_size(lang: Language, key: &'static str, bytes: u64) -> String {
    match lang {
        Language::Zh => format!("{} {}", tr(lang, key), format_size(bytes)),
        Language::En => format!("{} {}", format_size(bytes), tr(lang, key)),
    }
}

/// Usage ratio 0.0–1.0 when total is known; None if total is 0.
pub fn traffic_usage_ratio(upload: u64, download: u64, total: u64) -> Option<f32> {
    if total == 0 {
        None
    } else {
        let used = upload.saturating_add(download) as f64;
        Some((used / total as f64).clamp(0.0, 1.0) as f32)
    }
}

/// Truncate a string to at most `max_chars` Unicode characters, appending `...`.
#[cfg(test)]
pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        s.to_string()
    } else if max_chars <= 3 {
        s.chars().take(max_chars).collect()
    } else {
        let head: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", head)
    }
}

/// East Asian wide / emoji code points occupy a full em.
fn is_wide(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x115F
        | 0x2E80..=0xA4CF
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE4F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6
        | 0x1F300..=0x1FAFF
        | 0x20000..=0x3FFFD)
}

/// A field label followed by its colon: full-width "：" in Chinese, where the
/// ASCII colon looks cramped against CJK glyphs.
pub fn with_colon(lang: Language, label: &str) -> String {
    match lang {
        Language::Zh => format!("{label}："),
        Language::En => format!("{label}:"),
    }
}

/// "Label: value" with a language-appropriate separator.
pub fn labelled(lang: Language, label: &str, value: impl std::fmt::Display) -> String {
    match lang {
        Language::Zh => format!("{label}：{value}"),
        Language::En => format!("{label}: {value}"),
    }
}

/// Approximate advance of `c`, in em, for the proportional UI face. Values
/// are Segoe UI's advances rounded up a little so Helvetica Neue and the
/// common Linux sans faces still fit; CJK falls back to Noto Sans CJK at a
/// full em. iced 0.14 cannot ellipsize, so fixed-width labels are clipped
/// here, erring wide so a label never runs under its neighbour.
fn sans_em(c: char) -> f32 {
    if is_wide(c) {
        return 1.0;
    }
    match c {
        '.' | ',' | ':' | ';' | '\'' | '|' | '·' => 0.24,
        'i' | 'j' | 'l' => 0.25,
        ' ' => 0.28,
        '!' | 'I' => 0.28,
        '(' | ')' | '[' | ']' => 0.31,
        'f' | 't' | 'r' => 0.36,
        'J' => 0.38,
        '-' | '/' | '\\' | '"' | '_' => 0.41,
        'c' | 'k' | 's' | 'v' | 'x' | 'y' | 'z' => 0.48,
        'a' | 'e' => 0.53,
        'w' => 0.74,
        'm' => 0.88,
        'a'..='z' => 0.55,
        '0'..='9' => 0.56,
        'E' | 'F' | 'L' => 0.52,
        'B' | 'K' | 'P' | 'R' | 'S' | 'T' | 'X' | 'Y' | 'Z' => 0.59,
        'A' | 'C' | 'V' => 0.66,
        'M' | 'W' => 0.93,
        'A'..='Z' => 0.74,
        '…' => 0.76,
        '→' => 0.88,
        _ => 0.62,
    }
}

/// Monospace face (Consolas / SF Mono / DejaVu Sans Mono) advance in em.
fn mono_em(c: char) -> f32 {
    if is_wide(c) { 1.0 } else { 0.56 }
}

/// Semibold labels run 2–4% wider than the regular weight.
pub const SEMIBOLD_WIDTH: f32 = 1.03;

/// The light display weight runs about 5% narrower than regular; 0.96 keeps a
/// little slack for platforms that fall back to the regular face.
pub const LIGHT_WIDTH: f32 = 0.96;

fn fit_with(s: &str, max_px: f32, size: f32, em: fn(char) -> f32) -> String {
    let total: f32 = s.chars().map(em).sum::<f32>() * size;
    if total <= max_px {
        return s.to_string();
    }
    let budget = max_px - em('…') * size;
    let mut used = 0.0;
    let mut out = String::new();
    for c in s.chars() {
        let w = em(c) * size;
        if used + w > budget {
            break;
        }
        used += w;
        out.push(c);
    }
    let trimmed = out
        .trim_end()
        .trim_end_matches(['·', '-', '/', '→', ','])
        .trim_end();
    format!("{trimmed}…")
}

/// Estimated rendered width of `s` in the proportional UI font.
pub fn text_width(s: &str, size: f32) -> f32 {
    s.chars().map(sans_em).sum::<f32>() * size
}

/// Ellipsize `s` so it fits `max_px` at `size` in the proportional UI font.
pub fn fit_width(s: &str, max_px: f32, size: f32) -> String {
    fit_with(s, max_px, size, sans_em)
}

/// Ellipsize `s` so it fits `max_px` at `size` in the monospace font.
pub fn fit_mono(s: &str, max_px: f32, size: f32) -> String {
    fit_with(s, max_px, size, mono_em)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Language;

    #[test]
    fn truncate_short_unchanged() {
        assert_eq!(truncate_chars("hello", 10), "hello");
    }

    #[test]
    fn truncate_long() {
        assert_eq!(truncate_chars("abcdefghij", 6), "abc...");
    }

    #[test]
    fn truncate_unicode_counts_chars_not_bytes() {
        // 6 CJK chars → keep 3 + "..." when max is 6
        let s = "一二三四五六七八";
        let out = truncate_chars(s, 6);
        assert_eq!(out.chars().count(), 6); // 3 chars + 3 dots
        assert!(out.ends_with("..."));
    }

    #[test]
    fn traffic_ratio() {
        assert_eq!(traffic_usage_ratio(50, 50, 200), Some(0.5));
        assert_eq!(traffic_usage_ratio(1, 1, 0), None);
        assert_eq!(traffic_usage_ratio(100, 0, 100), Some(1.0));
    }

    #[test]
    fn format_traffic_usage_lang_uses_i18n_suffixes() {
        let en = format_traffic_usage_lang(Language::En, 512, 512, 0);
        assert!(en.contains("used"), "en line: {en}");
        let zh = format_traffic_usage_lang(Language::Zh, 512, 512, 0);
        assert!(zh.contains("已用"), "zh line: {zh}");

        let en_total = format_traffic_usage_lang(Language::En, 100, 100, 1000);
        assert!(en_total.contains("left"), "en total: {en_total}");
        let zh_total = format_traffic_usage_lang(Language::Zh, 100, 100, 1000);
        assert!(zh_total.contains("剩余"), "zh total: {zh_total}");
    }

    #[test]
    fn format_size_and_speed_use_three_significant_digits() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(999), "999 B");
        assert_eq!(format_size(1000), "0.98 KB");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(385_024), "376 KB");
        assert_eq!(format_size(6_721_372), "6.41 MB");
        assert_eq!(format_size(12_897_484), "12.3 MB");
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(42_310_000_000), "39.4 GB");
        assert_eq!(format_size(1_099_511_627_776), "1.00 TB");
        assert_eq!(format_speed(0), "0 B/s");
        assert_eq!(format_speed(2048), "2.00 KB/s");
        // 1023.9 KB would print four digits; it moves up a unit instead.
        assert_eq!(format_size(1_048_000), "1.00 MB");
    }

    #[test]
    fn fit_width_keeps_short_text_and_ellipsizes_long_text() {
        assert_eq!(fit_width("HK-01", 200.0, 14.0), "HK-01");
        let out = fit_width("香港 Hong Kong · Premium 01", 120.0, 14.0);
        assert!(out.ends_with('…'), "{out}");
        assert!(text_width(&out, 14.0) <= 120.0, "{out}");
        // Separators never dangle before the ellipsis.
        let out = fit_width("Proxy → 香港 Hong Kong", 70.0, 13.0);
        assert!(!out.contains("→…"), "{out}");
        assert_eq!(fit_mono("abcdefghij", 34.0, 10.0), "abcde…");
        assert_eq!(fit_mono("abc", 0.0, 10.0), "…");
    }
}
