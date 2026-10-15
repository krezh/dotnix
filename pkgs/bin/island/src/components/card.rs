use gpui::{Div, SharedString, div, prelude::*, rgba};

use crate::{
    theme::{CARD_ALPHA, CARD_BORDER_ALPHA, SURFACE1, with_alpha},
    ui::{self, Radius},
};

pub const WIDTH: f32 = 270.0;
pub const GAP: f32 = 12.0;
/// Samples a history chart holds.
pub const SLOTS: usize = 40;

/// A card's width is fixed when it is created; content truncates rather than stretching it.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    pub fn width(self) -> f32 {
        match self {
            Size::Small => WIDTH,
            Size::Medium => 360.0,
            Size::Large => 2.0 * WIDTH + GAP,
        }
    }
}

/// A short value with its colour, as shown in a card or tile header.
pub struct Stat {
    pub text: SharedString,
    pub color: u32,
}

pub fn surface() -> Div {
    div().relative().child(ui::outlined(
        Radius::Panel,
        rgba(with_alpha(SURFACE1, CARD_ALPHA)).into(),
        rgba(with_alpha(SURFACE1, CARD_BORDER_ALPHA)).into(),
    ))
}

pub fn stat(text: impl Into<SharedString>, color: u32) -> Stat {
    Stat {
        text: text.into(),
        color,
    }
}
