use std::time::Duration;

use serde_json::Value;

use super::countdown;
use crate::features::ai::{
    session::{Ask, Request},
    state::Hold,
};

fn request(expires_at: i64) -> Request {
    Request {
        id: 1,
        session: "s1".to_owned(),
        project: "dotnix".to_owned(),
        ask: Ask::Permission {
            tool: "Bash".to_owned(),
            subject: "touch a".to_owned(),
            reason: String::new(),
            suggestion: None,
            input: Value::Null,
        },
        received_at: 0,
        expires_at,
        lifetime: 60_000,
    }
}

#[test]
fn a_new_request_counts_down_all_its_time() {
    let bar = countdown(&request(60_000), Hold::default()).unwrap();
    assert_eq!(bar.left, 1.0);
    assert_eq!(bar.running, Some(Duration::from_secs(60)));
}

#[test]
fn a_held_request_stands_still_where_it_was() {
    let hold = Hold {
        since: Some(30_000),
        released_at: 0,
        round: 1,
    };
    let bar = countdown(&request(60_000), hold).unwrap();
    assert_eq!(bar.left, 0.5);
    assert_eq!(bar.running, None);
    assert_eq!(bar.round, 1);
}

#[test]
fn a_released_request_carries_on_from_there() {
    let hold = Hold {
        since: None,
        released_at: 50_000,
        round: 2,
    };
    let bar = countdown(&request(80_000), hold).unwrap();
    assert_eq!(bar.left, 0.5);
    assert_eq!(bar.running, Some(Duration::from_secs(30)));
    assert_eq!(bar.round, 2);
}
