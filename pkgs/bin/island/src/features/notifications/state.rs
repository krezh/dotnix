use std::sync::Arc;

use super::{history::Earlier, notification::Notification};

pub enum Event {
    /// New, or in place of one the app sent before.
    Show(Notification),
    Close(u32),
    Locked(bool),
    Clear,
    /// Timed out on one island; once no island shows it, it waits in the missed list, or its app
    /// hears of it when transient.
    Expired(Notification),
    /// Dismissed or acted on, on one island; it goes from the others.
    Withdrawn(u32),
    History(Vec<Earlier>),
}

#[derive(Clone, Default)]
pub struct State {
    pub(super) missed: Arc<Vec<Notification>>,
    /// This session's included.
    pub(super) history: Arc<Vec<Earlier>>,
    pub(super) dnd: bool,
    pub(super) private: bool,
    pub(super) locked: bool,
    /// Counts every change, so comparing two states does not compare every notification.
    pub(super) version: u64,
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}
