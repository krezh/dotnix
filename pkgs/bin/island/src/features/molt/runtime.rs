//! Fetches every 15 minutes or when asked; in between, checks every minute against what was last
//! fetched, without the network, so the counts follow local changes such as a pull. Also runs
//! molt's screens while its panel is open: Home and its repository check and pull, and each tool's
//! work, such as a cleanup, which carries on with the panel closed.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, RecvTimeoutError, Sender},
    },
    thread,
    time::{Duration, Instant},
};

use molt_core::{
    build::{
        Target,
        flow::{self as build, AfterBuild},
    },
    cleanup::flow as cleanup,
    config::load_config,
    protocol::ActivationMode,
    report::{flow as review, notes},
    home::flow::{self, Home, Msg},
    inputs::flow as inputs,
    search::flow as search,
    watchlist::flow as watchlist,
};
use tracing::{debug, info, warn};

use molt_core::report::notes::Changelog;

use super::{
    cache::{self, now},
    check::check,
    format::IslandUnits,
    state::Updates,
};
use crate::{
    components::input::{Edit, edit},
    features::Feed,
    key::Key,
    supervisor::Ended,
};

const INTERVAL: Duration = Duration::from_secs(15 * 60);
const TICK: Duration = Duration::from_secs(60);

enum Request {
    Fetch,
    /// Without the network.
    Look,
    /// Reads Home afresh, as the panel opens.
    OpenHome,
    Home(Msg),
    OpenInputs,
    Inputs(inputs::Msg),
    OpenWatchlist,
    Watchlist(watchlist::Msg),
    OpenSearch,
    Search(search::Msg),
    OpenCleanup,
    Cleanup(cleanup::Msg),
    StartBuild(Target, AfterBuild),
    Build(Box<build::Msg>),
    Review(review::Msg),
    OpenNotes(usize),
    Notes(notes::Msg),
    /// The build's or switch's end was seen.
    Seen,
    /// A key typed into one of the screens' fields, applied here in order so none is lost.
    Typed(Field, Key),
}

/// The fields typing can go to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    InputsFilter,
    InputsPin,
    WatchFilter,
    WatchTrack,
    SearchQuery,
    CleanupFilter,
    CleanupJournals,
    BuildFilter,
    ReviewFilter,
}

/// What Clean up runs, sharing the flag that stops it.
struct Cleaning {
    cancel: Arc<AtomicBool>,
}

impl Cleaning {
    fn spawn(&self, effects: Vec<cleanup::Effect>, requests: &Sender<Request>) {
        for effect in effects {
            let requests = requests.clone();
            let cancel = Arc::clone(&self.cancel);
            thread::spawn(move || {
                cleanup::run(effect, &cancel, &|msg| {
                    let _ = requests.send(Request::Cleanup(msg));
                });
            });
        }
    }

    fn update(&self, state: &mut Updates, msg: cleanup::Msg, requests: &Sender<Request>) {
        if let Some(screen) = &mut state.cleanup {
            self.spawn(screen.update(msg), requests);
        }
    }

    /// Opens Clean up afresh, unless a cleanup is under way, which it then shows.
    fn open(&mut self, state: &mut Updates, requests: &Sender<Request>) {
        if state
            .cleanup
            .as_ref()
            .is_some_and(|screen| screen.phase == cleanup::Phase::Cleaning)
        {
            return;
        }
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        let policy = load_config()
            .map(|config| config.cleanup)
            .unwrap_or_default();
        let (screen, effect) = cleanup::Cleanup::open(policy);
        state.cleanup = Some(screen);
        self.spawn(vec![effect], requests);
    }
}

/// What builds, switches and release notes run, and what they keep between runs.
#[derive(Default)]
struct Building {
    generation: u64,
    cancel: Arc<AtomicBool>,
    notes: HashMap<String, Changelog>,
}

/// Whether the reviewed build is now the running system, so Home no longer offers it.
fn switched(review: &review::Review) -> bool {
    review.done.contains(&ActivationMode::Switch)
}

/// Sends `tick` every `every` until `done` is set.
fn ticker(every: Duration, done: Arc<AtomicBool>, requests: Sender<Request>, tick: fn() -> Request) {
    thread::spawn(move || {
        while !done.load(Ordering::Relaxed) {
            thread::sleep(every);
            if requests.send(tick()).is_err() {
                return;
            }
        }
    });
}

