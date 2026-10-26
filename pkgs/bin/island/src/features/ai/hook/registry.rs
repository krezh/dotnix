use serde_json::Value;

use super::input::{Hook, Origin};
use super::live::short_path;
use crate::features::ai::session::{Activity, Exchange, Live, Session, Step, StepState};

/// Steps kept across a session's history.
const MAX_STEPS: usize = 80;
const MAX_TARGET: usize = 60;

fn last_part(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_owned()
}

/// The repository the session works in, named by its root (the folder holding `.jj` or `.git`),
/// rather than whichever folder inside it the session started in.
fn project(cwd: &str) -> String {
    let start = std::path::Path::new(cwd);
    start
        .ancestors()
        .find(|folder| folder.join(".jj").exists() || folder.join(".git").exists())
        .map_or_else(|| last_part(cwd), |root| last_part(&root.to_string_lossy()))
}

fn clip(text: &str) -> String {
    let line = text.lines().next().unwrap_or_default().trim();
    match line.char_indices().nth(MAX_TARGET) {
        Some((end, _)) => format!("{}…", &line[..end]),
        None => line.to_owned(),
    }
}

fn field<'a>(input: &'a Value, key: &str) -> &'a str {
    input.get(key).and_then(Value::as_str).unwrap_or_default()
}

/// What a tool call does, as a verb and what it does it to.
pub fn describe(tool: &str, input: &Value) -> (String, String) {
    let (verb, target) = match tool {
        "Read" => ("Read", short_path(field(input, "file_path"))),
        "Edit" | "MultiEdit" => ("Edit", short_path(field(input, "file_path"))),
        "NotebookEdit" => ("Edit", short_path(field(input, "notebook_path"))),
        "Write" => ("Write", short_path(field(input, "file_path"))),
        // Claude says what each command is for, which reads better than the command itself.
        "Bash" if !field(input, "description").is_empty() => {
            ("", clip(field(input, "description")))
        }
        "Bash" => ("Run", clip(field(input, "command"))),
        "Grep" => ("Search", format!("\"{}\"", clip(field(input, "pattern")))),
        "Glob" => ("Find", clip(field(input, "pattern"))),
        "WebFetch" => ("Fetch", clip(field(input, "url"))),
        "WebSearch" => ("Search the web", clip(field(input, "query"))),
        "Task" | "Agent" => ("Agent", clip(field(input, "description"))),
        _ => match tool
            .strip_prefix("mcp__")
            .and_then(|rest| rest.split_once("__"))
        {
            Some((server, name)) => return (name.replace('_', " "), server.to_owned()),
            None => return (tool.to_owned(), String::new()),
        },
    };
    (verb.to_owned(), target)
}

/// A step as one line, such as "Edit ai/overview.rs"; one described in words is just that.
pub fn label(verb: &str, target: &str) -> String {
    match (verb.is_empty(), target.is_empty()) {
        (true, _) => target.to_owned(),
        (false, true) => verb.to_owned(),
        (false, false) => format!("{verb} {target}"),
    }
}

fn doing(verb: &str, target: &str) -> String {
    let gerund = match verb {
        "" => return target.to_owned(),
        "Read" => "Reading",
        "Edit" => "Editing",
        "Write" => "Writing",
        "Run" => "Running",
        "Search" => "Searching for",
        "Find" => "Finding",
        "Fetch" => "Fetching",
        "Search the web" => "Searching the web for",
        "Agent" => "Running an agent:",
        other => return format!("Using {other}"),
    };
    if target.is_empty() {
        gerund.to_owned()
    } else {
        format!("{gerund} {target}")
    }
}

/// The exchange going on now; one is started for a session seen working before its prompt.
fn current(found: &mut Session) -> &mut Exchange {
    if found.history.is_empty() {
        found.history.push(Exchange::default());
    }
    let last = found.history.len() - 1;
    &mut found.history[last]
}

/// Lets go of the oldest exchanges once there are more steps than are kept; the latest stays.
fn trim(found: &mut Session) {
    while found.history.len() > 1
        && found
            .history
            .iter()
            .map(|exchange| exchange.steps.len())
            .sum::<usize>()
            > MAX_STEPS
    {
        found.history.remove(0);
    }
}

fn session<'a>(sessions: &'a mut Vec<Session>, origin: &Origin, now: i64) -> &'a mut Session {
    let index = match sessions.iter().position(|known| known.id == origin.session) {
        Some(index) => index,
        None => {
            sessions.push(Session {
                id: origin.session.clone(),
                project: project(&origin.cwd),
                cwd: origin.cwd.clone(),
                model: String::new(),
                mode: String::new(),
                activity: Activity::Idle,
                doing: "Waiting for your prompt".to_owned(),
                history: Vec::new(),
                seq: 0,
                started_at: now,
                active_at: now,
            });
            sessions.len() - 1
        }
    };
    let found = &mut sessions[index];
    if !origin.cwd.is_empty() && origin.cwd != found.cwd {
        found.cwd.clone_from(&origin.cwd);
        found.project = project(&origin.cwd);
    }
    if !origin.mode.is_empty() {
        found.mode.clone_from(&origin.mode);
    }
    found.active_at = now;
    found
}

