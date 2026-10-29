use std::{
    cell::RefCell,
    collections::HashMap,
    sync::mpsc::{self, Receiver, Sender},
};

use super::{
    Event, State,
    history::{self, Write},
    lock,
    notification::{Notification, Reason, Signal},
    server,
    settings::Settings,
};
use crate::{
    features::{
        Feed,
        notice::{self, Origin, Post, Reply},
    },
    supervisor::Ended,
};

pub struct Notifications {
    pub(super) feed: Feed<State, Event>,
    /// The truth the published state copies.
    pub(super) store: RefCell<State>,
    pub(super) signals: Sender<Signal>,
    pub(super) history: Sender<Write>,
    pub(super) history_days: u32,
    pub(super) private_apps: Vec<String>,
    pub(super) hide_on_lock: bool,
    /// Those features posted that island still has, by id.
    pub(super) posted: RefCell<HashMap<u32, Origin>>,
}

impl Notifications {
    pub(super) fn start(feed: Feed<State, Event>) -> Self {
        let settings: Settings = feed.settings();
        let (signals, outgoing) = mpsc::channel();
        feed.clone()
            .supervise(move |feed| server::serve(feed, &outgoing));
        feed.clone().supervise(lock::follow);
        if let Some(posts) = notice::take() {
            feed.clone().supervise(move |feed| deliver(feed, &posts));
        }
        let (history, writes) = mpsc::channel();
        let days = settings.history_days;
        feed.clone()
            .supervise(move |feed| history::work(feed, &writes, days));
        let notifications = Self {
            feed,
            store: RefCell::default(),
            signals,
            history,
            history_days: settings.history_days,
            private_apps: settings
                .privacy
                .private_apps
                .iter()
                .map(|app| app.to_lowercase())
                .collect(),
            hide_on_lock: settings.privacy.hide_on_lock,
            posted: RefCell::default(),
        };
        notifications.change(|store| store.private = settings.privacy.private);
        notifications
    }

    pub(super) fn change(&self, change: impl FnOnce(&mut State)) {
        let mut store = self.store.borrow_mut();
        change(&mut store);
        store.version += 1;
        self.feed.publish(store.clone());
    }

    /// To its app over D-Bus, or to the feature that posted it.
    pub(super) fn signal(&self, signal: Signal) {
        let id = match &signal {
            Signal::Closed(id, _) | Signal::Action(id, _) => *id,
        };
        let Some(origin) = self.posted.borrow().get(&id).cloned() else {
            let _ = self.signals.send(signal);
            return;
        };
        match signal {
            Signal::Action(_, action) => {
                notice::reply(&origin.feature, Reply::Action(origin.key, action));
            }
            Signal::Closed(_, reason) => {
                self.posted.borrow_mut().remove(&id);
                if reason == Reason::Dismissed {
                    notice::reply(&origin.feature, Reply::Dismissed(origin.key));
                }
            }
        }
    }

    /// The id the one `feature` posted under `key` has.
    pub(super) fn posted_id(&self, feature: &str, key: &str) -> Option<u32> {
        self.posted
            .borrow()
            .iter()
            .find(|(_, origin)| origin.feature == feature && origin.key == key)
            .map(|(id, _)| *id)
    }

    pub(super) fn hidden(&self, state: &State, notification: &Notification) -> bool {
        state.private
            || (self.hide_on_lock && state.locked)
            || self.private_apps.contains(&notification.app.to_lowercase())
    }
}

fn deliver(feed: &Feed<State, Event>, posts: &Receiver<Post>) -> Ended {
    for post in posts {
        if !feed.send(Event::Posted(post)) {
            break;
        }
    }
    Ended::Unwanted
}
