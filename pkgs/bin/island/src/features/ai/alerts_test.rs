use std::sync::Arc;

use serde_json::{Value, json};

use super::{Alerts, notice};
use crate::features::ai::{
    hook::{Hook, parse},
    session::{Ask, Request},
    state::State,
};

fn hook(event: &str, extra: Value) -> Hook {
    let mut value = json!({"session_id": "s1", "cwd": "/home/krezh/dotnix"});
    value
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    parse(event, &value.to_string()).unwrap()
}

#[test]
fn a_finished_turn_says_where_with_the_start_of_the_reply() {
    let stop = hook("Stop", json!({"last_assistant_message": "**Done.** The build passes.\n\nMore."}));
    let notice = notice(&State::default(), &stop).unwrap();
    assert_eq!(notice.app, "Claude Code");
    assert_eq!(notice.title, "Finished in dotnix");
    assert_eq!(notice.body, "Done. The build passes.");
}

#[test]
fn claude_codes_own_notifications_pass_through() {
    let asking = hook(
        "Notification",
        json!({"notification_type": "elicitation_dialog", "message": "Claude needs your input"}),
    );
    let notice = notice(&State::default(), &asking).unwrap();
    assert_eq!(notice.title, "Claude needs your input");
    assert_eq!(notice.body, "dotnix");
}

#[test]
fn waiting_after_a_finished_turn_is_not_notified_again() {
    let idle = hook(
        "Notification",
        json!({"notification_type": "idle_prompt", "message": "Claude is waiting for your input"}),
    );
    assert_eq!(notice(&State::default(), &idle), None);
}

#[test]
fn a_notification_waits_and_goes_once_a_request_arrives() {
    let mut alerts = Alerts::default();
    let state = State::default();
    let asking = hook(
        "Notification",
        json!({"notification_type": "permission_prompt", "message": "Claude needs your permission"}),
    );
    assert_eq!(alerts.tell(&state, &asking, 0), None);
    assert_eq!(alerts.next_due(), Some(1500));
    assert!(alerts.due(&state, 1499).is_empty());
    alerts.asked("s1");
    assert!(alerts.due(&state, 1500).is_empty());

    alerts.tell(&state, &asking, 2000);
    assert_eq!(alerts.due(&state, 3500).len(), 1);
    assert_eq!(alerts.next_due(), None);
}

#[test]
fn a_finished_turn_is_notified_at_once() {
    let mut alerts = Alerts::default();
    let stop = hook("Stop", json!({"last_assistant_message": "Done."}));
    assert!(alerts.tell(&State::default(), &stop, 0).is_some());
}

#[test]
fn a_request_island_already_shows_is_not_notified_again() {
    let state = State {
        requests: Arc::new(vec![Request {
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
            expires_at: 60_000,
            lifetime: 60_000,
        }]),
        ..State::default()
    };
    let asking = hook(
        "Notification",
        json!({"notification_type": "permission_prompt", "message": "Claude needs your permission"}),
    );
    assert_eq!(notice(&state, &asking), None);
}
