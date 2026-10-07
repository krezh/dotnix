use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use super::filesystem::{home, items_for_paths};
use super::model::{CleanupGroup, CleanupKind};

pub(super) fn scan(cancellation: &AtomicBool) -> CleanupGroup {
    let Some(home) = home() else {
        return CleanupGroup::unavailable(CleanupKind::Caches, "HOME is unavailable");
    };
    let cache_home = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".cache"));
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    append_children(
        &cache_home,
        &home,
        "Cache",
        cancellation,
        &mut seen,
        &mut candidates,
    );

    for (relative, label) in [
        (".cargo/registry", "Cargo registry"),
        (".cargo/git", "Cargo Git cache"),
        (".gradle/caches", "Gradle cache"),
        (".m2/repository", "Maven repository cache"),
        ("go/pkg/mod", "Go module cache"),
        (".local/share/pnpm/store", "pnpm store"),
        (".bun/install/cache", "Bun package cache"),
    ] {
        append_path(
            home.join(relative),
            &home,
            label,
            &mut seen,
            &mut candidates,
        );
    }

    let flatpak_apps = home.join(".var/app");
    if let Ok(apps) = fs::read_dir(flatpak_apps) {
        for app in apps.flatten() {
            let name = app.file_name().to_string_lossy().into_owned();
            append_path(
                app.path().join("cache"),
                &home,
                format!("Flatpak · {name}"),
                &mut seen,
                &mut candidates,
            );
        }
    }

    let mut items = items_for_paths(candidates, &home, cancellation);
    items.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then_with(|| left.detail.cmp(&right.detail))
    });
    let reclaimable = items.iter().filter_map(|item| item.bytes).sum();
    CleanupGroup {
        kind: CleanupKind::Caches,
        available: true,
        reclaimable,
        items,
        note: Some(
            "Caches are recreated by their owning applications and package managers".to_owned(),
        ),
    }
}

fn append_children(
    directory: &Path,
    home: &Path,
    prefix: &str,
    cancellation: &AtomicBool,
    seen: &mut HashSet<PathBuf>,
    candidates: &mut Vec<(PathBuf, String)>,
) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        if cancellation.load(Ordering::Relaxed) {
            break;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        append_path(
            entry.path(),
            home,
            format!("{prefix} · {name}"),
            seen,
            candidates,
        );
    }
}

fn append_path(
    path: PathBuf,
    home: &Path,
    label: impl Into<String>,
    seen: &mut HashSet<PathBuf>,
    candidates: &mut Vec<(PathBuf, String)>,
) {
    if !path.starts_with(home) || !seen.insert(path.clone()) {
        return;
    }
    candidates.push((path, label.into()));
}
