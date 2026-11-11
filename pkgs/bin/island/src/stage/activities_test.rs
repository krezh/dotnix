use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use super::{ANNOUNCE_FOR, Activities, DONE_FOR};
use crate::activity::{Activity, Standing};

fn activity(key: &str, standing: Standing) -> Activity {
    Activity {
        key: key.to_owned().into(),
        glyph: None,
        color: 0,
        standing,
        progress: None,
        title: key.to_owned().into(),
        detail: None,
        summary: "".into(),
        aside: None,
        card: Vec::new(),
        on_open: Rc::new(|_| {}),
    }
}

fn keys(activities: &Activities) -> Vec<String> {
    activities
        .shown
        .iter()
        .map(|(_, activity)| activity.key.to_string())
        .collect()
}

#[test]
fn the_most_pressing_comes_first() {
    let mut activities = Activities::default();
    let now = Instant::now();
    activities.update(
        vec![
            ("a", activity("running", Standing::Running)),
            ("a", activity("done", Standing::Done)),
            ("b", activity("failed", Standing::Failed)),
            ("b", activity("ready", Standing::NeedsYou)),
        ],
        None,
        now,
    );
    assert_eq!(keys(&activities), ["failed", "ready", "running", "done"]);
}

#[test]
fn a_routine_result_folds_away() {
    let mut activities = Activities::default();
    let now = Instant::now();
    activities.update(vec![("a", activity("done", Standing::Done))], None, now);
    assert_eq!(activities.due(now), Some(now + DONE_FOR));
    let later = now + DONE_FOR + Duration::from_millis(1);
    activities.update(vec![("a", activity("done", Standing::Done))], None, later);
    assert!(activities.shown.is_empty());
}

#[test]
fn a_result_that_waits_stays_until_its_panel_opens() {
    let mut activities = Activities::default();
    let now = Instant::now();
    let later = now + Duration::from_secs(60);
    activities.update(
        vec![("a", activity("failed", Standing::Failed))],
        None,
        later,
    );
    activities.update(
        vec![("a", activity("failed", Standing::Failed))],
        Some("b"),
        later,
    );
    assert_eq!(keys(&activities), ["failed"]);
    activities.update(
        vec![("a", activity("failed", Standing::Failed))],
        Some("a"),
        later,
    );
    assert!(activities.shown.is_empty());
    activities.update(
        vec![("a", activity("failed", Standing::Failed))],
        None,
        later,
    );
    assert!(activities.shown.is_empty());
}

#[test]
fn running_shows_while_its_panel_is_open() {
    let mut activities = Activities::default();
    let now = Instant::now();
    activities.update(
        vec![("a", activity("build", Standing::Running))],
        Some("a"),
        now,
    );
    assert_eq!(keys(&activities), ["build"]);
}

#[test]
fn a_start_or_failure_is_said_once() {
    let mut activities = Activities::default();
    let now = Instant::now();
    activities.update(vec![("a", activity("build", Standing::Running))], None, now);
    assert!(activities.announcing(now));
    let quiet = now + ANNOUNCE_FOR;
    assert!(!activities.announcing(quiet));
    activities.update(
        vec![("a", activity("build", Standing::Running))],
        None,
        quiet,
    );
    assert!(!activities.announcing(quiet));
    activities.update(
        vec![("a", activity("build", Standing::Failed))],
        None,
        quiet,
    );
    assert!(activities.announcing(quiet));
}

#[test]
fn a_waiting_result_leaves_the_fullscreen_ring_once_said() {
    let mut activities = Activities::default();
    let now = Instant::now();
    activities.update(
        vec![("a", activity("ready", Standing::NeedsYou))],
        None,
        now,
    );
    assert_eq!(
        activities.ringed().map(|a| a.key.to_string()),
        Some("ready".into())
    );
    activities.ring(now + ANNOUNCE_FOR);
    assert!(activities.ringed().is_none());
    assert_eq!(keys(&activities), ["ready"]);
}

#[test]
fn the_ring_keeps_work_under_way_below_a_waiting_result() {
    let mut activities = Activities::default();
    let now = Instant::now();
    activities.update(
        vec![
            ("a", activity("ready", Standing::NeedsYou)),
            ("b", activity("build", Standing::Running)),
        ],
        None,
        now,
    );
    activities.ring(now + ANNOUNCE_FOR);
    assert_eq!(
        activities.ringed().map(|a| a.key.to_string()),
        Some("build".into())
    );
}

#[test]
fn seen_on_one_island_hides_it_on_the_others() {
    let now = Instant::now();
    let all = || vec![("a", activity("ready", Standing::NeedsYou))];
    let (mut here, mut there) = (Activities::default(), Activities::default());
    here.update(all(), None, now);
    there.update(all(), None, now);
    here.update(all(), Some("a"), now);
    let seen = here.take_newly_seen();
    assert_eq!(seen.len(), 1);
    assert!(here.take_newly_seen().is_empty());
    assert!(there.see(&seen));
    there.update(all(), None, now);
    assert!(there.shown.is_empty());
}

#[test]
fn a_routine_result_is_not_said() {
    let mut activities = Activities::default();
    let now = Instant::now();
    activities.update(vec![("a", activity("done", Standing::Done))], None, now);
    assert!(!activities.announcing(now));
}
