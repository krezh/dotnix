use std::path::PathBuf;
use std::sync::Arc;

use serde::Deserialize;

use crate::build::{GcRoot, Target};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ChangeStatus {
    Added,
    Removed,
    Upgraded,
    Downgraded,
    Changed,
}

impl ChangeStatus {
    pub(crate) const ALL: [Self; 5] = [
        Self::Added,
        Self::Removed,
        Self::Upgraded,
        Self::Downgraded,
        Self::Changed,
    ];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Added => "Added",
            Self::Removed => "Removed",
            Self::Upgraded => "Upgraded",
            Self::Downgraded => "Downgraded",
            Self::Changed => "Changed",
        }
    }

    pub(crate) const fn class(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Upgraded => "upgraded",
            Self::Downgraded => "downgraded",
            Self::Changed => "changed",
        }
    }

    pub(crate) const fn is_paired(self) -> bool {
        matches!(self, Self::Upgraded | Self::Downgraded)
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "Added" => Some(Self::Added),
            "Removed" => Some(Self::Removed),
            "Upgraded" => Some(Self::Upgraded),
            "Downgraded" => Some(Self::Downgraded),
            "Changed" => Some(Self::Changed),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct Change {
    pub(crate) status: ChangeStatus,
    pub(crate) name: String,
    pub(crate) old: String,
    pub(crate) new: String,
    pub(crate) size: i64,
}

pub(crate) struct ReportMetadata {
    pub(crate) target: Target,
    pub(crate) flake: String,
    pub(crate) flake_dir: PathBuf,
    pub(crate) baseline: PathBuf,
    pub(crate) output: PathBuf,
    pub(crate) gc_root: Arc<GcRoot>,
}

#[derive(Clone)]
pub(crate) struct Report {
    pub(crate) target: Target,
    pub(crate) flake: String,
    pub(crate) flake_dir: PathBuf,
    pub(crate) baseline: PathBuf,
    pub(crate) output: PathBuf,
    _gc_root: Arc<GcRoot>,
    pub(crate) changes: Vec<Change>,
    pub(crate) paths: Option<(i64, i64, i64, i64)>,
    pub(crate) sizes: Option<(i64, i64)>,
}
pub(crate) fn parse_report(metadata: ReportMetadata, json: &[u8]) -> Result<Report, String> {
    let ReportMetadata {
        target,
        flake,
        flake_dir,
        baseline,
        output,
        gc_root,
    } = metadata;
    let report: DixReport = serde_json::from_slice(json)
        .map_err(|error| format!("dix returned invalid JSON: {error}"))?;
    let mut changes = Vec::new();
    for diff in report
        .diffs
        .into_iter()
        .filter(|diff| !diff.name.starts_with("nixos-system-"))
    {
        let mixed = diff.status == "Mixed";
        let mut status = if mixed {
            ChangeStatus::Changed
        } else {
            ChangeStatus::parse(&diff.status).ok_or_else(|| {
                format!(
                    "dix returned unknown status {:?} for {}",
                    diff.status, diff.name
                )
            })?
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
            name: diff.name,
            old,
            new,
            size: diff.size_delta,
        });
    }
    let paths = report
        .paths
        .map(|paths| (paths.old, paths.new, paths.added, paths.removed));
    let sizes = match (report.size_old, report.size_new) {
        (Some(old), Some(new)) => Some((old, new)),
        (None, None) => None,
        _ => return Err("dix returned only one closure size".to_owned()),
    };
    Ok(Report {
        target,
        flake,
        flake_dir,
        baseline,
        output,
        _gc_root: gc_root,
        changes,
        paths,
        sizes,
    })
}

#[derive(Deserialize)]
struct DixReport {
    diffs: Vec<DixDiff>,
    #[serde(default)]
    paths: Option<DixPaths>,
    #[serde(default)]
    size_old: Option<i64>,
    #[serde(default)]
    size_new: Option<i64>,
}

