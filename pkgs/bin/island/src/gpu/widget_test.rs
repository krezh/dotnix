use super::*;

#[test]
fn widgets_side_by_side_make_one_set() {
    let widgets = [(false, 10.0), (true, 20.0), (true, 30.0), (false, 5.0)];
    assert_eq!(sets(widgets.into_iter(), 100.0), [(110.0, 50.0)]);
}

#[test]
fn a_widget_between_splits_a_set() {
    let widgets = [(true, 20.0), (false, 5.0), (true, 30.0)];
    assert_eq!(sets(widgets.into_iter(), 0.0), [(0.0, 20.0), (25.0, 30.0)]);
}

#[test]
fn no_group_means_no_sets() {
    assert!(sets([(false, 10.0)].into_iter(), 0.0).is_empty());
}
