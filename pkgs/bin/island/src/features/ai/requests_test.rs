use futures::channel::oneshot;
use serde_json::{Value, json};

use super::{MOST_MS, Waiting, settle};
use crate::features::ai::{
    hook::{Hook, parse},
    state::State,
};

fn hook(event: &str, session: &str, extra: Value) -> Hook {
    let mut value = json!({"session_id": session, "cwd": "/home/krezh/dotnix"});
    value
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    parse(event, &value.to_string()).unwrap()
}

fn bash(command: &str) -> Value {
    json!({"tool_name": "Bash", "tool_input": {"command": command}})
}

/// Asks permission for `command` in `session` at `at`, and gives back what the hook gets told.
fn ask_at(
    state: &mut State,
    waiting: &mut Waiting,
    session: &str,
    command: &str,
    at: i64,
) -> oneshot::Receiver<String> {
    let (reply, told) = oneshot::channel();
    waiting.ask(state, &hook("PermissionRequest", session, bash(command)), reply, at, 60_000);
    told
}

fn ask(
    state: &mut State,
    waiting: &mut Waiting,
    session: &str,
    command: &str,
) -> oneshot::Receiver<String> {
    ask_at(state, waiting, session, command, 0)
}

#[test]
fn the_call_running_drops_its_request_alone() {
    let mut state = State::default();
    let mut waiting = Waiting::default();
    let mut touch = ask(&mut state, &mut waiting, "s1", "touch a");
    let _rm = ask(&mut state, &mut waiting, "s1", "rm b");

    assert!(!settle(&mut state, &mut waiting, &hook("PreToolUse", "s1", bash("touch a")), 1));
    assert!(!settle(&mut state, &mut waiting, &hook("PostToolUse", "s2", bash("touch a")), 1));
    assert_eq!(state.requests.len(), 2);

    assert!(settle(&mut state, &mut waiting, &hook("PostToolUse", "s1", bash("touch a")), 1));
    assert_eq!(state.requests.len(), 1);
    assert_eq!(touch.try_recv().unwrap().as_deref(), Some(""));
}

#[test]
fn one_finished_call_drops_one_of_two_alike() {
    let mut state = State::default();
    let mut waiting = Waiting::default();
    let _first = ask(&mut state, &mut waiting, "s1", "touch a");
    let _second = ask(&mut state, &mut waiting, "s1", "touch a");

    assert!(settle(&mut state, &mut waiting, &hook("PostToolUse", "s1", bash("touch a")), 1));
    assert_eq!(state.requests.len(), 1);
    assert_eq!(state.requests[0].id, 2);
}

#[test]
fn a_finished_turn_drops_what_its_session_asked() {
    let mut state = State::default();
    let mut waiting = Waiting::default();
    let _denied = ask(&mut state, &mut waiting, "s1", "touch a");
    let _other = ask(&mut state, &mut waiting, "s2", "touch a");

    assert!(settle(&mut state, &mut waiting, &hook("Stop", "s1", json!({})), 1));
    assert_eq!(state.requests.len(), 1);
    assert_eq!(state.requests[0].session, "s2");

    let prompt = hook("UserPromptSubmit", "s2", json!({"prompt": "go on"}));
    assert!(settle(&mut state, &mut waiting, &prompt, 1));
    assert!(state.requests.is_empty());
}

#[test]
fn a_late_tell_leaves_what_was_asked_after_it() {
    let mut state = State::default();
    let mut waiting = Waiting::default();
    let _asked = ask_at(&mut state, &mut waiting, "s1", "touch a", 5_000);

    assert!(!settle(&mut state, &mut waiting, &hook("Stop", "s1", json!({})), 4_000));
    assert!(!settle(&mut state, &mut waiting, &hook("PostToolUse", "s1", bash("touch a")), 4_000));
    assert_eq!(state.requests.len(), 1);
}

#[test]
fn held_requests_keep_their_time_until_no_island_holds_them() {
    let mut state = State::default();
    let mut waiting = Waiting::default();
    let _asked = ask(&mut state, &mut waiting, "s1", "touch a");

    assert!(waiting.hold(&mut state, 1, true, 50_000));
    assert!(!waiting.expire(&mut state, 70_000));
    assert_eq!(waiting.next_due(&state), Some(MOST_MS));
    assert!(!waiting.hold(&mut state, 2, true, 71_000));
    assert!(!waiting.hold(&mut state, 1, false, 80_000));
    assert_eq!(state.hold.since, Some(50_000));

    assert!(waiting.hold(&mut state, 2, false, 90_000));
    assert_eq!(state.hold.since, None);
    assert_eq!(state.requests[0].expires_at, 100_000);
    assert!(!waiting.expire(&mut state, 99_999));
    assert!(waiting.expire(&mut state, 100_000));
    assert!(state.requests.is_empty());
}

#[test]
fn a_held_request_still_goes_at_the_longest_it_may_wait() {
    let mut state = State::default();
    let mut waiting = Waiting::default();
    let _asked = ask(&mut state, &mut waiting, "s1", "touch a");

    waiting.hold(&mut state, 1, true, 50_000);
    assert!(!waiting.expire(&mut state, MOST_MS - 1));
    assert!(waiting.expire(&mut state, MOST_MS));
    assert!(state.requests.is_empty());
}

#[test]
fn a_request_let_go_never_outlasts_the_longest_wait() {
    let mut state = State::default();
    let mut waiting = Waiting::default();
    let _asked = ask(&mut state, &mut waiting, "s1", "touch a");

    waiting.hold(&mut state, 1, true, 10_000);
    waiting.hold(&mut state, 1, false, MOST_MS - 1_000);
    assert_eq!(state.requests[0].expires_at, MOST_MS);
}

#[test]
fn one_held_near_its_end_gets_a_moment_once_let_go() {
    let mut state = State::default();
    let mut waiting = Waiting::default();
    let _asked = ask(&mut state, &mut waiting, "s1", "touch a");
    waiting.hold(&mut state, 1, true, 59_500);
    waiting.hold(&mut state, 1, false, 59_600);
    assert_eq!(state.requests[0].expires_at, 62_600);
}

#[test]
fn an_ended_session_drops_what_it_asked() {
    let mut state = State::default();
    let mut waiting = Waiting::default();
    let _asked = ask(&mut state, &mut waiting, "s1", "touch a");
    assert!(settle(&mut state, &mut waiting, &hook("SessionEnd", "s1", json!({})), 1));
    assert!(state.requests.is_empty());
}
