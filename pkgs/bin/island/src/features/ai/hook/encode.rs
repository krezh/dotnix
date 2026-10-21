use serde_json::{Map, Value, json};

use crate::features::ai::session::{Answer, Ask, Request};

/// What the waiting hook prints for Claude Code: a decision, or nothing, which leaves the question
/// to the terminal.
pub fn answer(request: &Request, answer: &Answer) -> String {
    let output = match (&request.ask, answer) {
        (_, Answer::Terminal) => return String::new(),
        (Ask::Permission { suggestion, .. }, Answer::Allow | Answer::Always | Answer::Deny) => {
            let mut decision = Map::new();
            let behavior = if *answer == Answer::Deny {
                "deny"
            } else {
                "allow"
            };
            decision.insert("behavior".to_owned(), behavior.into());
            match answer {
                Answer::Always => {
                    if let Some(rule) = suggestion {
                        decision.insert("updatedPermissions".to_owned(), json!([rule]));
                    }
                }
                Answer::Deny => {
                    decision.insert("message".to_owned(), "Denied from island.".into());
                }
                _ => {}
            }
            json!({
                "hookSpecificOutput": {
                    "hookEventName": "PermissionRequest",
                    "decision": decision,
                }
            })
        }
        (Ask::Questions { questions, input }, Answer::Picked(picked)) => {
            let answers: Map<String, Value> = questions
                .iter()
                .zip(picked)
                .filter(|(_, labels)| !labels.is_empty())
                .map(|(question, labels)| (question.text.clone(), labels.join(", ").into()))
                .collect();
            let mut updated = input.as_object().cloned().unwrap_or_default();
            updated.insert("answers".to_owned(), Value::Object(answers));
            json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "allow",
                    "updatedInput": updated,
                }
            })
        }
        // An answer that does not fit the request leaves it to the terminal.
        _ => return String::new(),
    };
    output.to_string()
}

#[cfg(test)]
#[path = "encode_test.rs"]
mod tests;
