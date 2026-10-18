use std::{
    cell::RefCell,
    sync::mpsc::{self, Sender},
};

use super::{
    Event, State,
    history::{self, Write},
    lock,
    notification::{Notification, Signal},
    server,
    settings::Settings,
};
use crate::features::Feed;

pub struct Notifications {
    pub(super) feed: Feed<State, Event>,
    /// The truth the published state copies.
    pub(super) store: RefCell<State>,
    pub(super) signals: Sender<Signal>,
    pub(super) history: Sender<Write>,
    pub(super) history_days: u32,
    pub(super) private_apps: Vec<String>,
    pub(super) hide_on_lock: bool,
}

impl Notifications {
    pub(super) fn start(feed: Feed<State, Event>) -> Self {
        let settings: Settings = feed.settings();
        let (signals, outgoing) = mpsc::channel();
        feed.clone()
            .supervise(move |feed| server::serve(feed, &outgoing));
        feed.clone().supervise(lock::follow);
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

    pub(super) fn signal(&self, signal: Signal) {
        let _ = self.signals.send(signal);
    }

    pub(super) fn hidden(&self, state: &State, notification: &Notification) -> bool {
        state.private
            || (self.hide_on_lock && state.locked)
            || self.private_apps.contains(&notification.app.to_lowercase())
    }
}
