//! Locking, sleeping and ending the session; ending it closes every app first, then switches the
//! island off like an old screen.

mod action;
mod apps;
mod feature;
mod hyprland;
mod local;
mod logind;
mod popup;
mod process;
mod settings;
mod state;
mod surface;
mod worker;

pub use action::Action;
pub use feature::Session;
pub use logind::logind;
