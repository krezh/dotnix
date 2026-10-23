//! The Claude Code plugin that hands island the hook events it follows, as
//! `island ai-hook plugin <dir>` writes it when island is built.

use std::{fs, io, path::Path};

use serde_json::{Map, Value, json};

use crate::features::ai::requests::MOST_MS;

/// On `event`, for the tools `matcher` names, Claude Code runs `island ai-hook <run>`, and waits
/// for its answer when `waits`.
struct Entry {
    event: &'static str,
    matcher: &'static str,
    run: &'static str,
    waits: bool,
}

const fn tell(event: &'static str) -> Entry {
    Entry {
        event,
        matcher: "*",
        run: event,
        waits: false,
    }
}

const HOOKS: &[Entry] = &[
    tell("SessionStart"),
    tell("SessionEnd"),
    tell("UserPromptSubmit"),
    tell("Stop"),
    tell("Notification"),
    tell("PostToolUse"),
    // Every tool quietly, and questions again on a hook that waits for their answers.
    Entry {
        run: "ToolUse",
        ..tell("PreToolUse")
    },
    Entry {
        event: "PreToolUse",
        matcher: "AskUserQuestion",
        run: "PreToolUse",
        waits: true,
    },
    Entry {
        waits: true,
        ..tell("PermissionRequest")
    },
];

/// Seconds Claude Code gives a hook that only tells island something.
const TELL_SECONDS: i64 = 10;
/// Seconds it gives one that waits: a little past the longest a request waits, so an answer never
/// goes to a hook Claude Code already ended.
const WAIT_SECONDS: i64 = MOST_MS / 1000 + 15;

/// `word` for a shell, so a path with spaces stays one argument.
fn quoted(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
}

/// `hooks/hooks.json`, with each hook running `island`.
fn hooks(island: &str) -> Value {
    let mut events = Map::new();
    for entry in HOOKS {
        let mut hook = json!({
            "type": "command",
            "command": format!("{} ai-hook {}", quoted(island), entry.run),
            "timeout": if entry.waits { WAIT_SECONDS } else { TELL_SECONDS },
        });
        if !entry.waits {
            hook["async"] = Value::Bool(true);
        }
        let groups = events
            .entry(entry.event)
            .or_insert_with(|| Value::Array(Vec::new()));
        if let Value::Array(groups) = groups {
            groups.push(json!({ "matcher": entry.matcher, "hooks": [hook] }));
        }
    }
    json!({
        "description": "Follow Claude Code sessions in island and answer their requests there",
        "hooks": events,
    })
}

/// Writes the plugin into `dir`, its hooks running `island`.
pub fn write(dir: &Path, island: &Path) -> io::Result<()> {
    let manifest = json!({
        "name": "island",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Follow Claude Code sessions in island and answer their requests there",
    });
    let hooks = hooks(&island.to_string_lossy());
    for (path, value) in [
        (dir.join(".claude-plugin/plugin.json"), manifest),
        (dir.join("hooks/hooks.json"), hooks),
    ] {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(&value)? + "\n")?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "plugin_test.rs"]
mod tests;