#[derive(Deserialize)]
struct DixDiff {
    name: String,
    status: String,
    size_delta: i64,
    versions: Vec<DixVersion>,
    #[serde(default)]
    has_omitted_versions: bool,
}

#[derive(Deserialize)]
struct DixVersion {
    kind: String,
    #[serde(default)]
    old: Option<DixVersionName>,
    #[serde(default)]
    new: Option<DixVersionName>,
    #[serde(default)]
    version: Option<DixVersionName>,
    #[serde(default)]
    old_amount: Option<i64>,
    #[serde(default)]
    new_amount: Option<i64>,
}

#[derive(Deserialize)]
struct DixVersionName {
    name: String,
}

#[derive(Deserialize)]
struct DixPaths {
    old: i64,
    new: i64,
    added: i64,
    removed: i64,
}
fn dix_version_change(
    status: ChangeStatus,
    package: &str,
    versions: &[DixVersion],
    has_omitted_versions: bool,
) -> Result<Option<(String, String)>, String> {
    let mut changed = Vec::new();
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut amount_changed = Vec::new();
    for version in versions {
        match version.kind.as_str() {
            "changed" => changed.push((
                required_version_name(version.old.as_ref(), package, "changed.old")?,
                required_version_name(version.new.as_ref(), package, "changed.new")?,
            )),
            "added" => added.push(required_version_name(
                version.version.as_ref(),
                package,
                "added.version",
            )?),
            "removed" => removed.push(required_version_name(
                version.version.as_ref(),
                package,
                "removed.version",
            )?),
            "amount_changed" => {
                let name = required_version_name(
                    version.version.as_ref(),
                    package,
                    "amount_changed.version",
                )?;
                if version.old_amount.is_none() || version.new_amount.is_none() {
                    return Err(format!("dix omitted amount_changed counts for {package}"));
                }
                amount_changed.push(name);
            }
            kind => {
                return Err(format!(
                    "dix returned unknown version kind {kind:?} for {package}"
                ));
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

fn required_version_name(
    version: Option<&DixVersionName>,
    package: &str,
    field: &str,
) -> Result<String, String> {
    version
        .map(|version| version.name.clone())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| format!("dix omitted {field} for {package}"))
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
    use std::{env, path::PathBuf, sync::Arc};

    use crate::build::{GcRoot, Target};
    fn test_gc_root() -> Arc<GcRoot> {
        Arc::new(GcRoot {
            path: env::temp_dir().join(format!("swix-test-gc-root-{}", std::process::id())),
        })
    }

    fn parse_test_report(json: &[u8]) -> Result<Report, String> {
        parse_report(
            ReportMetadata {
                target: Target::NixOs,
                flake: "host".to_owned(),
                flake_dir: PathBuf::from("/flake"),
                baseline: PathBuf::from("/nix/store/old-system"),
                output: PathBuf::from("/nix/store/new-system"),
                gc_root: test_gc_root(),
            },
            json,
        )
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
    fn parses_a_dix_upgrade() {
        let report = parse_test_report(
            br#"{"diffs":[{"name":"demo","status":"Upgraded","size_delta":10,"versions":[{"kind":"changed","old":{"name":"1.0-bin"},"new":{"name":"2.0-bin"}}]}]}"#,
        )
        .unwrap();
        assert_eq!(report.changes.len(), 1);
        assert_eq!(report.changes[0].old, "1.0");
        assert_eq!(report.changes[0].new, "2.0");
    }

    #[test]
    fn parses_every_supported_dix_status_and_metrics() {
        let report = parse_test_report(
            br#"{
                "diffs": [
                    {"name":"a","status":"Added","size_delta":1,"versions":[{"kind":"added","version":{"name":"1"}}]},
                    {"name":"b","status":"Removed","size_delta":-2,"versions":[{"kind":"removed","version":{"name":"1"}}]},
                    {"name":"c","status":"Upgraded","size_delta":3,"versions":[{"kind":"changed","old":{"name":"1"},"new":{"name":"2"}}]},
                    {"name":"d","status":"Downgraded","size_delta":-4,"versions":[{"kind":"changed","old":{"name":"2"},"new":{"name":"1"}}]},
                    {"name":"e","status":"Changed","size_delta":0,"versions":[]},
                    {"name":"X-Restart-Triggers-dbus-broker","status":"Changed","size_delta":0,"versions":[{"kind":"amount_changed","version":{"name":"1"},"old_amount":1,"new_amount":2}]},
                    {"name":"chomp","status":"Downgraded","size_delta":0,"versions":[{"kind":"removed","version":{"name":"0.1.0"}}],"has_omitted_versions":true},
                    {"name":"graphics-drivers","status":"Downgraded","size_delta":0,"versions":[{"kind":"removed","version":{"name":"570.1"}},{"kind":"amount_changed","version":{"name":"565.2"},"old_amount":1,"new_amount":2}],"has_omitted_versions":false},
                    {"name":"abseil-cpp","status":"Downgraded","size_delta":-7721032,"versions":[{"kind":"removed","version":{"name":"20260107.1-dev"}},{"kind":"amount_changed","version":{"name":"20260107.1"},"old_amount":3,"new_amount":2}],"has_omitted_versions":false},
                    {"name":"dhcpcd","status":"Upgraded","size_delta":-511536,"versions":[{"kind":"added","version":{"name":"10.3.2_fish-completions"}},{"kind":"amount_changed","version":{"name":"10.3.2"},"old_amount":2,"new_amount":1}],"has_omitted_versions":false},
                    {"name":"util-linux","status":"Mixed","size_delta":141704,"versions":[{"kind":"changed","old":{"name":"2.42.3-dev"},"new":{"name":"2.42.3-man"}},{"kind":"removed","version":{"name":"2.42.3"}}],"has_omitted_versions":true}
                ],
                "paths":{"old":10,"new":11,"added":2,"removed":1},
                "size_old":100,
                "size_new":110
            }"#,
        )
        .unwrap();
        assert_eq!(report.changes.len(), 10);
        assert_eq!(report.changes[5].name, "chomp");
        assert_eq!(report.changes[5].old, "0.1.0");
        assert_eq!(report.changes[5].new, "...");
        assert_eq!(report.changes[6].name, "graphics-drivers");
        assert_eq!(report.changes[6].old, "570.1");
        assert_eq!(report.changes[6].new, "565.2");
        assert_eq!(report.changes[7].name, "abseil-cpp");
        assert_eq!(report.changes[7].status, ChangeStatus::Changed);
        assert!(report.changes[7].old.is_empty());
        assert_eq!(report.changes[7].new, "20260107.1");
        assert_eq!(report.changes[8].name, "dhcpcd");
        assert_eq!(report.changes[8].status, ChangeStatus::Changed);
        assert!(report.changes[8].old.is_empty());
        assert_eq!(report.changes[8].new, "10.3.2");
        assert_eq!(report.changes[9].name, "util-linux");
        assert_eq!(report.changes[9].status, ChangeStatus::Changed);
        assert!(report.changes[9].old.is_empty());
        assert_eq!(report.changes[9].new, "2.42.3");
        assert_eq!(report.paths, Some((10, 11, 2, 1)));
        assert_eq!(report.sizes, Some((100, 110)));
    }

    #[test]
    fn rejects_incomplete_or_unknown_dix_data() {
        assert!(parse_test_report(br#"{}"#).is_err());
        assert!(
            parse_test_report(
                br#"{"diffs":[{"name":"demo","status":"Unexpected","size_delta":0,"versions":[]}]}"#
            )
            .is_err()
        );
        assert!(
            parse_test_report(
                br#"{"diffs":[{"name":"demo","status":"Upgraded","size_delta":0,"versions":[]}]}"#
            )
            .is_err()
        );
        assert!(parse_test_report(br#"{"diffs":[],"size_old":1}"#).is_err());
    }
}
