use std::collections::HashMap;

use super::{MOST_POSTED, newest_unknown};
use crate::features::gotify::state::{Entry, Extras, RawMessage};

fn entry(id: u64) -> Entry {
    Entry::from_raw(
        RawMessage {
            id,
            appid: 0,
            title: String::new(),
            message: String::new(),
            priority: 0,
            date: String::new(),
            extras: Extras::default(),
        },
        &HashMap::new(),
    )
}

#[test]
fn a_full_server_posts_only_its_newest() {
    let entries: Vec<Entry> = (0..300).rev().map(entry).collect();
    let posted: Vec<u64> = newest_unknown(&entries, &HashMap::new())
        .map(|entry| entry.id)
        .collect();
    assert_eq!(posted.len(), MOST_POSTED);
    assert_eq!(posted.first(), Some(&299));
}

#[test]
fn what_is_known_is_not_posted_again() {
    let entries = vec![entry(3), entry(2), entry(1)];
    let known = HashMap::from([(2, None), (1, None)]);
    let posted: Vec<u64> = newest_unknown(&entries, &known)
        .map(|entry| entry.id)
        .collect();
    assert_eq!(posted, vec![3]);
}
