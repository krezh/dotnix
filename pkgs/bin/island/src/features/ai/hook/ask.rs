use serde_json::Value;

use super::{input::Hook, registry};
use crate::features::ai::session::{Ask, Question, Request};

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn questions(input: &Value) -> Vec<Question> {
    input
        .get("questions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|question| Question {
            header: text(question, "header"),
            text: text(question, "question"),
            multi: question
                .get("multiSelect")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            options: question
                .get("options")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|option| (text(option, "label"), text(option, "description")))
                .collect(),
        })
        .filter(|question| !question.text.is_empty() && !question.options.is_empty())
        .collect()
}

/// The request a blocking hook makes of you; `None` for one island cannot show, which goes to the
/// terminal.
pub fn request(hook: &Hook, id: u64, now: i64, expires_at: i64) -> Option<Request> {
    let ask = match hook {
        Hook::Permission {
            tool,
            input,
            suggestion,
            ..
        } => {
            let subject = match tool.as_str() {
                "Bash" => text(input, "command"),
                "WebFetch" => text(input, "url"),
                _ => {
                    let path = text(input, "file_path");
                    if path.is_empty() {
                        registry::describe(tool, input).1
                    } else {
                        path
                    }
                }
            };
            Ask::Permission {
                tool: tool.clone(),
                subject,
                reason: text(input, "description"),
                suggestion: suggestion.clone(),
            }
        }
        Hook::Questions { input, .. } => {
            let questions = questions(input);
            if questions.is_empty() {
                return None;
            }
            Ask::Questions {
                questions,
                input: input.clone(),
            }
        }
        _ => return None,
    };
    let from = hook.from();
    Some(Request {
        id,
        session: from.session.clone(),
        project: from
            .cwd
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_owned(),
        ask,
        received_at: now,
        expires_at,
    })
}
