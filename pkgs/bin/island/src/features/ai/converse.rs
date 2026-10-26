use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};

use serde_json::Value;

/// Says only what a quick question needs, so a chat stays a chat.
const SYSTEM_PROMPT: &str = "You are answering a quick question in a small chat window on the \
user's desktop. Answer directly and briefly in Markdown. Use a fenced code block for commands \
or code. You have no tools.";

/// Marks the chat's own Claude Code process, so island's hooks leave it out of the sessions.
pub const CHAT_ENV: &str = "ISLAND_CHAT";

/// What one line of `claude -p --output-format stream-json` says.
#[derive(Debug, PartialEq)]
pub enum Streamed {
    /// The conversation's id, for a follow-up to resume it.
    Session(String),
    /// More of the reply as it arrives.
    Text(String),
    /// The whole reply at once, when nothing arrived piece by piece.
    Whole(String),
    /// The turn is over; an error says why it failed.
    Done(Result<(), String>),
}

pub fn parse(line: &str) -> Option<Streamed> {
    let value: Value = serde_json::from_str(line).ok()?;
    let text = |value: &Value, key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    match value.get("type")?.as_str()? {
        "system" if text(&value, "subtype").as_deref() == Some("init") => {
            Some(Streamed::Session(text(&value, "session_id")?))
        }
        "stream_event" => {
            let delta = value.get("event")?.get("delta")?;
            (text(delta, "type").as_deref() == Some("text_delta"))
                .then(|| text(delta, "text"))
                .flatten()
                .map(Streamed::Text)
        }
        "assistant" => {
            let content = value.get("message")?.get("content")?.as_array()?;
            let whole: String = content
                .iter()
                .filter(|part| text(part, "type").as_deref() == Some("text"))
                .filter_map(|part| text(part, "text"))
                .collect();
            (!whole.is_empty()).then_some(Streamed::Whole(whole))
        }
        "result" => {
            let failed = value
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            Some(Streamed::Done(if failed {
                Err(text(&value, "result").unwrap_or_else(|| "Claude could not answer".to_owned()))
            } else {
                Ok(())
            }))
        }
        _ => None,
    }
}

/// A running reply, which can be stopped.
#[derive(Clone)]
pub struct Running(Arc<Mutex<Child>>);

impl Running {
    pub fn stop(&self) {
        if let Ok(mut child) = self.0.lock() {
            let _ = child.kill();
        }
    }
}

/// Starts `program -p` (Claude Code) on `prompt` in `dir`, following up `resume` if given, and
/// passes each line it streams to `each` on a thread of its own, ending with [`Streamed::Done`].
pub fn start(
    program: &str,
    prompt: &str,
    model: &str,
    resume: Option<&str>,
    dir: &Path,
    each: impl Fn(Streamed) + Send + 'static,
) -> Result<Running, String> {
    std::fs::create_dir_all(dir)
        .map_err(|error| format!("cannot make the chat folder: {error}"))?;
    let mut command = Command::new(program);
    command
        .current_dir(dir)
        .env(CHAT_ENV, "1")
        .args(["-p", prompt])
        .args([
            "--output-format",
            "stream-json",
            "--include-partial-messages",
            "--verbose",
        ])
        .args(["--model", model])
        .args([
            "--tools",
            "",
            "--strict-mcp-config",
            "--setting-sources",
            "",
        ])
        .args(["--system-prompt", SYSTEM_PROMPT])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(session) = resume {
        command.args(["--resume", session]);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot start claude: {error}"))?;
    let stdout = child.stdout.take().ok_or("claude gave no output")?;
    let running = Running(Arc::new(Mutex::new(child)));
    let waited = running.clone();
    thread::Builder::new()
        .name("ai-chat".to_owned())
        .spawn(move || {
            let mut finished = false;
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(streamed) = parse(&line) {
                    finished |= matches!(streamed, Streamed::Done(_));
                    each(streamed);
                }
            }
            let status = waited.0.lock().ok().and_then(|mut child| child.wait().ok());
            if !finished {
                let stopped = status.is_none_or(|status| !status.success());
                each(Streamed::Done(if stopped {
                    Err("Claude stopped before answering".to_owned())
                } else {
                    Ok(())
                }));
            }
        })
        .map_err(|error| format!("cannot follow claude: {error}"))?;
    Ok(running)
}

fn config_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = std::env::var_os("CLAUDE_CONFIG_DIR") {
        dirs.push(PathBuf::from(dir));
    }
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        dirs.push(PathBuf::from(dir).join("claude"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(&home).join(".config/claude"));
        dirs.push(PathBuf::from(home).join(".claude"));
    }
    dirs
}

/// Claude Code keeps a project's sessions in a folder named after its path.
pub fn project_folder(dir: &Path) -> String {
    dir.to_string_lossy().replace(['/', '.'], "-")
}

/// Removes what Claude Code saved of the conversation, so a chat leaves nothing behind.
pub fn forget(dir: &Path, session: &str) {
    let folder = project_folder(dir);
    for config in config_dirs() {
        let project = config.join("projects").join(&folder);
        let _ = std::fs::remove_file(project.join(format!("{session}.jsonl")));
        let _ = std::fs::remove_dir_all(project.join(session));
        let _ = std::fs::remove_dir(&project);
    }
}

#[cfg(test)]
#[path = "converse_test.rs"]
mod tests;
