use serde_json::Value;

use crate::features::ai::session::{LineKind, Live, LiveLine};

/// The most lines a live step keeps, enough to follow without crowding the view.
const MAX_LINES: usize = 14;
/// The end of a command's output that is kept.
const MAX_OUTPUT: usize = 8;

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

/// The file's name with its folder, such as "ai/overview.rs".
pub fn short_path(path: &str) -> String {
    let mut parts = path.trim_end_matches('/').rsplit('/');
    match (parts.next(), parts.next()) {
        (Some(name), Some(folder)) if !folder.is_empty() => format!("{folder}/{name}"),
        (Some(name), _) => name.to_owned(),
        _ => path.to_owned(),
    }
}

fn plain(lines: impl IntoIterator<Item = String>, kind: LineKind) -> Vec<LiveLine> {
    lines
        .into_iter()
        .map(|text| LiveLine {
            number: None,
            text,
            kind,
        })
        .collect()
}

/// What a tool shows while it runs, from its input alone; `None` for one with nothing to follow.
pub fn before(tool: &str, input: &Value) -> Option<Live> {
    match tool {
        "Edit" | "MultiEdit" => {
            let edit = if tool == "MultiEdit" {
                input.get("edits")?.as_array()?.first()?
            } else {
                input
            };
            let mut lines = plain(
                text(edit, "old_string").lines().map(str::to_owned),
                LineKind::Removed,
            );
            lines.extend(plain(
                text(edit, "new_string").lines().map(str::to_owned),
                LineKind::Added,
            ));
            lines.truncate(MAX_LINES);
            Some(Live::Editing {
                file: short_path(text(input, "file_path")),
                lines,
            })
        }
        "Write" => Some(Live::Editing {
            file: short_path(text(input, "file_path")),
            lines: plain(
                text(input, "content")
                    .lines()
                    .take(MAX_LINES)
                    .map(str::to_owned),
                LineKind::Added,
            ),
        }),
        "Bash" => Some(Live::Running {
            command: text(input, "command").to_owned(),
            output: Vec::new(),
            done: false,
        }),
        // Its lines come once the read is done.
        "Read" => Some(Live::Reading {
            file: short_path(text(input, "file_path")),
            lines: Vec::new(),
        }),
        _ => None,
    }
}

/// The lines of a file a read returned, numbered from where it started.
fn read(response: &Value) -> Option<Live> {
    let file = response.get("file")?;
    let content = file.get("content")?.as_str()?;
    let start = file
        .get("startLine")
        .and_then(Value::as_u64)
        .and_then(|start| u32::try_from(start).ok())
        .unwrap_or(1);
    let lines = content
        .lines()
        .take(MAX_LINES)
        .zip(start..)
        .map(|(text, number)| LiveLine {
            number: Some(number),
            text: text.to_owned(),
            kind: LineKind::Same,
        })
        .collect();
    Some(Live::Reading {
        file: short_path(text(file, "filePath")),
        lines,
    })
}

/// The first part of an edit's patch, numbered as the file reads after it.
fn patch(input: &Value, response: &Value) -> Option<Live> {
    let hunk = response.get("structuredPatch")?.as_array()?.first()?;
    let mut old = hunk
        .get("oldStart")?
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())?;
    let mut new = hunk
        .get("newStart")?
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())?;
    let mut lines = Vec::new();
    for line in hunk
        .get("lines")?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
    {
        let (kind, rest) = match line.chars().next() {
            Some('+') => (LineKind::Added, &line[1..]),
            Some('-') => (LineKind::Removed, &line[1..]),
            Some(' ') => (LineKind::Same, &line[1..]),
            _ => (LineKind::Same, line),
        };
        let number = match kind {
            LineKind::Removed => {
                old += 1;
                old - 1
            }
            LineKind::Added => {
                new += 1;
                new - 1
            }
            LineKind::Same => {
                old += 1;
                new += 1;
                new - 1
            }
        };
        lines.push(LiveLine {
            number: Some(number),
            text: rest.to_owned(),
            kind,
        });
    }
    lines.truncate(MAX_LINES);
    let path = text(response, "filePath");
    let path = if path.is_empty() {
        text(input, "file_path")
    } else {
        path
    };
    Some(Live::Editing {
        file: short_path(path),
        lines,
    })
}

fn ran(input: &Value, response: &Value) -> Live {
    let mut output: Vec<String> = text(response, "stdout")
        .lines()
        .chain(text(response, "stderr").lines())
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect();
    let skip = output.len().saturating_sub(MAX_OUTPUT);
    output.drain(..skip);
    Live::Running {
        command: text(input, "command").to_owned(),
        output,
        done: true,
    }
}

/// What a tool shows once it ran, from what it returned.
pub fn after(tool: &str, input: &Value, response: &Value) -> Option<Live> {
    match tool {
        "Read" => read(response),
        "Edit" | "MultiEdit" => patch(input, response).or_else(|| before(tool, input)),
        "Bash" => Some(ran(input, response)),
        _ => None,
    }
}

#[cfg(test)]
#[path = "live_test.rs"]
mod tests;
