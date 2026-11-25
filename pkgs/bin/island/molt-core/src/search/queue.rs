//! Which search request runs next: only the newest typed query is fetched once typing settles,
//! and pages already fetched for this query are shown again without asking the catalog.

use std::collections::VecDeque;
use std::sync::Arc;

use crate::search::model::{SearchRequest, SearchResults, SearchScope};

const CACHED_PAGES: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct CachedPage {
    pub scope: SearchScope,
    pub offset: usize,
    pub results: Arc<SearchResults>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Queue {
    pub current: SearchRequest,
    pub pending: Option<SearchRequest>,
    pub ready: bool,
    pub running: bool,
    pub closed: bool,
    pub cache: VecDeque<CachedPage>,
    /// Each category's total for the current query, by `SearchScope::index`.
    pub counts: [Option<usize>; 4],
}

pub type SearchPages = Vec<(SearchScope, SearchResults)>;

impl Default for Queue {
    fn default() -> Self {
        Self {
            current: SearchRequest {
                generation: 0,
                query: String::new(),
                scope: SearchScope::All,
                offset: 0,
            },
            pending: None,
            ready: false,
            running: false,
            closed: false,
            cache: VecDeque::new(),
            counts: [None; 4],
        }
    }
}

impl Queue {
    pub fn replace(&mut self, query: String, scope: SearchScope, offset: usize, ready: bool) {
        if query != self.current.query {
            self.cache.clear();
            self.counts = [None; 4];
        }
        self.current = SearchRequest {
            generation: self.current.generation.wrapping_add(1),
            query,
            scope,
            offset,
        };
        self.pending =
            (!self.current.query.trim().is_empty() && !self.closed).then(|| self.current.clone());
        self.ready = ready;
    }

    pub fn take_cached(&mut self) -> Option<(SearchRequest, Arc<SearchResults>)> {
        if self.closed || !self.ready {
            return None;
        }
        let request = self.pending.as_ref()?;
        let index = self
            .cache
            .iter()
            .position(|page| page.scope == request.scope && page.offset == request.offset)?;
        let page = self.cache.remove(index)?;
        let results = Arc::clone(&page.results);
        self.cache.push_back(page);
        Some((self.pending.take()?, results))
    }

    pub fn forget_current(&mut self) {
        let (scope, offset) = (self.current.scope, self.current.offset);
        self.cache
            .retain(|page| page.scope != scope || page.offset != offset);
    }

    pub fn remember(&mut self, scope: SearchScope, offset: usize, results: Arc<SearchResults>) {
        self.counts[scope.index()] = Some(results.total);
        self.cache
            .retain(|page| page.scope != scope || page.offset != offset);
        if self.cache.len() == CACHED_PAGES {
            // First pages stay populated even after browsing many later pages.
            let oldest = self
                .cache
                .iter()
                .position(|page| page.offset != 0)
                .unwrap_or(0);
            self.cache.remove(oldest);
        }
        self.cache.push_back(CachedPage {
            scope,
            offset,
            results,
        });
    }

    pub fn take_ready(&mut self) -> Option<SearchRequest> {
        if self.closed || self.running || !self.ready {
            return None;
        }
        let request = self.pending.take()?;
        self.running = true;
        Some(request)
    }

    pub fn complete(&mut self, request: &SearchRequest) -> bool {
        self.running = false;
        !self.closed
            && request.generation == self.current.generation
            && !self.current.query.trim().is_empty()
    }

    pub fn finish(
        &mut self,
        request: &SearchRequest,
        result: Result<SearchPages, String>,
        cancelled: bool,
    ) -> Result<(), String> {
        let present = self.complete(request);
        if cancelled || self.closed || request.query != self.current.query {
            return Ok(());
        }
        match result {
            Ok(pages) => {
                for (scope, results) in pages {
                    self.remember(scope, request.offset, Arc::new(results));
                }
                if present {
                    self.pending = Some(request.clone());
                }
                Ok(())
            }
            Err(error) if present || self.pending.is_some() => {
                self.pending = None;
                Err(error)
            }
            Err(_) => Ok(()),
        }
    }

    pub fn close(&mut self) {
        self.closed = true;
        self.pending = None;
        self.cache.clear();
        self.counts = [None; 4];
    }
}
