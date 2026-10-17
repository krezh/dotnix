use super::{ROWS, reveal, stepped};

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
