//! Shared human-readable byte / speed formatters used across UI pages.

use crate::state::Language;
use crate::ui::i18n::tr;

pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn format_size_precise(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{:.2} B", bytes as f64)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn format_speed(bytes: u64) -> String {
    format!("{}/s", format_size_precise(bytes))
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

/// East Asian wide / emoji code points occupy roughly two Latin columns.
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

/// Truncate to an approximate rendered width, where CJK counts as two
/// columns. Keeps mixed CJK/Latin labels on a single line in fixed cards.
pub fn truncate_width(s: &str, max_cols: usize) -> String {
    let width = |c: char| if is_wide(c) { 2 } else { 1 };
    if s.chars().map(width).sum::<usize>() <= max_cols {
        return s.to_string();
    }
    let budget = max_cols.saturating_sub(1);
    let mut used = 0;
    let mut out = String::new();
    for c in s.chars() {
        let w = width(c);
        if used + w > budget {
            break;
        }
        used += w;
        out.push(c);
    }
    let trimmed = out.trim_end();
    format!("{trimmed}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Language;

    #[test]
    fn truncate_width_counts_cjk_as_two_columns() {
        assert_eq!(truncate_width("HK-01", 10), "HK-01");
        assert_eq!(truncate_width("香港 Hong Kong", 14), "香港 Hong Kong");
        assert_eq!(truncate_width("香港 Hong Kong", 13), "香港 Hong Ko…");
        let out = truncate_width("新加坡备用线路 · Singapore", 12);
        assert_eq!(out, "新加坡备用…");
        assert_eq!(truncate_width("abcdefghij", 6), "abcde…");
        assert_eq!(truncate_width("abcdefghij", 0), "…");
    }

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
    fn format_size_and_speed_use_binary_units() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        let speed = format_speed(2048);
        assert!(speed.ends_with("/s"), "speed={speed}");
        assert!(
            speed.contains("KB") || speed.contains("2.00"),
            "speed={speed}"
        );
    }
}
