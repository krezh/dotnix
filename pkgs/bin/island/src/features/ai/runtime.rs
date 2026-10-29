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
    alerts::Alerts,
    cache,
    conversation::{Chat, Conversation, MODELS, model_index},
    converse::Streamed,
    format, history,
    hook::{self, Hook, server::Call},
    model::{self, Notice, Tracker},
    notify,
    requests::{self, Waiting},
    session::{Answer, Event},
    settings::Settings,
    sources::{self, Failure},
    state::{Provider, Source, SourceReport, State},
};
use crate::{features::Feed, supervisor::Ended};

/// Claude's usage comes from Claude Code's status line while it runs; the usage API is asked at
/// the start, and then only once nothing has been heard of Claude's usage for this long.
const CLAUDE_QUIET: Duration = Duration::from_secs(30 * 60);
/// How long to leave the usage API alone after it asks for that without saying how long: this
/// at first, doubling each time it asks again, up to `BACKOFF_MOST`.
const BACKOFF_FIRST: Duration = Duration::from_secs(5 * 60);
const BACKOFF_MOST: Duration = Duration::from_secs(30 * 60);
/// The longest a wait it names is kept to.
const WAIT_MOST: Duration = Duration::from_secs(60 * 60);
/// A session quiet this long without saying it ended, such as one whose terminal was closed, is
/// forgotten.
const STALE_MS: i64 = 6 * 60 * 60 * 1000;

enum Message {
    Refresh,
    Opened,
    Snapshot(Option<String>, Option<Provider>),
    Read(Source, Result<SourceReport, Failure>),
    Call(Call),
    Answer(u64, Answer),
    /// The pointer came over an island, by its view's number, or left it.
    Hold(u64, bool),
    Chat(ChatMessage),
}

enum ChatMessage {
    Send(String),
    Retry,
    New,
    Model(usize),
    Streamed(u64, Streamed),
}

pub struct Ai {
    pub(super) settings: Arc<Settings>,
    requests: Sender<Message>,
    pending: Arc<AtomicBool>,
    /// Closing it stops the hook server.
    _hooks: Option<Sender<()>>,
}

impl Ai {
    pub(super) fn refresh(&self) {
        if !self.pending.swap(true, Ordering::Relaxed) {
            let _ = self.requests.send(Message::Refresh);
        }
    }

    pub(super) fn opened(&self) {
        let _ = self.requests.send(Message::Opened);
    }

    pub(super) fn answer(&self, id: u64, answer: Answer) {
        let _ = self.requests.send(Message::Answer(id, answer));
    }

    /// Holds what waits on you while the pointer is over `island`, until the result is dropped.
    pub(super) fn hold(&self, island: u64) -> Held {
        let _ = self.requests.send(Message::Hold(island, true));
        Held {
            island,
            requests: self.requests.clone(),
        }
    }

    pub(super) fn chat_send(&self, text: String) {
        let _ = self.requests.send(Message::Chat(ChatMessage::Send(text)));
    }

    pub(super) fn chat_retry(&self) {
        let _ = self.requests.send(Message::Chat(ChatMessage::Retry));
    }

    pub(super) fn chat_new(&self) {
        let _ = self.requests.send(Message::Chat(ChatMessage::New));
    }

    pub(super) fn chat_model(&self, model: usize) {
        let _ = self.requests.send(Message::Chat(ChatMessage::Model(model)));
    }
}

/// One island's hold on what waits on you. Dropping it lets go, also when the island goes while
/// the pointer is still over it, as when its monitor is unplugged.
pub(super) struct Held {
    island: u64,
    requests: Sender<Message>,
}

impl Drop for Held {
    fn drop(&mut self) {
        let _ = self.requests.send(Message::Hold(self.island, false));
    }
}

/// A chat left alone this long is cleared, as one closed and walked away from.
const CHAT_IDLE: Duration = Duration::from_secs(10 * 60);

fn chat(
    state: &mut State,
    conversation: &mut Conversation,
    settings: &Settings,
    sender: &Sender<Message>,
    message: ChatMessage,
) -> bool {
    let chat = Arc::make_mut(&mut state.chat);
    match message {
        ChatMessage::Send(text) => {
            let replies = sender.clone();
            conversation.send(chat, &settings.chat.command, text, move |run, streamed| {
                let _ = replies.send(Message::Chat(ChatMessage::Streamed(run, streamed)));
            });
            true
        }
        ChatMessage::Retry => {
            let replies = sender.clone();
            conversation.retry(chat, &settings.chat.command, move |run, streamed| {
                let _ = replies.send(Message::Chat(ChatMessage::Streamed(run, streamed)));
            });
            true
        }
        ChatMessage::New => {
            conversation.clear(chat);
            true
        }
        ChatMessage::Model(model) => {
            chat.model = model.min(MODELS.len() - 1);
            true
        }
        ChatMessage::Streamed(run, streamed) => conversation.streamed(chat, run, streamed),
    }
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn begin(state: &mut State, sources: &[Source], sender: &Sender<Message>) -> usize {
    state.attempted_at = Some(now());
    state.refreshing = !sources.is_empty();
    for &source in sources {
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
                Err(format!("cannot start usage collection: {error}").into()),
            ));
        }
    }
    sources.len()
}

