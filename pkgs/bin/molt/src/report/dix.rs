//! Converting dix's closure diff into a `Report`, including how package versions are paired
//! and compacted.

use std::sync::Arc;

use ::dix::{DiffReport, DiffStatus, PackageDiff, Version, VersionDiff};

use super::{Change, ChangeStatus, Report, ReportMetadata};

pub(crate) fn report_from_diff(
    metadata: ReportMetadata,
    report: &DiffReport,
) -> Result<Report, String> {
    let ReportMetadata {
        target,
        flake,
        flake_dir,
        baseline,
        profile,
        output,
        gc_root,
    } = metadata;
    let count = |count: usize| i64::try_from(count).unwrap_or(i64::MAX);
    let paths = report.path_stats();
    Ok(Report {
        target,
        flake,
        flake_dir,
        baseline,
        profile,
        output,
        _gc_root: gc_root,
        changes: changes_from_diffs(report.diffs())?,
        paths: Some((
            count(paths.old_count()),
            count(paths.new_count()),
            count(paths.added_count()),
            count(paths.removed_count()),
        )),
        sizes: Some((report.size_old().bytes(), report.size_new().bytes())),
        operational_impact: Ok(Vec::new()),
        config: Ok(Arc::default()),
        built_in: None,
    })
}

fn changes_from_diffs(diffs: &[PackageDiff]) -> Result<Vec<Change>, String> {
    let mut changes = Vec::new();
    for diff in diffs
        .iter()
        .filter(|diff| !diff.name.starts_with("nixos-system-"))
    {
        let mixed = diff.status == DiffStatus::Mixed;
        let mut status = match diff.status {
            DiffStatus::Added => ChangeStatus::Added,
            DiffStatus::Removed => ChangeStatus::Removed,
            DiffStatus::Upgraded => ChangeStatus::Upgraded,
            DiffStatus::Downgraded => ChangeStatus::Downgraded,
            DiffStatus::Changed | DiffStatus::Mixed => ChangeStatus::Changed,
        };
        let Some((mut old, mut new)) = dix_version_change(
            status,
            &diff.name,
            &diff.versions,
            diff.has_omitted_versions,
        )?
        else {
            continue;
        };
        if status.is_paired() && old == new {
            status = ChangeStatus::Changed;
            old.clear();
        }
        if mixed {
            old = compact_dix_version(&old);
            new = compact_dix_version(&new);
            if old == new {
                old.clear();
            }
        }
        changes.push(Change {
            status,
            name: diff.name.clone(),
            old,
            new,
            size: diff.size.delta().bytes(),
        });
    }
    Ok(changes)
}

fn dix_version_change(
    status: ChangeStatus,
    package: &str,
    versions: &[VersionDiff],
    has_omitted_versions: bool,
) -> Result<Option<(String, String)>, String> {
    let mut changed = Vec::new();
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut amount_changed = Vec::new();
    for version in versions {
        match version {
            VersionDiff::Changed { old, new } => {
                changed.push((version_name(&old.version), version_name(&new.version)));
            }
            VersionDiff::Added(version) => added.push(version_name(&version.version)),
            VersionDiff::Removed(version) => removed.push(version_name(&version.version)),
            VersionDiff::AmountChanged { version, .. } => {
                amount_changed.push(version_name(version));
            }
        }
    }

    if status == ChangeStatus::Changed
        && !amount_changed.is_empty()
        && changed.is_empty()
        && added.is_empty()
        && removed.is_empty()
    {
        return Ok(None);
    }

    let mut old = String::new();
    let mut new = String::new();
    if let Some((changed_old, changed_new)) = changed
        .into_iter()
        .min_by_key(|(old, new)| old.len() + new.len())
    {
        (old, new) = compact_versions(&changed_old, &changed_new);
        if status == ChangeStatus::Downgraded && compare_dix_versions(&old, &new).is_lt() {
            std::mem::swap(&mut old, &mut new);
        }
    } else if status == ChangeStatus::Downgraded {
        let mut candidates = added
            .iter()
            .chain(&removed)
            .chain(&amount_changed)
            .map(|version| compact_dix_version(version))
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| compare_dix_versions(left, right));
        candidates.dedup();
        if let (Some(lowest), Some(highest)) = (candidates.first(), candidates.last()) {
            if lowest != highest {
                old.clone_from(highest);
                new.clone_from(lowest);
            } else if !has_omitted_versions {
                old.clone_from(lowest);
                new.clone_from(lowest);
            }
        }
    }

    if old.is_empty() {
        old = removed
            .iter()
            .map(|version| compact_dix_version(version))
            .min_by_key(String::len)
            .unwrap_or_default();
    }
    if new.is_empty() {
        new = added
            .iter()
            .map(|version| compact_dix_version(version))
            .min_by_key(String::len)
            .unwrap_or_default();
    }

    if status.is_paired() {
        if has_omitted_versions {
            if old.is_empty() {
                old = "...".to_owned();
            }
            if new.is_empty() {
                new = "...".to_owned();
            }
        } else if old.is_empty() || new.is_empty() {
            let known = if old.is_empty() {
                new.clone()
            } else {
                old.clone()
            };
            if old.is_empty() {
                old.clone_from(&known);
            }
            if new.is_empty() {
                new = known;
            }
        }
    }
    if status.is_paired() && old.is_empty() && new.is_empty() {
        return Err(format!(
            "dix returned {} without version data for {package}",
            status.label()
        ));
    }
    Ok(Some((old, new)))
}

