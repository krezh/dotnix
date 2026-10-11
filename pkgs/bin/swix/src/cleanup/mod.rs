mod caches;
mod development;
mod disk;
mod estimate;
mod docker;
mod filesystem;
mod games;
mod journal;
mod model;
mod nix;
mod trash;
pub(crate) mod ui;

use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use swix::command::{self, OutputLimits};

use crate::activation;
use crate::config::CleanupPolicy;

pub(crate) use estimate::CleanupEstimate;
pub(crate) use model::{
    CleanupEvent, CleanupGroup, CleanupItem, CleanupKind, DiskUsage, ScanEvent, Volume,
};

/// Scans `kinds`, sending each group as it finishes, then the cleanup volumes.
pub(crate) fn scan(
    kinds: &[CleanupKind],
    policy: CleanupPolicy,
    cancellation: Arc<AtomicBool>,
    sender: Sender<ScanEvent>,
) {
    thread::scope(|scope| {
        for &kind in kinds {
            let cancellation = Arc::clone(&cancellation);
            let sender = sender.clone();
            let policy = policy.clone();
            scope.spawn(move || {
                let group = match kind {
                    CleanupKind::Nix => nix::scan(&cancellation, policy.keep_generations),
                    CleanupKind::Docker => docker::scan(&cancellation),
                    CleanupKind::Development => development::scan(&cancellation),
                    CleanupKind::Caches => caches::scan(&cancellation),
                    CleanupKind::Games => games::scan_leftovers(&cancellation),
                    CleanupKind::GamePrefixes => games::scan_prefixes(&cancellation),
                    CleanupKind::Trash => trash::scan(&cancellation),
                    CleanupKind::Journals => journal::scan(),
                };
                if !cancellation.load(Ordering::Relaxed) {
                    let _ = sender.send(ScanEvent::Group(group));
                }
            });
        }
    });
    if !cancellation.load(Ordering::Relaxed) {
        let _ = sender.send(ScanEvent::Complete(disk::cleanup_volumes()));
    }
}

/// Cleans every selected group at once: each file cleaner and Docker on its own thread, and
/// journals then Nix together on one more, since the privileged helper takes a single request at
/// a time.
pub(crate) fn clean(
    groups: Vec<CleanupGroup>,
    policy: CleanupPolicy,
    cancellation: Arc<AtomicBool>,
    sender: Sender<CleanupEvent>,
) {
    let started = Instant::now();
    let before = disk::cleanup_volumes();
    let groups = groups
        .into_iter()
        .map(|group| (group.kind, group))
        .collect::<HashMap<_, _>>();
    let freed = Freed::default();

    thread::scope(|scope| {
        for (&kind, group) in &groups {
            if matches!(kind, CleanupKind::Nix | CleanupKind::Journals) {
                continue;
            }
            let (cancellation, sender, freed) = (&*cancellation, sender.clone(), &freed);
            scope.spawn(move || {
                let _ = sender.send(CleanupEvent::Started(kind));
                let result = match kind {
                    CleanupKind::Docker => docker::clean(cancellation).inspect(|bytes| {
                        freed.record_at(Path::new("/var/lib/docker"), *bytes);
                    }),
                    CleanupKind::Games => games::clean_leftovers(group, cancellation, freed),
                    CleanupKind::GamePrefixes => games::clean_prefixes(group, cancellation, freed),
                    _ => filesystem::clean_group(group, cancellation, freed),
                };
                let _ = sender.send(CleanupEvent::Finished(kind, result));
            });
        }
        scope.spawn(|| clean_system_groups(&groups, &policy, &cancellation, &sender, &freed));
    });

    let _ = sender.send(CleanupEvent::Complete {
        before,
        after: disk::cleanup_volumes(),
        elapsed: started.elapsed(),
    });
}

