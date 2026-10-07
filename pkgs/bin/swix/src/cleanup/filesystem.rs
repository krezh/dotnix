use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

use super::model::{CleanupGroup, CleanupItem};

pub(super) fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub(super) fn directory_size(path: &Path, cancellation: &AtomicBool) -> io::Result<u64> {
    let mut total = 0_u64;
    let mut pending = vec![path.to_owned()];
    while let Some(directory) = pending.pop() {
        if cancellation.load(Ordering::Relaxed) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "scan cancelled"));
        }
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        for entry in entries.flatten() {
            let metadata = match entry.file_type() {
                Ok(kind) if kind.is_symlink() => continue,
                Ok(kind) if kind.is_dir() => {
                    pending.push(entry.path());
                    continue;
                }
                Ok(_) => entry.metadata(),
                Err(error) => Err(error),
            };
            if let Ok(metadata) = metadata {
                total = total.saturating_add(metadata.len());
            }
        }
    }
    Ok(total)
}

pub(super) fn item_for_path(
    path: PathBuf,
    root: &Path,
    label: impl Into<String>,
    cancellation: &AtomicBool,
) -> Option<CleanupItem> {
    let metadata = fs::symlink_metadata(&path).ok()?;
    let bytes = if metadata.file_type().is_symlink() {
        0
    } else if metadata.is_dir() {
        directory_size(&path, cancellation).ok()?
    } else {
        metadata.len()
    };
    Some(CleanupItem {
        label: label.into(),
        detail: display_path(&path, root),
        bytes: Some(bytes),
        path: Some(path),
    })
}
pub(super) fn items_for_paths(
    candidates: Vec<(PathBuf, String)>,
    root: &Path,
    cancellation: &AtomicBool,
) -> Vec<CleanupItem> {
    if candidates.is_empty() {
        return Vec::new();
    }
    let worker_count = thread::available_parallelism()
        .map_or(1, usize::from)
        .min(4)
        .min(candidates.len());
    let work = Mutex::new(candidates.into_iter());
    thread::scope(|scope| {
        let workers = (0..worker_count)
            .map(|_| {
                scope.spawn(|| {
                    let mut items = Vec::new();
                    loop {
                        let candidate = work.lock().expect("cleanup work queue poisoned").next();
                        let Some((path, label)) = candidate else {
                            break;
                        };
                        if cancellation.load(Ordering::Relaxed) {
                            break;
                        }
                        if let Some(item) = item_for_path(path, root, label, cancellation) {
                            items.push(item);
                        }
                    }
                    items
                })
            })
            .collect::<Vec<_>>();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().unwrap())
            .collect()
    })
}

pub(super) fn clean_group(group: &CleanupGroup, cancellation: &AtomicBool) -> Result<u64, String> {
    let home = home().ok_or("HOME is unavailable")?;
    if group.items.is_empty() {
        return Ok(0);
    }
    let worker_count = thread::available_parallelism()
        .map_or(1, usize::from)
        .min(4)
        .min(group.items.len());
    let next = AtomicUsize::new(0);
    let results = thread::scope(|scope| {
        (0..worker_count)
            .map(|_| {
                scope.spawn(|| {
                    let mut removed = 0_u64;
                    let mut failures = Vec::new();
                    loop {
                        if cancellation.load(Ordering::Relaxed) {
                            break;
                        }
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(item) = group.items.get(index) else {
                            break;
                        };
                        let Some(path) = item.path.as_ref() else {
                            continue;
                        };
                        if !path.starts_with(&home) || path == &home {
                            failures.push(format!("refused unsafe path {}", path.display()));
                            continue;
                        }
                        match remove_path(path) {
                            Ok(()) => {
                                removed = removed.saturating_add(item.bytes.unwrap_or(0));
                            }
                            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                            Err(error) => failures.push(format!("{}: {error}", path.display())),
                        }
                    }
                    (removed, failures)
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>()
    });
    if cancellation.load(Ordering::Relaxed) {
        return Err(swix::command::CANCELLED.to_owned());
    }
    let mut removed = 0_u64;
    let mut failures = Vec::new();
    for (worker_removed, mut worker_failures) in results {
        removed = removed.saturating_add(worker_removed);
        failures.append(&mut worker_failures);
    }
    if failures.is_empty() {
        Ok(removed)
    } else {
        Err(format!(
            "removed {} item(s), but {} path(s) failed: {}",
            group.items.len().saturating_sub(failures.len()),
            failures.len(),
            failures[0]
        ))
    }
}

fn remove_path(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn display_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .map(|relative| format!("~/{}", relative.display()))
        .unwrap_or_else(|_| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::cleanup::model::CleanupKind;

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
            let root = home().unwrap().join(format!(
                ".cache/swix-clean-group-test-{}-{id}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn removes_independent_paths_and_totals_their_sizes() {
        let fixture = Fixture::new();
        let mut items = Vec::new();
        for index in 0..8 {
            let path = fixture.root.join(format!("cache-{index}"));
            fs::create_dir(&path).unwrap();
            fs::write(path.join("data"), [index as u8; 64]).unwrap();
            items.push(CleanupItem {
                label: format!("cache-{index}"),
                detail: path.display().to_string(),
                bytes: Some(64),
                path: Some(path),
            });
        }
        let group = CleanupGroup {
            kind: CleanupKind::Caches,
            available: true,
            reclaimable: 8 * 64,
            items,
            note: None,
        };

        assert_eq!(
            clean_group(&group, &AtomicBool::new(false)).unwrap(),
            8 * 64
        );
        assert!(group.items.iter().all(|item| {
            item.path
                .as_ref()
                .is_some_and(|path| !path.try_exists().unwrap())
        }));
    }
}
