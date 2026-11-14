#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PreReleaseKind {
    Dev = 0,
    Other = 1,
    Alpha = 2,
    Beta = 3,
    Rc = 4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedVersion {
    core: Vec<u64>,
    revision: u64,
    pre: Option<(PreReleaseKind, Vec<u64>, String)>,
}

#[allow(dead_code)]
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

fn parse_pre_release(suffix: &str) -> (PreReleaseKind, Vec<u64>, String) {
    let lower = suffix.trim().to_ascii_lowercase();
    let kind = if lower.starts_with("dev")
        || lower.starts_with("nightly")
        || lower.starts_with("snapshot")
    {
        PreReleaseKind::Dev
    } else if lower.starts_with("alpha") {
        PreReleaseKind::Alpha
    } else if lower.starts_with("beta") {
        PreReleaseKind::Beta
    } else if lower.starts_with("rc") {
        PreReleaseKind::Rc
    } else {
        PreReleaseKind::Other
    };

    // Split numeric runs out of multi-part tags such as "alpha.1.2" or "rc-1"
    // so multi-segment pre-releases compare numerically segment by segment.
    let mut nums = Vec::new();
    let mut cur = String::new();
    for ch in lower.chars() {
        if ch.is_ascii_digit() {
            cur.push(ch);
        } else if !cur.is_empty() {
            if let Ok(v) = cur.parse::<u64>() {
                nums.push(v);
            }
            cur.clear();
        }
    }
    if !cur.is_empty()
        && let Ok(v) = cur.parse::<u64>()
    {
        nums.push(v);
    }
    if nums.is_empty() {
        nums.push(0);
    }

    (kind, nums, lower)
}

fn parse_version(tag: &str) -> Option<ParsedVersion> {
    let normalized = tag.trim().trim_start_matches(['v', 'V']);
    let without_build = normalized.split('+').next().unwrap_or(normalized);
    let (core_str, revision, pre_str) = match without_build.split_once('-') {
        Some((core, suffix)) => {
            if let Some((rev_part, rest)) = suffix.split_once('-') {
                if !rev_part.is_empty() && rev_part.chars().all(|c| c.is_ascii_digit()) {
                    (core, rev_part.parse::<u64>().unwrap_or(0), Some(rest))
                } else {
                    (core, 0, Some(suffix))
                }
            } else if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
                (core, suffix.parse::<u64>().unwrap_or(0), None)
            } else {
                (core, 0, Some(suffix))
            }
        }
        None => (without_build, 0, None),
    };

    if core_str.is_empty() {
        return None;
    }

    let mut core = Vec::new();
    for part in core_str.split('.') {
        let Ok(num) = part.parse::<u64>() else {
            return None;
        };
        core.push(num);
    }
    if core.is_empty() {
        return None;
    }

    let pre = pre_str.map(parse_pre_release);
    Some(ParsedVersion {
        core,
        revision,
        pre,
    })
}

fn cmp_numeric_slices(a: &[u64], b: &[u64]) -> std::cmp::Ordering {
    let len = a.len().max(b.len());
    for i in 0..len {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        match av.cmp(&bv) {
            std::cmp::Ordering::Equal => continue,
            ord => return ord,
        }
    }
    std::cmp::Ordering::Equal
}

#[allow(dead_code)]
pub fn normalize_version_tag(tag: &str) -> Vec<u64> {
    let Some(parsed) = parse_version(tag) else {
        return Vec::new();
    };
    let mut result = parsed.core;
    result.push(parsed.revision);
    if let Some((_, nums, _)) = parsed.pre {
        result.push(
            nums.last()
                .copied()
                .unwrap_or_else(|| extract_trailing_number(tag)),
        );
    }
    result
}

pub fn is_remote_version_newer(local_pkg_version: &str, remote_tag: &str) -> bool {
    let (Some(local), Some(remote)) = (parse_version(local_pkg_version), parse_version(remote_tag))
    else {
        return false;
    };

    match cmp_numeric_slices(&remote.core, &local.core) {
        std::cmp::Ordering::Greater => return true,
        std::cmp::Ordering::Less => return false,
        std::cmp::Ordering::Equal => {}
    }

    match remote.revision.cmp(&local.revision) {
        std::cmp::Ordering::Greater => return true,
        std::cmp::Ordering::Less => return false,
        std::cmp::Ordering::Equal => {}
    }

    match (&local.pre, &remote.pre) {
        (None, None) => false,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (Some((l_kind, l_nums, l_raw)), Some((r_kind, r_nums, r_raw))) => {
            match r_kind.cmp(l_kind) {
                std::cmp::Ordering::Greater => return true,
                std::cmp::Ordering::Less => return false,
                std::cmp::Ordering::Equal => {}
            }
            match cmp_numeric_slices(r_nums, l_nums) {
                std::cmp::Ordering::Greater => true,
                std::cmp::Ordering::Less => false,
                std::cmp::Ordering::Equal => r_raw > l_raw,
            }
        }
    }
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
        assert!(!is_remote_version_newer(
            "invalid.version",
            "another.invalid"
        ));
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

    #[test]
    fn four_part_versions_are_not_prereleases() {
        assert!(!is_remote_version_newer("1.2.3.4", "1.2.3.4"));
        assert!(is_remote_version_newer("1.2.3.4", "1.2.3.5"));
        assert!(!is_remote_version_newer("1.2.3.5", "1.2.3.4"));
        assert!(!is_remote_version_newer("1.2.3.4", "1.2.3.4-rc1"));
        assert!(is_remote_version_newer("1.2.3.4-rc1", "1.2.3.4"));
        assert!(is_remote_version_newer("1.2.3.4-rc1", "1.2.3.4-rc2"));
        assert!(is_remote_version_newer("1.2.3.4", "1.2.3.4-1"));
        assert!(!is_remote_version_newer("1.2.3.4-1", "1.2.3.4"));
    }

    #[test]
    fn prerelease_stage_precedence_and_multi_hyphen_tags() {
        // alpha < beta < rc regardless of trailing counter
        assert!(is_remote_version_newer("1.2.3-alpha.9", "1.2.3-beta.1"));
        assert!(is_remote_version_newer("1.2.3-beta.2", "1.2.3-rc1"));
        assert!(!is_remote_version_newer("1.2.3-rc1", "1.2.3-beta.9"));
        assert!(!is_remote_version_newer("1.2.3-beta.1", "1.2.3-alpha.9"));
        assert!(is_remote_version_newer("1.2.3-dev.5", "1.2.3-alpha.1"));

        // Multi-hyphen revision + prerelease
        assert!(is_remote_version_newer(
            "2026.7.13-1-beta.2",
            "2026.7.13-1-rc.1"
        ));
        assert!(is_remote_version_newer("2026.7.13-1-rc.1", "2026.7.13-1"));
        assert!(!is_remote_version_newer("2026.7.13-1", "2026.7.13-1-rc.1"));
        assert!(is_remote_version_newer("2026.7.13", "2026.7.13-1-alpha.1"));
        assert!(!is_remote_version_newer("1.0.x", "1.0.1"));
    }
}
