use super::settle;

const MOST: f32 = 300.0;

#[test]
fn scrolling_within_its_ends_pulls_nothing() {
    assert_eq!(settle(0.0, -120.0, 10.0, MOST), (0.0, -120.0));
}

#[test]
fn scrolling_past_the_top_pulls_by_what_is_past_it() {
    assert_eq!(settle(0.0, 6.0, 10.0, MOST), (6.0, 0.0));
}

#[test]
fn scrolling_past_the_bottom_pulls_the_other_way() {
    assert_eq!(settle(0.0, -310.0, -14.0, MOST), (-10.0, -MOST));
}

#[test]
fn a_pull_grows_while_it_keeps_its_place_at_the_end() {
    assert_eq!(settle(6.0, 10.0, 10.0, MOST), (16.0, 0.0));
    assert_eq!(settle(-10.0, -305.0, -5.0, MOST), (-15.0, -MOST));
}

#[test]
fn a_pull_gives_way_before_it_scrolls_again() {
    assert_eq!(settle(16.0, -4.0, -4.0, MOST), (12.0, 0.0));
    assert_eq!(settle(6.0, -10.0, -10.0, MOST), (0.0, -4.0));
    assert_eq!(settle(-6.0, -290.0, 10.0, MOST), (0.0, -296.0));
}