fn clean_system_groups(
    groups: &HashMap<CleanupKind, CleanupGroup>,
    policy: &CleanupPolicy,
    cancellation: &AtomicBool,
    sender: &Sender<CleanupEvent>,
    freed: &Freed,
) {
    if cancellation.load(Ordering::Relaxed) {
        return;
    }

    if groups.contains_key(&CleanupKind::Journals) {
        let journals = Path::new("/var/log");
        let before = FreeSpace::of(journals, freed);
        let _ = sender.send(CleanupEvent::Started(CleanupKind::Journals));
        let result = activation::clean_system(false, true, policy, cancellation)
            .map(|()| reclaimed_since(before, journals, freed));
        let _ = sender.send(CleanupEvent::Finished(CleanupKind::Journals, result));
    }

    if cancellation.load(Ordering::Relaxed) {
        return;
    }
    if !groups.contains_key(&CleanupKind::Nix) {
        return;
    }
    let store = Path::new("/nix");
    let before = FreeSpace::of(store, freed);
    let _ = sender.send(CleanupEvent::Started(CleanupKind::Nix));
    let user_result = nix::clean_user_profiles(policy.keep_generations, cancellation);
    let system_result = activation::clean_system(true, false, policy, cancellation);
    let result = match (&user_result, &system_result) {
        (Ok(()), Ok(())) => Ok(reclaimed_since(before, store, freed)),
        (Err(user_error), Ok(())) => Err(format!(
            "system store collected, but user profile cleanup failed: {user_error}"
        )),
        (_, Err(system_error)) if !cancellation.load(Ordering::Relaxed) => {
            match nix::collect_store(cancellation) {
                Ok(()) => Err(format!(
                    "store collected, but privileged cleanup failed: {system_error}"
                )),
                Err(fallback_error) => Err(format!(
                    "{system_error}; local fallback failed: {fallback_error}"
                )),
            }
        }
        (_, Err(system_error)) => Err(system_error.clone()),
    };
    let _ = sender.send(CleanupEvent::Finished(CleanupKind::Nix, result));
}

/// Bytes the other cleaners removed, per filesystem device. Journals and Nix measure what they
/// free by the change in free space, so they leave out what others freed on the same filesystem
/// meanwhile.
#[derive(Default)]
pub(super) struct Freed(Mutex<HashMap<u64, u64>>);

impl Freed {
    pub(super) fn record(&self, device: u64, bytes: u64) {
        let mut devices = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let total = devices.entry(device).or_default();
        *total = total.saturating_add(bytes);
    }

    fn record_at(&self, path: &Path, bytes: u64) {
        if let Ok(metadata) = fs::metadata(path) {
            self.record(metadata.dev(), bytes);
        }
    }

    fn on(&self, device: u64) -> u64 {
        let devices = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        devices.get(&device).copied().unwrap_or(0)
    }
}

/// A filesystem's free space, and how much of it other cleaners had freed so far.
struct FreeSpace {
    device: u64,
    available: u64,
    others: u64,
}

impl FreeSpace {
    fn of(path: &Path, freed: &Freed) -> Option<Self> {
        let device = fs::metadata(path).ok()?.dev();
        Some(Self {
            device,
            available: disk::usage(path)?.available,
            others: freed.on(device),
        })
    }
}

fn reclaimed_since(before: Option<FreeSpace>, path: &Path, freed: &Freed) -> u64 {
    let (Some(before), Some(after)) = (before, FreeSpace::of(path, freed)) else {
        return 0;
    };
    if before.device != after.device {
        return 0;
    }
    after
        .available
        .saturating_sub(before.available)
        .saturating_sub(after.others.saturating_sub(before.others))
}

pub(super) fn run_command(
    command: &mut Command,
    name: &str,
    cancellation: &AtomicBool,
    timeout: Duration,
    output_limit: usize,
) -> Result<Output, String> {
    command::run(
        command,
        name,
        cancellation,
        timeout,
        OutputLimits {
            stdout: output_limit,
            stderr: output_limit.min(4 * 1024 * 1024),
        },
        |_| {},
        command_error,
    )
}

fn command_error(output: &[u8]) -> String {
    String::from_utf8_lossy(output).trim().to_owned()
}
