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
    assert_eq!(toasts.cards()[0].countdown(), Some(fresh), "the same on every redraw");

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
    assert!(running.abs_diff(Duration::from_secs(3)) < Duration::from_millis(50), "{running:?}");
    assert!(resumed.round > paused.round);
    assert!(toasts.expired(later + running - Duration::from_millis(1)).is_empty(), "not before the bar empties");
    assert_eq!(toasts.expired(later + running).len(), 1, "as the bar empties");
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
