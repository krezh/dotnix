use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        LazyLock, Mutex, MutexGuard,
        atomic::{AtomicU32, Ordering},
    },
    time::{Duration, SystemTime},
};

use crate::{components::icon::Picture, features::notice::Origin, sync::lock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expire {
    Default,
    Never,
    After(Duration),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    pub key: String,
    pub label: String,
    /// The icon its label names, when the app asked for icons instead of text (`action-icons`).
    pub icon: Option<PathBuf>,
    /// A panel, by name, and its view, that it opens, for one a feature posted.
    pub opens: Option<(String, String)>,
}

#[derive(Clone, Debug)]
pub struct Notification {
    pub id: u32,
    pub app: String,
    pub summary: String,
    pub body: String,
    pub critical: bool,
    pub icon: Option<Picture>,
    pub image: Option<Picture>,
    /// The feature, by name, whose mark leads it in place of the icon (hint `x-island-lead`).
    pub lead: Option<String>,
    pub transient: bool,
    pub resident: bool,
    pub expire: Expire,
    pub actions: Vec<Action>,
    pub received: SystemTime,
    /// For one a feature posted itself; one from D-Bus has none.
    pub posted: Option<Posted>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Posted {
    pub origin: Origin,
    /// Names its source in the overline.
    pub via: bool,
    pub detail: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Expired = 1,
    Dismissed = 2,
    Closed = 3,
}

#[derive(Clone, Debug)]
pub enum Signal {
    Closed(u32, Reason),
    Action(u32, String),
}

pub const DEFAULT_ACTION: &str = "default";

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

static OWNERS: LazyLock<Mutex<HashMap<u32, String>>> = LazyLock::new(Mutex::default);

fn owners() -> MutexGuard<'static, HashMap<u32, String>> {
    lock(&OWNERS)
}

fn fresh_id() -> u32 {
    loop {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        if id != 0 {
            return id;
        }
    }
}

/// Keeps an id an app chose, known or not: scripts replace by a fixed id of their own. Owned by
/// another app, or 0, it gives a fresh one that nothing holds.
fn claim_from(
    owners: &mut HashMap<u32, String>,
    replaces: u32,
    app: &str,
    mut fresh: impl FnMut() -> u32,
) -> u32 {
    let kept = replaces != 0 && owners.get(&replaces).is_none_or(|owner| owner == app);
    let id = if kept {
        replaces
    } else {
        loop {
            let id = fresh();
            if !owners.contains_key(&id) {
                break id;
            }
        }
    };
    owners.insert(id, app.to_owned());
    id
}

pub fn claim_id(replaces: u32, app: &str) -> u32 {
    claim_from(&mut owners(), replaces, app, fresh_id)
}

pub fn next_id() -> u32 {
    claim_id(0, "island")
}

pub fn release_id(id: u32) {
    owners().remove(&id);
}

impl Notification {
    pub fn default_action(&self) -> bool {
        self.actions
            .iter()
            .any(|action| action.key == DEFAULT_ACTION)
    }

    pub fn buttons(&self) -> impl Iterator<Item = &Action> {
        self.actions
            .iter()
            .filter(|action| action.key != DEFAULT_ACTION)
    }

    /// Where it is from, as the panel's sources name it; `None` for a desktop app's.
    pub fn source(&self) -> Option<&str> {
        self.posted
            .as_ref()
            .map(|posted| posted.origin.source.as_str())
    }

    pub fn timeout(&self, default: Duration) -> Option<Duration> {
        if self.critical {
            return None;
        }
        match self.expire {
            Expire::Default => Some(default),
            Expire::Never => None,
            Expire::After(after) => Some(after),
        }
    }
}

#[cfg(test)]
#[path = "notification_test.rs"]
mod tests;
