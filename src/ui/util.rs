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
        | 0x231A..=0x23FF
        | 0x2600..=0x27BF
        | 0x2B00..=0x2BFF
        | 0x2E80..=0xA4CF
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE10..=0xFE1F
        | 0xFE30..=0xFE6F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6
        | 0x1F000..=0x1FAFF
        | 0x20000..=0x3FFFD)
}

fn is_regional_indicator(c: char) -> bool {
    ('\u{1F1E6}'..='\u{1F1FF}').contains(&c)
}

fn is_combining_or_extend(c: char) -> bool {
    matches!(c as u32,
        0x0300..=0x036F
        | 0x0483..=0x0489
        | 0x0591..=0x05BD
        | 0x05BF
        | 0x05C1..=0x05C2
        | 0x05C4..=0x05C5
        | 0x05C7
        | 0x0610..=0x061A
        | 0x064B..=0x065F
        | 0x0670
        | 0x06D6..=0x06ED
        | 0x0900..=0x0903
        | 0x093A..=0x094F
        | 0x0951..=0x0957
        | 0x0962..=0x0963
        | 0x0E31
        | 0x0E34..=0x0E3A
        | 0x0E47..=0x0E4E
        | 0x1160..=0x11FF
        | 0x1AB0..=0x1AFF
        | 0x1DC0..=0x1DFF
        | 0x20D0..=0x20FF
        | 0xD7B0..=0xD7FF
        | 0xFE00..=0xFE0F
        | 0xFE20..=0xFE2F
        | 0x1F3FB..=0x1F3FF
        | 0xE0020..=0xE007F
        | 0xE0100..=0xE01EF)
}

fn is_zero_width_scalar(c: char) -> bool {
    matches!(
        c,
        '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}' | '\u{00AD}'
    ) || (c.is_control() && c != '\t')
        || is_combining_or_extend(c)
}

/// Split off the leading Unicode grapheme cluster from a non-empty UTF-8 slice.
fn next_grapheme(s: &str) -> (&str, &str) {
    let mut iter = s.char_indices();
    let Some((_, c0)) = iter.next() else {
        return ("", "");
    };
    let mut end = c0.len_utf8();

    // Keep CRLF together.
    if c0 == '\r' && s[end..].starts_with('\n') {
        end += 1;
        return (&s[..end], &s[end..]);
    }

    // Keep regional-indicator pairs (flag emoji) together.
    if is_regional_indicator(c0)
        && let Some(c1) = s[end..].chars().next()
        && is_regional_indicator(c1)
    {
        end += c1.len_utf8();
    }

    loop {
        while let Some(c) = s[end..].chars().next() {
            if is_combining_or_extend(c) {
                end += c.len_utf8();
            } else {
                break;
            }
        }
        // Glue ZWJ (`\u{200D}`) plus the joined base glyph (and its extenders).
        if let Some('\u{200D}') = s[end..].chars().next() {
            end += '\u{200D}'.len_utf8();
            if let Some(joined) = s[end..].chars().next()
                && !matches!(joined, '\r' | '\n')
            {
                end += joined.len_utf8();
                continue;
            }
        }
        break;
    }

    (&s[..end], &s[end..])
}

fn last_grapheme_start(s: &str) -> usize {
    let mut last_start = 0;
    let mut pos = 0;
    let mut rest = s;
    while !rest.is_empty() {
        let (cluster, next) = next_grapheme(rest);
        last_start = pos;
        pos += cluster.len();
        rest = next;
    }
    last_start
}

fn cluster_em(cluster: &str, base_em: fn(char) -> f32) -> f32 {
    let Some(c0) = cluster.chars().next() else {
        return 0.0;
    };
    if cluster.contains('\u{200D}') {
        return 1.25;
    }
    if is_regional_indicator(c0) {
        return if cluster.chars().nth(1).is_some() {
            1.3
        } else {
            1.0
        };
    }
    if is_zero_width_scalar(c0) {
        return 0.0;
    }
    let w = base_em(c0);
    if cluster.contains('\u{FE0F}') {
        w.max(1.0)
    } else {
        w
    }
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
        '—' => 1.0,
        '–' => 0.56,
        '•' => 0.40,
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

fn measure_with(s: &str, size: f32, em: fn(char) -> f32) -> f32 {
    let mut sum_em = 0.0f32;
    let mut rest = s;
    while !rest.is_empty() {
        let (cluster, next) = next_grapheme(rest);
        sum_em += cluster_em(cluster, em);
        rest = next;
    }
    sum_em * size
}

fn trim_dangling_suffix(s: &str) -> &str {
    s.trim_end_matches(|c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '·' | '-' | '–' | '—' | '/' | '→' | ',' | '，' | '、' | '|' | '•'
            )
    })
}

