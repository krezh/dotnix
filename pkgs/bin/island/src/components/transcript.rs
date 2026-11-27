//! A conversation: your turns on the right, the other side's on the left beside its mark, each
//! reply on a surface of its own under a faint line saying who and how long, the newest at the
//! bottom, with dots while a reply is on its way. A reply reads as text, lists, quotes, tables and
//! code, and may say what it looked up and end in numbered chips for where it read things.

use std::path::PathBuf;

use crate::panel::SharedString;

use crate::{
    components::{code::Code, table::Table, text::Text},
    panel::Handler,
};

/// One entry of a list, under the entries `depth` levels out.
pub struct Entry {
    pub depth: usize,
    /// Its number in a numbered list; a bullet without.
    pub number: Option<u64>,
    pub text: Text,
}

pub enum Piece {
    Text(Text),
    List(Vec<Entry>),
    /// Quoted words, set off by a faint rule.
    Quote(Text),
    Table(Table),
    Code(Code),
}

/// A faint line under a reply's overline, such as what it looked up: a glyph in `tint`, then
/// words in colors of their own.
pub struct Detail {
    pub glyph: Option<PathBuf>,
    pub tint: u32,
    pub parts: Vec<(SharedString, u32)>,
}

/// Where a reply read something, as a numbered chip after its actions.
pub struct Source {
    pub label: SharedString,
    pub on_click: Handler,
}

pub enum Turn {
    Mine(Text),
    /// `overline` says who answered and how long it took; `streaming` while more is coming.
    Theirs {
        overline: SharedString,
        detail: Option<Detail>,
        pieces: Vec<Piece>,
        streaming: bool,
        /// Small buttons under a finished reply, such as Copy.
        actions: Vec<(SharedString, Handler)>,
        /// Numbered from 1, in `source_tint`, after the actions.
        sources: Vec<Source>,
        source_tint: u32,
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
