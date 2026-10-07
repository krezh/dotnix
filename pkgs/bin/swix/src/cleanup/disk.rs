use std::collections::HashSet;
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use super::model::DiskUsage;

pub(super) fn usage(path: &Path) -> Option<DiskUsage> {
    let path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // `path` is NUL-terminated and `stats` points to writable storage.
    if unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) } != 0 {
        return None;
    }
    // A successful `statvfs` call initialized the complete structure.
    let stats = unsafe { stats.assume_init() };
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

pub(super) fn cleanup_usage() -> Option<DiskUsage> {
    let mut paths = Vec::with_capacity(3);
    if let Some(home) = std::env::var_os("HOME") {
        paths.push(PathBuf::from(home));
    }
    paths.push(PathBuf::from("/nix"));
    paths.push(PathBuf::from("/var/log"));

    let mut devices = HashSet::new();
    let mut combined = None;
    for path in paths {
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        let device = metadata.dev();
        if !devices.insert(device) {
            continue;
        }
        let Some(current) = usage(&path) else {
            continue;
        };
        let total = combined.get_or_insert(DiskUsage::default());
        total.total = total.total.saturating_add(current.total);
        total.used = total.used.saturating_add(current.used);
        total.available = total.available.saturating_add(current.available);
    }
    combined.or_else(|| usage(Path::new("/")))
}
