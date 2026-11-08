//! A panel's cards: a surface of a fixed width with an optional title, holding blocks; frames,
//! the smaller surfaces inside them, such as one app's row or one message; and cards shared
//! between two columns of about the same height.

use std::{path::PathBuf, time::Duration};

use crate::panel::SharedString;

use crate::panel::Handler;

/// Inside a raised frame, and between what it holds.
pub const RAISED_PAD: f32 = 14.0;
pub const RAISED_GAP: f32 = 8.0;

/// Time running out, drawn as a soft blurred tint inside a frame that drains toward its start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Countdown {
    /// 0–1.
    pub left: f32,
    /// `None` while paused.
    pub running: Option<Duration>,
    /// Counts each pause and resume, so the tint starts a new animation from where it is.
    pub round: u32,
    /// The tint's color, such as the app's own.
    pub color: u32,
}

pub enum Look {
    /// A quiet surface inset in its card, such as one of many rows in a grid.
    Inset,
    /// Its own outlined surface, filling the space it is given, such as a message; urgent ones
    /// have a red outline.
    Raised { urgent: bool },
}

pub struct FrameView {
    pub id: SharedString,
    pub look: Look,
    pub on_click: Option<Handler>,
    /// On a right click.
    pub on_dismiss: Option<Handler>,
    pub countdown: Option<Countdown>,
    /// Tinted as the chosen one of several, such as a picked answer; the tint eases in and out.
    pub selected: bool,
    pub motion: Motion,
    /// Shows only this much of a raised frame from its top, such as one folding away: its outline
    /// closes above the cut, and what it holds keeps its size and is cut off. `None` for all of it.
    pub shown_height: Option<f32>,
}

/// How a frame comes and goes, for one that is not placed and moved by its owner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Motion {
    #[default]
    Still,
    /// Rises in the first time it is drawn.
    Appear,
    /// Fades out, such as one just answered; its owner keeps it drawn for [`reveal::LEAVE`].
    Leave,
}

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
    /// As wide as two medium cards side by side.
    Wide,
    /// As wide as a board of three small tiles, so a panel keeps one width across its views.
    Full,
    /// For text to read, such as a conversation: about 85 characters a line at reading size.
    Reading,
}

impl Size {
    pub fn width(self) -> f32 {
        match self {
            Size::Small => WIDTH,
            Size::Medium => 360.0,
            Size::Large => 2.0 * WIDTH + GAP,
            Size::Wide => 2.0 * Size::Medium.width() + GAP,
            Size::Full => 3.0 * WIDTH + 2.0 * GAP,
            Size::Reading => 760.0,
        }
    }
}

/// A card's heading, with a fainter detail at the other end, such as "Output … 48 kHz · stereo".
pub struct Title {
    pub text: SharedString,
    pub detail: SharedString,
    /// Before the text, in its color, such as what the card is about.
    pub glyph: Option<(PathBuf, u32)>,
    /// A › at the end that opens more of what the card shows.
    pub open: Option<Handler>,
}

impl Title {
    pub fn with_detail(text: impl Into<SharedString>, detail: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            detail: detail.into(),
            glyph: None,
            open: None,
        }
    }

    pub fn glyph(mut self, glyph: Option<PathBuf>, color: u32) -> Self {
        self.glyph = glyph.map(|path| (path, color));
        self
    }

    pub fn opens(mut self, handler: Handler) -> Self {
        self.open = Some(handler);
        self
    }
}

impl From<&'static str> for Title {
    fn from(text: &'static str) -> Self {
        Self::with_detail(text, "")
    }
}

impl From<String> for Title {
    fn from(text: String) -> Self {
        Self::with_detail(text, "")
    }
}

impl From<SharedString> for Title {
    fn from(text: SharedString) -> Self {
        Self::with_detail(text, "")
    }
}

pub struct Stat {
    pub text: SharedString,
    pub color: u32,
}

pub fn stat(text: impl Into<SharedString>, color: u32) -> Stat {
    Stat {
        text: text.into(),
        color,
    }
}

pub const BALANCED: usize = 2;
/// Taken for a card not yet drawn.
const UNDRAWN_H: f32 = 160.0;
/// Past this many cards, every split is too many to try, and each goes under the shorter column.
const TRIED: usize = 16;

/// Which of two columns each of `keys`' cards goes in, in order, before their heights are known:
/// as if each were as tall as a card usually is.
pub fn balance(keys: &[SharedString]) -> Vec<usize> {
    balance_heights(&vec![UNDRAWN_H + GAP; keys.len()])
}

/// As [`balance`], for cards whose heights are known.
pub fn balance_heights(heights: &[f32]) -> Vec<usize> {
    if heights.len() > TRIED {
        let mut filled = [0.0f32; BALANCED];
        return heights
            .iter()
            .map(|height| {
                let column = usize::from(filled[1] < filled[0]);
                filled[column] += height;
                column
            })
            .collect();
    }
    // Every split, the first card always on the left; the lowest bits are the first cards, so
    // among equal splits the one keeping the earliest cards on the left wins.
    let taller = |split: u32| {
        let mut filled = [0.0f32; BALANCED];
        for (index, height) in heights.iter().enumerate() {
            filled[(split >> index & 1) as usize] += height;
        }
        filled[0].max(filled[1])
    };
    let best = (0..1u32 << heights.len().saturating_sub(1))
        .map(|split| split << 1)
        .min_by(|a, b| taller(*a).total_cmp(&taller(*b)))
        .unwrap_or(0);
    (0..heights.len())
        .map(|index| (best >> index & 1) as usize)
        .collect()
}
