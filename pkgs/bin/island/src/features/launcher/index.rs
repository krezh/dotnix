use std::{
    collections::{HashMap, HashSet, VecDeque},
    ffi::OsString,
    fs,
    os::unix::fs::MetadataExt,
    path::{Component, Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        mpsc::{self, Receiver, SyncSender},
    },
    time::{Duration, Instant},
};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::desktop::{self, App};

const QUIET: Duration = Duration::from_millis(200);
const MAX_BATCH: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub(super) struct Refresh {
    sender: SyncSender<()>,
    error: Arc<Mutex<Option<String>>>,
}

impl Refresh {
    pub(super) fn channel() -> (Self, Receiver<()>) {
        let (sender, receiver) = mpsc::sync_channel(1);
        (
            Self {
                sender,
                error: Arc::new(Mutex::new(None)),
            },
            receiver,
        )
    }

    pub(super) fn request(&self) {
        let _ = self.sender.try_send(());
    }

    fn fail(&self, error: String) {
        *self
            .error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(error);
        self.request();
    }

    fn check(&self) -> Result<(), String> {
        match self
            .error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    pub(super) fn wait(receiver: &Receiver<()>) -> bool {
        if receiver.recv().is_err() {
            return false;
        }
        let deadline = Instant::now() + MAX_BATCH;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() || receiver.recv_timeout(QUIET.min(remaining)).is_err() {
                return true;
            }
        }
    }
}

#[derive(Default, PartialEq, Eq)]
struct Interest {
    children: HashSet<OsString>,
    desktops: bool,
}

type Plan = HashMap<PathBuf, Interest>;

fn immutable(path: &Path) -> bool {
    path.starts_with("/nix/store")
}

fn child(plan: &mut Plan, parent: &Path, name: &std::ffi::OsStr) {
    if !immutable(parent) {
        if let Some(interest) = plan.get_mut(parent) {
            if !interest.desktops
                || Path::new(name)
                    .extension()
                    .is_none_or(|extension| extension != "desktop")
            {
                if !interest.children.contains(name) {
                    interest.children.insert(name.to_owned());
                }
            }
        } else {
            plan.insert(
                parent.to_owned(),
                Interest {
                    children: HashSet::from([name.to_owned()]),
                    desktops: false,
                },
            );
        }
    }
}

fn components(path: &Path) -> impl DoubleEndedIterator<Item = OsString> + '_ {
    path.components().map(|part| part.as_os_str().to_owned())
}

/** Track every mutable link boundary, including links reached through another link's target. */
fn resolve(path: &Path, plan: &mut Plan) -> Option<PathBuf> {
    let mut pending: VecDeque<_> = components(path).collect();
    let mut resolved = if path.is_absolute() {
        PathBuf::from("/")
    } else {
        std::env::current_dir().ok()?
    };
    let mut links = 0;
    while let Some(name) = pending.pop_front() {
        match Path::new(&name).components().next()? {
            Component::RootDir => {
                resolved = PathBuf::from("/");
                continue;
            }
            Component::CurDir => continue,
            Component::ParentDir => {
                resolved.pop();
                continue;
            }
            _ => {}
        }
        child(plan, &resolved, &name);
        let next = resolved.join(&name);
        let metadata = fs::symlink_metadata(&next).ok()?;
        if metadata.is_symlink() {
            links += 1;
            if links > 40 {
                return None;
            }
            let target = fs::read_link(&next).ok()?;
            for part in components(&target).rev() {
                pending.push_front(part);
            }
        } else {
            resolved = next;
        }
    }
    Some(resolved)
}

fn plan(dirs: &[PathBuf]) -> Plan {
    let mut plan = Plan::new();
    for dir in dirs {
        let Some(resolved) = resolve(dir, &mut plan) else {
            continue;
        };
        if !immutable(&resolved) {
            plan.entry(resolved).or_default().desktops = true;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "desktop")
                && entry.file_type().is_ok_and(|kind| kind.is_symlink())
            {
                resolve(&path, &mut plan);
            }
        }
    }
    plan
}

fn relevant(plan: &Plan, event: &Event) -> bool {
    if event.need_rescan() {
        return true;
    }
    if matches!(event.kind, EventKind::Access(_)) {
        return false;
    }
    event.paths.iter().any(|path| {
        if plan.contains_key(path) {
            return true;
        }
        let Some(interest) = path.parent().and_then(|parent| plan.get(parent)) else {
            return false;
        };
        path.file_name()
            .is_some_and(|name| interest.children.contains(name))
            || (interest.desktops
                && path
                    .extension()
                    .is_some_and(|extension| extension == "desktop"))
    })
}

pub(super) struct Index {
    dirs: Vec<PathBuf>,
    watcher: RecommendedWatcher,
    watched: HashMap<PathBuf, (u64, u64)>,
    interests: Arc<RwLock<Plan>>,
    refresh: Refresh,
    apps: Option<Arc<Vec<App>>>,
}

impl Index {
    pub(super) fn new(dirs: Vec<PathBuf>, refresh: Refresh) -> Result<Self, String> {
        let interests = Arc::new(RwLock::new(Plan::new()));
        let callback_interests = interests.clone();
        let callback_refresh = refresh.clone();
        let watcher =
            notify::recommended_watcher(move |event: notify::Result<Event>| match event {
                Ok(event) => {
                    let interests = callback_interests
                        .read()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    if relevant(&interests, &event) {
                        callback_refresh.request();
                    }
                }
                Err(error) => callback_refresh.fail(format!("application watcher: {error}")),
            })
            .map_err(|error| format!("cannot start application watcher: {error}"))?;
        Ok(Self {
            dirs,
            watcher,
            watched: HashMap::new(),
            interests,
            refresh,
            apps: None,
        })
    }

    fn reconcile(&mut self) -> Result<bool, String> {
        self.refresh.check()?;
        let next = plan(&self.dirs);
        let identities: HashMap<_, _> = next
            .keys()
            .filter_map(|path| {
                let metadata = fs::metadata(path).ok()?;
                metadata
                    .is_dir()
                    .then(|| (path.clone(), (metadata.dev(), metadata.ino())))
            })
            .collect();
        let changed = self.watched != identities
            || *self
                .interests
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                != next;
        *self
            .interests
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = next;

        for (path, identity) in &identities {
            if self.watched.get(path) == Some(identity) {
                continue;
            }
            if self.watched.remove(path).is_some() {
                let _ = self.watcher.unwatch(path);
            }
            match self.watcher.watch(path, RecursiveMode::NonRecursive) {
                Ok(()) => {
                    self.watched.insert(path.clone(), *identity);
                }
                Err(_) if !path.is_dir() => self.refresh.request(),
                Err(error) => return Err(format!("cannot watch {}: {error}", path.display())),
            }
        }
        let obsolete: Vec<_> = self
            .watched
            .keys()
            .filter(|path| !identities.contains_key(*path))
            .cloned()
            .collect();
        for path in obsolete {
            self.watched.remove(&path);
            let _ = self.watcher.unwatch(&path);
        }
        Ok(changed)
    }

    pub(super) fn scan(&mut self) -> Result<Option<Arc<Vec<App>>>, String> {
        self.reconcile()?;
        let apps = desktop::load(&self.dirs);
        if self.reconcile()? {
            self.refresh.request();
            return Ok(None);
        }
        if self
            .apps
            .as_ref()
            .is_some_and(|previous| **previous == apps)
        {
            return Ok(None);
        }
        let apps = Arc::new(apps);
        self.apps = Some(apps.clone());
        Ok(Some(apps))
    }
}

#[cfg(test)]
#[path = "index_test.rs"]
mod tests;
