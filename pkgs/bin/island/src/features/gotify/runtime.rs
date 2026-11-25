use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    time::Instant,
};

use tracing::{debug, info, warn};
use ureq::Agent;

use super::{
    client,
    settings::Settings,
    state::{DEFAULT_ACTION, Endpoint, Entry, OPEN_ACTION, RawMessage, SOURCE, State},
};
use crate::{
    features::{
        Feature, Feed,
        notice::{self, Post, Reply, Trouble},
    },
    launch,
    supervisor::{self, Ended, Reconnect},
};

const MOST_POSTED: usize = 50;

pub enum Request {
    Refresh,
    Clear,
}

enum WorkerEvent {
    Request(Request),
    Reply(Reply),
    Connected(u64, Agent),
    Arrived(u64, RawMessage),
    StreamEnded(u64, String),
}

#[derive(Clone)]
pub struct Gotify {
    sender: Sender<WorkerEvent>,
}

impl Gotify {
    pub(super) fn request(&self, request: Request) {
        let _ = self.sender.send(WorkerEvent::Request(request));
    }
}

pub(super) fn start(feed: Feed<State>) -> Gotify {
    let settings: Settings = feed.settings();
    let (sender, receiver) = mpsc::channel();
    if settings.is_enabled() {
        let replies = notice::listen(Gotify::NAME);
        supervisor::forward(
            "island-gotify-replies",
            replies,
            sender.clone(),
            WorkerEvent::Reply,
        );
        let worker_tx = sender.clone();
        feed.supervise(move |feed| run(feed, &settings, &worker_tx, &receiver));
    }
    Gotify { sender }
}

fn spawn_stream(generation: u64, endpoint: Endpoint, token: String, sender: Sender<WorkerEvent>) {
    supervisor::spawn("island-gotify-ws", move || {
        let (agent, mut stream) = match client::connect_stream(&endpoint, &token) {
            Ok(pair) => pair,
            Err(error) => {
                let _ = sender.send(WorkerEvent::StreamEnded(generation, error));
                return;
            }
        };
        if sender
            .send(WorkerEvent::Connected(generation, agent))
            .is_err()
        {
            return;
        }
        loop {
            match client::read_message(&mut stream) {
                Ok(raw) => {
                    if sender.send(WorkerEvent::Arrived(generation, raw)).is_err() {
                        return;
                    }
                }
                Err(error) => {
                    let _ = sender.send(WorkerEvent::StreamEnded(generation, error));
                    return;
                }
            }
        }
    });
}

/// The live stream, by generation, so events from one already replaced are ignored.
struct Stream {
    generation: u64,
    started: Instant,
    /// A stream's read cannot be interrupted, so a second is never started beside one connecting.
    connecting: bool,
    agent: Option<Agent>,
}

impl Default for Stream {
    fn default() -> Self {
        Self {
            generation: 0,
            started: Instant::now(),
            connecting: false,
            agent: None,
        }
    }
}

impl Stream {
    fn start(&mut self, endpoint: &Endpoint, token: String, sender: &Sender<WorkerEvent>) {
        self.generation += 1;
        self.started = Instant::now();
        self.connecting = true;
        spawn_stream(self.generation, endpoint.clone(), token, sender.clone());
    }

    fn stop(&mut self) {
        if let Some(agent) = self.agent.take() {
            agent.shutdown();
        }
    }
}

fn trouble(trouble: Option<(&str, String)>) {
    notice::post(Post::Trouble {
        feature: Gotify::NAME.to_owned(),
        trouble: trouble.map(|(title, detail)| Trouble {
            title: format!("{SOURCE} {title}"),
            detail,
        }),
    });
}

/// The server lists its newest first.
fn newest_unknown<'a>(
    entries: &'a [Entry],
    known: &HashMap<u64, Option<String>>,
) -> impl Iterator<Item = &'a Entry> {
    entries
        .iter()
        .filter(|entry| !known.contains_key(&entry.id))
        .take(MOST_POSTED)
}

/// What the worker knows of the server.
struct Server {
    api: Agent,
    endpoint: Endpoint,
    settings: Settings,
    token: String,
    apps: HashMap<u64, String>,
    /// Each message on the server, by id, with where clicking it goes.
    messages: HashMap<u64, Option<String>>,
}

