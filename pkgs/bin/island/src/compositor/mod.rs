//! A new compositor implements [`Compositor`] and is listed once in `registry.rs`; the rest of
//! island only sees [`Snapshot`](crate::wm::Snapshot), [`windows`] and [`backend`].

mod backend;
mod hyprland;
mod registry;
mod windows;

pub use backend::{Compositor, backend};
pub use registry::start;
pub use windows::{on_change, windows};
