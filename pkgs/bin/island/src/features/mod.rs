//! A feature implements [`Feature`] and is listed once in `registry.rs`; nothing else names it.

pub mod ai;
pub mod audio;
pub mod battery;
pub mod bluetooth;
pub mod brightness;
pub mod caffeine;
pub mod clipboard;
pub mod clock;
pub mod gotify;
pub mod headset;
pub mod launcher;
pub mod media;
pub mod molt;
pub mod network;
pub mod notice;
pub mod notifications;
pub mod osd;
pub mod session;
pub mod tray;
pub mod workspaces;
pub mod zowie;

mod feature;
mod feed;
mod instance;
mod islands;
mod panel_cx;
mod registry;
mod widget_cx;

pub use battery::Battery;
pub use feature::{AnyLocal, Dashboard, Feature, GlyphSpec, IslandInfo, Local, Outcome};
pub use feed::{Feed, Refreshed, parse_settings};
pub use instance::Features;
pub use islands::Islands;
pub use osd::Osd;
pub use panel_cx::PanelCx;
pub use registry::*;
pub use widget_cx::WidgetCx;
