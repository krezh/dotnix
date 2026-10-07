mod caches;
mod development;
mod disk;
mod docker;
mod filesystem;
mod journal;
mod model;
mod nix;
mod trash;

use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use swix::command::{self, OutputLimits};

use crate::activation;

pub(crate) use model::{
    CleanupEvent, CleanupGroup, CleanupItem, CleanupKind, DiskUsage, ScanEvent, format_size,
};

pub(crate) fn scan(cancellation: Arc<AtomicBool>, sender: Sender<ScanEvent>) {
    thread::scope(|scope| {
        for kind in CleanupKind::ALL {
            let cancellation = Arc::clone(&cancellation);
            let sender = sender.clone();
            scope.spawn(move || {
                let group = match kind {
                    CleanupKind::Nix => nix::scan(&cancellation),
                    CleanupKind::Docker => docker::scan(&cancellation),
                    CleanupKind::Development => development::scan(&cancellation),
                    CleanupKind::Caches => caches::scan(&cancellation),
                    CleanupKind::Trash => trash::scan(&cancellation),
                    CleanupKind::Journals => journal::scan(&cancellation),
                };
                if !cancellation.load(Ordering::Relaxed) {
                    let _ = sender.send(ScanEvent::Group(group));
                }
            });
        }
    });
    if !cancellation.load(Ordering::Relaxed) {
        let _ = sender.send(ScanEvent::Complete(disk::cleanup_usage()));
    }
}

pub(crate) fn clean(
    groups: Vec<CleanupGroup>,
    cancellation: Arc<AtomicBool>,
    sender: Sender<CleanupEvent>,
) {
    let started = Instant::now();
    let before = disk::cleanup_usage();
    let groups = groups
        .into_iter()
        .map(|group| (group.kind, group))
        .collect::<HashMap<_, _>>();

    thread::scope(|scope| {
        for kind in [
            CleanupKind::Development,
            CleanupKind::Caches,
            CleanupKind::Trash,
            CleanupKind::Docker,
        ] {
            let Some(group) = groups.get(&kind).cloned() else {
                continue;
            };
            let cancellation = Arc::clone(&cancellation);
            let sender = sender.clone();
            scope.spawn(move || {
                let _ = sender.send(CleanupEvent::Started(kind));
                let result = match kind {
                    CleanupKind::Docker => docker::clean(&cancellation),
                    _ => filesystem::clean_group(&group, &cancellation),
                };
                let _ = sender.send(CleanupEvent::Finished(kind, result));
            });
        }
    });

    clean_system_groups(&groups, &cancellation, &sender);
    let _ = sender.send(CleanupEvent::Complete {
        before,
        after: disk::cleanup_usage(),
        elapsed: started.elapsed(),
    });
}

fn clean_system_groups(
    groups: &HashMap<CleanupKind, CleanupGroup>,
    cancellation: &AtomicBool,
    sender: &Sender<CleanupEvent>,
) {
    if cancellation.load(Ordering::Relaxed) {
        return;
    }

    if groups.contains_key(&CleanupKind::Journals) {
        let before = disk::usage(Path::new("/var/log"));
        let _ = sender.send(CleanupEvent::Started(CleanupKind::Journals));
        let result = activation::clean_system(false, true, cancellation)
            .map(|()| reclaimed_since(before, Path::new("/var/log")));
        let _ = sender.send(CleanupEvent::Finished(CleanupKind::Journals, result));
    }

    if cancellation.load(Ordering::Relaxed) {
        return;
    }
    if !groups.contains_key(&CleanupKind::Nix) {
        return;
    }
    let before = disk::usage(Path::new("/nix"));
    let _ = sender.send(CleanupEvent::Started(CleanupKind::Nix));
    let user_result = nix::clean_user_profiles(cancellation);
    let system_result = activation::clean_system(true, false, cancellation);
    let result = match (&user_result, &system_result) {
        (Ok(()), Ok(())) => Ok(reclaimed_since(before, Path::new("/nix"))),
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

fn reclaimed_since(before: Option<DiskUsage>, path: &Path) -> u64 {
    before.zip(disk::usage(path)).map_or(0, |(before, after)| {
        after.available.saturating_sub(before.available)
    })
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
