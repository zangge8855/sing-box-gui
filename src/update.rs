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
    let (core, revision, pre_release_num) = match without_build.rsplit_once('-') {
        Some((core, suffix)) if suffix.chars().all(|c| c.is_ascii_digit()) => {
            (core, suffix.parse::<u64>().unwrap_or(0), None)
        }
        Some((core, suffix)) => (core, 0, Some(extract_trailing_number(suffix))),
        _ => (without_build, 0, None),
    };

    let mut parts: Vec<u64> = core
        .split('.')
        .filter_map(|p| p.parse::<u64>().ok())
        .collect();
    parts.push(revision);
    if let Some(num) = pre_release_num {
        parts.push(num);
    }
    parts
}

pub fn is_remote_version_newer(local_pkg_version: &str, remote_tag: &str) -> bool {
    let local = normalize_version_tag(&format!("v{}", local_pkg_version));
    let remote = normalize_version_tag(remote_tag);
    if local.is_empty() || remote.is_empty() {
        return remote_tag.trim().trim_start_matches('v')
            != local_pkg_version.trim().trim_start_matches('v');
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
}
