//! An island without its drawing: everything a frontend shows and animates, so each frontend only
//! draws it and feeds it input.

mod morph;
mod motion;
mod panel;
pub mod report;
#[allow(clippy::module_inception)]
mod stage;

pub use morph::{Morph, Shape};
pub use motion::{CONTENT_DRIFT, CONTENT_IN, CONTENT_OUT, FOLD_SPRING, SHAPE_SPRING};
pub use panel::{COLLAPSED_LAYER, Grab, LAYERS, Mode, Panel};
pub use stage::{Held, Stage, collapsed_width};

pub const COLLAPSED_H: f32 = 38.0;

/// Room for panels wider than the bar, such as a list beside text to read.
pub const SURFACE_W: f32 = 1240.0;
pub const SURFACE_H: f32 = 1000.0;
