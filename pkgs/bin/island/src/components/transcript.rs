//! A conversation: your turns on the right, the other side's on the left beside its mark, each
//! reply on a surface of its own under a faint line saying who and how long, the newest at the
//! bottom, with dots while a reply is on its way.

use std::path::PathBuf;

use crate::panel::SharedString;

use crate::{
    components::{code::Code, text::Text},
    panel::Handler,
};

pub enum Piece {
    Text(Text),
    Code(Code),
}

pub enum Turn {
    Mine(Text),
    /// `overline` says who answered and how long it took; `streaming` while more is coming.
    Theirs {
        overline: SharedString,
        pieces: Vec<Piece>,
        streaming: bool,
        /// Small buttons under a finished reply, such as Copy.
        actions: Vec<(SharedString, Handler)>,
    },
    /// A reply on its way with nothing of it yet.
    Waiting {
        overline: SharedString,
    },
}

pub struct Transcript {
    pub turns: Vec<Turn>,
    /// Marks the other side's turns, tinted with `color`.
    pub glyph: Option<PathBuf>,
    pub color: u32,
    /// Turns from this one on rise in as they first show; those before, already there, such as
    /// history, are shown still.
    pub appear_from: usize,
}