impl Building {
    fn start(&mut self, state: &mut Updates, target: Target, after: AfterBuild, requests: &Sender<Request>) {
        if state.build.as_ref().is_some_and(build::Build::running) {
            return;
        }
        let config = match load_config() {
            Ok(config) => config,
            Err(error) => {
                warn!("cannot start a build: {error}");
                return;
            }
        };
        self.generation += 1;
        self.cancel = Arc::new(AtomicBool::new(false));
        let (screen, effect) = build::Build::start(config, target, after, self.generation);
        state.build = Some(screen);
        self.run_build(effect, requests);
    }

    fn run_build(&self, effect: build::Effect, requests: &Sender<Request>) {
        let cancel = Arc::clone(&self.cancel);
        let requests = requests.clone();
        thread::spawn(move || {
            build::run(effect, &cancel, &|msg| {
                let _ = requests.send(Request::Build(Box::new(msg)));
            });
        });
    }

    /// Cancelling stops nix and drops the build at once, so a new one can start.
    fn build(&mut self, state: &mut Updates, msg: build::Msg, requests: &Sender<Request>) {
        let Some(screen) = &mut state.build else {
            return;
        };
        if matches!(msg, build::Msg::Cancel) {
            if screen.running() {
                self.cancel.store(true, Ordering::Relaxed);
                state.build = None;
            }
            return;
        }
        let was_running = screen.running();
        if let Some(effect) = screen.update(msg, &IslandUnits) {
            match effect {
                build::Effect::Log(_) => {
                    let requests = requests.clone();
                    thread::spawn(move || {
                        build::run(effect, &Arc::new(AtomicBool::new(false)), &|msg| {
                            let _ = requests.send(Request::Build(Box::new(msg)));
                        });
                    });
                }
                effect => self.run_build(effect, requests),
            }
        }
        if was_running && let build::Stage::Ready(report) = &screen.stage {
            let switch = screen.after == AfterBuild::Switch;
            let target = report.target;
            state.review = Some(review::Review::open((**report).clone()));
            if let Some(home) = &mut state.home {
                home.reviewed.retain(|reviewed| *reviewed != target);
                home.reviewed.push(target);
            }
            if switch {
                self.review(state, review::Msg::Activate(ActivationMode::Switch), requests);
            }
        }
    }

    fn review(&mut self, state: &mut Updates, msg: review::Msg, requests: &Sender<Request>) {
        let Some(screen) = &mut state.review else {
            return;
        };
        let effect = screen.update(msg);
        if switched(screen)
            && let Some(home) = &mut state.home
        {
            let target = screen.report.target;
            home.reviewed.retain(|reviewed| *reviewed != target);
        }
        let Some(effect) = effect else {
            return;
        };
        state.seen_activation = false;
        if let Some(build) = &state.build {
            state.seen_build = build.generation;
        }
        let done = Arc::new(AtomicBool::new(false));
        ticker(
            Duration::from_millis(100),
            Arc::clone(&done),
            requests.clone(),
            || Request::Review(review::Msg::Tick),
        );
        let requests = requests.clone();
        thread::spawn(move || {
            let msg = review::run(effect, &AtomicBool::new(false));
            done.store(true, Ordering::Relaxed);
            let _ = requests.send(Request::Review(msg));
        });
    }

    fn open_notes(&mut self, state: &mut Updates, index: usize, requests: &Sender<Request>) {
        let Some(review) = &state.review else {
            return;
        };
        let Some(change) = review.report.changes.get(index) else {
            return;
        };
        self.generation += 1;
        let probe = notes::Notes::open(change.clone(), &review.report, self.generation, None).0;
        let cached = self.notes.get(&probe.cache_key()).cloned();
        let (screen, effect) =
            notes::Notes::open(change.clone(), &review.report, self.generation, cached);
        state.notes = Some(screen);
        let Some(effect) = effect else {
            return;
        };
        let done = Arc::new(AtomicBool::new(false));
        ticker(
            Duration::from_secs(1),
            Arc::clone(&done),
            requests.clone(),
            || Request::Notes(notes::Msg::Tick),
        );
        let requests = requests.clone();
        thread::spawn(move || {
            notes::run(effect, &AtomicBool::new(false), &|msg| {
                let _ = requests.send(Request::Notes(msg));
            });
            done.store(true, Ordering::Relaxed);
        });
    }

    fn notes(&mut self, state: &mut Updates, msg: notes::Msg) {
        let Some(screen) = &mut state.notes else {
            return;
        };
        screen.update(msg);
        if let Some(Ok(changelog)) = &screen.result {
            self.notes
                .entry(screen.cache_key())
                .or_insert_with(|| changelog.clone());
        }
    }
}

