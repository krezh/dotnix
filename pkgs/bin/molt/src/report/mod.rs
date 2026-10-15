//! What a reviewed build changes: packages from dix, configuration files, release notes, and the
//! services activation would touch.

pub(crate) mod config_diff;
mod dix;
mod services;
pub(crate) mod ui;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::build::{GcRoot, Target};
use crate::ui::theme;
use config_diff::ConfigDiff;

pub(crate) use self::dix::report_from_diff;
pub(crate) use services::{ServiceGroup, service_impact};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ChangeStatus {
    Added,
    Removed,
    Upgraded,
    Downgraded,
    Changed,
}

impl ChangeStatus {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Added => "Added",
            Self::Removed => "Removed",
            Self::Upgraded => "Upgraded",
            Self::Downgraded => "Downgraded",
            Self::Changed => "Changed",
        }
    }

    /// The tone class this status is drawn in.
    pub(crate) fn tone(self) -> &'static str {
        theme::tone(match self {
            Self::Added => theme::GREEN,
            Self::Removed => theme::RED,
            Self::Upgraded => theme::BLUE,
            Self::Downgraded => theme::YELLOW,
            Self::Changed => theme::SAPPHIRE,
        })
    }

    pub(crate) const fn is_paired(self) -> bool {
        matches!(self, Self::Upgraded | Self::Downgraded)
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

/// Drops the versionless entries dix reports for generated files, such as `slumber-config.toml`:
/// the report lists those files as configuration changes instead.
pub(crate) fn remove_config_files(changes: &mut Vec<Change>, config: &ConfigDiff) {
    let files = config
        .changes
        .iter()
        .filter_map(|file| file.store_name.as_deref())
        .collect::<HashSet<_>>();
    changes.retain(|change| {
        !(change.old.is_empty() && change.new.is_empty() && files.contains(change.name.as_str()))
    });
}

pub(crate) struct ReportMetadata {
    pub(crate) target: Target,
    pub(crate) flake: String,
    pub(crate) flake_dir: PathBuf,
    pub(crate) baseline: PathBuf,
    pub(crate) profile: PathBuf,
    pub(crate) output: PathBuf,
    pub(crate) gc_root: Arc<GcRoot>,
}

#[derive(Clone)]
pub(crate) struct Report {
    pub(crate) target: Target,
    pub(crate) flake: String,
    pub(crate) flake_dir: PathBuf,
    /// The running configuration the changes are compared against.
    pub(crate) baseline: PathBuf,
    /// Where the profile pointed when the report was built; activation refuses to go ahead if it
    /// has moved since, unless to this report's own output.
    pub(crate) profile: PathBuf,
    pub(crate) output: PathBuf,
    _gc_root: Arc<GcRoot>,
    pub(crate) changes: Vec<Change>,
    pub(crate) paths: Option<(i64, i64, i64, i64)>,
    pub(crate) sizes: Option<(i64, i64)>,
    pub(crate) operational_impact: Result<Vec<String>, String>,
    /// Generated configuration files that differ from the active generation's.
    pub(crate) config: Result<Arc<ConfigDiff>, String>,
    /// Wall time from evaluation start to the finished comparison, when built this session.
    pub(crate) built_in: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use config_diff::{ConfigChange, ConfigContent};

    #[test]
    fn drops_versionless_entries_listed_as_files() {
        let config = ConfigDiff {
            changes: vec![ConfigChange {
                status: ChangeStatus::Added,
                path: "~/.config/slumber/config.toml".to_owned(),
                store_name: Some("slumber-config.toml".to_owned()),
                content: ConfigContent::Binary,
            }],
            store_path_only: Vec::new(),
        };
        let change = |name: &str, new: &str| Change {
            status: ChangeStatus::Added,
            name: name.to_owned(),
            old: String::new(),
            new: new.to_owned(),
            size: 0,
        };
        let mut changes = vec![
            change("slumber-config.toml", ""),
            change("slumber-config.toml", "1.0"),
            change("other.toml", ""),
        ];
        remove_config_files(&mut changes, &config);
        let kept = changes
            .iter()
            .map(|change| (change.name.as_str(), change.new.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(kept, [("slumber-config.toml", "1.0"), ("other.toml", "")]);
    }
}
