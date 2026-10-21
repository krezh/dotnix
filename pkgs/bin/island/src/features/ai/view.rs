use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

use super::{
    session::Request,
    state::{Provider, Source, State},
};
use crate::{components::reveal, features::Local};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Account {
    source: Source,
    id: String,
}

impl Account {
    pub(super) fn of(provider: &Provider) -> Self {
        Self {
            source: provider.source,
            id: provider.id.clone(),
        }
    }

    fn matches(&self, provider: &Provider) -> bool {
        self.source == provider.source && self.id == provider.id
    }

    pub(super) fn element_id(&self) -> String {
        format!("ai-account-{}-{}", self.source.name(), self.id)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Sessions,
    Chat,
    Usage,
}

impl Tab {
    pub(super) const ALL: [Tab; 3] = [Tab::Sessions, Tab::Chat, Tab::Usage];

    pub(super) fn label(self) -> &'static str {
        match self {
            Tab::Sessions => "Sessions",
            Tab::Chat => "Chat",
            Tab::Usage => "Usage",
        }
    }
}

/// How far into a request's questions you are, and what you picked for each.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Picks {
    pub(super) step: usize,
    pub(super) chosen: Vec<Vec<usize>>,
}

#[derive(Default)]
pub struct View {
    order: Vec<Account>,
    seen: Option<Arc<Vec<Provider>>>,
    open: bool,
    pub(super) tab: Tab,
    /// The session the Sessions view shows; the first one when none is picked.
    pub(super) session: Option<String>,
    /// Steps, by their number, opened or closed by hand; the rest follow what is happening.
    toggled: HashSet<u64>,
    /// Each session's last step number as the panel opened: what came before is history, shown
    /// still; only what arrives while it shows moves in.
    seen_steps: HashMap<String, u64>,
    /// The chat's turns as the panel opened, likewise shown still.
    pub(super) seen_lines: usize,
    /// Requests waiting now, which keep the pop-up showing.
    pending: Vec<u64>,
    picks: HashMap<u64, Picks>,
    pub(super) draft: String,
    /// The requests as last seen, to tell which went.
    known: Vec<Request>,
    /// Ones just gone, still sliding out, until then.
    pub(super) leaving: Vec<(Request, Instant)>,
}

impl View {
    pub(super) fn sync(&mut self, state: &State, open: bool) {
        // One answered or timed out slides out of the pop-up before it goes.
        let now = Instant::now();
        for gone in self
            .known
            .iter()
            .filter(|known| !state.requests.iter().any(|request| request.id == known.id))
        {
            self.leaving.push((gone.clone(), now + reveal::LEAVE));
        }
        self.known = state.requests.to_vec();
        if open && !self.open {
            self.seen_steps = state
                .sessions
                .iter()
                .map(|session| (session.id.clone(), session.seq))
                .collect();
            self.seen_lines = state.chat.lines.len();
        }
        if self.open == open
            && self
                .seen
                .as_ref()
                .is_some_and(|seen| Arc::ptr_eq(seen, &state.providers))
        {
            return;
        }
        self.open = open;
        if !open {
            self.order.clear();
        } else {
            self.order.retain(|account| {
                state
                    .providers
                    .iter()
                    .any(|provider| !provider.is_empty() && account.matches(provider))
            });
        }
        for provider in state
            .providers
            .iter()
            .filter(|provider| !provider.is_empty())
        {
            if !self.order.iter().any(|account| account.matches(provider)) {
                self.order.push(Account::of(provider));
            }
        }
        self.seen = Some(state.providers.clone());
    }

    pub(super) fn ordered<'a>(&self, state: &'a State) -> Vec<&'a Provider> {
        if self.order.is_empty() {
            return state
                .providers
                .iter()
                .filter(|provider| !provider.is_empty())
                .collect();
        }
        self.order
            .iter()
            .filter_map(|account| {
                state
                    .providers
                    .iter()
                    .find(|provider| !provider.is_empty() && account.matches(provider))
            })
            .collect()
    }
}

impl View {
    pub(super) fn set_pending(&mut self, ids: Vec<u64>) {
        self.picks.retain(|id, _| ids.contains(id));
        self.pending = ids;
    }

    pub(super) fn picks(&self, id: u64) -> Picks {
        self.picks.get(&id).cloned().unwrap_or_default()
    }

    /// Picks `option` of the question being asked; one of a single choice replaces the last.
    pub(super) fn pick(&mut self, id: u64, option: usize, multi: bool) {
        let picks = self.picks.entry(id).or_default();
        let step = picks.step;
        if picks.chosen.len() <= step {
            picks.chosen.resize(step + 1, Vec::new());
        }
        let chosen = &mut picks.chosen[step];
        if !multi {
            chosen.clear();
            chosen.push(option);
        } else if let Some(at) = chosen.iter().position(|picked| *picked == option) {
            chosen.remove(at);
        } else {
            chosen.push(option);
        }
    }

    /// Whether something numbered `seq` in `session` arrived since the panel opened; a session
    /// that started since then is all new.
    pub(super) fn fresh(&self, session: &str, seq: u64) -> bool {
        self.seen_steps.get(session).is_none_or(|seen| seq > *seen)
    }

    /// A step is open on its own while it is the one being worked on; picking it flips that.
    pub(super) fn step_open(&self, seq: u64, following: bool) -> bool {
        following != self.toggled.contains(&seq)
    }

    pub(super) fn toggle_step(&mut self, seq: u64) {
        if !self.toggled.remove(&seq) {
            self.toggled.insert(seq);
        }
    }

    /// Only once something is picked for the question being asked.
    pub(super) fn next_question(&mut self, id: u64) {
        let picks = self.picks.entry(id).or_default();
        if picks.chosen.get(picks.step).is_some_and(|chosen| !chosen.is_empty()) {
            picks.step += 1;
        }
    }
}

impl Local for View {
    fn panel_closed(&mut self) {
        self.order.clear();
        self.seen = None;
        self.open = false;
    }

    fn popup(&self) -> bool {
        !self.pending.is_empty() || !self.leaving.is_empty()
    }

    /// Only the chat is typed into.
    fn keyboard(&self) -> bool {
        self.tab == Tab::Chat
    }

    fn show(&mut self, view: &str) {
        if let Some(tab) = Tab::ALL
            .into_iter()
            .find(|tab| tab.label().eq_ignore_ascii_case(view))
        {
            self.tab = tab;
        }
    }

    fn due(&self) -> Option<Instant> {
        self.leaving.iter().map(|(_, until)| *until).min()
    }
}

impl View {
    /// Lets go of the requests that have finished sliding out.
    pub(super) fn settle(&mut self) {
        let now = Instant::now();
        self.leaving.retain(|(_, until)| *until > now);
    }
}

#[cfg(test)]
#[path = "view_test.rs"]
mod tests;
