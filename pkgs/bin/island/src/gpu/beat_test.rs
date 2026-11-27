use super::*;

const REFRESH: Duration = Duration::from_nanos(6_944_444);

fn shown_at(millis: u64) -> Shown {
    Shown {
        at: Duration::from_millis(millis),
        refresh: REFRESH,
    }
}

#[test]
fn the_frame_drawn_now_is_shown_at_the_next_refresh() {
    let shown = shown_at(100);
    let now = Duration::from_millis(100) + Duration::from_millis(3);
    assert_eq!(ahead_of(shown, now), REFRESH - Duration::from_millis(3));
}

#[test]
fn a_late_start_still_lands_on_a_refresh() {
    let shown = shown_at(100);
    let now = Duration::from_millis(100) + REFRESH + Duration::from_millis(1);
    assert_eq!(ahead_of(shown, now), REFRESH - Duration::from_millis(1));
}

#[test]
fn frames_started_at_different_times_are_a_refresh_apart() {
    let shown = shown_at(100);
    let early = Duration::from_millis(100) + Duration::from_micros(500);
    let late = Duration::from_millis(100) + REFRESH + Duration::from_millis(4);
    let first = early + ahead_of(shown, early);
    let second = late + ahead_of(shown, late);
    assert_eq!(second - first, REFRESH);
}

#[test]
fn nothing_is_told_by_a_stale_or_unknown_refresh() {
    assert_eq!(
        ahead_of(shown_at(100), Duration::from_secs(5)),
        Duration::ZERO
    );
    let unknown = Shown {
        at: Duration::from_millis(100),
        refresh: Duration::ZERO,
    };
    assert_eq!(
        ahead_of(unknown, Duration::from_millis(103)),
        Duration::ZERO
    );
    assert_eq!(
        ahead_of(shown_at(100), Duration::from_millis(50)),
        Duration::ZERO
    );
}
