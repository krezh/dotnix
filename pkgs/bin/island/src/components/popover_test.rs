use std::time::{Duration, Instant};

use gpui::{Bounds, IntoElement, Pixels, point, px, size};

use super::{Morph, Popovers, cover, opener_shown, rows_shown};

const FRAME: Duration = Duration::from_millis(16);

fn chip(left: f32, width: f32) -> Option<Bounds<Pixels>> {
    Some(Bounds {
        origin: point(px(left), px(80.0)),
        size: size(px(width), px(30.0)),
    })
}

/// Runs `frames` frames, each with what it opens from at `anchor`, returning when it got to.
fn run(
    morph: &mut Morph,
    open: bool,
    anchor: Option<Bounds<Pixels>>,
    from: Instant,
    frames: u32,
) -> Instant {
    let mut now = from;
    for _ in 0..frames {
        now += FRAME;
        morph.advance(open, anchor, now);
    }
    now
}

#[test]
fn it_starts_exactly_at_what_opens_it_however_long_it_rested() {
    let start = Instant::now();
    let mut morph = Morph::new(start);
    morph.advance(true, chip(100.0, 80.0), start + Duration::from_secs(60));
    assert_eq!(morph.grown.value(), 0.0);
}

#[test]
fn it_takes_no_clicks_until_its_rows_show() {
    let start = Instant::now();
    let mut morph = Morph::new(start);
    let mut now = start;
    let mut was_live = false;
    for _ in 0..60 {
        now = run(&mut morph, true, chip(100.0, 80.0), now, 1);
        let growth = morph.growth(chip(100.0, 80.0));
        if growth.live {
            assert_eq!(rows_shown(growth.look), 1.0, "live only once its rows show");
            was_live = true;
        }
    }
    assert!(was_live, "it does get there");
}

#[test]
fn it_takes_no_clicks_from_the_moment_it_closes() {
    let start = Instant::now();
    let mut morph = Morph::new(start);
    let now = run(&mut morph, true, chip(100.0, 80.0), start, 60);
    assert!(morph.growth(chip(100.0, 80.0)).live);
    run(&mut morph, false, chip(100.0, 80.0), now, 1);
    let growth = morph.growth(chip(100.0, 80.0));
    assert!(growth.shown, "still drawn as it closes");
    assert!(!growth.open && !growth.live, "but out of the way");
}

#[test]
fn closing_turns_back_from_where_it_is() {
    let start = Instant::now();
    let mut morph = Morph::new(start);
    let now = run(&mut morph, true, chip(100.0, 80.0), start, 6);
    let reached = morph.grown.value();
    assert!(reached > 0.1 && reached < 0.95, "caught partway");
    run(&mut morph, false, chip(100.0, 80.0), now, 1);
    assert!(
        (morph.grown.value() - reached).abs() < 0.2,
        "no jump as it turns back"
    );
}

#[test]
fn it_leaves_as_it_stood_when_something_is_picked() {
    let start = Instant::now();
    let mut morph = Morph::new(start);
    let now = run(&mut morph, true, chip(100.0, 80.0), start, 60);
    assert_eq!(morph.hold(Some(0)), Some(0));
    run(&mut morph, false, chip(100.0, 80.0), now, 1);
    assert_eq!(
        morph.hold(Some(2)),
        Some(0),
        "closing, it keeps the old choice"
    );
    let now = run(&mut morph, false, chip(100.0, 80.0), now, 60);
    run(&mut morph, true, chip(100.0, 80.0), now, 1);
    assert_eq!(
        morph.hold(Some(2)),
        Some(2),
        "opened again, it shows the new one"
    );
}

#[test]
fn the_same_words_never_show_twice() {
    for step in 0..=100 {
        let look = step as f32 / 100.0;
        assert!(
            opener_shown(look) == 0.0 || rows_shown(look) == 0.0,
            "at {look} both show"
        );
        assert!(
            cover(look) < 1.0 || rows_shown(look) == 1.0,
            "at {look} it is dark before its rows have arrived"
        );
    }
}

#[test]
fn it_follows_what_opens_it_smoothly_only_while_shown() {
    let start = Instant::now();
    let mut morph = Morph::new(start);
    let now = run(&mut morph, false, chip(100.0, 80.0), start, 2);
    let now = run(&mut morph, false, chip(300.0, 40.0), now, 1);
    assert_eq!(morph.left.value(), 300.0, "at rest it is simply there");
    let now = run(&mut morph, true, chip(300.0, 40.0), now, 60);
    run(&mut morph, false, chip(260.0, 120.0), now, 1);
    let left = morph.left.value();
    assert!(
        left > 290.0,
        "picking a wider choice does not jump the list"
    );
}

#[test]
fn a_panel_that_is_not_showing_closes_what_it_opened() {
    let popovers = Popovers::default();
    let id = "pick".into();
    popovers.enter_panel(1.0, true);
    assert!(popovers.grow(&id, true).open);
    popovers.enter_panel(0.6, false);
    assert!(
        !popovers.grow(&id, true).open,
        "however it is asked to stay"
    );
}

#[test]
fn nothing_is_shown_for_a_panel_faded_away() {
    let popovers = Popovers::default();
    popovers.enter_panel(0.0, false);
    popovers.show(gpui::Empty.into_any_element());
    assert!(popovers.overlay().0.is_none());
}

#[test]
fn each_island_keeps_its_own_motion() {
    let (one, other) = (Popovers::default(), Popovers::default());
    let id = "pick".into();
    assert!(one.grow(&id, true).open);
    assert!(!other.grow(&id, false).open);
    assert!(one.grow(&id, true).open, "the other island changed nothing");
}
