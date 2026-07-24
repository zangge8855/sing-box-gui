fn extract_trailing_number(s: &str) -> u64 {
    let mut num_str = String::new();
    for c in s.chars().rev() {
        if c.is_ascii_digit() {
            num_str.insert(0, c);
        } else {
            break;
        }
    }
    num_str.parse::<u64>().unwrap_or(0)
}

pub fn normalize_version_tag(tag: &str) -> Vec<u64> {
    let normalized = tag.trim().trim_start_matches('v');
    let without_build = normalized.split('+').next().unwrap_or(normalized);
    let (core, revision, pre_release_num) = match without_build.split_once('-') {
        Some((core, suffix)) if suffix.chars().all(|c| c.is_ascii_digit()) => {
            (core, suffix.parse::<u64>().unwrap_or(0), None)
        }
        Some((core, suffix)) => (core, 0, Some(extract_trailing_number(suffix))),
        _ => (without_build, 0, None),
    };

    let parts: Vec<u64> = core
        .split('.')
        .filter_map(|p| p.parse::<u64>().ok())
        .collect();

    if parts.is_empty() {
        return Vec::new();
    }

    let mut result = parts;
    result.push(revision);
    if let Some(num) = pre_release_num {
        result.push(num);
    }
    result
}

pub fn is_remote_version_newer(local_pkg_version: &str, remote_tag: &str) -> bool {
    let local = normalize_version_tag(local_pkg_version);
    let remote = normalize_version_tag(remote_tag);
    if local.is_empty() || remote.is_empty() {
        return false;
    }

    // Compare core + revision components (first 4 items)
    for i in 0..4 {
        let l = local.get(i).copied().unwrap_or(0);
        let r = remote.get(i).copied().unwrap_or(0);
        match r.cmp(&l) {
            std::cmp::Ordering::Greater => return true,
            std::cmp::Ordering::Less => return false,
            std::cmp::Ordering::Equal => continue,
        }
    }

    // Compare pre-release tag (5th item if present, where absent/release = u64::MAX)
    let l_pre = if local.len() > 4 { local[4] } else { u64::MAX };
    let r_pre = if remote.len() > 4 {
        remote[4]
    } else {
        u64::MAX
    };
    r_pre > l_pre
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_version_tag_supports_same_day_revisions() {
        assert_eq!(normalize_version_tag("v2026.7.9"), vec![2026, 7, 9, 0]);
        assert_eq!(normalize_version_tag("2026.7.9-2"), vec![2026, 7, 9, 2]);
        assert_eq!(normalize_version_tag("v1.2.3+beta"), vec![1, 2, 3, 0]);
        assert_eq!(normalize_version_tag("v1.2.3-preview"), vec![1, 2, 3, 0, 0]);
        assert_eq!(normalize_version_tag("v1.0.0-alpha.1"), vec![1, 0, 0, 0, 1]);
        assert_eq!(normalize_version_tag("v1.0.0-alpha-1"), vec![1, 0, 0, 0, 1]);
        assert_eq!(normalize_version_tag("custom"), Vec::<u64>::new());
    }

    #[test]
    fn same_day_revision_ordering_is_numeric() {
        assert!(is_remote_version_newer("2026.7.13", "v2026.7.13-1"));
        assert!(is_remote_version_newer("2026.7.13-1", "v2026.7.13-2"));
        assert!(!is_remote_version_newer("2026.7.13-2", "v2026.7.13-1"));
    }

    #[test]
    fn test_pre_release_ordering() {
        // remote is beta/rc vs local release
        assert!(!is_remote_version_newer("1.2.3", "v1.2.3-rc1"));
        assert!(is_remote_version_newer("1.2.3-rc1", "v1.2.3"));
        // remote is rc2 vs local rc1
        assert!(is_remote_version_newer("1.2.3-rc1", "v1.2.3-rc2"));
        assert!(!is_remote_version_newer("1.2.3-rc2", "v1.2.3-rc1"));
    }

    #[test]
    fn test_is_remote_version_newer_edge_cases() {
        assert!(!is_remote_version_newer("2026.7.24", "2026.7.24"));
        assert!(!is_remote_version_newer("v2026.7.24", "2026.7.24"));
        assert!(is_remote_version_newer("2026.7.23", "v2026.7.24"));
        assert!(is_remote_version_newer("v1.0.0-alpha.1", "1.0.0"));
        assert!(!is_remote_version_newer("1.0.0", "v1.0.0-alpha.1"));
        assert!(!is_remote_version_newer("custom", "custom"));
    }

    #[test]
    fn test_adversarial_version_stress_cases() {
        // 1. Identical versions
        assert!(!is_remote_version_newer("2026.7.24", "2026.7.24"));
        assert!(!is_remote_version_newer("v2026.7.24", "2026.7.24"));
        assert!(!is_remote_version_newer("2026.7.24", "v2026.7.24"));
        assert!(!is_remote_version_newer("v2026.7.24", "v2026.7.24"));
        assert!(!is_remote_version_newer("1.0.0", "1.0.0"));

        // 2. Newer versions
        assert!(is_remote_version_newer("2026.7.23", "2026.7.24"));
        assert!(is_remote_version_newer("1.0.0", "1.0.1"));
        assert!(is_remote_version_newer("1.0.0", "1.1.0"));
        assert!(is_remote_version_newer("1.0.0", "2.0.0"));
        assert!(is_remote_version_newer("2026.7.24", "2026.7.24-1"));

        // 3. Older versions
        assert!(!is_remote_version_newer("2026.7.24", "2026.7.23"));
        assert!(!is_remote_version_newer("1.0.1", "1.0.0"));
        assert!(!is_remote_version_newer("2.0.0", "1.9.9"));
        assert!(!is_remote_version_newer("2026.7.24-2", "2026.7.24-1"));

        // 4. Pre-release versions
        assert!(is_remote_version_newer("v1.0.0-alpha.1", "1.0.0"));
        assert!(!is_remote_version_newer("1.0.0", "v1.0.0-beta.2"));
        assert!(is_remote_version_newer("v1.0.0-alpha.1", "v1.0.0-alpha.2"));
        assert!(!is_remote_version_newer("v1.0.0-beta.2", "v1.0.0-alpha.1"));

        // 5. Non-numeric / malformed strings
        assert!(!is_remote_version_newer("", ""));
        assert!(!is_remote_version_newer("custom", "custom"));
        assert!(!is_remote_version_newer("latest", "2026.7.24"));
        assert!(!is_remote_version_newer("2026.7.24", "latest"));
        assert!(!is_remote_version_newer("v", "1.0.0"));
        assert!(!is_remote_version_newer("1.0.0", "v"));
        assert!(!is_remote_version_newer("1.0.0.0.0", "1.0.0.0.0"));
        assert!(!is_remote_version_newer("invalid.version", "another.invalid"));
        assert!(!is_remote_version_newer("", "2026.7.24"));
        assert!(!is_remote_version_newer("2026.7.24", ""));

        // Whitespace handling
        assert!(is_remote_version_newer(" 2026.7.23 ", " 2026.7.24 "));

        // Direct normalize_version_tag checks
        assert_eq!(normalize_version_tag(""), Vec::<u64>::new());
        assert_eq!(normalize_version_tag("custom"), Vec::<u64>::new());
        assert_eq!(normalize_version_tag("latest"), Vec::<u64>::new());
        assert_eq!(normalize_version_tag("v"), Vec::<u64>::new());
        assert_eq!(normalize_version_tag("1.0.0.0.0"), vec![1, 0, 0, 0, 0, 0]);
        assert_eq!(normalize_version_tag("2026.7.24"), vec![2026, 7, 24, 0]);
        assert_eq!(normalize_version_tag("v2026.7.24"), vec![2026, 7, 24, 0]);
    }
}
