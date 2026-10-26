use serde_json::{Value, json};

use super::answer;
use crate::features::ai::session::{Answer, Ask, Question, Request};

fn request(ask: Ask) -> Request {
    Request {
        id: 1,
        session: "s1".to_owned(),
        project: "dotnix".to_owned(),
        ask,
        received_at: 0,
        expires_at: 0,
        lifetime: 0,
    }
}

fn permission() -> Request {
    request(Ask::Permission {
        tool: "Bash".to_owned(),
        subject: "nix build".to_owned(),
        reason: String::new(),
        suggestion: Some(
            json!({"type": "addRules", "rules": [{"toolName": "Bash", "ruleContent": "nix build:*"}], "behavior": "allow", "destination": "localSettings"}),
        ),
        input: json!({"command": "nix build"}),
    })
}

fn parsed(output: &str) -> Value {
    serde_json::from_str(output).unwrap()
}

#[test]
fn allow_and_deny_follow_the_permission_request_shape() {
    let allow = parsed(&answer(&permission(), &Answer::Allow));
    assert_eq!(
        allow,
        json!({"hookSpecificOutput": {"hookEventName": "PermissionRequest", "decision": {"behavior": "allow"}}})
    );
    let deny = parsed(&answer(&permission(), &Answer::Deny));
    assert_eq!(deny["hookSpecificOutput"]["decision"]["behavior"], "deny");
    assert!(deny["hookSpecificOutput"]["decision"]["message"].is_string());
}

#[test]
fn always_hands_back_the_suggested_rule() {
    let always = parsed(&answer(&permission(), &Answer::Always));
    let decision = &always["hookSpecificOutput"]["decision"];
    assert_eq!(decision["behavior"], "allow");
    assert_eq!(decision["updatedPermissions"][0]["type"], "addRules");
}

#[test]
fn questions_echo_the_input_with_answers_joined() {
    let input = json!({"questions": [{"question": "Which?"}, {"question": "Columns?"}]});
    let ask = Ask::Questions {
        questions: vec![
            Question {
                header: "Layout".to_owned(),
                text: "Which?".to_owned(),
                multi: false,
                options: Vec::new(),
            },
            Question {
                header: "Columns".to_owned(),
                text: "Columns?".to_owned(),
                multi: true,
                options: Vec::new(),
            },
        ],
        input: input.clone(),
    };
    let picked = Answer::Picked(vec![
        vec!["Cards".to_owned()],
        vec!["Owner".to_owned(), "Size".to_owned()],
    ]);
    let output = parsed(&answer(&request(ask), &picked));
    let hook = &output["hookSpecificOutput"];
    assert_eq!(hook["hookEventName"], "PreToolUse");
    assert_eq!(hook["permissionDecision"], "allow");
    assert_eq!(hook["updatedInput"]["questions"], input["questions"]);
    assert_eq!(
        hook["updatedInput"]["answers"],
        json!({"Which?": "Cards", "Columns?": "Owner, Size"})
    );
}

#[test]
fn the_terminal_gets_nothing() {
    assert_eq!(answer(&permission(), &Answer::Terminal), "");
    assert_eq!(answer(&permission(), &Answer::Picked(Vec::new())), "");
}
