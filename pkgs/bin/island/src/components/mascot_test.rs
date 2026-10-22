use std::time::{Duration, Instant};

use super::{Activity, Body, Extra, Mood};
use crate::theme::{GREEN, LAVENDER, YELLOW};

fn mood(activity: Activity) -> Mood {
    Mood {
        activity,
        tired: 0.0,
        color: LAVENDER,
        cheer_color: GREEN,
        near: false,
        loading: false,
    }
}

/// Steps `body` at 60 frames a second for `seconds`, and gives back the time it got to.
fn run(body: &mut Body, from: Instant, seconds: f32) -> Instant {
    let dt = 1.0 / 60.0;
    let mut now = from;
    for _ in 0..(seconds / dt) as usize {
        now += Duration::from_secs_f32(dt);
        body.step(dt, now);
    }
    now
}

#[test]
fn a_calm_one_rests_until_something_is_due() {
    let start = Instant::now();
    let mut body = Body::new(false, mood(Activity::Rest), start, 0);
    run(&mut body, start, 0.5);
    assert!(body.is_settled());
    assert!(body.due().is_some_and(|at| at > start));
}

#[test]
fn a_lively_one_never_rests() {
    let start = Instant::now();
    let mut body = Body::new(true, mood(Activity::Rest), start, 0);
    run(&mut body, start, 0.5);
    assert!(!body.is_settled());
}

#[test]
fn asking_makes_it_hop_and_land() {
    let start = Instant::now();
    let mut body = Body::new(false, mood(Activity::Rest), start, 0);
    body.set(
        Mood {
            color: YELLOW,
            ..mood(Activity::Asking)
        },
        start,
    );
    assert!(!body.is_settled());
    let now = run(&mut body, start, 0.2);
    assert!(body.lift > 0.0);
    run(&mut body, now, 1.0);
    assert!(body.alert.value() > 0.9);
    assert!(matches!(body.pose(now).extra, Extra::Asking(_)));
}

#[test]
fn a_calm_one_keeps_moving_while_busy() {
    let start = Instant::now();
    let mut body = Body::new(false, mood(Activity::Busy), start, 0);
    let mut hopped = false;
    let mut now = start;
    for _ in 0..30 {
        now = run(&mut body, now, 0.1);
        hopped |= body.lift > 0.0;
        assert!(!body.is_settled());
    }
    assert!(hopped);
    body.set(mood(Activity::Rest), now);
    run(&mut body, now, 1.5);
    assert!(!body.moving());
}

#[test]
fn asleep_it_shuts_its_eyes() {
    let start = Instant::now();
    let mut body = Body::new(false, mood(Activity::Asleep), start, 0);
    let now = run(&mut body, start, 1.0);
    let pose = body.pose(now);
    assert!(pose.asleep);
    assert!(pose.lid > 0.9);
}

#[test]
fn loading_shows_a_spinner_and_keeps_it_moving() {
    let start = Instant::now();
    let mut body = Body::new(false, mood(Activity::Rest), start, 0);
    body.set(
        Mood {
            loading: true,
            ..mood(Activity::Rest)
        },
        start,
    );
    let now = run(&mut body, start, 0.5);
    assert!(!body.is_settled());
    assert!(matches!(body.pose(now).extra, Extra::Loading(_)));

    body.set(
        Mood {
            loading: true,
            ..mood(Activity::Asking)
        },
        now,
    );
    assert!(matches!(body.pose(now).extra, Extra::Asking(_)));
}

#[test]
fn a_poke_sets_it_wobbling() {
    let start = Instant::now();
    let mut body = Body::new(false, mood(Activity::Rest), start, 0);
    body.poke(start);
    let now = run(&mut body, start, 0.1);
    assert!(!body.is_settled());
    assert!(body.pose(now).wobble.iter().any(|(_, size, _)| size.abs() > 0.01));
}
