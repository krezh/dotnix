//! User interface domain
//!
//! Handles user-facing UI components including notifications and Wayland selection overlay.

pub mod notify;
pub mod wayland;

pub use notify::Notifier;
pub use wayland::App;
