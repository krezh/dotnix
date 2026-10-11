use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum CleanupKind {
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
    pub(crate) const ALL: [Self; 8] = [
        Self::Nix,
        Self::Docker,
        Self::Development,
        Self::Caches,
        Self::Games,
        Self::GamePrefixes,
        Self::Trash,
        Self::Journals,
    ];

    pub(crate) const fn title(self) -> &'static str {
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

    pub(crate) const fn description(self) -> &'static str {
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

    pub(crate) const fn icon(self) -> &'static str {
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
    pub(crate) const fn color(self) -> crate::ui::theme::Color {
        match self {
            Self::Nix => crate::ui::theme::BLUE,
            Self::Docker => crate::ui::theme::SAPPHIRE,
            Self::Development => crate::ui::theme::MAUVE,
            Self::Caches => crate::ui::theme::PEACH,
            Self::Games => crate::ui::theme::GREEN,
            Self::GamePrefixes => crate::ui::theme::YELLOW,
            Self::Trash => crate::ui::theme::PINK,
            Self::Journals => crate::ui::theme::SKY,
        }
    }

    /// The tone class for [`Self::color`].
    pub(crate) fn tone(self) -> &'static str {
        crate::ui::theme::tone(self.color())
    }

    /// Whether a scan that finds something selects this category. Prefixes hold games' leftover
    /// settings and saves, so they wait for an explicit choice.
    pub(crate) const fn selected_by_default(self) -> bool {
        !matches!(self, Self::GamePrefixes)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CleanupItem {
    pub(crate) label: String,
    pub(crate) detail: String,
    pub(crate) bytes: Option<u64>,
    pub(crate) path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub(crate) struct CleanupGroup {
    pub(crate) kind: CleanupKind,
    pub(crate) available: bool,
    pub(crate) reclaimable: u64,
    pub(crate) items: Vec<CleanupItem>,
    pub(crate) note: Option<String>,
    /// Bytes per filesystem device for a category spread over several; empty when everything
    /// is on its one volume.
    pub(crate) volumes: Vec<(u64, u64)>,
}

impl CleanupGroup {
    pub(super) fn unavailable(kind: CleanupKind, reason: impl Into<String>) -> Self {
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

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DiskUsage {
    pub(crate) total: u64,
    pub(crate) used: u64,
    pub(crate) available: u64,
}

/// One filesystem cleanup touches: where it is mounted, its usage, and the categories whose
/// removals free space on it.
#[derive(Clone, Debug)]
pub(crate) struct Volume {
    pub(crate) mount: PathBuf,
    pub(crate) device: u64,
    pub(crate) usage: DiskUsage,
    pub(crate) kinds: Vec<CleanupKind>,
}

#[derive(Debug)]
pub(crate) enum ScanEvent {
    Group(CleanupGroup),
    Complete(Vec<Volume>),
}

#[derive(Debug)]
pub(crate) enum CleanupEvent {
    Started(CleanupKind),
    Finished(CleanupKind, Result<u64, String>),
    Complete {
        before: Vec<Volume>,
        after: Vec<Volume>,
        elapsed: Duration,
    },
}
