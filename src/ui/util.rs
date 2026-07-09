/// Shared human-readable byte / speed formatters used across UI pages.

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
pub fn format_traffic_usage(upload: u64, download: u64, total: u64) -> String {
    let used = upload.saturating_add(download);
    if total == 0 {
        format!("{} used", format_size(used))
    } else {
        let remain = total.saturating_sub(used);
        format!(
            "{} / {} ({} left)",
            format_size(used),
            format_size(total),
            format_size(remain)
        )
    }
}
