//! Boards of stat tiles, laid out in columns that end level.

use std::path::PathBuf;

use crate::panel::SharedString;

use crate::{
    components::{
        card::{self, Stat},
        chart::{Phase, Series},
    },
    panel::Handler,
};

/// Height of one row unit. A tile spans whole units, so every column of a board ends level.
pub const UNIT: f32 = 196.0;

pub struct Tile {
    pub id: SharedString,
    /// Row units tall: 1 or 2.
    pub span: usize,
    pub title: Option<(SharedString, Option<Stat>)>,
    pub on_click: Option<Handler>,
    pub parts: Vec<Part>,
}

pub struct Meter {
    pub label: SharedString,
    pub fraction: f32,
    pub value: SharedString,
    pub color: u32,
    /// A tick across the bar at this fraction, such as how much of a quota's window has passed.
    pub mark: Option<f32>,
}

pub struct Volume {
    pub mount: SharedString,
    pub usage: SharedString,
    pub fraction: f32,
    pub color: u32,
}

pub struct Segment {
    pub weight: f32,
    pub color: Option<u32>,
}

pub struct Drive {
    pub name: SharedString,
    pub size: SharedString,
    pub segments: Vec<Segment>,
}

/// What leads an entry: a glyph in a color, or a dot, such as whether something is up.
pub enum EntryLead {
    Glyph(Option<PathBuf>, u32),
    Dot(u32),
}

/// One row of a list in a tile, such as a network to join; clickable when it has a handler.
pub struct Entry {
    pub lead: Option<EntryLead>,
    pub title: SharedString,
    pub title_color: Option<u32>,
    pub detail: SharedString,
    pub value: SharedString,
    pub value_color: u32,
    /// The row tinted in this color, such as the one in use.
    pub chosen: Option<u32>,
    pub on_click: Option<Handler>,
}

/// One building block of a tile. Parts stack top to bottom; `Spacer` pushes the rest down.
pub enum Part {
    /// A history chart; `grow` lets it take the tile's spare height. `phase` is the clock of a
    /// history sampled on its own; otherwise the system sampler sets the pace.
    Chart {
        series: Vec<Series>,
        grow: bool,
        phase: Option<Phase>,
    },
    Cores {
        loads: Vec<f32>,
        color: u32,
    },
    /// Label, bar and value rows; `scroll` lets a long list scroll inside the tile.
    Meters {
        rows: Vec<Meter>,
        scroll: bool,
    },
    /// Each optionally led by a coloured dot.
    Legend(Vec<(Option<u32>, SharedString)>),
    Facts(Vec<(SharedString, SharedString)>),
    Hero {
        name: SharedString,
        lines: Vec<SharedString>,
    },
    /// Scrolls past what fits.
    Volumes(Vec<Volume>),
    Drives(Vec<Drive>),
    /// Rows of a list; `scroll` lets a long one scroll inside the tile.
    Entries {
        entries: Vec<Entry>,
        scroll: bool,
    },
    Spacer,
}

pub fn height(span: usize) -> f32 {
    let span = span.max(1) as f32;
    UNIT * span + card::GAP * (span - 1.0)
}
