use std::time::{Duration, Instant};

use super::{Activity, Body, Mood};

fn mood(near: bool, loading: bool) -> Mood {
    Mood {
        activity: Activity::Rest,
        tired: 0.0,
        color: 0,
        cheer_color: 0,
        near,
        loading,
        warning: false,
    }
}

fn settled_due(body: &mut Body, now: Instant) -> Option<Instant> {
    body.wake(now);
    body.due()
}

#[test]
fn nothing_stays_due_once_woken_while_the_pointer_is_near() {
    let start = Instant::now();
    let mut body = Body::new(true, mood(true, false), start, 1);
    let later = start + Duration::from_secs(60);
    assert!(settled_due(&mut body, later).is_some_and(|due| due > later));
}

#[test]
fn nothing_stays_due_once_woken_while_loading() {
    let start = Instant::now();
    let mut body = Body::new(true, mood(false, true), start, 1);
    let later = start + Duration::from_secs(60);
    assert!(settled_due(&mut body, later).is_some_and(|due| due > later));
}

#[test]
fn a_body_made_calm_turns_lively_when_told() {
    let mut body = Body::new(false, mood(false, false), Instant::now(), 1);
    assert!(body.is_settled());
    body.set_lively(true);
    assert!(!body.is_settled());
}

#[test]
fn glances_stay_due_at_rest() {
    let start = Instant::now();
    let mut body = Body::new(true, mood(false, false), start, 1);
    let later = start + Duration::from_secs(60);
    let due = settled_due(&mut body, later).expect("a glance or blink is due");
    assert!(due > later);
    assert!(due < later + Duration::from_secs(10));
}
