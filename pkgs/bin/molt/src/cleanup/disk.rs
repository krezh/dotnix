use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use super::filesystem::home;
use super::games::library_roots;
use super::model::{CleanupKind, DiskUsage, Volume};

pub(super) fn usage(path: &Path) -> Option<DiskUsage> {
    let stats = rustix::fs::statvfs(path).ok()?;
    let block_size = stats.f_frsize;
    let total = stats.f_blocks.saturating_mul(block_size);
    let available = stats.f_bavail.saturating_mul(block_size);
    let free = stats.f_bfree.saturating_mul(block_size);
    Some(DiskUsage {
        total,
        used: total.saturating_sub(free),
        available,
    })
}

/// The filesystems cleanup frees space on, in category order, each with the categories it hosts.
pub(super) fn cleanup_volumes() -> Vec<Volume> {
    let mut volumes: Vec<Volume> = Vec::new();
    for kind in CleanupKind::ALL {
        for path in anchors(kind) {
            let Ok(metadata) = fs::metadata(&path) else {
                continue;
            };
            let device = metadata.dev();
            if let Some(volume) = volumes.iter_mut().find(|volume| volume.device == device) {
                if !volume.kinds.contains(&kind) {
                    volume.kinds.push(kind);
                }
                continue;
            }
            let Some(usage) = usage(&path) else {
                continue;
            };
            volumes.push(Volume {
                mount: mount_point(&path, device),
                device,
                usage,
                kinds: vec![kind],
            });
        }
    }
    volumes
}

/// Paths whose filesystems hold what `kind` removes; game data is spread over every Steam library.
fn anchors(kind: CleanupKind) -> Vec<PathBuf> {
    match kind {
        CleanupKind::Nix => vec![PathBuf::from("/nix")],
        CleanupKind::Docker => vec![PathBuf::from("/var/lib/docker")],
        CleanupKind::Journals => vec![PathBuf::from("/var/log")],
        CleanupKind::Development | CleanupKind::Caches | CleanupKind::Trash => {
            home().into_iter().collect()
        }
        CleanupKind::Games | CleanupKind::GamePrefixes => library_roots(),
    }
}

/// The topmost ancestor of `path` still on `device`: where its filesystem is mounted.
fn mount_point(path: &Path, device: u64) -> PathBuf {
    let mut mount = path;
    while let Some(parent) = mount.parent()
        && fs::metadata(parent).is_ok_and(|metadata| metadata.dev() == device)
    {
        mount = parent;
    }
    mount.to_path_buf()
}
