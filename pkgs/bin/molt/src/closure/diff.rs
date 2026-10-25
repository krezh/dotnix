use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

use crate::report::{Change, ChangeStatus};

use super::name::{compact_version, compact_versions, split_store_name};
use super::store::{StorePath, query_closures};
use super::version::compare_versions;

pub(crate) struct ClosureDiff {
    pub(crate) changes: Vec<Change>,
    pub(crate) paths: (i64, i64, i64, i64),
    pub(crate) sizes: (i64, i64),
}

pub(crate) fn query_diff(old: &Path, new: &Path) -> Result<ClosureDiff, String> {
    let (old, new) = query_closures(old, new)?;
    diff_closures(&old, &new)
}

#[derive(Default)]
struct Package<'a> {
    versions: BTreeSet<String>,
    outputs: BTreeMap<&'a str, BTreeSet<&'a str>>,
    size: i64,
}

fn packages(paths: &[StorePath]) -> BTreeMap<&str, Package<'_>> {
    let mut packages = BTreeMap::<&str, Package<'_>>::new();
    for path in paths {
        let (name, version) = split_store_name(&path.path);
        let package = packages.entry(name).or_default();
        let outputs = package.outputs.entry(version).or_default();
        if outputs.is_empty() {
            package.versions.insert(compact_version(version));
        }
        outputs.insert(path.path.as_str());
        package.size += path.nar_size;
    }
    packages
}

fn diff_closures(old: &[StorePath], new: &[StorePath]) -> Result<ClosureDiff, String> {
    let count = |count: usize| i64::try_from(count).unwrap_or(i64::MAX);
    let old_paths = old
        .iter()
        .map(|path| path.path.as_str())
        .collect::<HashSet<_>>();
    let new_paths = new
        .iter()
        .map(|path| path.path.as_str())
        .collect::<HashSet<_>>();
    let paths = (
        count(old_paths.len()),
        count(new_paths.len()),
        count(new_paths.difference(&old_paths).count()),
        count(old_paths.difference(&new_paths).count()),
    );
    let sizes = (
        old.iter().map(|path| path.nar_size).sum(),
        new.iter().map(|path| path.nar_size).sum(),
    );
    let old = packages(old);
    let mut new = packages(new);
    let mut changes = Vec::new();
    for (name, old) in old {
        if let Some(change) = package_change(name, Some(&old), new.remove(name).as_ref())? {
            changes.push(change);
        }
    }
    for (name, new) in new {
        if let Some(change) = package_change(name, None, Some(&new))? {
            changes.push(change);
        }
    }
    changes.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    Ok(ClosureDiff {
        changes,
        paths,
        sizes,
    })
}

fn shortest(versions: &BTreeSet<String>) -> String {
    versions
        .iter()
        .min_by(|left, right| left.len().cmp(&right.len()).then_with(|| left.cmp(right)))
        .cloned()
        .unwrap_or_default()
}

fn package_change(
    name: &str,
    old: Option<&Package<'_>>,
    new: Option<&Package<'_>>,
) -> Result<Option<Change>, String> {
    if name.starts_with("nixos-system-") {
        return Ok(None);
    }
    let size = new.map_or(0, |package| package.size) - old.map_or(0, |package| package.size);
    let (status, old, new) = match (old, new) {
        (None, Some(new)) => (ChangeStatus::Added, String::new(), shortest(&new.versions)),
        (Some(old), None) => (ChangeStatus::Removed, shortest(&old.versions), String::new()),
        (Some(old), Some(new)) if old.versions != new.versions => version_change(old, new),
        (Some(old), Some(new)) => {
            let paths_changed = old.outputs != new.outputs;
            let multiplicity_only = paths_changed
                && old.outputs.keys().eq(new.outputs.keys())
                && old.outputs.iter().all(|(version, paths)| {
                    let new_paths = &new.outputs[version];
                    paths.is_subset(new_paths) || new_paths.is_subset(paths)
                });
            let rebuilt_package = paths_changed && !old.versions.contains("");
            if multiplicity_only || (!rebuilt_package && size.unsigned_abs() <= 8192) {
                return Ok(None);
            }
            (ChangeStatus::Changed, String::new(), String::new())
        }
        (None, None) => return Ok(None),
    };
    if status.is_paired() && old.is_empty() && new.is_empty() {
        return Err(format!(
            "closure diff returned {} without version data for {name}",
            status.label()
        ));
    }
    Ok(Some(Change {
        status,
        name: name.to_owned(),
        old,
        new,
        size,
    }))
}

