//! How a feature's worker reaches the islands: its state, live levels, events and settings.

use futures::channel::mpsc::UnboundedSender;
use serde::de::DeserializeOwned;
use tracing::error;

use crate::supervisor::{self, Ended};

pub struct Feed<S, E = ()> {
    pub(super) name: &'static str,
    pub(super) sender: UnboundedSender<S>,
    pub(super) live: Option<UnboundedSender<Vec<f32>>>,
    pub(super) events: Option<UnboundedSender<E>>,
    pub(super) settings: Option<toml::Value>,
}

impl<S, E> Clone for Feed<S, E> {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            sender: self.sender.clone(),
            live: self.live.clone(),
            events: self.events.clone(),
            settings: self.settings.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Refreshed<T> {
    pub value: T,
    /// A fresh reading asked for is on its way, so the widget can show it is busy.
    pub refreshing: bool,
}

pub fn parse_settings<T: DeserializeOwned>(settings: &toml::Value) -> Result<T, String> {
    settings
        .clone()
        .try_into()
        .map_err(|error: toml::de::Error| error.to_string().trim().to_owned())
}

impl<S: Send + 'static, E: Send + 'static> Feed<S, E> {
    pub(crate) fn new(name: &'static str, sender: UnboundedSender<S>) -> Self {
        Self {
            name,
            sender,
            live: None,
            events: None,
            settings: None,
        }
    }

    pub fn publish(&self, state: S) -> bool {
        self.sender.unbounded_send(state).is_ok()
    }

    pub fn send(&self, event: E) -> bool {
        self.events
            .as_ref()
            .is_some_and(|events| events.unbounded_send(event).is_ok())
    }

    pub fn settings<T: DeserializeOwned + Default>(&self) -> T {
        self.settings
            .as_ref()
            .and_then(|settings| parse_settings(settings).ok())
            .unwrap_or_default()
    }

    pub fn wants_live(&self) -> bool {
        self.live.is_some()
    }

    pub fn publish_live(&self, levels: Vec<f32>) -> bool {
        self.live
            .as_ref()
            .is_some_and(|live| live.unbounded_send(levels).is_ok())
    }

    pub fn supervise(self, mut run: impl FnMut(&Feed<S, E>) -> Ended + Send + 'static) {
        let name = self.name;
        if let Err(error) = supervisor::supervise(name, move || run(&self)) {
            error!(feature = name, "cannot start the worker: {error}");
        }
    }
}
