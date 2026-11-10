//! A value from 0 to 1 that can be dragged, optionally with a live level over its fill. Hovering
//! one that can be dragged grows a handle out of its fill's end; holding it grows a little more.
//! A large one carries its glyph, name and amount inside it instead, light over the empty track and
//! dark over the fill.

use std::{path::PathBuf, rc::Rc};

use crate::panel::SharedString;

use crate::{panel::Ui, spring::SpringConfig};

pub type OnChange = Rc<dyn Fn(f32, &mut dyn Ui)>;

pub const FILL_SPRING: SpringConfig = SpringConfig::new(420.0, 41.0, 1.0);
pub const TRACK_H: f32 = 10.0;
pub const THIN_H: f32 = 4.0;
pub const HIT_H: f32 = 16.0;
/// The handle sticks out 4 px past each side of the track while hovered, 5 px while held.
pub const HANDLE_W: f32 = 4.0;
pub const HOVER_H: f32 = TRACK_H + 8.0;
pub const HELD_H: f32 = TRACK_H + 10.0;
/// The furthest a pull past either end stretches the track.
pub const STRETCH_REACH: f32 = 24.0;

/// What a large slider shows inside itself.
#[derive(Clone)]
pub struct SliderLabel {
    pub glyph: Option<PathBuf>,
    pub label: SharedString,
    pub amount: SharedString,
}

pub struct Slider {
    pub value: f32,
    pub color: u32,
    /// Dims the fill and draws a live level over it, as a share of the value: 1 reaches the end
    /// of the fill.
    pub signal: Option<f32>,
    pub on_change: Option<OnChange>,
    /// Makes it large, carrying these inside it.
    pub label: Option<SliderLabel>,
    /// A thin line only showing how far along something is, such as progress, with no room kept
    /// round it for the pointer.
    pub thin: bool,
}