fn typed(
    state: &mut Updates,
    cleaning: &Cleaning,
    building: &mut Building,
    field: Field,
    key: &Key,
    requests: &Sender<Request>,
) {
    let text = match field {
        Field::InputsFilter => state.inputs.as_ref().map(|screen| screen.query.clone()),
        Field::InputsPin => state.inputs.as_ref().and_then(|screen| screen.pin.clone()),
        Field::WatchFilter => state.watchlist.as_ref().map(|screen| screen.query.clone()),
        Field::WatchTrack => state.watchlist.as_ref().map(|screen| screen.track.clone()),
        Field::SearchQuery => state.search.as_ref().map(|screen| {
            if screen.restored {
                String::new()
            } else {
                screen.query.clone()
            }
        }),
        Field::CleanupFilter => state.cleanup.as_ref().map(|screen| screen.query.clone()),
        Field::CleanupJournals => state.cleanup.as_ref().map(|screen| screen.journals.clone()),
        Field::BuildFilter => state.build.as_ref().map(|screen| screen.query.clone()),
        Field::ReviewFilter => state.review.as_ref().map(|screen| screen.query.clone()),
    };
    let Some(mut text) = text else {
        return;
    };
    let edited = edit(&mut text, key);
    match (field, edited) {
        (_, Edit::Ignored) => {}
        (Field::InputsFilter, Edit::Changed) => {
            inputs_msg(state, inputs::Msg::Query(text), requests);
        }
        (Field::InputsFilter, Edit::Cancel) => {
            inputs_msg(state, inputs::Msg::Query(String::new()), requests);
        }
        (Field::InputsPin, Edit::Changed) => inputs_msg(state, inputs::Msg::PinText(text), requests),
        (Field::InputsPin, Edit::Send) => inputs_msg(state, inputs::Msg::ApplyPin, requests),
        (Field::InputsPin, Edit::Cancel) => inputs_msg(state, inputs::Msg::TogglePin, requests),
        (Field::WatchFilter, Edit::Changed) => {
            watchlist_msg(state, watchlist::Msg::Query(text), requests);
        }
        (Field::WatchFilter, Edit::Cancel) => {
            watchlist_msg(state, watchlist::Msg::Query(String::new()), requests);
        }
        (Field::WatchTrack, Edit::Changed) => {
            watchlist_msg(state, watchlist::Msg::TrackText(text), requests);
        }
        (Field::WatchTrack, Edit::Send) => watchlist_msg(state, watchlist::Msg::Track, requests),
        (Field::SearchQuery, Edit::Changed) => search_msg(state, search::Msg::Query(text), requests),
        (Field::SearchQuery, Edit::Cancel) => {
            search_msg(state, search::Msg::Query(String::new()), requests);
        }
        (Field::CleanupFilter, Edit::Changed) => {
            cleaning.update(state, cleanup::Msg::Query(text), requests);
        }
        (Field::CleanupFilter, Edit::Cancel) => {
            cleaning.update(state, cleanup::Msg::Query(String::new()), requests);
        }
        (Field::CleanupJournals, Edit::Changed) => {
            cleaning.update(state, cleanup::Msg::Journals(text), requests);
        }
        (Field::BuildFilter, Edit::Changed) => {
            building.build(state, build::Msg::Query(text), requests);
        }
        (Field::BuildFilter, Edit::Cancel) => {
            building.build(state, build::Msg::Query(String::new()), requests);
        }
        (Field::ReviewFilter, Edit::Changed) => {
            building.review(state, review::Msg::Query(text), requests);
        }
        (Field::ReviewFilter, Edit::Cancel) => {
            building.review(state, review::Msg::Query(String::new()), requests);
        }
        _ => {}
    }
}

fn spawn_search(effects: Vec<search::Effect>, requests: &Sender<Request>) {
    for effect in effects {
        let requests = requests.clone();
        thread::spawn(move || {
            let config = load_config().ok();
            let msg = search::run(effect, config.as_ref(), &AtomicBool::new(false));
            let _ = requests.send(Request::Search(msg));
        });
    }
}

fn search_msg(state: &mut Updates, msg: search::Msg, requests: &Sender<Request>) {
    if let Some(screen) = &mut state.search {
        spawn_search(screen.update(msg), requests);
    }
}

fn watchlist_msg(state: &mut Updates, msg: watchlist::Msg, requests: &Sender<Request>) {
    let Some(screen) = &mut state.watchlist else {
        return;
    };
    if let Some(effect) = screen.update(msg) {
        let requests = requests.clone();
        thread::spawn(move || {
            let msg = watchlist::run(effect, &AtomicBool::new(false));
            let _ = requests.send(Request::Watchlist(msg));
        });
    }
}