impl Server {
    fn token(&self) -> String {
        self.settings
            .read_token()
            .unwrap_or_else(|_| self.token.clone())
    }

    fn fetch(&self, token: &str) -> Result<Vec<Entry>, String> {
        client::fetch_messages(&self.api, &self.endpoint, token, &self.apps)
    }

    /// Those on the server wait with what was missed; those gone from it leave island too. Only
    /// what is new is posted, so a check that finds nothing changed changes nothing.
    fn sync(&mut self, state: &mut State) {
        let token = self.token();
        let mut entries = self.fetch(&token);
        let unnamed = |entries: &[Entry], apps: &HashMap<u64, String>| {
            entries
                .iter()
                .any(|entry| entry.app_id != 0 && !apps.contains_key(&entry.app_id))
        };
        if let Ok(fetched) = &entries
            && (self.apps.is_empty() || unnamed(fetched, &self.apps))
            && let Ok(apps) = client::fetch_applications(&self.api, &self.endpoint, &token)
        {
            self.apps = apps;
            entries = self.fetch(&token);
        }
        let entries = match entries {
            Ok(entries) => entries,
            Err(error) => {
                state.error = Some(error.clone());
                trouble(Some(("unreachable", error)));
                return;
            }
        };
        let entries: Vec<Entry> = entries
            .into_iter()
            .filter(|entry| entry.priority >= self.settings.min_priority)
            .collect();
        if state.error.take().is_some() && state.connected {
            trouble(None);
        }
        let known = std::mem::replace(
            &mut self.messages,
            entries
                .iter()
                .map(|entry| (entry.id, entry.url.clone()))
                .collect(),
        );
        state.messages = self.messages.len();
        let gone = known.keys().any(|id| !self.messages.contains_key(id));
        for entry in newest_unknown(&entries, &known) {
            notice::post(Post::Show(entry.notice(true)));
        }
        if gone {
            notice::post(Post::Keep {
                feature: Gotify::NAME.to_owned(),
                keys: entries.iter().map(Entry::key).collect(),
            });
        }
    }

    fn arrived(&mut self, raw: RawMessage, state: &mut State) {
        if raw.priority < self.settings.min_priority {
            debug!(
                id = raw.id,
                priority = raw.priority,
                min_priority = self.settings.min_priority,
                "ignoring Gotify message below minimum priority"
            );
            return;
        }
        if raw.appid != 0
            && !self.apps.contains_key(&raw.appid)
            && let Ok(apps) = client::fetch_applications(&self.api, &self.endpoint, &self.token())
        {
            self.apps = apps;
        }
        let entry = Entry::from_raw(raw, &self.apps);
        debug!(
            id = entry.id,
            app = entry.app,
            priority = entry.priority,
            "Gotify message arrived"
        );
        notice::post(Post::Show(entry.notice(false)));
        if self.settings.auto_delete {
            if let Err(error) =
                client::delete_message(&self.api, &self.endpoint, &self.token(), entry.id)
            {
                warn!(id = entry.id, "{error}");
            }
            return;
        }
        self.messages.insert(entry.id, entry.url);
        state.messages = self.messages.len();
    }

    fn replied(&mut self, reply: Reply, state: &mut State) {
        match reply {
            Reply::Dismissed(key) => {
                let Ok(id) = key.parse::<u64>() else {
                    return;
                };
                if self.messages.remove(&id).is_none() {
                    return;
                }
                state.messages = self.messages.len();
                if let Err(error) =
                    client::delete_message(&self.api, &self.endpoint, &self.token(), id)
                {
                    warn!(id, "{error}");
                }
            }
            Reply::Action(key, action) if action == DEFAULT_ACTION || action == OPEN_ACTION => {
                let url = key
                    .parse::<u64>()
                    .ok()
                    .and_then(|id| self.messages.get(&id).cloned().flatten());
                if let Some(url) = url {
                    launch::open(&url, "Gotify link");
                }
            }
            Reply::Action(..) | Reply::Retry => {}
            Reply::Looked => self.sync(state),
        }
    }

