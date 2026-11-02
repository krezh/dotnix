//! Which island this is: the real one, or a preview running beside it with its own data and names,
//! which leaves everything only one island can own to the real one.

use std::sync::atomic::{AtomicBool, Ordering};

const REAL: &str = "io.github.krezh.Island";
pub const PREVIEW_NAME: &str = "io.github.krezh.Island.Preview";

/// Every bus name an island can hold.
pub const INSTANCES: [&str; 2] = [REAL, PREVIEW_NAME];

static PREVIEW: AtomicBool = AtomicBool::new(false);

/// Before anything asks for a folder or a name.
pub fn enter_preview() {
    PREVIEW.store(true, Ordering::Relaxed);
}

pub fn preview() -> bool {
    PREVIEW.load(Ordering::Relaxed)
}

fn named(real: &'static str, preview: &'static str) -> &'static str {
    if self::preview() { preview } else { real }
}

/// Where its state, cache and runtime files go.
pub fn folder() -> &'static str {
    named("island", "island-preview")
}

/// The bus name `island ctl` reaches it at.
pub fn instance() -> &'static str {
    named(REAL, PREVIEW_NAME)
}

/// Its layer surfaces' namespace.
pub fn namespace() -> &'static str {
    named("island", "island-preview")
}
