use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use futures::channel::oneshot;

use super::{
    hook::{self, Hook},
    session::{Answer, Ask, Request},
    state::State,
};

/// The least time a request gets once the pointer leaves, so one held near its end does not go
/// the moment the pointer does.
const RESUME_AT_LEAST_MS: i64 = 3000;
/// The longest a request waits, held or not. Claude Code gives the hook a little longer (set in
/// `hook::plugin`), so an answer never goes to a hook it already ended.
pub(super) const MOST_MS: i64 = 60 * 60 * 1000;

/// Requests waiting on you, each with the hook call that waits for its answer.
#[derive(Default)]
pub(super) struct Waiting {
    next_id: u64,
    replies: HashMap<u64, oneshot::Sender<String>>,
    /// The islands the pointer is over, by their view's number.
    holders: HashSet<u64>,
}

impl Waiting {
    /// One a pop-up cannot show goes straight back to the terminal.
    pub(super) fn ask(
        &mut self,
        state: &mut State,
        hook: &Hook,
        reply: oneshot::Sender<String>,
        now: i64,
        wait_ms: i64,
    ) {
        hook::apply(Arc::make_mut(&mut state.sessions), hook, now);
        self.next_id += 1;
        let expires_at = now.saturating_add(wait_ms.min(MOST_MS));
        match hook::request(hook, self.next_id, now, expires_at) {
            Some(request) => {
                self.replies.insert(request.id, reply);
                Arc::make_mut(&mut state.requests).push(request);
            }
            None => {
                let _ = reply.send(String::new());
            }
        }
    }

    /// False when it was already answered, here or by running out of time.
    pub(super) fn answer(&mut self, state: &mut State, id: u64, answer: &Answer) -> bool {
        let Some(reply) = self.replies.remove(&id) else {
            return false;
        };
        let requests = Arc::make_mut(&mut state.requests);
        let Some(at) = requests.iter().position(|request| request.id == id) else {
            return false;
        };
        let request = requests.remove(at);
        let _ = reply.send(hook::answer(&request, answer));
        if *answer != Answer::Terminal {
            let sessions: &mut Vec<_> = Arc::make_mut(&mut state.sessions);
            hook::resumed(sessions, &request.session);
        }
        true
    }

    /// The pointer came over `island` or left it. Nothing runs out while it is over any island;
    /// once it is over none, each request gets back the time it was held. True when what waits
    /// on you shows that.
    pub(super) fn hold(&mut self, state: &mut State, island: u64, over: bool, now: i64) -> bool {
        let was = !self.holders.is_empty();
        if over {
            self.holders.insert(island);
        } else {
            self.holders.remove(&island);
        }
        let held = !self.holders.is_empty();
        if held == was {
            return false;
        }
        if held {
            state.hold.since = Some(now);
        } else if let Some(since) = state.hold.since.take() {
            for request in Arc::make_mut(&mut state.requests) {
                let held_for = now - since.max(request.received_at);
                request.expires_at = (request.expires_at + held_for)
                    .max(now + RESUME_AT_LEAST_MS)
                    .min(last(request));
            }
            state.hold.released_at = now;
        }
        state.hold.round += 1;
        !state.requests.is_empty()
    }

    /// Hands every request past its time back to the terminal; while held, only one past the
    /// longest it may wait. True when any was.
    pub(super) fn expire(&mut self, state: &mut State, now: i64) -> bool {
        let due: Vec<u64> = state
            .requests
            .iter()
            .filter(|request| self.due_at(request) <= now)
            .map(|request| request.id)
            .collect();
        let mut expired = false;
        for id in due {
            expired |= self.answer(state, id, &Answer::Terminal);
        }
        expired
    }

    pub(super) fn next_due(&self, state: &State) -> Option<i64> {
        state
            .requests
            .iter()
            .map(|request| self.due_at(request))
            .min()
    }

    fn due_at(&self, request: &Request) -> i64 {
        if self.holders.is_empty() {
            request.expires_at
        } else {
            last(request)
        }
    }

    pub(super) fn ids(state: &State) -> Vec<u64> {
        state.requests.iter().map(|request| request.id).collect()
    }
}

/// The latest `request` goes, however long it is held.
fn last(request: &Request) -> i64 {
    request.received_at.saturating_add(MOST_MS)
}

/// Drops what a session no longer waits on, as a hook it told at `told_at` shows: the call it
/// asked about has run, or its turn or session is over. Tells run on their own and can arrive
/// late, so only what was asked before then goes, and one finished call settles only the oldest
/// request for it. True when any went.
pub(super) fn settle(state: &mut State, waiting: &mut Waiting, hook: &Hook, told_at: i64) -> bool {
    let session = &hook.from().session;
    let mut before = state
        .requests
        .iter()
        .filter(|request| request.session == *session && request.received_at < told_at);
    let ids: Vec<u64> = match hook {
        Hook::End { .. } | Hook::Stop { .. } | Hook::Prompt { .. } => {
            before.map(|request| request.id).collect()
        }
        Hook::Tool {
            name,
            input,
            done: true,
            ..
        } => before
            .find(|request| {
                matches!(
                    &request.ask,
                    Ask::Permission { tool, input: asked, .. } if tool == name && asked == input
                )
            })
            .map(|request| request.id)
            .into_iter()
            .collect(),
        _ => Vec::new(),
    };
    let mut changed = false;
    for id in ids {
        changed |= waiting.answer(state, id, &Answer::Terminal);
    }
    changed
}

#[cfg(test)]
#[path = "requests_test.rs"]
mod tests;
