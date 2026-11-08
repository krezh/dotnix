//! A level shown with what it is of, in one of three shapes: a bar, such as the volume pop-up, with
//! a tinted glyph, a caption and label, and the amount; a ring round its glyph with the amount and
//! label under it, such as a battery; or a number with its label over it and a line of its recent
//! history under it, such as CPU use. Several share rows.

use std::path::PathBuf;

use crate::panel::SharedString;

use crate::components::chart::Series;

pub enum Shape {
    Bar,
    /// The level as a ring round the glyph, as a share of 100.
    Ring,
    /// The amount, in its unit, in place of the level, and the line of what it was before.
    History {
        amount: SharedString,
        unit: SharedString,
        history: Option<Series>,
    },
}

pub struct Meter {
    pub glyph: Option<PathBuf>,
    pub caption: SharedString,
    pub label: SharedString,
    /// Level and muted; `None` shows only what the meter is of, such as a device just switched to.
    pub level: Option<(f32, bool)>,
    pub color: u32,
    pub shape: Shape,
}
