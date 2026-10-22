//! Every compositor island supports, listed once, tried in order.

use futures::channel::mpsc::UnboundedReceiver;
use tracing::warn;

use super::{backend::try_start, hyprland};
use crate::wm::Snapshot;

macro_rules! compositors {
    ($($compositor:ty),* $(,)?) => {
        /// Without a supported compositor, island still opens on every output, with no workspaces
        /// and no fullscreen hiding.
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
