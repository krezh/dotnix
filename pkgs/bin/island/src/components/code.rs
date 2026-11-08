//! Monospaced lines, plain or as a diff, such as a command about to run or the change to a file.

use crate::panel::SharedString;

use crate::panel::Handler;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Change {
    Same,
    Added,
    Removed,
}

pub struct CodeLine {
    pub number: Option<u32>,
    pub text: SharedString,
    pub change: Change,
}

impl CodeLine {
    pub fn plain(text: impl Into<SharedString>) -> Self {
        Self {
            number: None,
            text: text.into(),
            change: Change::Same,
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

pub struct Code {
    /// A name over the lines, such as a file's, with a fainter detail at the other end.
    pub heading: Option<(SharedString, SharedString)>,
    pub lines: Vec<CodeLine>,
    /// Long lines wrap, such as a command; otherwise they are cut at the edge, such as source.
    pub wrap: bool,
    /// A Copy chip in the heading, which gets one if it has none.
    pub copy: Option<Handler>,
    pub motion: Motion,
}
