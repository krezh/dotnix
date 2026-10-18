use std::sync::Arc;

use super::{
    Notifications,
    notification::{DEFAULT_ACTION, Notification, Reason, Signal},
};

const KEEP: usize = 50;

impl Notifications {
    pub(super) fn add_missed(&self, notification: Notification) {
        let mut dropped = Vec::new();
        self.change(|store| {
            let missed = Arc::make_mut(&mut store.missed);
            match missed.iter().position(|item| item.id == notification.id) {
                Some(index) => missed[index] = notification,
                None => missed.insert(0, notification),
            }
            dropped = missed.split_off(missed.len().min(KEEP));
        });
        for notification in dropped {
            self.signal(Signal::Closed(notification.id, Reason::Expired));
        }
    }

    pub(super) fn take_missed(&self, id: u32) -> Option<Notification> {
        let index = self
            .store
            .borrow()
            .missed
            .iter()
            .position(|item| item.id == id)?;
        let mut taken = None;
        self.change(|store| taken = Some(Arc::make_mut(&mut store.missed).remove(index)));
        taken
    }

    pub(super) fn dismiss_missed(&self, id: u32) {
        if self.take_missed(id).is_some() {
            self.signal(Signal::Closed(id, Reason::Dismissed));
        }
    }

    pub(super) fn close_missed(&self, id: u32) {
        if self.take_missed(id).is_some() {
            self.signal(Signal::Closed(id, Reason::Closed));
        }
    }

    pub(super) fn invoke_missed(&self, id: u32, key: &str) {
        if self.take_missed(id).is_some() {
            self.signal(Signal::Action(id, key.to_owned()));
            self.signal(Signal::Closed(id, Reason::Dismissed));
        }
    }

    pub(super) fn activate_missed(&self, id: u32) {
        let default = self
            .store
            .borrow()
            .missed
            .iter()
            .any(|item| item.id == id && item.default_action());
        if default {
            self.invoke_missed(id, DEFAULT_ACTION);
        } else {
            self.dismiss_missed(id);
        }
    }

    pub(super) fn clear(&self) {
        let mut cleared = Vec::new();
        self.change(|store| cleared = std::mem::take(Arc::make_mut(&mut store.missed)));
        for notification in cleared {
            self.signal(Signal::Closed(notification.id, Reason::Dismissed));
        }
    }
}