fn fit_with(s: &str, max_px: f32, size: f32, em: fn(char) -> f32) -> String {
    if s.is_empty() {
        return String::new();
    }
    if !size.is_finite() || size <= 0.0 {
        return if max_px.is_nan() || max_px < 0.0 {
            "…".to_string()
        } else {
            s.to_string()
        };
    }
    if max_px.is_infinite() && max_px.is_sign_positive() {
        return s.to_string();
    }
    if max_px.is_nan() || max_px <= 0.0 {
        return "…".to_string();
    }
    let total = measure_with(s, size, em);
    if total <= max_px {
        return s.to_string();
    }
    let ellipsis_px = em('…') * size;
    let budget = max_px - ellipsis_px;
    if budget <= 0.0 {
        return "…".to_string();
    }
    let mut used = 0.0f32;
    let mut out = String::new();
    let mut rest = s;
    while !rest.is_empty() {
        let (cluster, next) = next_grapheme(rest);
        let w = cluster_em(cluster, em) * size;
        if used + w > budget {
            break;
        }
        used += w;
        out.push_str(cluster);
        rest = next;
    }
    let mut trimmed = trim_dangling_suffix(&out);
    let mut result = format!("{trimmed}…");
    if max_px >= ellipsis_px {
        while measure_with(&result, size, em) > max_px && !trimmed.is_empty() {
            let prev_end = last_grapheme_start(trimmed);
            trimmed = trim_dangling_suffix(&trimmed[..prev_end]);
            result = format!("{trimmed}…");
        }
    }
    result
}

/// Estimated rendered width of `s` in the proportional UI font.
pub fn text_width(s: &str, size: f32) -> f32 {
    if !size.is_finite() || size <= 0.0 {
        return 0.0;
    }
    measure_with(s, size, sans_em)
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

    #[test]
    fn fit_width_preserves_grapheme_clusters_and_handles_edge_budgets() {
        // Flag emoji pair is never split into a half-flag regional indicator.
        let flag = "🇭🇰🇺🇸🇯🇵";
        let out = fit_width(flag, 35.0, 14.0);
        assert!(out.ends_with('…'), "{out}");
        let prefix = out.trim_end_matches('…');
        assert_eq!(prefix.chars().count() % 2, 0, "split flag pair in {out:?}");

        // ZWJ emoji sequence stays intact or drops cleanly without dangling ZWJ.
        let family = "👨‍👩‍👧 Family Node";
        let out_zwj = fit_width(family, 28.0, 14.0);
        assert!(!out_zwj.contains('\u{200D}') || out_zwj.starts_with("👨‍👩‍👧"));

        // Combining accents stay attached to their base letter.
        let accented = "Cafe\u{0301} Express";
        let out_acc = fit_width(accented, 42.0, 14.0);
        assert!(out_acc.starts_with("Cafe\u{0301}") || !out_acc.ends_with('\u{0301}'));

        // Multiple chained separators never dangle.
        let chained = "Hong Kong · - Premium";
        let out_sep = fit_width(chained, 78.0, 14.0);
        assert!(
            !out_sep.ends_with("·…") && !out_sep.ends_with("-…"),
            "{out_sep}"
        );

        // Degenerate inputs never panic or overshoot.
        assert_eq!(fit_width("", 0.0, 14.0), "");
        assert_eq!(fit_width("abc", -10.0, 14.0), "…");
        assert_eq!(fit_width("abc", f32::NAN, 14.0), "…");
        assert_eq!(fit_width("abc", f32::INFINITY, 14.0), "abc");
        assert_eq!(text_width("abc", -5.0), 0.0);

        // Zero-width markers add zero advance width.
        assert_eq!(text_width("a\u{200B}b", 14.0), text_width("ab", 14.0));
    }
}
