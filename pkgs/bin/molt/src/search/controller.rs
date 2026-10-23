use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

use gtk::{glib, prelude::*};

use super::{
    client::SearchClient,
    model::{PAGE_SIZE, SearchRequest, SearchResults, SearchScope, scoped_query},
    ui::{SearchView, save_last_search},
};
use crate::app::state::UiState;

const CACHED_PAGES: usize = 16;

struct CachedPage {
    scope: SearchScope,
    offset: usize,
    results: Rc<SearchResults>,
}

struct Queue {
    current: SearchRequest,
    pending: Option<SearchRequest>,
    ready: bool,
    running: bool,
    closed: bool,
    cache: VecDeque<CachedPage>,
    /// Each category's total for the current query, by `SearchScope::index`.
    counts: [Option<usize>; 4],
}

impl Queue {
    fn new() -> Self {
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

    fn replace(&mut self, query: String, scope: SearchScope, offset: usize, ready: bool) {
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

    fn take_cached(&mut self) -> Option<(SearchRequest, Rc<SearchResults>)> {
        if self.closed || !self.ready {
            return None;
        }
        let request = self.pending.as_ref()?;
        let index = self
            .cache
            .iter()
            .position(|page| page.scope == request.scope && page.offset == request.offset)?;
        let page = self.cache.remove(index).unwrap();
        let results = Rc::clone(&page.results);
        self.cache.push_back(page);
        Some((self.pending.take().unwrap(), results))
    }

    fn forget_current(&mut self) {
        self.cache
            .retain(|page| page.scope != self.current.scope || page.offset != self.current.offset);
    }

    fn remember(&mut self, scope: SearchScope, offset: usize, results: Rc<SearchResults>) {
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

    fn take_ready(&mut self) -> Option<SearchRequest> {
        if self.closed || self.running || !self.ready {
            return None;
        }
        let request = self.pending.take()?;
        self.running = true;
        Some(request)
    }

    fn complete(&mut self, request: &SearchRequest) -> bool {
        self.running = false;
        !self.closed
            && request.generation == self.current.generation
            && !self.current.query.trim().is_empty()
    }

    fn finish(
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
                    self.remember(scope, request.offset, Rc::new(results));
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

    fn close(&mut self) {
        self.closed = true;
        self.pending = None;
        self.cache.clear();
        self.counts = [None; 4];
    }
}

type SearchPages = Vec<(SearchScope, SearchResults)>;
type Completion = (SearchClient, SearchRequest, Result<SearchPages, String>);

fn search_pages(
    client: &mut SearchClient,
    request: &SearchRequest,
    cancellation: &AtomicBool,
    prefetch: bool,
) -> Result<SearchPages, String> {
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
    Ok(scopes
        .into_iter()
        .zip(client.search(&requests, cancellation)?)
        .collect())
}

pub(super) struct Controller {
    view: Rc<SearchView>,
    queue: RefCell<Queue>,
    client: RefCell<Option<SearchClient>>,
    receiver: RefCell<Option<mpsc::Receiver<Completion>>>,
    active: RefCell<Option<(SearchRequest, Arc<AtomicBool>)>>,
    debounce: RefCell<Option<glib::SourceId>>,
    lifetime: Arc<AtomicBool>,
}

impl Controller {
    pub(super) fn connect(view: Rc<SearchView>, state: &UiState) -> Rc<Self> {
        let lifetime = Arc::clone(&view.lifetime);
        state.set_view_cancellation(&lifetime);
        let controller = Rc::new(Self {
            view,
            queue: RefCell::new(Queue::new()),
            client: RefCell::new(Some(SearchClient::new())),
            receiver: RefCell::new(None),
            active: RefCell::new(None),
            debounce: RefCell::new(None),
            lifetime,
        });
        let weak = Rc::downgrade(&controller);
        controller.view.entry.connect_changed(move |entry| {
            let Some(controller) = weak.upgrade() else {
                return;
            };
            let text = entry.text().to_string();
            if let (Some(scope), rest) = scoped_query(&text) {
                // A typed prefix picks the category and leaves only the query in the entry. The
                // entry is rewritten once this change has finished being handled.
                controller.view.select_scope(scope);
                let rest = rest.to_owned();
                let entry = entry.clone();
                glib::idle_add_local_once(move || {
                    entry.set_text(&rest);
                    entry.set_position(-1);
                });
                return;
            }
            controller.change(text, controller.view.scope(), 0, true);
        });
        for (option, scope) in controller.view.scopes.options.iter().zip(SearchScope::ALL) {
            let weak = Rc::downgrade(&controller);
            option.connect_toggled(move |option| {
                if option.is_active()
                    && let Some(controller) = weak.upgrade()
                    && !controller.view.syncing.get()
                {
                    controller.change(controller.view.entry.text().to_string(), scope, 0, false);
                }
            });
        }
        for (button, direction) in [
            (&controller.view.previous, -1),
            (&controller.view.next, 1),
            (&controller.view.retry, 0),
        ] {
            let weak = Rc::downgrade(&controller);
            button.connect_clicked(move |_| {
                if let Some(controller) = weak.upgrade() {
                    let current = controller.queue.borrow().current.clone();
                    let offset = match direction {
                        -1 => current.offset.saturating_sub(PAGE_SIZE),
                        1 => current.offset + PAGE_SIZE,
                        _ => current.offset,
                    };
                    if direction == 0 {
                        controller.queue.borrow_mut().forget_current();
                        if let Some((_, cancellation)) = controller.active.borrow().as_ref() {
                            cancellation.store(true, Ordering::Relaxed);
                        }
                    }
                    controller.change(current.query, current.scope, offset, false);
                }
            });
        }
        let weak = Rc::downgrade(&controller);
        glib::timeout_add_local(Duration::from_millis(40), move || {
            let Some(controller) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if controller.lifetime.load(Ordering::Relaxed) {
                controller.stop();
                return glib::ControlFlow::Break;
            }
            controller.poll();
            glib::ControlFlow::Continue
        });
        controller
    }

    fn change(self: &Rc<Self>, query: String, scope: SearchScope, offset: usize, debounce: bool) {
        if self.lifetime.load(Ordering::Relaxed) {
            self.stop();
            return;
        }
        if let Some(source) = self.debounce.borrow_mut().take() {
            source.remove();
        }
        if let Some((request, cancellation)) = self.active.borrow().as_ref()
            && request.query != query
        {
            cancellation.store(true, Ordering::Relaxed);
        }
        let blank = query.trim().is_empty();
        let cached = {
            let mut queue = self.queue.borrow_mut();
            queue.replace(query, scope, offset, !debounce);
            queue.take_cached()
        };
        self.view.set_counts(self.queue.borrow().counts);
        if let Some((request, results)) = cached {
            self.view.present(&request.query, request.offset, results);
            return;
        }
        self.view.reset(!blank);
        if blank {
            return;
        }
        if debounce {
            let weak = Rc::downgrade(self);
            let source = glib::timeout_add_local_once(Duration::from_millis(250), move || {
                if let Some(controller) = weak.upgrade() {
                    controller.debounce.borrow_mut().take();
                    controller.queue.borrow_mut().ready = true;
                    controller.dispatch();
                }
            });
            self.debounce.replace(Some(source));
        } else {
            self.dispatch();
        }
    }

    fn dispatch(&self) {
        if self.lifetime.load(Ordering::Relaxed) {
            self.stop();
            return;
        }
        let cached = self.queue.borrow_mut().take_cached();
        if let Some((request, results)) = cached {
            self.view.present(&request.query, request.offset, results);
            return;
        }
        if self.client.borrow().is_none() {
            return;
        }
        let Some(request) = self.queue.borrow_mut().take_ready() else {
            return;
        };
        save_last_search(request.scope, &request.query);
        let prefetch = request.offset == 0 && self.queue.borrow().cache.is_empty();
        let mut client = self.client.borrow_mut().take().unwrap();
        let cancellation = Arc::new(AtomicBool::new(false));
        self.active
            .replace(Some((request.clone(), Arc::clone(&cancellation))));
        let (sender, receiver) = mpsc::channel();
        self.receiver.replace(Some(receiver));
        std::thread::spawn(move || {
            let result = search_pages(&mut client, &request, &cancellation, prefetch);
            let _ = sender.send((client, request, result));
        });
    }

    fn poll(&self) {
        let completion = self
            .receiver
            .borrow()
            .as_ref()
            .map(mpsc::Receiver::try_recv);
        match completion {
            Some(Ok((client, request, result))) => {
                self.receiver.take();
                let cancelled = self
                    .active
                    .take()
                    .is_none_or(|(_, cancellation)| cancellation.load(Ordering::Relaxed));
                self.client.replace(Some(client));
                let outcome = self.queue.borrow_mut().finish(&request, result, cancelled);
                self.view.set_counts(self.queue.borrow().counts);
                if let Err(error) = outcome {
                    self.view.failure(&error);
                }
                self.dispatch();
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.receiver.take();
                self.client.replace(Some(SearchClient::new()));
                if let Some((request, _)) = self.active.take() {
                    if self.queue.borrow_mut().complete(&request) {
                        self.view.failure("Search failed: search worker disconnected; Retry to start a new request");
                    }
                }
                self.dispatch();
            }
            _ => {}
        }
    }

    pub(super) fn stop(&self) {
        self.lifetime.store(true, Ordering::Relaxed);
        self.queue.borrow_mut().close();
        if let Some(source) = self.debounce.borrow_mut().take() {
            source.remove();
        }
        if let Some((_, cancellation)) = self.active.borrow().as_ref() {
            cancellation.store(true, Ordering::Relaxed);
        }
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delayed_a_is_replaced_by_only_latest_ready_b() {
        let mut queue = Queue::new();
        queue.replace("A".into(), SearchScope::All, 0, true);
        let a = queue.take_ready().unwrap();
        queue.replace("discarded".into(), SearchScope::Packages, 0, false);
        queue.replace("B".into(), SearchScope::HomeManagerOptions, 0, false);
        assert!(queue.take_ready().is_none());
        assert!(!queue.complete(&a));
        assert!(queue.take_ready().is_none());
        queue.ready = true;
        let b = queue.take_ready().unwrap();
        assert_eq!(b.query, "B");
        assert_eq!(b.scope, SearchScope::HomeManagerOptions);
        assert!(queue.take_ready().is_none());
        assert!(queue.complete(&b));
        assert!(queue.take_ready().is_none());
    }

    #[test]
    fn clearing_scope_changes_and_navigation_reject_outstanding_results() {
        for transition in 0..3 {
            let mut queue = Queue::new();
            queue.replace("A".into(), SearchScope::All, 50, true);
            let a = queue.take_ready().unwrap();
            match transition {
                0 => queue.replace(" ".into(), SearchScope::All, 0, true),
                1 => queue.replace("A".into(), SearchScope::NixOsOptions, 0, true),
                _ => queue.close(),
            }
            assert!(!queue.complete(&a));
            if transition == 1 {
                let next = queue.take_ready().unwrap();
                assert_eq!(next.offset, 0);
                assert_eq!(next.scope, SearchScope::NixOsOptions);
                assert!(queue.complete(&next));
            } else {
                assert!(queue.take_ready().is_none());
            }
        }
    }

    fn load_page(queue: &mut Queue, scope: SearchScope, offset: usize, name: &str) {
        queue.replace("hyprland".into(), scope, offset, true);
        assert!(queue.take_cached().is_none());
        let request = queue.take_ready().unwrap();
        assert!(queue.complete(&request));
        let package = serde_json::from_value(serde_json::json!({
            "package_attr_name": name,
        }))
        .unwrap();
        queue.remember(
            scope,
            offset,
            Rc::new(SearchResults {
                total: 109,
                elapsed: Duration::from_millis(250),
                items: vec![super::super::model::SearchResultItem::Package(package)],
                revision: None,
            }),
        );
    }

    fn cached_name(results: &SearchResults) -> &str {
        let super::super::model::SearchResultItem::Package(hit) = &results.items[0] else {
            panic!("expected cached package");
        };
        &hit.attr_name
    }

    #[test]
    fn category_and_page_revisits_reuse_results_without_dispatch() {
        let mut queue = Queue::new();
        load_page(&mut queue, SearchScope::All, 0, "all-first");
        load_page(&mut queue, SearchScope::Packages, 0, "packages-first");
        load_page(&mut queue, SearchScope::All, 50, "all-second");
        for (scope, offset, expected) in [
            (SearchScope::All, 0, "all-first"),
            (SearchScope::Packages, 0, "packages-first"),
            (SearchScope::All, 50, "all-second"),
        ] {
            queue.replace("hyprland".into(), scope, offset, true);
            let (request, results) = queue.take_cached().unwrap();
            assert_eq!((request.scope, request.offset), (scope, offset));
            assert_eq!(cached_name(&results), expected);
            assert_eq!(results.total, 109);
            assert!(queue.take_ready().is_none());
        }
    }

    #[test]
    fn cached_category_is_available_while_stale_work_finishes() {
        let mut queue = Queue::new();
        load_page(&mut queue, SearchScope::All, 0, "hyprland");
        queue.replace("hyprland".into(), SearchScope::NixOsOptions, 0, true);
        let outstanding = queue.take_ready().unwrap();
        queue.replace("hyprland".into(), SearchScope::All, 0, true);
        let (_, cached) = queue.take_cached().unwrap();
        assert_eq!(cached_name(&cached), "hyprland");
        assert!(queue.running);
        assert!(!queue.complete(&outstanding));
        assert!(queue.take_ready().is_none());
    }

    #[test]
    fn query_changes_clearing_retry_and_navigation_invalidate_cached_results() {
        for transition in 0..4 {
            let mut queue = Queue::new();
            load_page(&mut queue, SearchScope::All, 0, "hyprland");
            match transition {
                0 => queue.replace("ripgrep".into(), SearchScope::All, 0, true),
                1 => {
                    queue.replace(String::new(), SearchScope::All, 0, true);
                    assert!(queue.take_ready().is_none());
                    queue.replace("hyprland".into(), SearchScope::All, 0, true);
                }
                2 => {
                    queue.forget_current();
                    queue.replace("hyprland".into(), SearchScope::All, 0, true);
                }
                _ => queue.close(),
            }
            assert!(queue.take_cached().is_none());
            assert_eq!(queue.take_ready().is_some(), transition != 3);
        }
    }

    #[test]
    fn zero_results_are_cached_and_pages_evict_in_recent_use_order() {
        let mut queue = Queue::new();
        for page in 0..CACHED_PAGES {
            load_page(
                &mut queue,
                SearchScope::All,
                page * PAGE_SIZE,
                &format!("page-{page}"),
            );
        }
        queue.replace("hyprland".into(), SearchScope::All, 0, true);
        assert_eq!(cached_name(&queue.take_cached().unwrap().1), "page-0");
        queue.replace("hyprland".into(), SearchScope::HomeManagerOptions, 0, true);
        let request = queue.take_ready().unwrap();
        assert!(queue.complete(&request));
        queue.remember(
            SearchScope::HomeManagerOptions,
            0,
            Rc::new(SearchResults {
                total: 0,
                elapsed: Duration::ZERO,
                items: Vec::new(),
                revision: None,
            }),
        );
        queue.replace("hyprland".into(), SearchScope::HomeManagerOptions, 0, true);
        assert_eq!(queue.take_cached().unwrap().1.total, 0);
        assert!(queue.take_ready().is_none());
        queue.replace("hyprland".into(), SearchScope::All, 0, true);
        assert_eq!(cached_name(&queue.take_cached().unwrap().1), "page-0");
        queue.replace("hyprland".into(), SearchScope::All, PAGE_SIZE, true);
        assert!(queue.take_cached().is_none());
        assert_eq!(queue.take_ready().unwrap().offset, PAGE_SIZE);
    }

    fn category_pages() -> SearchPages {
        use super::super::model::SearchResultItem;
        let package = || {
            serde_json::from_value(serde_json::json!({
                "package_attr_name": "hyprland",
            }))
            .unwrap()
        };
        let option = |name| {
            serde_json::from_value(serde_json::json!({
                "option_name": name,
            }))
            .unwrap()
        };
        [
            (SearchScope::All, 109, SearchResultItem::Package(package())),
            (
                SearchScope::Packages,
                71,
                SearchResultItem::Package(package()),
            ),
            (
                SearchScope::NixOsOptions,
                11,
                SearchResultItem::NixOsOption(option("programs.hyprland.enable")),
            ),
            (
                SearchScope::HomeManagerOptions,
                27,
                SearchResultItem::HomeManagerOption(option(
                    "wayland.windowManager.hyprland.enable",
                )),
            ),
        ]
        .into_iter()
        .map(|(scope, total, item)| {
            (
                scope,
                SearchResults {
                    total,
                    elapsed: Duration::from_millis(100),
                    items: vec![item],
                    revision: None,
                },
            )
        })
        .collect()
    }

    #[test]
    fn initial_query_populates_first_visits_to_every_category() {
        let mut queue = Queue::new();
        queue.replace("hyprland".into(), SearchScope::All, 0, true);
        let request = queue.take_ready().unwrap();
        queue.finish(&request, Ok(category_pages()), false).unwrap();
        assert_eq!(queue.take_cached().unwrap().1.total, 109);
        for (scope, expected_total) in [
            (SearchScope::Packages, 71),
            (SearchScope::NixOsOptions, 11),
            (SearchScope::HomeManagerOptions, 27),
        ] {
            queue.replace("hyprland".into(), scope, 0, true);
            let (request, results) = queue.take_cached().unwrap();
            assert_eq!(request.scope, scope);
            assert_eq!(results.total, expected_total);
            assert!(queue.take_ready().is_none());
        }
        for page in 1..=CACHED_PAGES {
            load_page(&mut queue, SearchScope::All, page * PAGE_SIZE, "later-page");
        }
        for scope in SearchScope::ALL {
            queue.replace("hyprland".into(), scope, 0, true);
            assert_eq!(queue.take_cached().unwrap().0.scope, scope);
            assert!(queue.take_ready().is_none());
        }
    }

    #[test]
    fn category_totals_follow_the_current_query() {
        let mut queue = Queue::new();
        queue.replace("hyprland".into(), SearchScope::All, 0, true);
        let request = queue.take_ready().unwrap();
        queue.finish(&request, Ok(category_pages()), false).unwrap();
        assert_eq!(queue.counts, [Some(109), Some(71), Some(11), Some(27)]);
        queue.replace("hyprland".into(), SearchScope::Packages, 0, true);
        assert_eq!(queue.counts[1], Some(71));
        queue.replace("ripgrep".into(), SearchScope::Packages, 0, true);
        assert_eq!(queue.counts, [None; 4]);
    }

    #[test]
    fn category_change_during_initial_load_reuses_the_same_batch() {
        let mut queue = Queue::new();
        queue.replace("hyprland".into(), SearchScope::All, 0, true);
        let request = queue.take_ready().unwrap();
        queue.replace("hyprland".into(), SearchScope::HomeManagerOptions, 0, true);
        assert!(queue.take_cached().is_none());
        assert!(queue.take_ready().is_none());
        queue.finish(&request, Ok(category_pages()), false).unwrap();
        let (selected, results) = queue.take_cached().unwrap();
        assert_eq!(selected.scope, SearchScope::HomeManagerOptions);
        assert_eq!(results.total, 27);
        assert!(
            matches!(&results.items[0], super::super::model::SearchResultItem::HomeManagerOption(hit) if hit.name == "wayland.windowManager.hyprland.enable")
        );
        assert!(queue.take_ready().is_none());
    }

    #[test]
    fn cancelled_or_obsolete_batches_cannot_repopulate_categories() {
        for transition in 0..3 {
            let mut queue = Queue::new();
            queue.replace("hyprland".into(), SearchScope::All, 0, true);
            let request = queue.take_ready().unwrap();
            queue.replace("ripgrep".into(), SearchScope::All, 0, true);
            if transition == 1 {
                queue.replace("hyprland".into(), SearchScope::All, 0, true);
            } else if transition == 2 {
                queue.close();
            }
            queue
                .finish(&request, Ok(category_pages()), transition == 1)
                .unwrap();
            assert!(queue.take_cached().is_none());
            assert_eq!(queue.take_ready().is_some(), transition != 2);
        }
    }

    #[test]
    fn failed_initial_batch_stops_after_a_category_change_until_retry() {
        let mut queue = Queue::new();
        queue.replace("hyprland".into(), SearchScope::All, 0, true);
        let request = queue.take_ready().unwrap();
        queue.replace("hyprland".into(), SearchScope::Packages, 0, true);
        assert_eq!(
            queue
                .finish(&request, Err("Search failed: HTTP 503".into()), false)
                .unwrap_err(),
            "Search failed: HTTP 503"
        );
        assert!(queue.take_cached().is_none());
        assert!(queue.take_ready().is_none());
        queue.replace("hyprland".into(), SearchScope::Packages, 0, true);
        assert_eq!(queue.take_ready().unwrap().scope, SearchScope::Packages);
    }
}
