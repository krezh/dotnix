use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

/// How much of the end of a transcript to look through for the model.
const TAIL: u64 = 256 * 1024;

/// The model named last in `text`, as Claude Code writes it on each reply.
pub fn last_model(text: &str) -> Option<String> {
    const KEY: &str = "\"model\":\"";
    let mut rest = text;
    let mut found = None;
    while let Some(at) = rest.find(KEY) {
        let after = &rest[at + KEY.len()..];
        if let Some(end) = after.find('"') {
            let model = &after[..end];
            if model.starts_with("claude-") {
                found = Some(model.to_owned());
            }
        }
        rest = after;
    }
    found
}

/// The model a session last answered with, read from the end of its transcript file.
pub fn model(path: &str) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let length = file.metadata().ok()?.len();
    file.seek(SeekFrom::Start(length.saturating_sub(TAIL)))
        .ok()?;
    let mut tail = Vec::new();
    file.read_to_end(&mut tail).ok()?;
    last_model(&String::from_utf8_lossy(&tail))
}

#[cfg(test)]
#[path = "transcript_test.rs"]
mod tests;
