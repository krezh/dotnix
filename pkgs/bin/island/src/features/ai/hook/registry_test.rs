use serde_json::json;

use super::{apply, describe, resumed};
use crate::features::ai::{
    hook::input::parse,
    session::{Activity, LineKind, Live, Session, Step, StepState},
};

fn hook(event: &str, extra: serde_json::Value) -> super::Hook {
    let mut value = json!({
        "session_id": "s1",
        "cwd": "/home/krezh/dotnix/pkgs/bin/island",
        "permission_mode": "default",
    });
    value
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    parse(event, &value.to_string()).unwrap()
}

fn steps(session: &Session) -> &[Step] {
    &session.current().unwrap().steps
}

#[test]
fn a_session_works_through_its_steps_and_finishes() {
    let mut sessions = Vec::new();
    apply(
        &mut sessions,
        &hook("SessionStart", json!({"model": "claude-opus-5-5"})),
        1,
    );
    assert_eq!(sessions[0].activity, Activity::Idle);

    apply(
        &mut sessions,
        &hook("UserPromptSubmit", json!({"prompt": "Add a pace tick"})),
        2,
    );
    assert_eq!(sessions[0].activity, Activity::Working);
    assert_eq!(sessions[0].current().unwrap().prompt, "Add a pace tick");

    let edit = json!({
        "tool_name": "Edit",
        "tool_input": {"file_path": "/x/src/features/ai/overview.rs", "old_string": "a", "new_string": "b\nc"},
    });
    apply(&mut sessions, &hook("PreToolUse", edit.clone()), 3);
    assert_eq!(sessions[0].doing, "Editing ai/overview.rs");
    assert_eq!(steps(&sessions[0])[0].state, StepState::Now);
    let Some(Live::Editing { lines, .. }) = &steps(&sessions[0])[0].live else {
        panic!("expected the edit to be followed");
    };
    let added: Vec<_> = lines
        .iter()
        .filter(|line| line.kind == LineKind::Added)
        .map(|line| line.text.as_str())
        .collect();
    assert_eq!(added, ["b", "c"]);

    apply(&mut sessions, &hook("PostToolUse", edit), 4);
    assert_eq!(steps(&sessions[0])[0].state, StepState::Done);

    apply(
        &mut sessions,
        &hook("Stop", json!({"last_assistant_message": "Done."})),
        5,
    );
    assert_eq!(sessions[0].activity, Activity::Idle);
    assert_eq!(sessions[0].reply(), "Done.");

    apply(
        &mut sessions,
        &hook("SessionEnd", json!({"reason": "other"})),
        6,
    );
    assert!(sessions.is_empty());
}

#[test]
fn each_prompt_adds_to_the_history_instead_of_replacing_it() {
    let mut sessions = Vec::new();
    let read = |file: &str| json!({"tool_name": "Read", "tool_input": {"file_path": file}});
    apply(
        &mut sessions,
        &hook("UserPromptSubmit", json!({"prompt": "first"})),
        1,
    );
    apply(&mut sessions, &hook("ToolUse", read("/x/a.rs")), 2);
    apply(
        &mut sessions,
        &hook("Stop", json!({"last_assistant_message": "One."})),
        3,
    );
    apply(
        &mut sessions,
        &hook("UserPromptSubmit", json!({"prompt": "second"})),
        4,
    );
    apply(&mut sessions, &hook("ToolUse", read("/x/b.rs")), 5);

    let history = &sessions[0].history;
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].prompt, "first");
    assert_eq!(history[0].reply, "One.");
    assert_eq!(history[0].steps[0].target, "x/a.rs");
    assert_eq!(history[1].steps[0].target, "x/b.rs");
    assert!(history[1].steps[0].seq > history[0].steps[0].seq);
}

#[test]
fn a_permission_waits_and_resumes() {
    let mut sessions = Vec::new();
    let ask = json!({
        "tool_name": "Bash",
        "tool_input": {"command": "nix build .#island", "description": "Build island"},
        "permission_suggestions": [{"type": "addRules"}],
    });
    apply(&mut sessions, &hook("PermissionRequest", ask), 1);
    assert_eq!(sessions[0].activity, Activity::Waiting);
    assert_eq!(sessions[0].doing, "Wants to: Build island");
    assert_eq!(steps(&sessions[0])[0].state, StepState::Waiting);

    resumed(&mut sessions, "s1");
    assert_eq!(sessions[0].activity, Activity::Working);
    assert_eq!(steps(&sessions[0])[0].state, StepState::Now);
    assert_eq!(sessions[0].doing, "Build island");

    let bare = json!({"tool_name": "Bash", "tool_input": {"command": "nix build .#island"}});
    apply(&mut sessions, &hook("PreToolUse", bare), 2);
    assert_eq!(sessions[0].doing, "Running nix build .#island");
}

#[test]
fn tools_read_as_a_verb_and_what_they_touch() {
    assert_eq!(
        describe("Grep", &json!({"pattern": "duration_ms"})),
        ("Search".to_owned(), "\"duration_ms\"".to_owned())
    );
    assert_eq!(
        describe("mcp__memini__memory_recall", &json!({})),
        ("memory recall".to_owned(), "memini".to_owned())
    );
}
