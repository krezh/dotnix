//! Text in a panel, in one of a few tones: plain, or styled in ranges with clickable links, held
//! to a number of lines.

use std::ops::Range;

use crate::panel::SharedString;

use crate::ui::SMALL_SIZE;

/// A body line, so wrapped text keeps an even rhythm.
pub const BODY_LINE: f32 = 17.0;
pub const BODY_SIZE: f32 = SMALL_SIZE;

pub enum Tone {
    /// Names what follows.
    Heading,
    /// A short line in a color.
    Caption(u32),
    /// Says something about the whole block, such as that it is empty.
    Note,
    /// Running text, such as a message.
    Body,
    /// Longer text meant to be read, such as a reply: larger, with room between its lines.
    Reading,
}

#[derive(Clone, Copy, Default)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub code: bool,
    /// Hidden until looked at closely, such as a spoiler.
    pub covered: bool,
    pub link: bool,
}

pub struct Text {
    pub text: SharedString,
    pub tone: Tone,
    pub spans: Vec<(Range<usize>, Style)>,
    /// Clicking these ranges opens their address.
    pub links: Vec<(Range<usize>, String)>,
    /// Longer text ends in an ellipsis.
    pub lines: Option<usize>,
    pub min_width: Option<f32>,
    /// Longer lines wrap, or with `lines` end in an ellipsis, rather than widening what holds it.
    pub max_width: Option<f32>,
}

impl Text {
    pub fn new(tone: Tone, text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            tone,
            spans: Vec::new(),
            links: Vec::new(),
            lines: None,
            min_width: None,
            max_width: None,
        }
    }
}