/// Keeps what a new reading shows of each limit's window, and sends what it alerts to.
fn settle(state: &mut State, notices: Vec<Notice>) {
    for window in history::record(state) {
        if let Err(error) = cache::keep(&window) {
            warn!("{error}");
        }
    }
    announce(notices);
}

fn announce(notices: Vec<Notice>) {
    notify::send(notices);
}

/// True when it changed what is waiting on you.
fn call(
    state: &mut State,
    waiting: &mut Waiting,
    alerts: &mut Alerts,
    settings: &Settings,
    call: Call,
) -> bool {
    let at = now();
    match call {
        Call::Tell(hook, told_at) => {
            announce(alerts.tell(state, &hook, at).into_iter().collect());
            let settled = requests::settle(state, waiting, &hook, told_at);
            hook::apply(Arc::make_mut(&mut state.sessions), &hook, at);
            settled
        }
        Call::Ask(hook, asked_at, reply) => {
            alerts.asked(&hook.from().session);
            let wait_ms =
                i64::try_from(settings.sessions.answer_seconds * 1000).unwrap_or(i64::MAX);
            waiting.ask(state, &hook, reply, asked_at.min(at), wait_ms);
            true
        }
    }
}

fn work(
    feed: &Feed<State, Event>,
    settings: &Settings,
    requests: &Receiver<Message>,
    sender: &Sender<Message>,
    pending: &AtomicBool,
) -> Ended {
    let interval = Duration::from_secs(settings.refresh_minutes * 60);
    let started = now();
    let windows = cache::windows(started).unwrap_or_else(|error| {
        warn!("{error}");
        Vec::new()
    });
    let mut state = State {
        mascot: settings.mascot,
        windows: Arc::new(windows),
        chat: Arc::new(Chat {
            model: model_index(&settings.chat.model),
            ..Chat::default()
        }),
        ..State::default()
    };
    let mut tracker = Tracker::default();
    let mut waiting = Waiting::default();
    let mut alerts = Alerts::default();
    let mut conversation = Conversation::default();
    let mut chatted = Instant::now();
    let mut in_flight = begin(&mut state, &settings.sources, sender);
    let mut next_poll = Instant::now() + interval;
    let mut queued = false;
    // Refresh asked for by hand, which asks the usage API whenever it allows.
    let mut by_hand = false;
    let mut fetched = Instant::now();
    // When Claude's usage was last heard of, from the usage API or the status line.
    let mut claude_heard = fetched;
    // While the usage API asked to be left alone.
    let mut claude_wait: Option<Instant> = None;
    // The latest passed reset the usage API was asked about, so each reset asks it only once.
    let mut claude_reset: Option<i64> = None;
    let mut backoff = BACKOFF_FIRST;
    if !feed.publish(state.clone()) {
        return Ended::Unwanted;
    }
    loop {
        let stale = state
            .sessions
            .iter()
            .map(|session| session.active_at + STALE_MS)
            .min();
        let due = waiting
            .next_due(&state)
            .into_iter()
            .chain(alerts.next_due())
            .chain(stale)
            .min()
            .map(|at| {
                Instant::now() + Duration::from_millis(u64::try_from(at - now()).unwrap_or(0))
            });
        let idle_chat =
            (!state.chat.busy && !state.chat.lines.is_empty()).then_some(chatted + CHAT_IDLE);
        let timeout = due
            .into_iter()
            .chain(idle_chat)
            .chain([next_poll])
            .min()
            .unwrap_or(next_poll)
            .saturating_duration_since(Instant::now());
        let mut changed = false;
        let mut asked = false;
        match requests.recv_timeout(timeout) {
            Ok(Message::Refresh) => {
                pending.store(false, Ordering::Relaxed);
                queued = true;
                by_hand = true;
            }
            Ok(Message::Opened) => {
                if fetched.elapsed() >= interval {
                    queued = true;
                }
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
                    Err(failure) => {
                        let repeated = source == Source::Claude
                            && state.issues.iter().any(|issue| {
                                issue.source == source && issue.message == failure.message
                            });
                        if !repeated {
                            warn!(
                                source = source.name(),
                                "AI usage refresh failed: {}", failure.message
                            );
                        }
                    }
                }
                // Asked to slow down: nothing asks again until the wait is over.
                let slowed = match &result {
                    Err(Failure {
                        wait: Some(asked), ..
                    }) => {
                        let wait = asked.unwrap_or(backoff).min(WAIT_MOST);
                        backoff = (backoff * 2).min(BACKOFF_MOST);
                        claude_wait = Some(Instant::now() + wait);
                        Some(wait)
                    }
                    Ok(_) if source == Source::Claude => {
                        backoff = BACKOFF_FIRST;
                        claude_wait = None;
                        None
                    }
                    _ => None,
                };
                let notices = tracker.apply(
                    &mut state,
                    source,
                    result.map_err(|failure| failure.message),
                    settings,
                );
                if let Some(wait) = slowed {
                    let at = now();
                    let again = at + i64::try_from(wait.as_millis()).unwrap_or(i64::MAX / 2);
                    if let Some(issue) = Arc::make_mut(&mut state.issues)
                        .iter_mut()
                        .find(|issue| issue.source == source)
                    {
                        issue.message = format!(
                            "The usage API asked island to slow down; it asks again in {}",
                            format::countdown(again, at)
                        );
                    }
                }
                settle(&mut state, notices);
                in_flight = in_flight.saturating_sub(1);
                state.refreshing = in_flight > 0;
                changed = true;
            }
            Ok(Message::Call(Call::Tell(Hook::Usage { readings, .. }, _))) => {
                if let Some(report) = model::status_line(&state, &readings, now()) {
                    claude_heard = Instant::now();
                    let notices = tracker.apply(&mut state, Source::Claude, Ok(report), settings);
                    settle(&mut state, notices);
                    changed = true;
                }
            }
            Ok(Message::Call(hook_call)) => {
                asked = call(&mut state, &mut waiting, &mut alerts, settings, hook_call);
                changed = true;
            }
            Ok(Message::Answer(id, answer)) => {
                asked = waiting.answer(&mut state, id, &answer);
                changed = asked;
            }
            Ok(Message::Hold(island, over)) => {
                changed = waiting.hold(&mut state, island, over, now());
            }
            Ok(Message::Chat(message)) => {
                chatted = Instant::now();
                changed = chat(&mut state, &mut conversation, settings, sender, message);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ended::Unwanted,
        }
        let at = now();
        if waiting.expire(&mut state, at) {
            asked = true;
            changed = true;
        }
        announce(alerts.due(&state, at));
        let instant = Instant::now();
        let claude_allowed = claude_wait.is_none_or(|until| instant >= until);
        // A reset that has passed leaves the last reading out of date, and the status line
        // repeats it until Claude Code asks again, so the usage API is asked for the new window.
        let reset = model::passed_reset(&state, at)
            .filter(|reset| claude_reset.is_none_or(|asked| *reset > asked));
        if reset.is_some() && claude_allowed {
            queued = true;
        }
        if in_flight == 0 && (queued || instant >= next_poll) {
            // Claude's usage API only when it allows, and when the status line has gone quiet,
            // a reset has passed or a refresh was asked for by hand.
            let claude = claude_allowed
                && (by_hand
                    || reset.is_some()
                    || instant.duration_since(claude_heard) >= CLAUDE_QUIET);
            let asking: Vec<Source> = settings
                .sources
                .iter()
                .copied()
                .filter(|source| *source != Source::Claude || claude)
                .collect();
            if asking.contains(&Source::Claude) {
                claude_heard = instant;
                claude_reset = reset.or(claude_reset);
            }
            in_flight = begin(&mut state, &asking, sender);
            next_poll = instant + interval;
            queued = false;
            by_hand = false;
            fetched = instant;
            changed = true;
        } else if in_flight > 0 && instant >= next_poll {
            next_poll = instant + interval;
        }
        if state
            .sessions
            .iter()
            .any(|session| at - session.active_at >= STALE_MS)
        {
            Arc::make_mut(&mut state.sessions).retain(|session| at - session.active_at < STALE_MS);
            changed = true;
        }
        if !state.chat.busy && !state.chat.lines.is_empty() && instant >= chatted + CHAT_IDLE {
            conversation.clear(Arc::make_mut(&mut state.chat));
            changed = true;
        }
        if changed && !feed.publish(state.clone()) {
            return Ended::Unwanted;
        }
        if asked {
            feed.send(Event::Requests(Waiting::ids(&state)));
        }
    }
}

/// Forwards hook calls from the bus to the worker.
fn relay(sender: Sender<Message>) -> Sender<Call> {
    let (calls, incoming) = mpsc::channel::<Call>();
    let _ = thread::Builder::new()
        .name("ai-hooks-relay".to_owned())
        .spawn(move || {
            for call in incoming {
                if sender.send(Message::Call(call)).is_err() {
                    break;
                }
            }
        });
    calls
}

pub(super) fn start(feed: Feed<State, Event>) -> Ai {
    let settings = Arc::new(feed.settings::<Settings>());
    let (requests, incoming) = mpsc::channel();
    let pending = Arc::new(AtomicBool::new(false));
    let hooks = settings.sessions.enable.then(|| {
        let (stop, stopped) = mpsc::channel::<()>();
        let calls = relay(requests.clone());
        feed.clone()
            .supervise(move |_| hook::server::serve(calls.clone(), &stopped));
        stop
    });
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
        _hooks: hooks,
    }
}
