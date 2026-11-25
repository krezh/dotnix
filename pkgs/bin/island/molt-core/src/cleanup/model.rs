use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum CleanupKind {
    Nix,
    Docker,
    Development,
    Caches,
    Games,
    GamePrefixes,
    Trash,
    Journals,
}

impl CleanupKind {
    pub const ALL: [Self; 8] = [
        Self::Nix,
        Self::Docker,
        Self::Development,
        Self::Caches,
        Self::Games,
        Self::GamePrefixes,
        Self::Trash,
        Self::Journals,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Self::Nix => "Nix store & generations",
            Self::Docker => "Docker",
            Self::Development => "Development artifacts",
            Self::Caches => "Application caches",
            Self::Games => "Game caches & Proton",
            Self::GamePrefixes => "Old game prefixes",
            Self::Trash => "Trash",
            Self::Journals => "System journals",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::Nix => "Old generations and\nunreferenced store paths",
            Self::Docker => "Unused images, containers,\nvolumes, and build cache",
            Self::Development => "Rust, JavaScript, Python,\nZig, and direnv build data",
            Self::Caches => "User, package-manager, and\nsandbox application caches",
            Self::Games => "Shader caches of uninstalled\ngames and unused Proton builds",
            Self::GamePrefixes => "Proton prefixes of uninstalled\ngames, keeping saves",
            Self::Trash => "Files in the freedesktop trash",
            Self::Journals => "Archived systemd journal\ndata",
        }
    }

    pub const fn icon(self) -> &'static str {
        match self {
            Self::Nix => "drive-harddisk-symbolic",
            Self::Docker => "package-x-generic-symbolic",
            Self::Development => "applications-engineering-symbolic",
            Self::Caches => "folder-symbolic",
            Self::Games => "input-gaming-symbolic",
            Self::GamePrefixes => "applications-games-symbolic",
            Self::Trash => "user-trash-symbolic",
            Self::Journals => "text-x-generic-symbolic",
        }
    }

    /// The colour that marks this category on its card, plan row, disk bar segment, and item list.
    pub const fn hue(self) -> crate::units::Hue {
        use crate::units::Hue;
        match self {
            Self::Nix => Hue::Blue,
            Self::Docker => Hue::Sapphire,
            Self::Development => Hue::Mauve,
            Self::Caches => Hue::Peach,
            Self::Games => Hue::Green,
            Self::GamePrefixes => Hue::Yellow,
            Self::Trash => Hue::Pink,
            Self::Journals => Hue::Sky,
        }
    }

    /// Whether a scan that finds something selects this category. Prefixes hold games' leftover
    /// settings and saves, so they wait for an explicit choice.
    pub const fn selected_by_default(self) -> bool {
        !matches!(self, Self::GamePrefixes)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanupItem {
    pub label: String,
    pub detail: String,
    pub bytes: Option<u64>,
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanupGroup {
    pub kind: CleanupKind,
    pub available: bool,
    pub reclaimable: u64,
    pub items: Vec<CleanupItem>,
    pub note: Option<String>,
    /// Bytes per filesystem device for a category spread over several; empty when everything
    /// is on its one volume.
    pub volumes: Vec<(u64, u64)>,
}

impl CleanupGroup {
    pub fn unavailable(kind: CleanupKind, reason: impl Into<String>) -> Self {
        Self {
            kind,
            available: false,
            reclaimable: 0,
            items: Vec::new(),
            note: Some(reason.into()),
            volumes: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiskUsage {
    pub total: u64,
    pub used: u64,
    pub available: u64,
}

/// One filesystem cleanup touches: where it is mounted, its usage, and the categories whose
/// removals free space on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Volume {
    pub mount: PathBuf,
    pub device: u64,
    pub usage: DiskUsage,
    pub kinds: Vec<CleanupKind>,
}

#[derive(Debug)]
pub enum ScanEvent {
    Group(CleanupGroup),
    Complete(Vec<Volume>),
}

#[derive(Debug)]
pub enum CleanupEvent {
    Started(CleanupKind),
    Finished(CleanupKind, Result<u64, String>),
    Complete {
        before: Vec<Volume>,
        after: Vec<Volume>,
        elapsed: Duration,
    },
}
