use std::time::{Duration, Instant};

use super::{Activity, Body, Feeling, Mood, Pose};

#[test]
fn each_feeling_is_found_by_its_name_and_looks_its_own() {
    for feeling in Feeling::ALL {
        assert_eq!(Feeling::named(feeling.name()), Some(feeling));
        assert_ne!(Pose::feeling(Some(feeling)), Pose::feeling(None));
    }
    assert_eq!(Feeling::named("unknown"), None);
}

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
fn nothing_due_is_left_behind_once_woken() {
    let start = Instant::now();
    for near in [false, true] {
        let mut body = Body::new(false, mood(near, false), start, 3);
        for step in 0..400 {
            let now = start + Duration::from_millis(step * 370);
            assert!(settled_due(&mut body, now).is_none_or(|due| due > now));
        }
    }
}

#[test]
fn four_quick_pokes_make_it_grumpy() {
    let start = Instant::now();
    let mut body = Body::new(false, mood(false, false), start, 1);
    for poke in 0..4 {
        body.poke(start + Duration::from_millis(poke * 200));
    }
    let now = start + Duration::from_millis(700);
    assert!(body.grumpy(now));
    assert!(!body.cheering(now));
    assert!(!body.grumpy(now + Duration::from_secs(3)));
}

#[test]
fn it_dozes_off_after_a_long_rest_and_wakes_with_a_start() {
    let start = Instant::now();
    let mut body = Body::new(false, mood(false, false), start, 1);
    body.wake(start + Duration::from_secs(46));
    assert!(body.dozing);
    body.set(mood(true, false), start + Duration::from_secs(47));
    assert!(!body.dozing);
}

#[test]
fn several_finished_at_once_hop_again() {
    let start = Instant::now();
    let mut body = Body::new(false, mood(false, false), start, 1);
    body.cheer(start, 3);
    assert_eq!(body.encore, 2);
    let mut now = start;
    for _ in 0..240 {
        now += Duration::from_millis(16);
        body.step(0.016, now);
    }
    assert_eq!(body.encore, 0);
}

#[test]
fn its_eyes_follow_the_pointer_while_near() {
    let start = Instant::now();
    let mut body = Body::new(false, mood(true, false), start, 1);
    body.point(Some((3.0, 0.0)), start);
    assert!(body.look_x.target() > 0.5);
    body.point(Some((-3.0, 0.0)), start);
    assert!(body.look_x.target() < -0.5);
    body.point(None, start);
    assert_eq!(body.look_x.target(), 0.0);
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
