//! State and keyboard navigation for searchable lists.

use gpui::Keystroke;

pub const ROWS: usize = 8;

#[derive(Default)]
pub struct Selection {
    pub(crate) query: String,
    pub(crate) selected: usize,
    pub(crate) top: usize,
}

impl Selection {
    pub(crate) fn select(&mut self, position: usize, count: usize) {
        self.selected = position.min(count.saturating_sub(1));
        self.top = reveal(self.selected, self.top, count);
    }

    pub(crate) fn step(&mut self, step: i32, count: usize) {
        self.select(
            stepped(self.selected.min(count.saturating_sub(1)), step, count),
            count,
        );
    }

    /// Moves the rows in view, leaving the selection where it is, even out of view.
    pub(crate) fn scroll(&mut self, rows: i32, count: usize) {
        self.top = stepped(self.top, rows, count.saturating_sub(ROWS) + 1);
    }

    /// Selects `position` after the list changed, scrolling only if the selection moved.
    pub(crate) fn keep(&mut self, position: usize, count: usize) {
        if position == self.selected {
            self.selected = position.min(count.saturating_sub(1));
            self.top = self.top.min(count.saturating_sub(ROWS));
        } else {
            self.select(position, count);
        }
    }

    pub(crate) fn search(&mut self, change: impl FnOnce(&mut String)) {
        change(&mut self.query);
        self.selected = 0;
        self.top = 0;
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }
}

impl crate::features::Local for Selection {
    fn panel_closed(&mut self) {
        self.reset();
    }
}

/// Text typed into a search field, leaving shortcuts and control keys out.
pub fn typed(keystroke: &Keystroke) -> Option<&str> {
    keystroke
        .key_char
        .as_deref()
        .filter(|_| !keystroke.modifiers.control && !keystroke.modifiers.alt)
        .filter(|text| text.chars().all(|char| !char.is_control()))
}

/// Moves on from `top` as little as it can, so the list only scrolls when the selection reaches
/// its edge.
pub fn reveal(selected: usize, top: usize, count: usize) -> usize {
    let top = top.min(count.saturating_sub(ROWS));
    if selected < top {
        selected
    } else if selected >= top + ROWS {
        selected + 1 - ROWS
    } else {
        top
    }
}

pub fn stepped(selected: usize, step: i32, count: usize) -> usize {
    let moved = i64::try_from(selected)
        .unwrap_or(i64::MAX)
        .saturating_add(i64::from(step));
    usize::try_from(moved.max(0))
        .unwrap_or(0)
        .min(count.saturating_sub(1))
}

#[cfg(test)]
#[path = "list_test.rs"]
mod tests;