    fn clear(&mut self, state: &mut State) {
        match client::delete_all_messages(&self.api, &self.endpoint, &self.token()) {
            Ok(()) => {
                self.messages.clear();
                state.messages = 0;
                notice::post(Post::Keep {
                    feature: Gotify::NAME.to_owned(),
                    keys: Vec::new(),
                });
            }
            Err(error) => warn!("{error}"),
        }
    }
}

fn run(
    feed: &Feed<State>,
    settings: &Settings,
    sender: &Sender<WorkerEvent>,
    receiver: &Receiver<WorkerEvent>,
) -> Ended {
    let endpoint = match Endpoint::parse(&settings.url) {
        Ok(endpoint) => endpoint,
        Err(error) => {
            warn!(cause = error, "Gotify is misconfigured");
            return Ended::Unwanted;
        }
    };
    let token = match settings.read_token() {
        Ok(token) => token,
        Err(error) => return Ended::Failed(error),
    };

    let mut server = Server {
        api: client::api_agent(),
        endpoint: endpoint.clone(),
        settings: settings.clone(),
        token,
        apps: HashMap::new(),
        messages: HashMap::new(),
    };
    let mut state = State {
        server: Some(endpoint.display_url()),
        ..State::default()
    };
    server.sync(&mut state);

    let mut pace = Reconnect::default();
    let mut stream = Stream::default();
    stream.start(&endpoint, server.token(), sender);
    if !feed.publish(state.clone()) {
        return Ended::Unwanted;
    }

    let every = settings.sync_interval();
    // Deadlines, so a steady stream of events cannot keep pushing either back.
    let mut next_sync = every.map(|every| Instant::now() + every);
    let mut reconnect_at: Option<Instant> = None;
    loop {
        let deadline = [next_sync, reconnect_at].into_iter().flatten().min();
        let event = match deadline.map_or_else(
            || receiver.recv().map_err(|_| RecvTimeoutError::Disconnected),
            |at| receiver.recv_timeout(at.saturating_duration_since(Instant::now())),
        ) {
            Ok(event) => event,
            Err(RecvTimeoutError::Timeout) => {
                let now = Instant::now();
                let before = state.clone();
                if reconnect_at.is_some_and(|at| at <= now) {
                    reconnect_at = None;
                    server.sync(&mut state);
                    stream.start(&endpoint, server.token(), sender);
                }
                // Gotify's stream does not say when a message is deleted, so the server is asked
                // again now and then.
                if next_sync.is_some_and(|at| at <= now) {
                    next_sync = every.map(|every| now + every);
                    server.sync(&mut state);
                }
                if state != before && !feed.publish(state.clone()) {
                    stream.stop();
                    return Ended::Unwanted;
                }
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => break,
        };
        match event {
            WorkerEvent::Request(Request::Refresh) => {
                server.sync(&mut state);
                if !state.connected && !stream.connecting {
                    reconnect_at = None;
                    stream.stop();
                    stream.start(&endpoint, server.token(), sender);
                }
            }
            WorkerEvent::Request(Request::Clear) => server.clear(&mut state),
            WorkerEvent::Reply(Reply::Retry) => {
                let _ = sender.send(WorkerEvent::Request(Request::Refresh));
                continue;
            }
            WorkerEvent::Reply(reply) => server.replied(reply, &mut state),
            WorkerEvent::Connected(epoch, agent) if epoch == stream.generation => {
                info!(server = endpoint.authority, "connected to Gotify stream");
                stream.connecting = false;
                stream.agent = Some(agent);
                state.connected = true;
                state.error = None;
                trouble(None);
            }
            WorkerEvent::Arrived(epoch, raw) if epoch == stream.generation => {
                server.arrived(raw, &mut state);
            }
            WorkerEvent::StreamEnded(epoch, reason) if epoch == stream.generation => {
                warn!(cause = reason, "Gotify stream ended, reconnecting");
                trouble(Some((
                    "disconnected",
                    format!("{} · reconnecting", endpoint.authority),
                )));
                stream.connecting = false;
                stream.agent = None;
                state.connected = false;
                state.error = Some(reason);
                reconnect_at = Some(Instant::now() + pace.delay(stream.started));
            }
            _ => continue,
        }
        if !feed.publish(state.clone()) {
            stream.stop();
            return Ended::Unwanted;
        }
    }
    Ended::Unwanted
}

#[cfg(test)]
#[path = "runtime_test.rs"]
mod tests;
