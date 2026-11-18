//! The open windows, most recently focused first, kept apart from the workspaces' [`Snapshot`]
//! so that a window's title or focus changing reaches only what follows windows.
//!
//! [`Snapshot`]: crate::wm::Snapshot

use std::sync::{Arc, LazyLock, Mutex};

use crate::{sync::lock, wm::Window};

type Listener = Box<dyn Fn() + Send>;

static WINDOWS: LazyLock<Mutex<Arc<Vec<Window>>>> = LazyLock::new(Mutex::default);
static LISTENERS: LazyLock<Mutex<Vec<Listener>>> = LazyLock::new(Mutex::default);

/// The windows as they are now; the same list until they change.
pub fn windows() -> Arc<Vec<Window>> {
    lock(&WINDOWS).clone()
}

/// Calls `listener`, from the compositor's worker, each time the windows change.
pub fn on_change(listener: impl Fn() + Send + 'static) {
    lock(&LISTENERS).push(Box::new(listener));
}

/// The compositor's worker says what the windows are now.
pub(super) fn set(windows: &[Window]) {
    {
        let mut current = lock(&WINDOWS);
        if current.as_slice() == windows {
            return;
        }
        *current = Arc::new(windows.to_vec());
    }
    for listener in lock(&LISTENERS).iter() {
        listener();
    }
}
