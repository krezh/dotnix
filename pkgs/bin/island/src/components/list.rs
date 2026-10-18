//! Keyboard navigation for searchable lists, such as the launcher's and the clipboard's.

use gpui::Keystroke;

pub const ROWS: usize = 8;

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
    let moved = i64::try_from(selected).unwrap_or(i64::MAX).saturating_add(i64::from(step));
    usize::try_from(moved.max(0)).unwrap_or(0).min(count.saturating_sub(1))
}

#[cfg(test)]
#[path = "list_test.rs"]
mod tests;
