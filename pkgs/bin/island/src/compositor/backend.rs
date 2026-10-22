//! What a compositor provides, [`Compositor`], and the one island runs on, reached through
//! [`backend`].

use std::sync::{Arc, OnceLock};

use futures::channel::mpsc::{UnboundedReceiver, unbounded};
use tracing::{info, warn};

use crate::{features::Feed, wm::Snapshot};

pub trait Compositor: Send + Sync + Sized + 'static {
    const NAME: &'static str;

    fn detect() -> bool;

    /// Publishes a [`Snapshot`] on every change; `None` when it cannot connect.
    fn start(feed: Feed<Snapshot>) -> Option<Self>;

    fn focus_workspace(&self, id: i64);

    fn focus_relative(&self, step: i32);

    /// So the island lines up with windows.
    fn top_gap(&self) -> Option<f32> {
        None
    }

    fn focused_class(&self) -> Option<String> {
        None
    }

    /// Where this process's island surface on `monitor` sits on the desktop, in logical pixels.
    /// Another bar's exclusive zone moves it off the monitor's top edge.
    fn surface_origin(&self, _monitor: &str) -> Option<(i32, i32)> {
        None
    }
}

pub trait Backend: Send + Sync {
    fn focus_workspace(&self, id: i64);
    fn focus_relative(&self, step: i32);
    fn top_gap(&self) -> Option<f32>;
    fn focused_class(&self) -> Option<String>;
    fn surface_origin(&self, monitor: &str) -> Option<(i32, i32)>;
}

impl<C: Compositor> Backend for C {
    fn focus_workspace(&self, id: i64) {
        Compositor::focus_workspace(self, id);
    }

    fn focus_relative(&self, step: i32) {
        Compositor::focus_relative(self, step);
    }

    fn top_gap(&self) -> Option<f32> {
        Compositor::top_gap(self)
    }

    fn focused_class(&self) -> Option<String> {
        Compositor::focused_class(self)
    }

    fn surface_origin(&self, monitor: &str) -> Option<(i32, i32)> {
        Compositor::surface_origin(self, monitor)
    }
}

static BACKEND: OnceLock<Arc<dyn Backend>> = OnceLock::new();

pub fn backend() -> Option<Arc<dyn Backend>> {
    BACKEND.get().cloned()
}

pub(super) fn try_start<C: Compositor>() -> Option<UnboundedReceiver<Snapshot>> {
    if !C::detect() {
        return None;
    }
    let (sender, snapshots) = unbounded();
    let Some(compositor) = C::start(Feed::new(C::NAME, sender)) else {
        warn!(compositor = C::NAME, "detected but cannot connect");
        return None;
    };
    let _ = BACKEND.set(Arc::new(compositor));
    info!(compositor = C::NAME, "using the compositor");
    Some(snapshots)
}
