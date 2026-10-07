use std::fs;
use std::sync::atomic::AtomicBool;

use super::filesystem::{home, items_for_paths};
use super::model::{CleanupGroup, CleanupKind};

pub(super) fn scan(cancellation: &AtomicBool) -> CleanupGroup {
    let Some(home) = home() else {
        return CleanupGroup::unavailable(CleanupKind::Trash, "HOME is unavailable");
    };
    let trash = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"))
        .join("Trash");
    let mut candidates = Vec::new();
    for (directory, prefix) in [
        (trash.join("files"), "Trash"),
        (trash.join("info"), "Metadata"),
    ] {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            candidates.push((entry.path(), format!("{prefix} · {name}")));
        }
    }
    let mut items = items_for_paths(candidates, &home, cancellation);
    items.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then_with(|| left.label.cmp(&right.label))
    });
    let reclaimable = items.iter().filter_map(|item| item.bytes).sum();
    CleanupGroup {
        kind: CleanupKind::Trash,
        available: true,
        reclaimable,
        items,
        note: None,
    }
}
