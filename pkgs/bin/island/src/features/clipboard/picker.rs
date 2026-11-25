//! What the open picker shows: the search, the filter and the selection, kept in view past the
//! group labels.

use super::filter::Filter;
use crate::components::{
    list::{ROWS, Selection},
    search::SECTION_SHARE,
};

#[derive(Default)]
pub struct Picker {
    pub(super) selection: Selection,
    pub(super) filter: Filter,
}

/// How the rows the picker shows are grouped: the pinned ones first, under their own label, then
/// the rest under theirs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Layout {
    pub(super) count: usize,
    pub(super) pinned: usize,
}

impl Layout {
    /// The label the row at `position` starts its group with.
    pub(super) fn section(self, position: usize) -> Option<&'static str> {
        if position == 0 && self.pinned > 0 {
            Some("Pinned")
        } else if position == self.pinned && position < self.count {
            Some("Recent")
        } else {
            None
        }
    }

    /// How many rows from `top` show whole, the labels among them taking their share.
    pub(super) fn whole(self, top: usize) -> usize {
        let labels = (top..(top + ROWS).min(self.count))
            .filter(|&position| self.section(position).is_some())
            .count();
        (ROWS as f32 - labels as f32 * SECTION_SHARE).floor() as usize
    }

    fn fit(self, selection: &mut Selection) {
        while selection.selected >= selection.top + self.whole(selection.top).max(1) {
            selection.top += 1;
        }
    }
}

impl Picker {
    pub(super) fn select(&mut self, position: usize, layout: Layout) {
        self.selection.select(position, layout.count);
        layout.fit(&mut self.selection);
    }

    pub(super) fn step(&mut self, step: i32, layout: Layout) {
        self.selection.step(step, layout.count);
        layout.fit(&mut self.selection);
    }

    pub(super) fn scroll(&mut self, rows: i32, layout: Layout) {
        self.selection.scroll(rows, layout.count);
    }

    pub(super) fn search(&mut self, change: impl FnOnce(&mut String)) {
        self.selection.search(change);
    }

    pub(super) fn filter(&mut self, filter: Filter) {
        self.filter = filter;
        self.selection.search(|_| {});
    }
}

impl crate::features::Local for Picker {
    fn panel_closed(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
#[path = "picker_test.rs"]
mod tests;
