//! What the Search screen knows and how it changes: the query and category, the page shown, the
//! picked result, and each result's check against the user's own flake.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use crate::cache;
use crate::config::Config;
use crate::search::client::SearchClient;
use crate::search::local::{self, LocalValue};
use crate::search::model::{
    PAGE_SIZE, SearchRequest, SearchResultItem, SearchResults, SearchScope, scoped_query,
};
use crate::search::queue::{Queue, SearchPages};

const LAST_SEARCH: &str = "last-search";
/// How long typing has to rest before the catalog is asked.
pub const SETTLE: Duration = Duration::from_millis(250);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Local {
    Running,
    Done(Result<LocalValue, String>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Search {
    pub config: Option<Config>,
    pub query: String,
    pub scope: SearchScope,
    pub queue: Queue,
    /// The page shown, its offset with it.
    pub shown: Option<(usize, Arc<SearchResults>)>,
    pub loading: bool,
    pub error: Option<String>,
    pub selected: usize,
    pub local: HashMap<String, Local>,
    /// The query is the last search, restored and selected, so typing replaces it.
    pub restored: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Query(String),
    Scope(SearchScope),
    Previous,
    Next,
    Retry,
    Select(usize),
    /// Moves the pick by this many results.
    Step(isize),
    /// Typing rested for this request's generation.
    Settled(u64),
    Fetched(SearchRequest, Result<SearchPages, String>),
    CheckLocally,
    Checked(String, Result<LocalValue, String>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Settle(u64),
    Fetch { request: SearchRequest, prefetch: bool },
    Evaluate { key: String, item: SearchResultItem },
}

pub fn local_key(item: &SearchResultItem) -> String {
    let category = match item {
        SearchResultItem::Package(_) => "Package",
        SearchResultItem::NixOsOption(_) => "NixOS option",
        SearchResultItem::HomeManagerOption(_) => "Home Manager option",
    };
    format!("{category}:{}", item.name())
}

fn parse_last_search(text: &str) -> Option<(SearchScope, &str)> {
    let (scope, query) = text.split_once('\n')?;
    let scope = *SearchScope::ALL.get(scope.parse::<usize>().ok()?)?;
    let query = query.trim();
    (!query.is_empty()).then_some((scope, query))
}

impl Search {
    /// Opens on the last search, which runs again at once.
    pub fn open(config: Option<Config>) -> (Self, Vec<Effect>) {
        let mut screen = Self {
            config,
            query: String::new(),
            scope: SearchScope::All,
            queue: Queue::default(),
            shown: None,
            loading: false,
            error: None,
            selected: 0,
            local: HashMap::new(),
            restored: false,
        };
        let last = cache::read(LAST_SEARCH);
        let effects = match last.as_deref().and_then(parse_last_search) {
            Some((scope, query)) => {
                screen.scope = scope;
                screen.query = query.to_owned();
                screen.restored = true;
                screen.change(screen.query.clone(), scope, 0, false)
            }
            None => Vec::new(),
        };
        (screen, effects)
    }

    pub fn current(&self) -> Option<&SearchResultItem> {
        self.shown
            .as_ref()
            .and_then(|(_, results)| results.items.get(self.selected))
    }

    fn present(&mut self, offset: usize, results: Arc<SearchResults>) {
        self.loading = false;
        self.error = None;
        self.selected = 0;
        self.shown = Some((offset, results));
    }

    fn change(&mut self, query: String, scope: SearchScope, offset: usize, settle: bool) -> Vec<Effect> {
        let blank = query.trim().is_empty();
        self.queue.replace(query, scope, offset, !settle);
        if let Some((request, results)) = self.queue.take_cached() {
            self.present(request.offset, results);
            return Vec::new();
        }
        self.shown = None;
        self.error = None;
        self.loading = !blank;
        if blank {
            return Vec::new();
        }
        if settle {
            vec![Effect::Settle(self.queue.current.generation)]
        } else {
            self.dispatch()
        }
    }

    fn dispatch(&mut self) -> Vec<Effect> {
        if let Some((request, results)) = self.queue.take_cached() {
            self.present(request.offset, results);
            return Vec::new();
        }
        let Some(request) = self.queue.take_ready() else {
            return Vec::new();
        };
        cache::write(
            LAST_SEARCH,
            &format!("{}\n{}\n", request.scope.index(), request.query.trim()),
        );
        let prefetch = request.offset == 0 && self.queue.cache.is_empty();
        vec![Effect::Fetch { request, prefetch }]
    }

    pub fn update(&mut self, msg: Msg) -> Vec<Effect> {
        match msg {
            Msg::Query(text) => {
                self.restored = false;
                if let (Some(scope), rest) = scoped_query(&text) {
                    self.scope = scope;
                    self.query = rest.to_owned();
                } else {
                    self.query = text;
                }
                self.change(self.query.clone(), self.scope, 0, true)
            }
            Msg::Scope(scope) => {
                self.scope = scope;
                self.change(self.query.clone(), scope, 0, false)
            }
            Msg::Previous => {
                let current = self.queue.current.clone();
                self.change(
                    current.query,
                    current.scope,
                    current.offset.saturating_sub(PAGE_SIZE),
                    false,
                )
            }
            Msg::Next => {
                let current = self.queue.current.clone();
                self.change(current.query, current.scope, current.offset + PAGE_SIZE, false)
            }
            Msg::Retry => {
                self.queue.forget_current();
                let current = self.queue.current.clone();
                self.change(current.query, current.scope, current.offset, false)
            }
            Msg::Select(index) => {
                if self
                    .shown
                    .as_ref()
                    .is_some_and(|(_, results)| index < results.items.len())
                {
                    self.selected = index;
                }
                Vec::new()
            }
            Msg::Step(by) => {
                let count = self
                    .shown
                    .as_ref()
                    .map_or(0, |(_, results)| results.items.len());
                if count > 0 {
                    self.selected = self
                        .selected
                        .saturating_add_signed(by)
                        .min(count - 1);
                }
                Vec::new()
            }
            Msg::Settled(generation) if generation == self.queue.current.generation => {
                self.queue.ready = true;
                self.dispatch()
            }
            Msg::Fetched(request, result) => {
                if let Err(error) = self.queue.finish(&request, result, false) {
                    self.loading = false;
                    self.shown = None;
                    self.error = Some(error);
                }
                self.dispatch()
            }
            Msg::CheckLocally => {
                let Some(item) = self.current().cloned() else {
                    return Vec::new();
                };
                let key = local_key(&item);
                if self.config.is_none() || matches!(self.local.get(&key), Some(Local::Running)) {
                    return Vec::new();
                }
                self.local.insert(key.clone(), Local::Running);
                vec![Effect::Evaluate { key, item }]
            }
            Msg::Checked(key, result) => {
                self.local.insert(key, Local::Done(result));
                Vec::new()
            }
            _ => Vec::new(),
        }
    }
}

/// Runs an [`Effect`] to the end; `Settle` only waits.
pub fn run(effect: Effect, config: Option<&Config>, cancellation: &AtomicBool) -> Msg {
    match effect {
        Effect::Settle(generation) => {
            std::thread::sleep(SETTLE);
            Msg::Settled(generation)
        }
        Effect::Fetch { request, prefetch } => {
            let count = if prefetch { SearchScope::ALL.len() } else { 1 };
            let scopes: Vec<SearchScope> = std::iter::once(request.scope)
                .chain(
                    SearchScope::ALL
                        .into_iter()
                        .filter(|scope| *scope != request.scope),
                )
                .take(count)
                .collect();
            let requests: Vec<SearchRequest> = scopes
                .iter()
                .map(|&scope| SearchRequest {
                    scope,
                    ..request.clone()
                })
                .collect();
            let result = SearchClient::new()
                .search(&requests, cancellation)
                .map(|results| scopes.into_iter().zip(results).collect());
            Msg::Fetched(request, result)
        }
        Effect::Evaluate { key, item } => {
            let result = match config {
                Some(config) => local::evaluate(config, &item, cancellation),
                None => Err("Molt has no configuration".to_owned()),
            };
            Msg::Checked(key, result)
        }
    }
}
