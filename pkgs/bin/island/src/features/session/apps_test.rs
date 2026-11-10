use std::collections::HashSet;

use super::{open_for_test, should_term, still_open};
use crate::features::session::{hyprland::Client, process::Identity};

fn identity(started: u64) -> Identity {
    Identity { pid: 1234, started }
}

fn client(address: &str) -> Client {
    Client {
        address: address.to_owned(),
        class: "Editor".to_owned(),
        title: "Unsaved".to_owned(),
        pid: 1234,
    }
}

#[test]
fn open_until_both_its_process_and_its_window_are_gone() {
    let app = open_for_test("0xabcd", 1234, true, Some(identity(10)));
    assert!(still_open(&app, &[], &HashSet::from([identity(10)])));
    assert!(still_open(&app, &[client("0xabcd")], &HashSet::new()));
    assert!(!still_open(&app, &[], &HashSet::new()));
}

#[test]
fn a_reused_number_does_not_keep_the_old_process_open() {
    let app = open_for_test("0xabcd", 1234, true, Some(identity(10)));
    assert!(!still_open(&app, &[], &HashSet::from([identity(11)])));
}

#[test]
fn a_process_with_a_window_is_asked_through_the_window() {
    let living = HashSet::from([identity(10)]);
    let windowed = HashSet::from([1234]);
    assert!(!should_term(
        identity(10),
        &living,
        &windowed,
        &HashSet::new(),
        true
    ));
    assert!(should_term(
        identity(10),
        &living,
        &HashSet::new(),
        &HashSet::new(),
        false
    ));
    assert!(!should_term(
        identity(10),
        &living,
        &HashSet::new(),
        &living,
        false
    ));
    assert!(should_term(
        identity(10),
        &living,
        &HashSet::new(),
        &living,
        true
    ));
}

#[test]
fn nothing_gone_or_reused_is_signalled() {
    assert!(!should_term(
        identity(10),
        &HashSet::from([identity(11)]),
        &HashSet::new(),
        &HashSet::new(),
        true
    ));
}