pub(crate) fn compact_dix_version(version: &str) -> String {
    const OUTPUT_SUFFIXES: &[&str] = &[
        "-fish-completions",
        "_fish-completions",
        "-fhsenv-profile",
        "-fhsenv-rootfs",
        "-libgcc",
        "-bwrap",
        "-extracted",
        "-patched",
        "-init",
        "-bin",
        "-dev",
        "-doc",
        "-info",
        "-lib",
        "-man",
        "-out",
        "-static",
        "-debug",
    ];
    OUTPUT_SUFFIXES
        .iter()
        .find_map(|suffix| {
            version
                .strip_suffix(suffix)
                .filter(|base| base.chars().any(|character| character.is_ascii_digit()))
                .map(str::to_owned)
        })
        .unwrap_or_else(|| version.to_owned())
}

fn compare_dix_versions(left: &str, right: &str) -> std::cmp::Ordering {
    let numeric_parts = |version: &str| {
        version
            .split(|character: char| !character.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .map(|part| part.parse::<u64>().unwrap_or(u64::MAX))
            .collect::<Vec<_>>()
    };
    let left_parts = numeric_parts(left);
    let right_parts = numeric_parts(right);
    for index in 0..left_parts.len().max(right_parts.len()) {
        match left_parts
            .get(index)
            .copied()
            .unwrap_or_default()
            .cmp(&right_parts.get(index).copied().unwrap_or_default())
        {
            std::cmp::Ordering::Equal => {}
            ordering => return ordering,
        }
    }
    left.cmp(right)
}

/// dix's version name for a store path without one, such as a generated `foo.toml`.
const DIX_NO_VERSION: &str = "<none>";

fn version_name(version: &Version) -> String {
    if version.name == DIX_NO_VERSION {
        String::new()
    } else {
        version.name.clone()
    }
}

pub(crate) fn compact_versions(old: &str, new: &str) -> (String, String) {
    let old_parts = old.split('-').collect::<Vec<_>>();
    let new_parts = new.split('-').collect::<Vec<_>>();
    let mut suffix = 0;
    while suffix + 1 < old_parts.len().min(new_parts.len()) {
        let old_part = old_parts[old_parts.len() - 1 - suffix];
        let new_part = new_parts[new_parts.len() - 1 - suffix];
        let output_suffix = old_part == new_part
            && old_part.chars().any(char::is_alphabetic)
            && old_part
                .chars()
                .all(|character| character.is_alphanumeric() || matches!(character, '_' | '.'));
        if !output_suffix {
            break;
        }
        suffix += 1;
    }
    if suffix == 0 {
        return (old.to_owned(), new.to_owned());
    }
    (
        old_parts[..old_parts.len() - suffix].join("-"),
        new_parts[..new_parts.len() - suffix].join("-"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroUsize;

    use ::dix::{DerivationSelectionStatus, PackageSizeDelta, VersionAmount};

    fn package(
        name: &str,
        status: DiffStatus,
        size_delta: i64,
        versions: Vec<VersionDiff>,
        has_omitted_versions: bool,
    ) -> PackageDiff {
        PackageDiff {
            name: name.to_owned(),
            versions,
            status,
            selection: DerivationSelectionStatus::Selected,
            has_omitted_versions,
            // Parsing lets dix pick the `size` version, so molt doesn't depend on it.
            size: PackageSizeDelta::new(
                "0".parse().unwrap(),
                size_delta.to_string().parse().unwrap(),
            ),
        }
    }

    fn one(version: &str) -> VersionAmount {
        VersionAmount::new(version, NonZeroUsize::MIN)
    }

    fn changed(old: &str, new: &str) -> VersionDiff {
        VersionDiff::Changed {
            old: one(old),
            new: one(new),
        }
    }

    fn amount_changed(version: &str, old: usize, new: usize) -> VersionDiff {
        VersionDiff::AmountChanged {
            version: version.into(),
            old_amount: NonZeroUsize::new(old).unwrap(),
            new_amount: NonZeroUsize::new(new).unwrap(),
        }
    }

    #[test]
    fn strips_only_shared_output_suffixes() {
        assert_eq!(
            compact_versions("1.6.5-bwrap", "1.8.3-bwrap"),
            ("1.6.5".to_owned(), "1.8.3".to_owned())
        );
        assert_eq!(
            compact_versions("1.2.0-rc1", "1.3.0"),
            ("1.2.0-rc1".to_owned(), "1.3.0".to_owned())
        );
        assert_eq!(compact_dix_version("10.3.2_fish-completions"), "10.3.2");
    }

    #[test]
    fn converts_a_dix_upgrade() {
        let changes = changes_from_diffs(&[package(
            "demo",
            DiffStatus::Upgraded,
            10,
            vec![changed("1.0-bin", "2.0-bin")],
            false,
        )])
        .unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].old, "1.0");
        assert_eq!(changes[0].new, "2.0");
        assert_eq!(changes[0].size, 10);
    }

    #[test]
    fn reads_dix_placeholder_as_no_version() {
        let changes = changes_from_diffs(&[package(
            "slumber-config.toml",
            DiffStatus::Added,
            1,
            vec![VersionDiff::Added(one(DIX_NO_VERSION))],
            false,
        )])
        .unwrap();
        assert!(changes[0].old.is_empty());
        assert!(changes[0].new.is_empty());
    }

    #[test]
    fn converts_every_dix_status() {
        let changes = changes_from_diffs(&[
            package("nixos-system-host-26.05", DiffStatus::Changed, 0, vec![], false),
            package("a", DiffStatus::Added, 1, vec![VersionDiff::Added(one("1"))], false),
            package("b", DiffStatus::Removed, -2, vec![VersionDiff::Removed(one("1"))], false),
            package("c", DiffStatus::Upgraded, 3, vec![changed("1", "2")], false),
            package("d", DiffStatus::Downgraded, -4, vec![changed("2", "1")], false),
            package("e", DiffStatus::Changed, 0, vec![], false),
            package(
                "X-Restart-Triggers-dbus-broker",
                DiffStatus::Changed,
                0,
                vec![amount_changed("1", 1, 2)],
                false,
            ),
            package(
                "chomp",
                DiffStatus::Downgraded,
                0,
                vec![VersionDiff::Removed(one("0.1.0"))],
                true,
            ),
            package(
                "graphics-drivers",
                DiffStatus::Downgraded,
                0,
                vec![
                    VersionDiff::Removed(one("570.1")),
                    amount_changed("565.2", 1, 2),
                ],
                false,
            ),
            package(
                "abseil-cpp",
                DiffStatus::Downgraded,
                -7_721_032,
                vec![
                    VersionDiff::Removed(one("20260107.1-dev")),
                    amount_changed("20260107.1", 3, 2),
                ],
                false,
            ),
            package(
                "dhcpcd",
                DiffStatus::Upgraded,
                -511_536,
                vec![
                    VersionDiff::Added(one("10.3.2_fish-completions")),
                    amount_changed("10.3.2", 2, 1),
                ],
                false,
            ),
            package(
                "util-linux",
                DiffStatus::Mixed,
                141_704,
                vec![
                    changed("2.42.3-dev", "2.42.3-man"),
                    VersionDiff::Removed(one("2.42.3")),
                ],
                true,
            ),
        ])
        .unwrap();
        assert_eq!(changes.len(), 10);
        assert_eq!(changes[0].name, "a");
        assert_eq!(changes[3].status, ChangeStatus::Downgraded);
        assert_eq!((changes[3].old.as_str(), changes[3].new.as_str()), ("2", "1"));
        assert_eq!(changes[5].name, "chomp");
        assert_eq!(changes[5].old, "0.1.0");
        assert_eq!(changes[5].new, "...");
        assert_eq!(changes[6].name, "graphics-drivers");
        assert_eq!(changes[6].old, "570.1");
        assert_eq!(changes[6].new, "565.2");
        assert_eq!(changes[7].name, "abseil-cpp");
        assert_eq!(changes[7].status, ChangeStatus::Changed);
        assert!(changes[7].old.is_empty());
        assert_eq!(changes[7].new, "20260107.1");
        assert_eq!(changes[7].size, -7_721_032);
        assert_eq!(changes[8].name, "dhcpcd");
        assert_eq!(changes[8].status, ChangeStatus::Changed);
        assert!(changes[8].old.is_empty());
        assert_eq!(changes[8].new, "10.3.2");
        assert_eq!(changes[9].name, "util-linux");
        assert_eq!(changes[9].status, ChangeStatus::Changed);
        assert!(changes[9].old.is_empty());
        assert_eq!(changes[9].new, "2.42.3");
    }

    #[test]
    fn rejects_paired_status_without_versions() {
        assert!(
            changes_from_diffs(&[package("demo", DiffStatus::Upgraded, 0, vec![], false)])
                .is_err()
        );
    }
}
