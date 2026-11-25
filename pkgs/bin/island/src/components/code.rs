//! Monospaced lines, such as a command about to run or a file's source.

use crate::panel::SharedString;

use crate::panel::Handler;

pub struct CodeLine {
    pub number: Option<u32>,
    pub text: SharedString,
    /// In a color of its own, the whole line washed faintly in it when `washed`, such as a line a
    /// diff adds.
    pub color: Option<(u32, bool)>,
}

impl CodeLine {
    pub fn plain(text: impl Into<SharedString>) -> Self {
        Self {
            number: None,
            text: text.into(),
            color: None,
        }
    }
}

/// How the lines come in the first time they are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Motion {
    #[default]
    Still,
    /// They rise in, such as what a step returned just now; never for what was there already.
    Appear,
}

/// Lines held to a height, scrolling under soft edges, such as a log.
pub struct Scroll {
    /// Unique among what the panel shows, so the place scrolled to is kept.
    pub key: SharedString,
    pub height: f32,
    /// Opens at the newest line and stays on it as lines arrive, until scrolled up.
    pub follows: bool,
}

pub struct Code {
    /// A name over the lines, such as a file's, with a fainter detail at the other end.
    pub heading: Option<(SharedString, SharedString)>,
    pub lines: Vec<CodeLine>,
    /// Long lines wrap, such as a command; otherwise they are cut at the edge, such as source.
    pub wrap: bool,
    /// A Copy chip in the heading, which gets one if it has none.
    pub copy: Option<Handler>,
    pub motion: Motion,
    /// Otherwise every line shows.
    pub scroll: Option<Scroll>,
}
