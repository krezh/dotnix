//! Rows of cells under column headings, such as files with their sizes or packages with their
//! versions: each column a share of the width or a set width, its cells at its start or its end.
//! Words never get cut short: a cell whose words do not fit wraps and its row grows. A cell may be
//! a bar of tinted parts, a dot, or words in several colors; a row may carry a second line, be
//! washed in a color, or open something; a chip or colored words may head the rows under them.

use crate::{
    components::chip::Chip,
    panel::{Handler, SharedString},
};

pub enum Width {
    /// This share of what the set widths leave.
    Share(f32),
    Fixed(f32),
}

pub struct Column {
    /// Its heading; the row of headings is left out when every one is empty.
    pub title: SharedString,
    pub width: Width,
    /// Its cells at its end, such as amounts.
    pub end: bool,
}

pub enum Cell {
    Text {
        text: SharedString,
        color: u32,
        strong: bool,
        mono: bool,
    },
    /// Words in colors of their own, one after another, such as an old version, an arrow and the
    /// new one; what does not fit goes on to the next line.
    Parts(Vec<(SharedString, u32)>, bool),
    /// Shares of the whole laid end to end from the start, each in its color, over a track.
    Bar(Vec<(f32, u32)>),
    /// A dot in a color, such as how a row stands.
    Dot(u32),
}

impl Cell {
    pub fn text(text: impl Into<SharedString>, color: u32) -> Self {
        Self::Text {
            text: text.into(),
            color,
            strong: false,
            mono: false,
        }
    }

    pub fn strong(text: impl Into<SharedString>, color: u32) -> Self {
        Self::Text {
            text: text.into(),
            color,
            strong: true,
            mono: false,
        }
    }

    pub fn mono(text: impl Into<SharedString>, color: u32) -> Self {
        Self::Text {
            text: text.into(),
            color,
            strong: false,
            mono: true,
        }
    }
}

#[derive(Default)]
pub struct Line {
    pub cells: Vec<Cell>,
    /// What it says when pointed at, such as what clicking it opens.
    pub tip: Option<SharedString>,
    /// A second, quieter line in the monospaced face under the cell of this column, such as what
    /// a build is doing now.
    pub under: Option<(usize, SharedString)>,
    /// Washed in a color, such as one that failed.
    pub wash: Option<u32>,
    /// The moving border, while what it is is under way.
    pub busy: bool,
    pub on_click: Option<Handler>,
}

pub enum Row {
    Line(Line),
    /// Heads the rows under it.
    Heading(Chip),
    /// Words in a color heading the rows under them, such as a group's name and count.
    Group(SharedString, u32),
}

impl Row {
    pub fn cells(cells: Vec<Cell>, tip: Option<SharedString>) -> Self {
        Self::Line(Line {
            cells,
            tip,
            ..Line::default()
        })
    }
}

pub struct Table {
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
}
