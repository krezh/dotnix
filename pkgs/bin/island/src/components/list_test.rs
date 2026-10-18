use super::{ROWS, Selection, reveal, stepped};
use crate::features::Local;

#[test]
fn keeps_the_window_still_while_the_selection_is_in_view() {
    assert_eq!(reveal(3, 2, 30), 2);
    assert_eq!(reveal(2 + ROWS - 1, 2, 30), 2);
}

#[test]
fn scrolls_only_as_far_as_the_selection_needs() {
    assert_eq!(reveal(1, 2, 30), 1);
    assert_eq!(reveal(2 + ROWS, 2, 30), 3);
    assert_eq!(reveal(0, 25, 5), 0);
}

#[test]
fn steps_stay_within_the_list() {
    assert_eq!(stepped(0, -1, 5), 0);
    assert_eq!(stepped(3, 4, 5), 4);
    assert_eq!(stepped(0, 3, 0), 0);
}

#[test]
fn selection_tracks_scrolling_and_clamps_after_results_shrink() {
    let mut selection = Selection::default();
    selection.select(ROWS + 2, 30);
    assert_eq!((selection.selected, selection.top), (ROWS + 2, 3));
    selection.step(ROWS as i32, 30);
    assert_eq!(
        (selection.selected, selection.top),
        (2 * ROWS + 2, ROWS + 3)
    );
    selection.step(-1, 3);
    assert_eq!((selection.selected, selection.top), (1, 0));
    selection.select(usize::MAX, 0);
    assert_eq!((selection.selected, selection.top), (0, 0));
}

#[test]
fn query_changes_and_panel_close_reset_navigation() {
    let mut selection = Selection::default();
    selection.select(20, 30);
    selection.search(|query| query.push_str("notes"));
    assert_eq!(selection.query, "notes");
    assert_eq!((selection.selected, selection.top), (0, 0));

    selection.select(20, 30);
    selection.search(String::clear);
    assert!(selection.query.is_empty());
    assert_eq!((selection.selected, selection.top), (0, 0));

    selection.search(|query| query.push_str("again"));
    selection.select(20, 30);
    selection.panel_closed();
    assert!(selection.query.is_empty());
    assert_eq!((selection.selected, selection.top), (0, 0));
}
