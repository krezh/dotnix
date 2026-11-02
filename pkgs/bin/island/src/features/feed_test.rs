use std::{
    cell::{Cell, RefCell},
    sync::mpsc,
    time::Duration,
};

use futures::{StreamExt, channel::mpsc::unbounded};
use zbus::block_on;

use super::{Feed, Refreshed, Wake};
use crate::supervisor::Ended;

#[test]
fn publishes_what_changed_and_what_was_asked_for() {
    let (sender, states) = unbounded();
    let feed: Feed<Refreshed<u8>> = Feed::new("test", sender);
    let (wake, wakes) = mpsc::channel();
    for wakened in [Wake::Retimed, Wake::Asked, Wake::Asked] {
        wake.send(wakened).unwrap();
    }
    drop(wake);

    let readings = [1, 1, 2, 2];
    let next = Cell::new(0);
    let reported = RefCell::new(Vec::new());
    let ended = feed.poll(
        &wakes,
        || Duration::from_secs(60),
        || {
            next.set(next.get() + 1);
            readings[next.get() - 1]
        },
        |reading| reported.borrow_mut().push(*reading),
    );
    drop(feed);

    assert!(matches!(ended, Ended::Unwanted));
    assert_eq!(next.get(), readings.len());
    assert_eq!(reported.into_inner(), vec![1, 2]);
    let published: Vec<(u8, bool)> = block_on(states.collect::<Vec<_>>())
        .into_iter()
        .map(|state| (state.value, state.refreshing))
        .collect();
    assert_eq!(
        published,
        vec![(1, false), (1, true), (2, false), (2, true), (2, false)]
    );
}

#[test]
fn stops_when_nobody_is_listening() {
    let (sender, states) = unbounded();
    let feed: Feed<Refreshed<u8>> = Feed::new("test", sender);
    drop(states);
    let (_wake, wakes) = mpsc::channel();
    let ended = feed.poll(&wakes, || Duration::ZERO, || 1, |_| ());
    assert!(matches!(ended, Ended::Unwanted));
}
