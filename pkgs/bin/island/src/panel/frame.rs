//! What a panel is drawn with in GPUI: where it scrolls, the room it has and what it shares.

use std::cell::Cell;

use gpui::{Hsla, ScrollHandle};

use super::model::COLUMNS;
use crate::components::{marquee::TextLayouts, popover::Popovers, tooltip::Tooltips};

pub struct Frame<'a> {
    pub hover: bool,
    pub scroll: &'a ScrollHandle,
    /// One for each column of a [`Block::Columns`].
    pub columns: &'a [ScrollHandle; COLUMNS],
    /// For a [`Block::Placed`] that scrolls; a panel shows one at a time.
    pub placed: &'a ScrollHandle,
    /// True on the first drawing after the panel opened; the columns clear it.
    pub opened: &'a Cell<bool>,
    pub island_h: f32,
    pub island_w: f32,
    pub min_width: f32,
    pub max_width: f32,
    pub max_height: f32,
    pub phase: f32,
    pub layouts: &'a TextLayouts,
    pub tips: &'a Tooltips,
    pub popovers: &'a Popovers,
    /// The island's surface color, which what scrolls fades into at its edges.
    pub glass: Hsla,
}