/// Runs what Inputs asks for on a thread of its own, its fetch progress and result coming back as
/// requests.
fn spawn_inputs(effect: inputs::Effect, directory: PathBuf, requests: Sender<Request>) {
    thread::spawn(move || {
        let progress = requests.clone();
        let msg = inputs::run(effect, &directory, &AtomicBool::new(false), move |fetched| {
            let _ = progress.send(Request::Inputs(inputs::Msg::Progress(fetched)));
        });
        let _ = requests.send(Request::Inputs(msg));
    });
}

fn open_inputs(state: &mut Updates, requests: &Sender<Request>) {
    let undo = state.inputs.as_ref().and_then(|screen| screen.undo.clone());
    let opened = match load_config() {
        Ok(config) => {
            let (screen, effect) =
                inputs::Inputs::open(config.flake_dir, config.home_flake.is_some(), undo);
            if let Some(effect) = effect {
                spawn_inputs(effect, screen.directory.clone(), requests.clone());
            }
            screen
        }
        Err(error) => {
            let (mut screen, _) = inputs::Inputs::open(PathBuf::new(), false, undo);
            screen.load_error = Some(error);
            screen
        }
    };
    state.inputs = Some(opened);
}

fn inputs_msg(state: &mut Updates, msg: inputs::Msg, requests: &Sender<Request>) {
    let Some(screen) = &mut state.inputs else {
        return;
    };
    let (effect, _) = screen.update(msg);
    if let Some(effect) = effect {
        spawn_inputs(effect, screen.directory.clone(), requests.clone());
    }
}

/// Hands `msg` to Home, and runs on a thread of its own whatever Home asks for, its result coming
/// back as another request.
fn home(state: &mut Updates, msg: Msg, requests: &Sender<Request>) {
    let Some(home) = &mut state.home else {
        return;
    };
    if let Some(effect) = home.update(msg) {
        spawn(effect, home.clone(), requests.clone());
    }
}

fn spawn(effect: flow::Effect, home: Home, requests: Sender<Request>) {
    thread::spawn(move || {
        let msg = flow::run(effect, &home, &AtomicBool::new(false));
        let _ = requests.send(Request::Home(msg));
    });
}

