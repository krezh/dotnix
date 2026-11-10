use std::time::{Duration, Instant};

use super::{Due, Look, Request, Target, batch, merge, postpone};
use crate::control::Change;

#[test]
fn a_burst_of_events_waits_for_the_last() {
    let now = Instant::now();
    let later = now + Duration::from_secs(1);
    let first = postpone(None, now, Look::Read);
    assert_eq!(
        postpone(Some(first), later, Look::Read),
        Due {
            at: later,
            look: Look::Read
        }
    );
}

#[test]
fn finding_the_screens_covers_reading_them() {
    let now = Instant::now();
    let found = postpone(None, now, Look::Find);
    assert_eq!(postpone(Some(found), now, Look::Read).look, Look::Find);
    let read = postpone(None, now, Look::Read);
    assert_eq!(postpone(Some(read), now, Look::Find).look, Look::Find);
}

#[test]
fn queued_steps_add_up() {
    assert_eq!(
        merge(Change::Step(0.05), Change::Step(0.05)),
        Change::Step(0.1)
    );
}

#[test]
fn a_step_after_a_set_moves_the_set() {
    assert_eq!(
        merge(Change::Set(0.5), Change::Step(-0.1)),
        Change::Set(0.4)
    );
    assert_eq!(
        merge(Change::Set(0.98), Change::Step(0.05)),
        Change::Set(1.0)
    );
}

#[test]
fn a_set_overrides_what_came_before() {
    assert_eq!(
        merge(Change::Step(0.05), Change::Set(0.2)),
        Change::Set(0.2)
    );
}

fn request(target: Target, change: Option<Change>) -> Request {
    Request {
        target,
        change,
        replies: Vec::new(),
    }
}

fn changes(requests: Vec<Request>) -> Vec<(Target, Option<Change>)> {
    batch(requests)
        .into_iter()
        .map(|request| (request.target, request.change))
        .collect()
}

#[test]
fn presses_for_the_same_screens_join() {
    let step = Some(Change::Step(0.05));
    assert_eq!(
        changes(vec![
            request(Target::Focused, step),
            request(Target::Focused, step),
            request(Target::Focused, None),
        ]),
        vec![(Target::Focused, Some(Change::Step(0.1)))],
    );
}

#[test]
fn presses_for_other_screens_stay_apart() {
    let step = Some(Change::Step(0.05));
    let dp1 = Target::Named("DP-1".to_owned());
    assert_eq!(
        changes(vec![
            request(dp1.clone(), step),
            request(Target::All, step),
            request(dp1.clone(), step),
        ]),
        vec![(dp1.clone(), step), (Target::All, step), (dp1, step)],
    );
}

#[test]
fn a_read_before_a_change_stays_apart() {
    let step = Some(Change::Step(0.05));
    assert_eq!(
        changes(vec![request(Target::All, None), request(Target::All, step)]),
        vec![(Target::All, None), (Target::All, step)],
    );
}
