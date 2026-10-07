use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum CleanupKind {
    Nix,
    Docker,
    Development,
    Caches,
    Trash,
    Journals,
}

impl CleanupKind {
    pub(crate) const ALL: [Self; 6] = [
        Self::Nix,
        Self::Docker,
        Self::Development,
        Self::Caches,
        Self::Trash,
        Self::Journals,
    ];

    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::Nix => "Nix store & generations",
            Self::Docker => "Docker",
            Self::Development => "Development artifacts",
            Self::Caches => "Application caches",
            Self::Trash => "Trash",
            Self::Journals => "System journals",
        }
    }

    pub(crate) const fn description(self) -> &'static str {
        match self {
            Self::Nix => "Old generations and unreferenced store paths",
            Self::Docker => "Unused images, containers, volumes, and build cache",
            Self::Development => "Rust, JavaScript, Python, Zig, and direnv build data",
            Self::Caches => "User, package-manager, and sandbox application caches",
            Self::Trash => "Files in the freedesktop trash",
            Self::Journals => "Archived systemd journal data",
        }
    }

    pub(crate) const fn icon(self) -> &'static str {
        match self {
            Self::Nix => "drive-harddisk-symbolic",
            Self::Docker => "package-x-generic-symbolic",
            Self::Development => "applications-engineering-symbolic",
            Self::Caches => "folder-symbolic",
            Self::Trash => "user-trash-symbolic",
            Self::Journals => "text-x-generic-symbolic",
        }
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
}

impl CleanupGroup {
    pub(super) fn unavailable(kind: CleanupKind, reason: impl Into<String>) -> Self {
        Self {
            kind,
            available: false,
            reclaimable: 0,
            items: Vec::new(),
            note: Some(reason.into()),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DiskUsage {
    pub(crate) total: u64,
    pub(crate) used: u64,
    pub(crate) available: u64,
}

#[derive(Debug)]
pub(crate) enum ScanEvent {
    Group(CleanupGroup),
    Complete(Option<DiskUsage>),
}

#[derive(Debug)]
pub(crate) enum CleanupEvent {
    Started(CleanupKind),
    Finished(CleanupKind, Result<u64, String>),
    Complete {
        before: Option<DiskUsage>,
        after: Option<DiskUsage>,
        elapsed: Duration,
    },
}

pub(crate) fn format_size(bytes: u64) -> String {
    let value = bytes as f64;
    let (value, unit) = if value >= 1024.0 * 1024.0 * 1024.0 * 1024.0 {
        (value / (1024.0 * 1024.0 * 1024.0 * 1024.0), "TiB")
    } else if value >= 1024.0 * 1024.0 * 1024.0 {
        (value / (1024.0 * 1024.0 * 1024.0), "GiB")
    } else if value >= 1024.0 * 1024.0 {
        (value / (1024.0 * 1024.0), "MiB")
    } else if value >= 1024.0 {
        (value / 1024.0, "KiB")
    } else {
        (value, "B")
    };
    format!("{value:.1} {unit}")
}

pub(super) fn parse_size(text: &str) -> Option<u64> {
    let token = text.split_whitespace().next()?.trim();
    let split = token
        .find(|character: char| !character.is_ascii_digit() && character != '.')
        .unwrap_or(token.len());
    let value = token[..split].parse::<f64>().ok()?;
    let unit = token[split..].trim().to_ascii_uppercase();
    let multiplier = match unit.as_str() {
        "" | "B" => 1.0,
        "K" | "KB" | "KIB" => 1024.0,
        "M" | "MB" | "MIB" => 1024.0 * 1024.0,
        "G" | "GB" | "GIB" => 1024.0 * 1024.0 * 1024.0,
        "T" | "TB" | "TIB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((value * multiplier).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cleanup_tool_sizes() {
        assert_eq!(parse_size("11.06GB (97%)"), Some(11_875_584_573));
        assert_eq!(parse_size("344.3MB"), Some(361_024_717));
        assert_eq!(parse_size("3.5G"), Some(3_758_096_384));
        assert_eq!(parse_size("0B"), Some(0));
        assert_eq!(parse_size("unknown"), None);
    }

    #[test]
    fn formats_large_cleanup_totals() {
        assert_eq!(format_size(1536), "1.5 KiB");
        assert_eq!(format_size(2 * 1024 * 1024 * 1024), "2.0 GiB");
    }
}
