use super::*;

#[test]
fn a_shown_frame_waits_for_the_compositor() {
    let mut turn = Turn::new();
    assert!(turn.ready());
    assert!(turn.start());
    assert!(!turn.ready());
    turn.answered();
    assert!(turn.ready());
    assert!(turn.start());
}

#[test]
fn a_missed_frame_waits_before_trying_again() {
    let now = Instant::now();
    let mut turn = Turn::new();
    turn.start();
    turn.missed(now);
    assert!(!turn.ready());
    assert!(!turn.elapse(now));
    assert!(!turn.ready());
    assert!(turn.elapse(now + RETRY));
    assert!(turn.ready());
    assert_eq!(turn.again(), None);
}

#[test]
fn trying_again_does_not_ask_the_compositor_twice() {
    let now = Instant::now();
    let mut turn = Turn::new();
    assert!(turn.start());
    turn.missed(now);
    turn.elapse(now + RETRY);
    assert!(!turn.start());
    turn.answered();
    assert!(turn.start());
}
