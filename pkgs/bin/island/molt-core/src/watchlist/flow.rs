//! What the Watchlist screen knows and how it changes: the tracked pull requests, the filter and
//! pick, tracking a new one, untracking, and refreshing them all from GitHub.

use std::sync::atomic::AtomicBool;

use crate::watchlist::WatchedPr;
use crate::watchlist::db::{delete_pr, list_prs, open_db, save_pr};
use crate::watchlist::github::{fetch_pr, parse_pr_ref};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Watchlist {
    pub prs: Vec<WatchedPr>,
    pub query: String,
    pub selected: Option<i64>,
    /// What is typed in the track field.
    pub track: String,
    pub tracking: bool,
    pub refreshing: bool,
    /// The line beside Refresh all, and whether it is an error.
    pub status: Option<(String, bool)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    Query(String),
    Select(i64),
    /// Moves the pick by this many shown pull requests.
    Step(isize),
    TrackText(String),
    Track,
    Untrack,
    Refresh,
    /// The tracked pull request's id and the list read again, or why it could not be tracked.
    Tracked(Result<(i64, Vec<WatchedPr>), String>),
    Reloaded(Vec<WatchedPr>),
    Refreshed(Vec<WatchedPr>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Track {
        owner: String,
        repo: String,
        number: u64,
    },
    Untrack(i64),
    Refresh(Vec<WatchedPr>),
}

pub fn matches(pr: &WatchedPr, query: &str) -> bool {
    let query = query.to_lowercase();
    query.is_empty()
        || pr.title.to_lowercase().contains(&query)
        || pr.pr_number.to_string().contains(&query)
        || pr.repo.to_lowercase().contains(&query)
}

fn read() -> Vec<WatchedPr> {
    open_db()
        .ok()
        .flatten()
        .and_then(|conn| list_prs(&conn).ok())
        .unwrap_or_default()
}

impl Watchlist {
    pub fn open() -> Self {
        let mut screen = Self {
            prs: Vec::new(),
            query: String::new(),
            selected: None,
            track: String::new(),
            tracking: false,
            refreshing: false,
            status: None,
        };
        screen.set_prs(read());
        screen
    }

    pub fn shown(&self) -> Vec<&WatchedPr> {
        self.prs
            .iter()
            .filter(|pr| matches(pr, &self.query))
            .collect()
    }

    pub fn current(&self) -> Option<&WatchedPr> {
        let selected = self.selected?;
        self.prs.iter().find(|pr| pr.id == selected)
    }

    fn set_prs(&mut self, prs: Vec<WatchedPr>) {
        let keep = self
            .selected
            .filter(|id| prs.iter().any(|pr| pr.id == *id));
        self.selected = keep.or_else(|| prs.first().map(|pr| pr.id));
        self.prs = prs;
    }

    pub fn update(&mut self, msg: Msg) -> Option<Effect> {
        match msg {
            Msg::Query(query) => {
                self.query = query;
                None
            }
            Msg::Select(id) => {
                if self.prs.iter().any(|pr| pr.id == id) {
                    self.selected = Some(id);
                }
                None
            }
            Msg::Step(by) => {
                let shown = self.shown();
                let at = shown.iter().position(|pr| Some(pr.id) == self.selected);
                let next = match at {
                    Some(at) => at.saturating_add_signed(by).min(shown.len().saturating_sub(1)),
                    None => 0,
                };
                if let Some(pr) = shown.get(next) {
                    self.selected = Some(pr.id);
                }
                None
            }
            Msg::TrackText(text) => {
                self.track = text;
                None
            }
            Msg::Track if !self.tracking => {
                let text = self.track.trim().to_owned();
                if text.is_empty() {
                    return None;
                }
                self.status = None;
                match parse_pr_ref(&text) {
                    Ok((owner, repo, number)) => {
                        self.tracking = true;
                        Some(Effect::Track {
                            owner,
                            repo,
                            number,
                        })
                    }
                    Err(error) => {
                        self.status = Some((error, true));
                        None
                    }
                }
            }
            Msg::Tracked(result) if self.tracking => {
                self.tracking = false;
                match result {
                    Ok((id, prs)) => {
                        self.track.clear();
                        self.status = None;
                        self.set_prs(prs);
                        if self.prs.iter().any(|pr| pr.id == id) {
                            self.selected = Some(id);
                        }
                    }
                    Err(error) => self.status = Some((error, true)),
                }
                None
            }
            Msg::Untrack => self.selected.map(Effect::Untrack),
            Msg::Reloaded(prs) => {
                self.set_prs(prs);
                None
            }
            Msg::Refresh if !self.refreshing => {
                self.refreshing = true;
                self.status = Some(("Refreshing pull requests…".to_owned(), false));
                Some(Effect::Refresh(self.prs.clone()))
            }
            Msg::Refreshed(prs) if self.refreshing => {
                self.refreshing = false;
                self.status = None;
                self.set_prs(prs);
                None
            }
            _ => None,
        }
    }
}

/// Runs an [`Effect`] to the end, reading the list again after it.
pub fn run(effect: Effect, cancellation: &AtomicBool) -> Msg {
    match effect {
        Effect::Track {
            owner,
            repo,
            number,
        } => Msg::Tracked(fetch_pr(&owner, &repo, number, None, cancellation).map(|pr| {
            let id = open_db()
                .ok()
                .flatten()
                .and_then(|conn| save_pr(&conn, &pr).ok())
                .unwrap_or(0);
            (id, read())
        })),
        Effect::Untrack(id) => {
            if let Ok(Some(conn)) = open_db() {
                let _ = delete_pr(&conn, id);
            }
            Msg::Reloaded(read())
        }
        Effect::Refresh(prs) => {
            let refreshed: Vec<WatchedPr> = prs
                .into_iter()
                .map(|pr| {
                    fetch_pr(&pr.owner, &pr.repo, pr.pr_number, Some(pr.id), cancellation)
                        .unwrap_or(pr)
                })
                .collect();
            if let Ok(Some(conn)) = open_db() {
                for pr in &refreshed {
                    let _ = save_pr(&conn, pr);
                }
            }
            Msg::Refreshed(read())
        }
    }
}
