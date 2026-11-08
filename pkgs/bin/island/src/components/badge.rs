//! A small picture standing for something in a row: a glyph, an app's icon, a picture, a letter,
//! the mark a choice shows (a check box or a radio dot), a ring with a value, or something its owner draws; any of
//! them with a small glyph at its corner, or pulsing.

use std::{path::PathBuf, rc::Rc, time::Duration};

use crate::panel::SharedString;

use crate::{components::icon::Picture, drawing::Drawing};

#[derive(Clone)]
pub enum Mark {
    /// A symbolic icon in the color.
    Glyph(PathBuf, u32),
    /// A full-color icon, such as an app's.
    Image(PathBuf),
    /// A picture that may be in memory rather than a file: cropped to the square with rounded
    /// corners, such as a photo, or fitted whole, such as an icon.
    Picture(Picture, bool),
    /// The first letter of the text, on the color, for something without an icon.
    Letter(SharedString, u32),
    /// Whether an option is picked: a check box when `many` may be, a radio dot when only one.
    /// `key` names its animation, so it fills and empties in place.
    Choice { picked: bool, many: bool },
    /// Drawn by its owner at the side it is given, and answering the pointer itself.
    Drawn(Rc<dyn Fn(f32) -> Drawing>),
    /// A ring filled part way round, with a short value in it, such as a battery's charge. `key`
    /// names its animation, so it glides to a new value in place.
    Ring {
        fraction: f32,
        color: u32,
        value: SharedString,
    },
    /// A mark with a small glyph in the color at its corner, such as where a message came from.
    Marked(Box<Mark>, PathBuf, u32),
    /// A mark fading and coming back, over and over, such as something still being tried.
    Pulsing(Box<Mark>),
}

pub const PULSE: Duration = Duration::from_millis(1600);
/// How faint a pulsing mark goes.
const PULSE_LOW: f32 = 0.45;

/// How much shows `t` of the way through a pulse.
pub fn pulsed(t: f32) -> f32 {
    let wave = 0.5 - 0.5 * (t * std::f32::consts::TAU).cos();
    1.0 - (1.0 - PULSE_LOW) * wave
}