fn work(feed: &Feed<Updates>, incoming: &mpsc::Receiver<Request>, requests: &Sender<Request>) -> Ended {
    let mut cache = cache::load();
    let mut state = Updates {
        report: cache.report.clone(),
        checked_at: (cache.attempted_at > 0).then_some(cache.attempted_at),
        ..Updates::default()
    };
    if !feed.publish(state.clone()) {
        return Ended::Unwanted;
    }
    let mut cleaning = Cleaning {
        cancel: Arc::new(AtomicBool::new(false)),
    };
    let mut building = Building::default();
    let mut forced = false;
    loop {
        let fetch = forced || now().saturating_sub(cache.attempted_at) >= INTERVAL.as_secs();
        if fetch {
            state.checking = true;
            state.refreshing = forced;
            if !feed.publish(state.clone()) {
                return Ended::Unwanted;
            }
            cache.attempted_at = now();
            state.checked_at = Some(cache.attempted_at);
        }
        debug!(forced, fetch, "checking for updates");
        let checked = check(fetch);
        let changed = match checked {
            Ok(report) => {
                if fetch {
                    info!(incoming = report.incoming, "update check finished");
                }
                let changed = state.report.as_ref() != Some(&report) || state.error.is_some();
                cache.report = Some(report.clone());
                state.report = Some(report);
                state.error = None;
                changed
            }
            Err(error) => {
                warn!(fetch, "update check failed: {error}");
                let changed = state.error.as_ref() != Some(&error);
                state.error = Some(error);
                changed
            }
        };
        if fetch || changed {
            state.checking = false;
            state.refreshing = false;
            cache::save(&cache);
            if !feed.publish(state.clone()) {
                return Ended::Unwanted;
            }
        }
        let due = Instant::now() + TICK;
        forced = loop {
            match incoming.recv_timeout(due.saturating_duration_since(Instant::now())) {
                Ok(Request::Fetch) => break true,
                Ok(Request::Look) | Err(RecvTimeoutError::Timeout) => break false,
                Ok(Request::OpenHome) => {
                    let reviewed = state
                        .review
                        .iter()
                        .filter(|review| !switched(review))
                        .map(|review| review.report.target)
                        .collect();
                    let (opened, effect) = Home::load(reviewed);
                    if let Some(effect) = effect {
                        spawn(effect, opened.clone(), requests.clone());
                    }
                    state.home = Some(opened);
                }
                Ok(Request::OpenInputs) => open_inputs(&mut state, requests),
                Ok(Request::OpenWatchlist) => {
                    state.watchlist = Some(watchlist::Watchlist::open());
                }
                Ok(Request::Watchlist(msg)) => watchlist_msg(&mut state, msg, requests),
                Ok(Request::OpenSearch) => {
                    let (screen, effects) = search::Search::open(load_config().ok());
                    state.search = Some(screen);
                    spawn_search(effects, requests);
                }
                Ok(Request::Search(msg)) => search_msg(&mut state, msg, requests),
                Ok(Request::OpenCleanup) => cleaning.open(&mut state, requests),
                Ok(Request::Cleanup(msg)) => cleaning.update(&mut state, msg, requests),
                Ok(Request::StartBuild(target, after)) => {
                    building.start(&mut state, target, after, requests);
                }
                Ok(Request::Build(msg)) => building.build(&mut state, *msg, requests),
                Ok(Request::Review(msg)) => building.review(&mut state, msg, requests),
                Ok(Request::OpenNotes(index)) => building.open_notes(&mut state, index, requests),
                Ok(Request::Notes(msg)) => building.notes(&mut state, msg),
                Ok(Request::Seen) => {
                    if let Some(build) = &state.build
                        && !build.running()
                    {
                        state.seen_build = build.generation;
                    }
                    if state
                        .review
                        .as_ref()
                        .is_some_and(|review| !review.activating())
                    {
                        state.seen_activation = true;
                    }
                }
                Ok(Request::Typed(field, key)) => {
                    typed(&mut state, &cleaning, &mut building, field, &key, requests);
                }
                Ok(Request::Inputs(msg)) => inputs_msg(&mut state, msg, requests),
                Ok(Request::Home(msg)) => {
                    let pulled = matches!(msg, Msg::Pulled(Ok(_)));
                    home(&mut state, msg, requests);
                    if pulled {
                        if !feed.publish(state.clone()) {
                            return Ended::Unwanted;
                        }
                        break false;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return Ended::Unwanted,
            }
            if !feed.publish(state.clone()) {
                return Ended::Unwanted;
            }
        };
    }
}

pub struct Molt {
    requests: Sender<Request>,
}

impl Molt {
    pub(super) fn start(feed: Feed<Updates>) -> Self {
        let (requests, incoming) = mpsc::channel();
        let own = requests.clone();
        feed.supervise(move |feed| work(feed, &incoming, &own));
        Self { requests }
    }

    pub(super) fn refresh(&self) {
        let _ = self.requests.send(Request::Fetch);
    }

    pub(super) fn look(&self) {
        let _ = self.requests.send(Request::Look);
    }

    pub(super) fn open_home(&self) {
        let _ = self.requests.send(Request::OpenHome);
    }

    pub(super) fn home(&self, msg: Msg) {
        let _ = self.requests.send(Request::Home(msg));
    }

    pub(super) fn open_inputs(&self) {
        let _ = self.requests.send(Request::OpenInputs);
    }

    pub(super) fn inputs(&self, msg: inputs::Msg) {
        let _ = self.requests.send(Request::Inputs(msg));
    }

    pub(super) fn open_watchlist(&self) {
        let _ = self.requests.send(Request::OpenWatchlist);
    }

    pub(super) fn watchlist(&self, msg: watchlist::Msg) {
        let _ = self.requests.send(Request::Watchlist(msg));
    }

    pub(super) fn typed(&self, field: Field, key: &Key) {
        let _ = self.requests.send(Request::Typed(field, key.clone()));
    }

    pub(super) fn open_search(&self) {
        let _ = self.requests.send(Request::OpenSearch);
    }

    pub(super) fn search(&self, msg: search::Msg) {
        let _ = self.requests.send(Request::Search(msg));
    }

    pub(super) fn open_cleanup(&self) {
        let _ = self.requests.send(Request::OpenCleanup);
    }

    pub(super) fn cleanup(&self, msg: cleanup::Msg) {
        let _ = self.requests.send(Request::Cleanup(msg));
    }

    pub(super) fn start_build(&self, target: Target, after: AfterBuild) {
        let _ = self.requests.send(Request::StartBuild(target, after));
    }

    pub(super) fn build(&self, msg: build::Msg) {
        let _ = self.requests.send(Request::Build(Box::new(msg)));
    }

    pub(super) fn review(&self, msg: review::Msg) {
        let _ = self.requests.send(Request::Review(msg));
    }

    pub(super) fn open_notes(&self, index: usize) {
        let _ = self.requests.send(Request::OpenNotes(index));
    }

    pub(super) fn seen(&self) {
        let _ = self.requests.send(Request::Seen);
    }
}
