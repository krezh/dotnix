use std::time::{Duration, Instant, SystemTime};

use super::Toasts;
use crate::features::notifications::notification::{Expire, Notification};

fn notification(expire: Expire, critical: bool) -> Notification {
    Notification {
        id: 1,
        app: "App".to_owned(),
        summary: "Hello".to_owned(),
        body: String::new(),
        critical,
        icon: None,
        image: None,
        lead: None,
        transient: false,
        resident: false,
        expire,
        actions: Vec::new(),
        received: SystemTime::now(),
    }
}

fn shown(notification: Notification) -> (Toasts, Instant) {
    let mut toasts = Toasts::default();
    toasts.show(notification);
    let start = Instant::now();
    toasts.set_shown(true);
    (toasts, start)
}

#[test]
fn counts_down_pauses_under_the_pointer_and_carries_on() {
    let (mut toasts, start) = shown(notification(Expire::After(Duration::from_secs(4)), false));
    let fresh = toasts.cards()[0].countdown().unwrap();
    assert!(fresh.left > 0.99);
    assert_eq!(fresh.running, Some(Duration::from_secs(4)));
    assert_eq!(
        toasts.cards()[0].countdown(),
        Some(fresh),
        "the same on every redraw"
    );

    toasts.set_pointer(true, start + Duration::from_secs(1));
    let paused = toasts.cards()[0].countdown().unwrap();
    assert!(paused.running.is_none());
    assert!((paused.left - 0.75).abs() < 0.01, "{}", paused.left);
    assert!(toasts.expired(start + Duration::from_secs(60)).is_empty());

    let later = start + Duration::from_secs(10);
    toasts.set_pointer(false, later);
    let resumed = toasts.cards()[0].countdown().unwrap();
    assert!((resumed.left - 0.75).abs() < 0.01, "{}", resumed.left);
    let running = resumed.running.unwrap();
    assert!(
        running.abs_diff(Duration::from_secs(3)) < Duration::from_millis(50),
        "{running:?}"
    );
    assert!(resumed.round > paused.round);
    assert!(
        toasts
            .expired(later + running - Duration::from_millis(1))
            .is_empty(),
        "not before the bar empties"
    );
    assert_eq!(
        toasts.expired(later + running).len(),
        1,
        "as the bar empties"
    );
}

#[test]
fn hidden_cards_wait_below_the_stack_and_rise_into_a_freed_place() {
    let mut toasts = Toasts::default();
    for id in 1..=4 {
        toasts.show(Notification {
            id,
            ..notification(Expire::Default, false)
        });
    }
    let below = |toasts: &Toasts| {
        let live: Vec<_> = toasts.cards().iter().filter(|card| !card.leaving).collect();
        live.windows(2)
            .all(|pair| pair[0].y.target() < pair[1].y.target())
    };
    let later = Instant::now() + Duration::from_secs(1);
    for _ in 0..120 {
        toasts.advance(1.0 / 60.0, later);
    }
    let oldest = toasts.cards()[3].y.target();
    assert_eq!(toasts.cards()[3].y.value(), oldest);
    assert!(!toasts.cards()[3].visible);
    assert!(below(&toasts), "newest on top, oldest at the bottom");

    let top = toasts.cards()[0].id;
    let before = toasts.drawn_y();
    let height = toasts.height();
    toasts.dismiss(top);
    assert!(
        toasts
            .drawn_y()
            .iter()
            .zip(&before)
            .all(|(now, was)| (now - was).abs() < 0.01),
        "nothing jumps as the fold starts: {:?} then {before:?}",
        toasts.drawn_y()
    );
    assert!(
        (toasts.height() - height).abs() < 0.01,
        "nor does the island"
    );
    assert!(toasts.cards()[3].visible);
    assert!(below(&toasts));

    let mut last = toasts.drawn_y();
    for frame in 1..=120 {
        toasts.advance(1.0 / 60.0, later);
        let Some(folding) = toasts.cards().iter().find(|card| card.leaving) else {
            break;
        };
        let edge = toasts.drawn_y()[0] + folding.height() * folding.fold.value();
        let drawn = toasts.drawn_y();
        assert!(
            drawn[1] >= edge - 0.01,
            "frame {frame}: the card below never passes the folding edge"
        );
        assert!(
            drawn.iter().zip(&last).all(|(now, was)| now <= was),
            "frame {frame}: the rest only rise"
        );
        assert!(
            toasts.height() <= height + 0.01,
            "frame {frame}: the island never grows"
        );
        last = drawn;
    }
    let risen = toasts
        .cards()
        .iter()
        .rev()
        .find(|card| !card.leaving)
        .unwrap();
    assert!(
        risen.y.value() < oldest,
        "the hidden one rose into the freed place"
    );
}

#[test]
fn a_card_shown_under_the_pointer_waits_and_critical_ones_never_count() {
    let mut toasts = Toasts::default();
    toasts.set_pointer(true, Instant::now());
    toasts.show(notification(Expire::Default, false));
    toasts.set_shown(true);
    let waiting = toasts.cards()[0].countdown().unwrap();
    assert!(waiting.running.is_none() && waiting.left > 0.99);

    let (critical, _) = shown(notification(Expire::Default, true));
    assert!(critical.cards()[0].countdown().is_none());
}
