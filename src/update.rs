pub fn normalize_version_tag(tag: &str) -> Vec<u64> {
    tag.trim()
        .trim_start_matches('v')
        .split('.')
        .filter_map(|p| p.split(|c: char| !c.is_ascii_digit()).next())
        .filter(|p| !p.is_empty())
        .filter_map(|p| p.parse::<u64>().ok())
        .collect()
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
    fn normalize_version_tag_strips_v_and_drops_suffix() {
        assert_eq!(normalize_version_tag("v2026.7.9"), vec![2026, 7, 9]);
        assert_eq!(normalize_version_tag("2026.7.9"), vec![2026, 7, 9]);
        // Non-numeric tail like "+beta" is dropped, the numeric head kept.
        assert_eq!(normalize_version_tag("v1.2.3+beta"), vec![1, 2, 3]);
    }
}