fn version_change(old: &Package<'_>, new: &Package<'_>) -> (ChangeStatus, String, String) {
    // Retained versions supply the counterpart for one-sided version changes.
    let removed = old.versions.difference(&new.versions).next().is_some();
    let added = new.versions.difference(&old.versions).next().is_some();
    let mut direction = None;
    let mut mixed = false;
    let mut representative: Option<(&str, &str)> = None;
    for old_version in &old.versions {
        if removed && new.versions.contains(old_version) {
            continue;
        }
        for new_version in &new.versions {
            if added && old.versions.contains(new_version) {
                continue;
            }
            let ordering = compare_versions(old_version, new_version);
            if old_version.is_empty()
                || new_version.is_empty()
                || ordering.is_eq()
                || direction.is_some_and(|direction| direction != ordering)
            {
                mixed = true;
            }
            direction = Some(ordering);
            let pair = (old_version.as_str(), new_version.as_str());
            if representative.is_none_or(|(old, new)| {
                (pair.0.len() + pair.1.len(), pair.0, pair.1) < (old.len() + new.len(), old, new)
            }) {
                representative = Some(pair);
            }
        }
    }
    let status = match (mixed, direction) {
        (false, Some(Ordering::Less)) => ChangeStatus::Upgraded,
        (false, Some(Ordering::Greater)) => ChangeStatus::Downgraded,
        _ => ChangeStatus::Changed,
    };
    let (old, new) = representative
        .map(|(old, new)| compact_versions(old, new))
        .unwrap_or_default();
    (status, old, new)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_path(hash: usize, name: &str, size: i64) -> StorePath {
        StorePath {
            path: format!("/nix/store/{hash:032x}-{name}"),
            nar_size: size,
        }
    }

    fn closure(paths: &[(&str, i64)]) -> Vec<StorePath> {
        paths
            .iter()
            .enumerate()
            .map(|(index, (name, size))| store_path(index, name, *size))
            .collect()
    }

    fn compare(old: &[(&str, i64)], new: &[(&str, i64)]) -> Vec<Change> {
        diff_closures(&closure(old), &closure(new)).unwrap().changes
    }

    fn assert_change(
        change: &Change,
        name: &str,
        status: ChangeStatus,
        old: &str,
        new: &str,
        size: i64,
    ) {
        assert_eq!(change.name, name);
        assert_eq!(change.status, status);
        assert_eq!(change.old, old);
        assert_eq!(change.new, new);
        assert_eq!(change.size, size);
    }

    #[test]
    fn compacts_output_suffixes_on_upgrades() {
        let changes = compare(&[("demo-1.0-bin", 20)], &[("demo-2.0-bin", 30)]);
        assert_eq!(changes.len(), 1);
        assert_change(&changes[0], "demo", ChangeStatus::Upgraded, "1.0", "2.0", 10);
        let changes = compare(&[("demo-1.2-qt_wrapper", 0)], &[("demo-1.3-qt_wrapper", 0)]);
        assert_change(&changes[0], "demo", ChangeStatus::Upgraded, "1.2", "1.3", 0);
    }

    #[test]
    fn keeps_versionless_entries_without_fabricated_versions() {
        let changes = compare(&[], &[("slumber-config.toml", 1)]);
        assert_change(
            &changes[0],
            "slumber-config.toml",
            ChangeStatus::Added,
            "",
            "",
            1,
        );
        let changes = compare(&[("slumber-config.toml", 1)], &[]);
        assert_change(
            &changes[0],
            "slumber-config.toml",
            ChangeStatus::Removed,
            "",
            "",
            -1,
        );
        let changes = compare(&[("slumber-config.toml", 1)], &[("slumber-config.toml", 8194)]);
        assert_change(
            &changes[0],
            "slumber-config.toml",
            ChangeStatus::Changed,
            "",
            "",
            8193,
        );
    }

    #[test]
    fn reports_every_status_in_package_order() {
        let changes = compare(
            &[
                ("b-1", 2),
                ("c-1", 1),
                ("d-2", 5),
                ("e-1", 1),
                ("nixos-system-host-26.05", 100),
            ],
            &[
                ("a-1", 1),
                ("c-2", 4),
                ("d-1", 1),
                ("e-1", 8194),
                ("nixos-system-host-26.11", 200),
            ],
        );
        assert_eq!(changes.len(), 5);
        assert_change(&changes[0], "a", ChangeStatus::Added, "", "1", 1);
        assert_change(&changes[1], "b", ChangeStatus::Removed, "1", "", -2);
        assert_change(&changes[2], "c", ChangeStatus::Upgraded, "1", "2", 3);
        assert_change(&changes[3], "d", ChangeStatus::Downgraded, "2", "1", -4);
        assert_change(&changes[4], "e", ChangeStatus::Changed, "", "", 8193);
    }

    #[test]
    fn dhcpcd_output_changes_report_size_without_versions() {
        let changes = compare(
            &[("dhcpcd-10.3.2", 1_000_000), ("dhcpcd-10.3.2", 1_000_000)],
            &[
                ("dhcpcd-10.3.2", 1_000_000),
                ("dhcpcd-10.3.2_fish-completions", 488_464),
            ],
        );
        assert_eq!(changes.len(), 1);
        assert_change(&changes[0], "dhcpcd", ChangeStatus::Changed, "", "", -511_536);
    }

    #[test]
    fn util_linux_output_changes_report_size_without_versions() {
        let changes = compare(
            &[("util-linux-2.42.3-dev", 10), ("util-linux-2.42.3", 10)],
            &[("util-linux-2.42.3-man", 141_724)],
        );
        assert_eq!(changes.len(), 1);
        assert_change(
            &changes[0],
            "util-linux",
            ChangeStatus::Changed,
            "",
            "",
            141_704,
        );
    }

    #[test]
    fn abseil_cpp_output_changes_report_size_without_versions() {
        let changes = compare(
            &[
                ("abseil-cpp-20260107.1-dev", 7_721_032),
                ("abseil-cpp-20260107.1", 10),
                ("abseil-cpp-20260107.1", 10),
                ("abseil-cpp-20260107.1", 10),
            ],
            &[("abseil-cpp-20260107.1", 15), ("abseil-cpp-20260107.1", 15)],
        );
        assert_eq!(changes.len(), 1);
        assert_change(
            &changes[0],
            "abseil-cpp",
            ChangeStatus::Changed,
            "",
            "",
            -7_721_032,
        );
    }

    #[test]
    fn ignores_multiplicity_only_changes_even_when_sizes_move() {
        assert!(
            compare(
                &[("X-Restart-Triggers-dbus-broker-1", 1)],
                &[
                    ("X-Restart-Triggers-dbus-broker-1", 1),
                    ("X-Restart-Triggers-dbus-broker-1", 20_000),
                ],
            )
            .is_empty()
        );
        assert!(
            compare(
                &[("demo-1", 1), ("demo-1", 20_000)],
                &[("demo-1", 1)],
            )
            .is_empty()
        );
    }

    #[test]
    fn compares_removed_versions_with_known_remaining_versions() {
        let changes = compare(
            &[("graphics-drivers-570.1", 10), ("graphics-drivers-565.2", 20)],
            &[("graphics-drivers-565.2", 20), ("graphics-drivers-565.2", 20)],
        );
        assert_change(
            &changes[0],
            "graphics-drivers",
            ChangeStatus::Downgraded,
            "570.1",
            "565.2",
            10,
        );
        let changes = compare(
            &[("chomp-0.1.0", 10), ("chomp-0.0.9", 20)],
            &[("chomp-0.0.9", 20)],
        );
        assert_change(
            &changes[0],
            "chomp",
            ChangeStatus::Downgraded,
            "0.1.0",
            "0.0.9",
            -10,
        );
    }

    #[test]
    fn requires_unambiguous_direction_for_multiple_versions() {
        for (old, new, status) in [
            (
                &["demo-1", "demo-4"][..],
                &["demo-2", "demo-3"][..],
                ChangeStatus::Changed,
            ),
            (
                &["demo-1", "demo-2"][..],
                &["demo-3", "demo-4"][..],
                ChangeStatus::Upgraded,
            ),
            (
                &["demo-3", "demo-4"][..],
                &["demo-1", "demo-2"][..],
                ChangeStatus::Downgraded,
            ),
            (
                &["demo-1"][..],
                &["demo-1", "demo-2"][..],
                ChangeStatus::Upgraded,
            ),
            (
                &["demo-1", "demo-2"][..],
                &["demo-1"][..],
                ChangeStatus::Downgraded,
            ),
            (
                &["demo-1", "demo-3"][..],
                &["demo-1", "demo-2", "demo-3"][..],
                ChangeStatus::Changed,
            ),
            (
                &["demo-1", "demo-2", "demo-3"][..],
                &["demo-1", "demo-3"][..],
                ChangeStatus::Changed,
            ),
        ] {
            let old = old.iter().map(|name| (*name, 1)).collect::<Vec<_>>();
            let new = new.iter().map(|name| (*name, 1)).collect::<Vec<_>>();
            let changes = compare(&old, &new);
            assert_eq!(changes.len(), 1);
            assert_eq!(changes[0].status, status);
        }
    }

    #[test]
    fn uses_nix_ordering_and_keeps_equal_or_versionless_changes_unpaired() {
        for (old, new) in [
            ("demo-1.0pre1", "demo-1.0"),
            ("demo-1.2.9", "demo-1.2.10"),
            ("demo-1.0", "demo-1.0.1"),
            ("demo-2.4a", "demo-2.4b"),
        ] {
            assert_eq!(
                compare(&[(old, 1)], &[(new, 1)])[0].status,
                ChangeStatus::Upgraded
            );
        }
        assert_eq!(
            compare(&[("demo-01", 1)], &[("demo-1", 1)])[0].status,
            ChangeStatus::Changed
        );
        assert_eq!(
            compare(&[("demo", 1)], &[("demo-1", 1)])[0].status,
            ChangeStatus::Changed
        );
    }

    #[test]
    fn chooses_the_shortest_representative_pair() {
        let changes = compare(
            &[("demo-1.000", 1), ("demo-2", 1)],
            &[("demo-3.000", 1), ("demo-4", 1)],
        );
        assert_change(&changes[0], "demo", ChangeStatus::Upgraded, "2", "4", 0);
    }

    #[test]
    fn reports_same_version_store_path_replacements_at_any_size_delta() {
        let old = [store_path(0, "demo-1.0", 20_000)];
        for delta in [0, 1, -1, 8192, -8192] {
            let new = [store_path(1, "demo-1.0", 20_000 + delta)];
            let diff = diff_closures(&old, &new).unwrap();
            assert_eq!(diff.changes.len(), 1);
            assert_change(&diff.changes[0], "demo", ChangeStatus::Changed, "", "", delta);
            assert_eq!(diff.paths, (1, 1, 1, 1));
        }
    }

    #[test]
    fn leaves_versionless_rebuilt_files_to_configuration_diff() {
        let old = [store_path(0, "slumber-config.toml", 20_000)];
        for delta in [0, 1, -1, 8192, -8192] {
            let new = [store_path(1, "slumber-config.toml", 20_000 + delta)];
            assert!(diff_closures(&old, &new).unwrap().changes.is_empty());
        }
        let new = [store_path(1, "slumber-config.toml", 28_193)];
        let diff = diff_closures(&old, &new).unwrap();
        assert_eq!(diff.changes.len(), 1);
        assert_change(
            &diff.changes[0],
            "slumber-config.toml",
            ChangeStatus::Changed,
            "",
            "",
            8193,
        );
    }

    #[test]
    fn multiplicity_changes_do_not_hide_rebuilt_paths() {
        let old = [
            store_path(0, "demo-1.0", 100),
            store_path(1, "demo-1.0", 100),
        ];
        let new = [
            store_path(0, "demo-1.0", 100),
            store_path(2, "demo-1.0", 100),
            store_path(3, "demo-1.0", 100),
        ];
        for (old, new, delta) in [(&old[..], &new[..], 100), (&new[..], &old[..], -100)] {
            let diff = diff_closures(old, new).unwrap();
            assert_eq!(diff.changes.len(), 1);
            assert_change(&diff.changes[0], "demo", ChangeStatus::Changed, "", "", delta);
        }
    }

    #[test]
    fn ignores_pure_multiplicity_changes_across_versions() {
        let old = [
            store_path(0, "demo-1.0", 10),
            store_path(1, "demo-1.0", 20_000),
            store_path(2, "demo-2.0", 10),
        ];
        let new = [
            store_path(0, "demo-1.0", 10),
            store_path(2, "demo-2.0", 10),
            store_path(3, "demo-2.0", 40_000),
        ];
        assert!(diff_closures(&old, &new).unwrap().changes.is_empty());
        assert!(diff_closures(&new, &old).unwrap().changes.is_empty());
    }

    #[test]
    fn reports_output_composition_changes_without_a_size_delta() {
        let changes = compare(&[("demo-1.0-bin", 100)], &[("demo-1.0-lib", 100)]);
        assert_eq!(changes.len(), 1);
        assert_change(&changes[0], "demo", ChangeStatus::Changed, "", "", 0);
    }

    #[test]
    fn size_only_threshold_is_strict_in_both_directions() {
        for delta in [8192, -8192] {
            assert!(compare(&[("demo-1", 20_000)], &[("demo-1", 20_000 + delta)]).is_empty());
        }
        for delta in [8193, -8193] {
            let changes = compare(&[("demo-1", 20_000)], &[("demo-1", 20_000 + delta)]);
            assert_change(&changes[0], "demo", ChangeStatus::Changed, "", "", delta);
        }
    }

    #[test]
    fn statistics_include_unchanged_and_filtered_paths() {
        let old = closure(&[
            ("shared-1", 10),
            ("removed-1", 20),
            ("nixos-system-host-26.05", 30),
        ]);
        let new = closure(&[
            ("shared-1", 10),
            ("added-1", 40),
            ("nixos-system-host-26.11", 50),
        ]);
        let diff = diff_closures(&old, &new).unwrap();
        assert_eq!(diff.paths, (3, 3, 2, 2));
        assert_eq!(diff.sizes, (60, 100));
        assert_eq!(diff.changes.len(), 2);
        assert_eq!(diff.changes[0].name, "added");
        assert_eq!(diff.changes[1].name, "removed");
        let same = diff_closures(&old, &old).unwrap();
        assert_eq!(same.paths, (3, 3, 0, 0));
        assert_eq!(same.sizes, (60, 60));
        assert!(same.changes.is_empty());
    }
}
