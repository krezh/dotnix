//! What Claude Code would tell you in a desktop notification, sent by island instead of the
//! terminal, so Jelly leads it.

use super::{hook::Hook, model::Notice, state::State};
use crate::markdown;

/// The most of a reply a "Finished" notification shows.
const REPLY_CHARS: usize = 160;

fn project(state: &State, session: &str, cwd: &str) -> String {
    state
        .sessions
        .iter()
        .find(|known| known.id == session)
        .map_or_else(
            || {
                cwd.trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or_default()
                    .to_owned()
            },
            |known| known.project.clone(),
        )
}

/// The first line of a reply, as shown, cut short when long.
fn gist(reply: &str) -> String {
    let plain = markdown::plain(reply);
    let line = plain
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default()
        .trim();
    if line.chars().count() > REPLY_CHARS {
        let cut: String = line.chars().take(REPLY_CHARS).collect();
        format!("{}…", cut.trim_end())
    } else {
        line.to_owned()
    }
}

/// Read from `state` as it was before `hook`. A request island already shows a pop-up for needs
/// no notification as well, and a session waiting for you was already told of as it finished.
pub(super) fn notice(state: &State, hook: &Hook) -> Option<Notice> {
    match hook {
        Hook::Notice {
            from,
            kind,
            message,
        } if !kind.is_empty() && kind != "idle_prompt" => {
            if state
                .requests
                .iter()
                .any(|request| request.session == from.session)
            {
                return None;
            }
            Some(Notice {
                key: format!("needs {}", from.session),
                session: Some(from.session.clone()),
                app: "Claude Code",
                title: if message.is_empty() {
                    "Claude Code needs you".to_owned()
                } else {
                    message.clone()
                },
                body: project(state, &from.session, &from.cwd),
                critical: false,
            })
        }
        Hook::Stop { from, reply } => Some(Notice {
            key: format!("finished {}", from.session),
            session: Some(from.session.clone()),
            app: "Claude Code",
            title: format!("Finished in {}", project(state, &from.session, &from.cwd)),
            body: gist(reply),
            critical: false,
        }),
        _ => None,
    }
}

/// How long one of Claude Code's own notifications waits, since the request it is about can
/// arrive just after it.
const SETTLE_MS: i64 = 1500;

/// Claude Code's own notifications, held back a moment and dropped if a request from their
/// session arrives meanwhile.
#[derive(Default)]
pub(super) struct Alerts {
    later: Vec<(i64, String, Notice)>,
}

impl Alerts {
    /// What to send now for `hook`, told at `now`.
    pub(super) fn tell(&mut self, state: &State, hook: &Hook, now: i64) -> Option<Notice> {
        let notice = notice(state, hook)?;
        if let Hook::Notice { from, .. } = hook {
            self.later
                .push((now + SETTLE_MS, from.session.clone(), notice));
            return None;
        }
        Some(notice)
    }

    /// `session` asked something of you, which a pop-up shows.
    pub(super) fn asked(&mut self, session: &str) {
        self.later.retain(|(_, from, _)| from != session);
    }

    /// The held notifications due by `now`, unless their session waits on a pop-up.
    pub(super) fn due(&mut self, state: &State, now: i64) -> Vec<Notice> {
        let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.later)
            .into_iter()
            .partition(|(at, _, _)| *at <= now);
        self.later = later;
        due.into_iter()
            .filter(|(_, session, _)| {
                !state
                    .requests
                    .iter()
                    .any(|request| request.session == *session)
            })
            .map(|(_, _, notice)| notice)
            .collect()
    }

    pub(super) fn next_due(&self) -> Option<i64> {
        self.later.iter().map(|(at, _, _)| *at).min()
    }
}

#[cfg(test)]
#[path = "alerts_test.rs"]
mod tests;
