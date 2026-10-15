//! Compositor backends: where island learns about monitors, workspaces and fullscreen windows, and
//! how it switches workspaces. Exactly one runs, picked at startup by [`start`]; island itself only
//! sees the neutral [`Snapshot`] model and [`backend`]. A new compositor implements [`Compositor`]
//! and is listed once in [`compositors!`].

mod hyprland;

use std::sync::{Arc, OnceLock};

use futures::channel::mpsc::{UnboundedReceiver, unbounded};
use tracing::{info, warn};

use crate::{features::Feed, wm::Snapshot};

pub trait Compositor: Send + Sync + Sized + 'static {
    const NAME: &'static str;

    /// Whether this compositor is the one running.
    fn detect() -> bool;

    /// Connects and starts the supervised event worker, publishing a [`Snapshot`] on every change;
    /// `None` when it cannot connect.
    fn start(feed: Feed<Snapshot>) -> Option<Self>;

    /// Switches to the workspace with this [`crate::wm::Workspace::id`].
    fn focus_workspace(&self, id: i64);

    /// Moves `step` workspaces along on the focused monitor.
    fn focus_relative(&self, step: i32);

    /// The compositor's gap above windows, so the island lines up with them.
    fn top_gap(&self) -> Option<f32> {
        None
    }
}

/// The running compositor with its type erased.
pub trait Backend: Send + Sync {
    fn focus_workspace(&self, id: i64);
    fn focus_relative(&self, step: i32);
    fn top_gap(&self) -> Option<f32>;
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
}

static BACKEND: OnceLock<Arc<dyn Backend>> = OnceLock::new();

/// The running compositor, if island found one it supports.
pub fn backend() -> Option<Arc<dyn Backend>> {
    BACKEND.get().cloned()
}

fn try_start<C: Compositor>() -> Option<UnboundedReceiver<Snapshot>> {
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

macro_rules! compositors {
    ($($compositor:ty),* $(,)?) => {
        /// Starts the first supported compositor that is running. Without one, island still opens
        /// on every output, with no workspaces and no fullscreen hiding.
        pub fn start() -> Option<UnboundedReceiver<Snapshot>> {
            let snapshots = None$(.or_else(try_start::<$compositor>))*;
            if snapshots.is_none() {
                warn!("no supported compositor is running; workspaces are off");
            }
            snapshots
        }
    };
}

compositors![hyprland::Hyprland];
