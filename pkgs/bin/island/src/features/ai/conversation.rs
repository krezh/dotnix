use std::path::PathBuf;

use super::converse::{self, Running, Streamed};
use crate::db;

/// Name and the alias Claude Code takes.
pub const MODELS: [(&str, &str); 3] = [("Haiku", "haiku"), ("Sonnet", "sonnet"), ("Opus", "opus")];

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub(super) mine: bool,
    pub(super) text: String,
    /// Into [`MODELS`]: the one asked, or the one answering.
    pub(super) model: usize,
    pub(super) at: i64,
    /// How long a reply took, once it is done.
    pub(super) took_ms: Option<i64>,
}

/// The chat as the panel shows it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Chat {
    pub(super) lines: Vec<Line>,
    /// While a reply is on its way.
    pub(super) busy: bool,
    pub(super) model: usize,
    pub(super) error: Option<String>,
}

/// The worker's side of the chat: the conversation to follow up, and the reply on its way.
#[derive(Default)]
pub struct Conversation {
    session: Option<String>,
    /// Counts each reply asked for, so what a stopped one still sends is ignored.
    run: u64,
    running: Option<Running>,
}

fn folder() -> PathBuf {
    db::runtime_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("chat")
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn model_index(alias: &str) -> usize {
    MODELS
        .iter()
        .position(|(_, known)| *known == alias)
        .unwrap_or(1)
}

impl Conversation {
    /// `each` gets what streams back, with the run it belongs to.
    pub fn send(
        &mut self,
        chat: &mut Chat,
        program: &str,
        text: String,
        each: impl Fn(u64, Streamed) + Send + 'static,
    ) {
        if chat.busy || text.trim().is_empty() {
            return;
        }
        self.run += 1;
        let run = self.run;
        let model = chat.model.min(MODELS.len() - 1);
        chat.error = None;
        chat.lines.push(Line {
            mine: true,
            text: text.clone(),
            model,
            at: now(),
            took_ms: None,
        });
        match converse::start(
            program,
            &text,
            MODELS[model].1,
            self.session.as_deref(),
            &folder(),
            move |streamed| each(run, streamed),
        ) {
            Ok(running) => {
                self.running = Some(running);
                chat.busy = true;
            }
            Err(error) => chat.error = Some(error),
        }
    }

    /// Asks the last question again, after its reply failed.
    pub fn retry(
        &mut self,
        chat: &mut Chat,
        program: &str,
        each: impl Fn(u64, Streamed) + Send + 'static,
    ) {
        if chat.busy || chat.error.is_none() {
            return;
        }
        if chat.lines.last().is_some_and(|line| !line.mine) {
            chat.lines.pop();
        }
        let Some(asked) = chat.lines.pop().filter(|line| line.mine) else {
            return;
        };
        self.send(chat, program, asked.text, each);
    }

    /// False for what a stopped reply still sends.
    pub fn streamed(&mut self, chat: &mut Chat, run: u64, streamed: Streamed) -> bool {
        if run != self.run {
            return false;
        }
        let replying = chat.lines.last().is_some_and(|line| !line.mine);
        let reply = |text: String, chat: &Chat| Line {
            mine: false,
            text,
            model: chat.lines.last().map_or(chat.model, |line| line.model),
            at: now(),
            took_ms: None,
        };
        match streamed {
            Streamed::Session(id) => self.session = Some(id),
            Streamed::Text(more) => match chat.lines.last_mut() {
                Some(line) if replying => line.text.push_str(&more),
                _ => {
                    let line = reply(more, chat);
                    chat.lines.push(line);
                }
            },
            // Only when nothing arrived piece by piece.
            Streamed::Whole(whole) => {
                if !replying {
                    let line = reply(whole, chat);
                    chat.lines.push(line);
                }
            }
            Streamed::Done(result) => {
                chat.busy = false;
                self.running = None;
                let asked_at = chat
                    .lines
                    .iter()
                    .rev()
                    .find(|line| line.mine)
                    .map(|line| line.at);
                if let (Some(line), Some(asked_at)) = (chat.lines.last_mut(), asked_at)
                    && !line.mine
                {
                    line.took_ms = Some(now() - asked_at);
                }
                if let Err(error) = result {
                    chat.error = Some(error);
                }
            }
        }
        true
    }

    /// Stops any reply and forgets the conversation, here and in Claude Code's own files.
    pub fn clear(&mut self, chat: &mut Chat) {
        self.run += 1;
        if let Some(running) = self.running.take() {
            running.stop();
        }
        if let Some(session) = self.session.take() {
            converse::forget(&folder(), &session);
        }
        let model = chat.model;
        *chat = Chat {
            model,
            ..Chat::default()
        };
    }
}
