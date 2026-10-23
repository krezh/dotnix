use serde_json::Value;

/// What every hook says about where it comes from.
#[derive(Clone, Debug, PartialEq)]
pub struct Origin {
    pub session: String,
    pub cwd: String,
    /// Such as "default" or "plan"; empty when not sent.
    pub mode: String,
    /// The session's transcript file, which names the model when the hooks do not.
    pub transcript: String,
}

/// One Claude Code hook call, by its event.
#[derive(Clone, Debug, PartialEq)]
pub enum Hook {
    Start { from: Origin, model: String },
    End { from: Origin },
    Prompt { from: Origin, prompt: String },
    /// Before a tool runs, or after when `done`, with what it returned.
    Tool {
        from: Origin,
        name: String,
        input: Value,
        done: bool,
        response: Value,
    },
    /// What Claude Code would tell you in a desktop notification, such as that it waits for you.
    Notice {
        from: Origin,
        kind: String,
        message: String,
    },
    Stop { from: Origin, reply: String },
    /// Blocks until answered.
    Permission {
        from: Origin,
        tool: String,
        input: Value,
        suggestion: Option<Value>,
    },
    /// Blocks until answered.
    Questions { from: Origin, input: Value },
    /// The plan's limits, as Claude Code passes them to its status line.
    Usage { from: Origin, readings: Vec<Reading> },
}

/// One limit as the status line reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    /// Named as the usage API names it, such as "five_hour".
    pub id: String,
    pub used: f64,
    /// Milliseconds since the epoch.
    pub resets_at: Option<i64>,
}

/// Each window of `rate_limits` that says how much is used.
fn readings(value: &Value) -> Vec<Reading> {
    ["five_hour", "seven_day"]
        .into_iter()
        .filter_map(|id| {
            let window = value.pointer(&format!("/rate_limits/{id}"))?;
            let used = window
                .get("used_percentage")?
                .as_f64()
                .filter(|used| used.is_finite())?;
            Some(Reading {
                id: id.to_owned(),
                used,
                resets_at: window
                    .get("resets_at")
                    .and_then(Value::as_i64)
                    .filter(|seconds| *seconds > 0)
                    .map(|seconds| seconds * 1000),
            })
        })
        .collect()
}

impl Hook {
    pub fn from(&self) -> &Origin {
        match self {
            Hook::Start { from, .. }
            | Hook::End { from }
            | Hook::Prompt { from, .. }
            | Hook::Tool { from, .. }
            | Hook::Notice { from, .. }
            | Hook::Stop { from, .. }
            | Hook::Permission { from, .. }
            | Hook::Questions { from, .. }
            | Hook::Usage { from, .. } => from,
        }
    }

    pub fn blocks(&self) -> bool {
        matches!(self, Hook::Permission { .. } | Hook::Questions { .. })
    }
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// The hook events [`parse`] reads.
pub const EVENTS: &[&str] = &[
    "SessionStart",
    "SessionEnd",
    "UserPromptSubmit",
    "PreToolUse",
    "ToolUse",
    "PostToolUse",
    "Notification",
    "Stop",
    "PermissionRequest",
    "StatusLine",
];

/// `event` is the hook event's name as Claude Code gives it, such as "PreToolUse"; `json` is what
/// it wrote to the hook's input.
pub fn parse(event: &str, json: &str) -> Result<Hook, String> {
    let value: Value =
        serde_json::from_str(json).map_err(|error| format!("hook input is not JSON: {error}"))?;
    let from = Origin {
        session: text(&value, "session_id"),
        cwd: text(&value, "cwd"),
        mode: text(&value, "permission_mode"),
        transcript: text(&value, "transcript_path"),
    };
    if from.session.is_empty() {
        return Err("hook input has no session_id".to_owned());
    }
    let tool = text(&value, "tool_name");
    let input = value.get("tool_input").cloned().unwrap_or(Value::Null);
    Ok(match event {
        "SessionStart" => Hook::Start {
            from,
            model: text(&value, "model"),
        },
        "SessionEnd" => Hook::End { from },
        "UserPromptSubmit" => Hook::Prompt {
            from,
            prompt: text(&value, "prompt"),
        },
        "PreToolUse" if tool == "AskUserQuestion" => Hook::Questions { from, input },
        // The quiet hook on every tool, which leaves questions to the one that waits for them, and
        // the terminal's own question dialog, which is a permission prompt to Claude Code and comes
        // only once you chose to answer there.
        "ToolUse" | "PermissionRequest" if tool == "AskUserQuestion" => Hook::Notice {
            from,
            kind: String::new(),
            message: String::new(),
        },
        "PreToolUse" | "ToolUse" | "PostToolUse" => Hook::Tool {
            from,
            name: tool,
            input,
            done: event == "PostToolUse",
            response: value.get("tool_response").cloned().unwrap_or(Value::Null),
        },
        "Notification" => Hook::Notice {
            from,
            kind: text(&value, "notification_type"),
            message: text(&value, "message"),
        },
        "Stop" => Hook::Stop {
            from,
            reply: text(&value, "last_assistant_message"),
        },
        "PermissionRequest" => Hook::Permission {
            from,
            tool,
            input,
            suggestion: value
                .get("permission_suggestions")
                .and_then(Value::as_array)
                .and_then(|suggestions| suggestions.first())
                .cloned(),
        },
        "StatusLine" => Hook::Usage {
            readings: readings(&value),
            from,
        },
        other => return Err(format!("unknown hook event {other}")),
    })
}

#[cfg(test)]
#[path = "input_test.rs"]
mod tests;
