//! What a reviewed build changes: packages, configuration files, release notes, and the services
//! activation would touch.

pub mod config_diff;
pub mod flow;
pub mod markdown;
pub mod notes;
pub mod present;
pub mod services;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::build::{GcRoot, Target};
use crate::closure::ClosureDiff;
use crate::units::Hue;
use config_diff::ConfigDiff;

pub use services::{ServiceGroup, service_impact};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChangeStatus {
    Added,
    Removed,
    Upgraded,
    Downgraded,
    Changed,
}

impl ChangeStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Added => "Added",
            Self::Removed => "Removed",
            Self::Upgraded => "Upgraded",
            Self::Downgraded => "Downgraded",
            Self::Changed => "Changed",
        }
    }

    /// The colour this status is drawn in.
    pub const fn hue(self) -> Hue {
        match self {
            Self::Added => Hue::Green,
            Self::Removed => Hue::Red,
            Self::Upgraded => Hue::Blue,
            Self::Downgraded => Hue::Yellow,
            Self::Changed => Hue::Sapphire,
        }
    }

    pub const fn is_paired(self) -> bool {
        matches!(self, Self::Upgraded | Self::Downgraded)
    }
}

#[derive(Clone)]
pub struct Change {
    pub status: ChangeStatus,
    pub name: String,
    pub old: String,
    pub new: String,
    pub size: i64,
}

/// Drops versionless generated files already listed as configuration changes.
pub fn remove_config_files(changes: &mut Vec<Change>, config: &ConfigDiff) {
    let files = config
        .changes
        .iter()
        .filter_map(|file| file.store_name.as_deref())
        .collect::<HashSet<_>>();
    changes.retain(|change| {
        !(change.old.is_empty() && change.new.is_empty() && files.contains(change.name.as_str()))
    });
}

/// The report's title: what the built configuration is ready for.
pub const fn ready_title(target: Target) -> &'static str {
    match target {
        Target::NixOs => "Ready to switch",
        Target::HomeManager => "Ready to activate",
    }
}

pub struct ReportMetadata {
    pub target: Target,
    pub flake: String,
    pub flake_dir: PathBuf,
    pub baseline: PathBuf,
    pub profile: PathBuf,
    pub output: PathBuf,
    pub gc_root: Arc<GcRoot>,
}

#[derive(Clone)]
pub struct Report {
    pub target: Target,
    pub flake: String,
    pub flake_dir: PathBuf,
    /// The running configuration the changes are compared against.
    pub baseline: PathBuf,
    /// Where the profile pointed when the report was built; activation refuses to go ahead if it
    /// has moved since, unless to this report's own output.
    pub profile: PathBuf,
    pub output: PathBuf,
    _gc_root: Arc<GcRoot>,
    pub changes: Vec<Change>,
    pub paths: Option<(i64, i64, i64, i64)>,
    pub sizes: Option<(i64, i64)>,
    pub operational_impact: Result<Vec<String>, String>,
    /// Generated configuration files that differ from the active generation's.
    pub config: Result<Arc<ConfigDiff>, String>,
    /// Wall time from evaluation start to the finished comparison, when built this session.
    pub built_in: Option<Duration>,
    /// Warnings nix printed while evaluating and building, each once, in the order first seen.
    pub warnings: Vec<String>,
}

pub fn report_from_diff(metadata: ReportMetadata, diff: ClosureDiff) -> Report {
    let ReportMetadata {
        target,
        flake,
        flake_dir,
        baseline,
        profile,
        output,
        gc_root,
    } = metadata;
    Report {
        target,
        flake,
        flake_dir,
        baseline,
        profile,
        output,
        _gc_root: gc_root,
        changes: diff.changes,
        paths: Some(diff.paths),
        sizes: Some(diff.sizes),
        operational_impact: Ok(Vec::new()),
        config: Ok(Arc::default()),
        built_in: None,
        warnings: Vec::new(),
    }
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
