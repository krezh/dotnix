use std::{
    path::PathBuf,
    time::{Duration, UNIX_EPOCH},
};

use super::oldest;

#[test]
fn only_the_oldest_go() {
    let file = |seconds: u64| {
        (
            UNIX_EPOCH + Duration::from_secs(seconds),
            PathBuf::from(format!("/art/{seconds}")),
        )
    };
    let gone = oldest(vec![file(2), file(4), file(1), file(3)], 2);
    assert_eq!(gone, vec![PathBuf::from("/art/2"), PathBuf::from("/art/1")]);
}

#[test]
fn nothing_goes_under_the_limit() {
    assert!(oldest(Vec::new(), 2).is_empty());
}
