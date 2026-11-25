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

#[test]
fn a_ring_sweeps_on_from_where_it_is() {
    let start = Instant::now();
    let first = Sweep::toward(None, 0.2, false, start);
    assert!((first.at(start) - 0.2).abs() < 1e-6);
    let onward = Sweep::toward(Some(&first), 0.6, false, start);
    let half = start + SWEEP / 2;
    let midway = onward.at(half);
    assert!(midway > 0.2 && midway < 0.6);
    let again = Sweep::toward(Some(&onward), 0.8, false, half);
    assert!((again.at(half) - midway).abs() < 1e-6);
    assert!((again.at(half + SWEEP) - 0.8).abs() < 1e-6);
    assert!(!again.running(half + SWEEP));
}

#[test]
fn a_ring_marks_itself_once_round() {
    let start = Instant::now();
    let sweep = Sweep::toward(None, 0.5, false, start);
    let full = Sweep::toward(Some(&sweep), 1.0, true, start);
    assert!(full.marked.is_none());
    let round = Sweep::toward(Some(&full), 1.0, true, start + SWEEP);
    assert_eq!(round.marked, Some(start + SWEEP));
}
