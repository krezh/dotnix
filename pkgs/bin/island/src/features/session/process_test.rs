use std::time::Duration;

use super::{Bell, parse_stat, readable};

#[test]
fn reads_names_with_spaces_and_parentheses() {
    let line = format!("1234 (an odd (name)) S 42 {} 9876 0", ["0"; 17].join(" "));
    let parsed = parse_stat(1234, &line).unwrap();
    assert_eq!(parsed.parent, 42);
    assert_eq!(parsed.identity.started, 9876);
    assert_eq!(parsed.name, "an odd (name)");
    assert!(!parsed.dead);
}

#[test]
fn knows_a_zombie_is_dead() {
    let line = format!("7 (gone) Z 1 {} 5 0", ["0"; 17].join(" "));
    assert!(parse_stat(7, &line).unwrap().dead);
}

#[test]
fn refuses_a_cut_short_line() {
    assert!(parse_stat(1, "1 (x) S").is_err());
    assert!(parse_stat(1, "garbage").is_err());
}

#[test]
fn a_bell_wakes_whoever_waits_until_quieted() {
    let bell = Bell::new().unwrap();
    assert!(!readable(bell.fd(), Duration::ZERO).unwrap());
    bell.ring();
    bell.ring();
    assert!(readable(bell.fd(), Duration::ZERO).unwrap());
    bell.quiet();
    assert!(!readable(bell.fd(), Duration::ZERO).unwrap());
}
