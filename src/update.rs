pub fn normalize_version_tag(tag: &str) -> Vec<u64> {
    let normalized = tag.trim().trim_start_matches('v');
    let without_build = normalized.split('+').next().unwrap_or(normalized);
    let (core, revision) = match without_build.rsplit_once('-') {
        Some((core, suffix)) if suffix.chars().all(|c| c.is_ascii_digit()) => {
            (core, suffix.parse::<u64>().unwrap_or(0))
        }
        Some((core, _)) => (core, 0),
        _ => (without_build, 0),
    };

    let mut parts: Vec<u64> = core
        .split('.')
        .filter_map(|p| p.parse::<u64>().ok())
        .collect();
    parts.push(revision);
    parts
}


pub fn is_remote_version_newer(local_pkg_version: &str, remote_tag: &str) -> bool {
    let local = normalize_version_tag(&format!("v{}", local_pkg_version));
    let remote = normalize_version_tag(remote_tag);
    if local.is_empty() || remote.is_empty() {
        return remote_tag.trim().trim_start_matches('v')
            != local_pkg_version.trim().trim_start_matches('v');
    }
    for (l, r) in local.iter().zip(remote.iter()) {
        match r.cmp(l) {
            std::cmp::Ordering::Greater => return true,
            std::cmp::Ordering::Less => return false,
            std::cmp::Ordering::Equal => continue,
        }
    }
    // Equal up to the shared length — longer remote (e.g. 2026.7.9 vs 2026.7)
    // means newer if the trailing components are non-zero.
    remote.len() > local.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_version_tag_supports_same_day_revisions() {
        assert_eq!(normalize_version_tag("v2026.7.9"), vec![2026, 7, 9, 0]);
        assert_eq!(normalize_version_tag("2026.7.9-2"), vec![2026, 7, 9, 2]);
        assert_eq!(normalize_version_tag("v1.2.3+beta"), vec![1, 2, 3, 0]);
        assert_eq!(normalize_version_tag("v1.2.3-preview"), vec![1, 2, 3, 0]);
    }

    #[test]
    fn same_day_revision_ordering_is_numeric() {
        assert!(is_remote_version_newer("2026.7.13", "v2026.7.13-1"));
        assert!(is_remote_version_newer("2026.7.13-1", "v2026.7.13-2"));
        assert!(!is_remote_version_newer("2026.7.13-2", "v2026.7.13-1"));
    }
}
