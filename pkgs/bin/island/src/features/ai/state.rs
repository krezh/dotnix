use std::{cmp::Ordering, sync::Arc};

use serde::{Deserialize, Serialize};

use super::{
    conversation::Chat,
    history::Window,
    session::{Request, Session},
    settings::Mascot,
};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Omp,
    Claude,
}

impl Source {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Omp => "omp",
            Self::Claude => "Claude Code",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    #[default]
    Ok,
    High,
    Exhausted,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Limit {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) used: f64,
    pub(super) remaining: f64,
    pub(super) resets_at: Option<i64>,
    pub(super) duration_ms: Option<i64>,
    pub(super) severity: Severity,
    pub(super) exhausts_at: Option<i64>,
    /// How much of the window is expected to be used by its reset, by the forecast.
    #[serde(skip)]
    pub(super) expected: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Credits {
    pub(super) count: u64,
    pub(super) expires_at: Option<i64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Freshness {
    Cached,
    #[default]
    Fresh,
    Stale,
}

impl Freshness {
    pub(super) fn is_fresh(self) -> bool {
        self == Self::Fresh
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Provider {
    pub(super) source: Source,
    pub(super) id: String,
    pub(super) provider_id: String,
    pub(super) name: String,
    pub(super) account: String,
    pub(super) plan: String,
    pub(super) limits: Vec<Limit>,
    pub(super) credits: Option<Credits>,
    pub(super) fetched_at: i64,
    #[serde(skip)]
    pub(super) freshness: Freshness,
}

impl Provider {
    pub(super) fn is_empty(&self) -> bool {
        self.limits.is_empty()
            && self
                .credits
                .as_ref()
                .is_none_or(|credits| credits.count == 0)
    }

    pub(super) fn severity(&self) -> Severity {
        self.limits
            .iter()
            .map(|limit| limit.severity)
            .max()
            .unwrap_or_default()
    }

    pub(super) fn headline(&self) -> Option<&Limit> {
        self.limits
            .iter()
            .min_by(|left, right| urgency(left, right))
    }
}

pub(super) fn urgency(left: &Limit, right: &Limit) -> Ordering {
    right
        .severity
        .cmp(&left.severity)
        .then_with(|| {
            if left.severity == Severity::Exhausted {
                left.resets_at
                    .unwrap_or(i64::MAX)
                    .cmp(&right.resets_at.unwrap_or(i64::MAX))
            } else {
                Ordering::Equal
            }
        })
        .then_with(|| right.used.total_cmp(&left.used))
        .then_with(|| left.id.cmp(&right.id))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Issue {
    pub(super) source: Source,
    pub(super) account: String,
    pub(super) message: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceReport {
    pub(super) providers: Vec<Provider>,
    pub(super) issues: Vec<Issue>,
}

/// Requests keep their time while the pointer is over an island.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hold {
    /// Since when, while held.
    pub(super) since: Option<i64>,
    /// When it was last let go, which their countdowns carry on from.
    pub(super) released_at: i64,
    /// Counts each hold and release, so a countdown bar animates afresh from where it is.
    pub(super) round: u32,
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub(super) providers: Arc<Vec<Provider>>,
    pub(super) issues: Arc<Vec<Issue>>,
    pub(super) refreshing: bool,
    pub(super) attempted_at: Option<i64>,
    pub(super) sessions: Arc<Vec<Session>>,
    /// Oldest first.
    pub(super) requests: Arc<Vec<Request>>,
    pub(super) hold: Hold,
    /// From the settings, for each island's own Jelly.
    pub(super) mascot: Mascot,
    pub(super) chat: Arc<Chat>,
    pub(super) windows: Arc<Vec<Window>>,
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.refreshing == other.refreshing
            && self.attempted_at == other.attempted_at
            && self.hold == other.hold
            && self.mascot == other.mascot
            && Arc::ptr_eq(&self.providers, &other.providers)
            && Arc::ptr_eq(&self.issues, &other.issues)
            && Arc::ptr_eq(&self.sessions, &other.sessions)
            && Arc::ptr_eq(&self.requests, &other.requests)
            && Arc::ptr_eq(&self.chat, &other.chat)
            && Arc::ptr_eq(&self.windows, &other.windows)
    }
}

impl State {
    pub(super) fn worst(&self) -> Option<(&Provider, &Limit)> {
        self.providers
            .iter()
            .flat_map(|provider| {
                provider
                    .limits
                    .iter()
                    .filter(|limit| limit.severity != Severity::Ok)
                    .map(move |limit| (provider, limit))
            })
            .min_by(|(_, left), (_, right)| urgency(left, right))
    }
}
