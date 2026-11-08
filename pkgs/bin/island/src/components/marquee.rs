//! Text that scrolls when it does not fit, and the text measuring it relies on.

use std::{
    cell::RefCell,
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::panel::SharedString;

/// Between the end of one pass and the start of the next.
pub const GAP: f32 = 40.0;
/// Px per second.
pub const SPEED: f32 = 28.0;
/// At each edge of the slot.
pub const FADE: f32 = 18.0;

/// Ambient motion does not need the display's full refresh rate (often 144–240 Hz), and each frame
/// rebuilds the whole island.
pub const TICK: Duration = Duration::from_micros(33_333);
pub const BRISK: Duration = Duration::from_micros(16_667);

pub fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

thread_local! {
    /// When each text hovered right now started to glide.
    static GLIDES: RefCell<HashMap<SharedString, Instant>> = RefCell::default();
}
