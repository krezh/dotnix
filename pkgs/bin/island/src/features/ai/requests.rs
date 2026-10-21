use std::{collections::HashMap, sync::Arc};

use futures::channel::oneshot;

use super::{
    hook::{self, Hook},
    session::Answer,
    state::State,
};

/// Requests waiting on you, each with the hook call that waits for its answer.
#[derive(Default)]
pub(super) struct Waiting {
    next_id: u64,
    replies: HashMap<u64, oneshot::Sender<String>>,
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
        match hook::request(hook, self.next_id, now, now.saturating_add(wait_ms)) {
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

    /// Hands every request past its time back to the terminal; true when any was.
    pub(super) fn expire(&mut self, state: &mut State, now: i64) -> bool {
        let due: Vec<u64> = state
            .requests
            .iter()
            .filter(|request| request.expires_at <= now)
            .map(|request| request.id)
            .collect();
        let mut expired = false;
        for id in due {
            expired |= self.answer(state, id, &Answer::Terminal);
        }
        expired
    }

    pub(super) fn next_due(state: &State) -> Option<i64> {
        state.requests.iter().map(|request| request.expires_at).min()
    }

    pub(super) fn ids(state: &State) -> Vec<u64> {
        state.requests.iter().map(|request| request.id).collect()
    }
}

/// When a session ends, whatever it still asks goes away with it.
pub(super) fn forget_session(state: &mut State, waiting: &mut Waiting, session: &str) -> bool {
    let ids: Vec<u64> = state
        .requests
        .iter()
        .filter(|request| request.session == session)
        .map(|request| request.id)
        .collect();
    let mut changed = false;
    for id in ids {
        changed |= waiting.answer(state, id, &Answer::Terminal);
    }
    changed
}
