use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
    },
    thread,
    time::{Duration, Instant},
};

use tracing::{debug, warn};

use super::{
    model::Tracker,
    notify,
    settings::Settings,
    sources,
    state::{Provider, Source, SourceReport, State},
};
use crate::{features::Feed, supervisor::Ended};

const TICK: Duration = Duration::from_secs(5);

enum Message {
    Refresh,
    Snapshot(Option<String>, Option<Provider>),
    Read(Source, Result<SourceReport, String>),
}

pub struct Ai {
    pub(super) settings: Arc<Settings>,
    requests: Sender<Message>,
    pending: Arc<AtomicBool>,
}

impl Ai {
    pub(super) fn refresh(&self) {
        if !self.pending.swap(true, Ordering::Relaxed) {
            let _ = self.requests.send(Message::Refresh);
        }
    }
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn begin(state: &mut State, settings: &Settings, sender: &Sender<Message>) -> usize {
    state.attempted_at = Some(now());
    state.refreshing = !settings.sources.is_empty();
    for &source in &settings.sources {
        let replies = sender.clone();
        if let Err(error) = thread::Builder::new()
            .name(format!("ai-{}", source.name()))
            .spawn(move || {
                let result = sources::fetch(source, |identity, snapshot| {
                    let _ = replies.send(Message::Snapshot(identity, snapshot));
                });
                let _ = replies.send(Message::Read(source, result));
            })
        {
            let _ = sender.send(Message::Read(
                source,
                Err(format!("cannot start usage collection: {error}")),
            ));
        }
    }
    settings.sources.len()
}

fn work(
    feed: &Feed<State>,
    settings: &Settings,
    requests: &Receiver<Message>,
    sender: &Sender<Message>,
    pending: &AtomicBool,
) -> Ended {
    let interval = Duration::from_secs(settings.refresh_minutes * 60);
    let mut state = State {
        now: now(),
        ..State::default()
    };
    let mut tracker = Tracker::default();
    let mut in_flight = begin(&mut state, settings, sender);
    let mut next_poll = Instant::now() + interval;
    let mut next_tick = Instant::now() + TICK;
    let mut queued = false;
    if !feed.publish(state.clone()) {
        return Ended::Unwanted;
    }
    loop {
        let timeout = next_tick
            .min(next_poll)
            .saturating_duration_since(Instant::now());
        let mut changed = false;
        match requests.recv_timeout(timeout) {
            Ok(Message::Refresh) => {
                pending.store(false, Ordering::Relaxed);
                queued = true;
            }
            Ok(Message::Snapshot(identity, snapshot)) => {
                tracker.restore_claude(&mut state, identity.as_deref(), snapshot, settings);
                changed = true;
            }
            Ok(Message::Read(source, result)) => {
                match &result {
                    Ok(report) => debug!(
                        source = source.name(),
                        providers = report.providers.len(),
                        issues = report.issues.len(),
                        "AI usage refreshed"
                    ),
                    Err(error) => {
                        let repeated = source == Source::Claude
                            && state.issues.iter().any(|issue| {
                                issue.source == source && issue.message == *error
                            });
                        if !repeated {
                            warn!(source = source.name(), "AI usage refresh failed: {error}");
                        }
                    }
                }
                let notices = tracker.apply(&mut state, source, result, settings);
                in_flight = in_flight.saturating_sub(1);
                state.refreshing = in_flight > 0;
                changed = true;
                if !notices.is_empty() {
                    let _ = thread::Builder::new()
                        .name("ai-notifications".to_owned())
                        .spawn(move || notify::send(notices));
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ended::Unwanted,
        }
        let instant = Instant::now();
        if in_flight == 0 && (queued || instant >= next_poll) {
            in_flight = begin(&mut state, settings, sender);
            next_poll = instant + interval;
            queued = false;
            changed = true;
        } else if in_flight > 0 && instant >= next_poll {
            next_poll = instant + interval;
        }
        if instant >= next_tick {
            state.now = now();
            next_tick = instant + TICK;
            changed = true;
        }
        if changed && !feed.publish(state.clone()) {
            return Ended::Unwanted;
        }
    }
}

pub(super) fn start(feed: Feed<State>) -> Ai {
    let settings = Arc::new(feed.settings::<Settings>());
    let (requests, incoming) = mpsc::channel();
    let pending = Arc::new(AtomicBool::new(false));
    let worker_sender = requests.clone();
    let worker_settings = settings.clone();
    let worker_pending = pending.clone();
    feed.supervise(move |feed| {
        work(
            feed,
            &worker_settings,
            &incoming,
            &worker_sender,
            &worker_pending,
        )
    });
    Ai {
        settings,
        requests,
        pending,
    }
}

pub(super) fn check(args: &[&str]) -> Result<(), String> {
    match args {
        ["ai", "refresh"] => Ok(()),
        _ => Err("expected ai refresh".to_owned()),
    }
}
