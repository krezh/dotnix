use std::{
    sync::atomic::{AtomicU32, Ordering},
    time::{Duration, SystemTime},
};

use crate::components::icon::Picture;

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
    pub transient: bool,
    pub resident: bool,
    pub expire: Expire,
    pub actions: Vec<Action>,
    pub received: SystemTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    Expired = 1,
    Dismissed = 2,
    Closed = 3,
}

/// What island tells the app that sent a notification.
#[derive(Clone, Debug)]
pub enum Signal {
    Closed(u32, Reason),
    Action(u32, String),
}

pub const DEFAULT_ACTION: &str = "default";

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

pub fn next_id() -> u32 {
    loop {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        if id != 0 {
            return id;
        }
    }
}

impl Notification {
    pub fn default_action(&self) -> bool {
        self.actions.iter().any(|action| action.key == DEFAULT_ACTION)
    }

    pub fn buttons(&self) -> impl Iterator<Item = &Action> {
        self.actions.iter().filter(|action| action.key != DEFAULT_ACTION)
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