fn push(found: &mut Session, verb: String, target: String, state: StepState, live: Option<Live>) {
    found.seq += 1;
    let seq = found.seq;
    found.doing = doing(&verb, &target);
    let steps = &mut current(found).steps;
    for step in steps.iter_mut() {
        if step.state == StepState::Now {
            step.state = StepState::Done;
        }
    }
    steps.push(Step {
        verb,
        target,
        state,
        live,
        seq,
    });
    trim(found);
}

/// Applies one hook to what is known of the sessions.
pub fn apply(sessions: &mut Vec<Session>, hook: &Hook, now: i64) {
    match hook {
        Hook::End { from } => {
            sessions.retain(|known| known.id != from.session);
            return;
        }
        Hook::Usage { .. } => return,
        _ => {}
    }
    let found = session(sessions, hook.from(), now);
    // The hooks name the model only on a fresh start; the transcript names it on every reply,
    // including after a switch of model.
    let transcript = &hook.from().transcript;
    if (found.model.is_empty() || matches!(hook, Hook::Stop { .. })) && !transcript.is_empty() {
        if let Some(model) = super::transcript::model(transcript) {
            found.model = model;
        }
    }
    match hook {
        Hook::Start { model, .. } => {
            if !model.is_empty() {
                found.model.clone_from(model);
            }
        }
        Hook::End { .. } | Hook::Usage { .. } => {}
        Hook::Prompt { prompt, .. } => {
            found.activity = Activity::Working;
            found.seq += 1;
            let seq = found.seq;
            found.history.push(Exchange {
                prompt: prompt.clone(),
                seq,
                ..Exchange::default()
            });
            trim(found);
            found.doing = "Thinking".to_owned();
        }
        Hook::Tool {
            name,
            input,
            done: false,
            ..
        } => {
            found.activity = Activity::Working;
            let (verb, target) = describe(name, input);
            push(
                found,
                verb,
                target,
                StepState::Now,
                super::live::before(name, input),
            );
        }
        Hook::Tool {
            name,
            input,
            done: true,
            response,
            ..
        } => {
            let (verb, target) = describe(name, input);
            let shown = super::live::after(name, input, response);
            found.seq += 1;
            let seq = found.seq;
            if let Some(step) = current(found)
                .steps
                .iter_mut()
                .rev()
                .find(|step| step.verb == verb && step.target == target)
            {
                step.state = StepState::Done;
                if let Some(shown) = shown {
                    step.live = Some(shown);
                    step.seq = seq;
                }
            }
        }
        Hook::Notice { kind, .. } => match kind.as_str() {
            "permission_prompt" | "agent_needs_input" | "elicitation_dialog" => {
                found.activity = Activity::Waiting
            }
            "idle_prompt" => {
                found.activity = Activity::Idle;
                found.doing = "Waiting for your reply".to_owned();
            }
            _ => {}
        },
        Hook::Stop { reply, .. } => {
            found.activity = Activity::Idle;
            found.doing = "Finished".to_owned();
            found.seq += 1;
            let seq = found.seq;
            let exchange = current(found);
            exchange.reply.clone_from(reply);
            exchange.reply_seq = seq;
            for step in &mut exchange.steps {
                step.state = StepState::Done;
            }
        }
        Hook::Permission { tool, input, .. } => {
            found.activity = Activity::Waiting;
            let (verb, target) = describe(tool, input);
            found.doing = if verb.is_empty() {
                format!("Wants to: {target}")
            } else {
                format!("Wants to {} {target}", verb.to_lowercase())
                    .trim_end()
                    .to_owned()
            };
            let wants = found.doing.clone();
            push(found, verb, target, StepState::Waiting, None);
            found.doing = wants;
        }
        Hook::Questions { input, .. } => {
            found.activity = Activity::Waiting;
            let count = input
                .get("questions")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            let what = if count == 1 {
                "1 question".to_owned()
            } else {
                format!("{count} questions")
            };
            push(
                found,
                "Ask".to_owned(),
                what.clone(),
                StepState::Waiting,
                None,
            );
            found.doing = format!("Asked {what}");
        }
    }
}

/// Once its request is answered, the session carries on.
pub fn resumed(sessions: &mut [Session], session: &str) {
    if let Some(found) = sessions.iter_mut().find(|known| known.id == session) {
        found.activity = Activity::Working;
        let mut now_doing = None;
        for step in &mut current(found).steps {
            if step.state == StepState::Waiting {
                step.state = StepState::Now;
                now_doing = Some(doing(&step.verb, &step.target));
            }
        }
        if let Some(now_doing) = now_doing {
            found.doing = now_doing;
        }
    }
}

#[cfg(test)]
#[path = "registry_test.rs"]
mod tests;
