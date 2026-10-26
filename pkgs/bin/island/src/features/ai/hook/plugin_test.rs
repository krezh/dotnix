use serde_json::json;

use super::{HOOKS, WAIT_SECONDS, hooks};
use crate::features::ai::hook::{EVENTS, parse};

#[test]
fn each_hook_waits_exactly_when_island_answers_it() {
    for entry in HOOKS {
        let tool = if entry.matcher == "*" {
            "Bash"
        } else {
            entry.matcher
        };
        let input = json!({ "session_id": "s1", "tool_name": tool }).to_string();
        let hook = parse(entry.run, &input).unwrap();
        assert_eq!(
            hook.blocks(),
            entry.waits,
            "{} on {}",
            entry.run,
            entry.matcher
        );
    }
}

#[test]
fn every_event_but_the_status_line_comes_from_a_hook() {
    for event in EVENTS.iter().filter(|event| **event != "StatusLine") {
        assert!(
            HOOKS.iter().any(|entry| entry.run == *event),
            "nothing runs {event}"
        );
    }
}

#[test]
fn questions_get_a_second_hook_on_their_tool_that_waits() {
    let value = hooks("/nix/store/x island/bin/island");
    let groups = value["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0]["matcher"], "*");
    assert_eq!(groups[0]["hooks"][0]["async"], true);
    assert_eq!(groups[1]["matcher"], "AskUserQuestion");
    assert_eq!(
        groups[1]["hooks"][0]["command"],
        "'/nix/store/x island/bin/island' ai-hook PreToolUse"
    );
    assert_eq!(groups[1]["hooks"][0]["timeout"], WAIT_SECONDS);
    assert!(groups[1]["hooks"][0].get("async").is_none());
}
