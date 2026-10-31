use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
    },
    thread,
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
    launch::{self, Launch},
    supervisor::{Ended, Reconnect},
};

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
    Reconnect(u64),
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
        let forward = sender.clone();
        let _ = thread::Builder::new()
            .name("island-gotify-replies".to_owned())
            .spawn(move || {
                for reply in replies {
                    if forward.send(WorkerEvent::Reply(reply)).is_err() {
                        return;
                    }
                }
            });
        let worker_tx = sender.clone();
        feed.supervise(move |feed| run(feed, &settings, &worker_tx, &receiver));
    }
    Gotify { sender }
}

fn spawn_stream(generation: u64, endpoint: Endpoint, token: String, sender: Sender<WorkerEvent>) {
    let _ = thread::Builder::new()
        .name("island-gotify-ws".to_owned())
        .spawn(move || {
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

fn trouble(trouble: Option<(&str, String)>) {
    notice::post(Post::Trouble {
        feature: Gotify::NAME.to_owned(),
        trouble: trouble.map(|(title, detail)| Trouble {
            title: format!("{SOURCE} {title}"),
            detail,
        }),
    });
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
        for entry in entries
            .iter()
            .filter(|entry| !known.contains_key(&entry.id))
        {
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
            && let Ok(apps) = client::fetch_applications(&self.api, &self.endpoint, &self.token)
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
                    launch::spawn(Launch {
                        id: "xdg-open".to_owned(),
                        name: "Gotify link".to_owned(),
                        argv: vec!["xdg-open".to_owned(), url],
                        directory: None,
                    });
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

    let mut generation: u64 = 1;
    let mut started = Instant::now();
    let pace: Arc<Mutex<Reconnect>> = Arc::default();
    let mut stream_agent: Option<Agent> = None;
    spawn_stream(generation, endpoint.clone(), server.token(), sender.clone());
    if !feed.publish(state.clone()) {
        return Ended::Unwanted;
    }

    let every = settings.sync_interval();
    loop {
        let event = match every.map_or_else(
            || receiver.recv().map_err(|_| RecvTimeoutError::Disconnected),
            |every| receiver.recv_timeout(every),
        ) {
            Ok(event) => event,
            // Gotify's stream does not say when a message is deleted, so the server is asked
            // again now and then.
            Err(RecvTimeoutError::Timeout) => {
                let before = state.clone();
                server.sync(&mut state);
                if state != before && !feed.publish(state.clone()) {
                    return Ended::Unwanted;
                }
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => break,
        };
        match event {
            WorkerEvent::Request(Request::Refresh) => {
                server.sync(&mut state);
                if !state.connected {
                    if let Some(agent) = stream_agent.take() {
                        agent.shutdown();
                    }
                    generation += 1;
                    started = Instant::now();
                    spawn_stream(generation, endpoint.clone(), server.token(), sender.clone());
                }
            }
            WorkerEvent::Request(Request::Clear) => server.clear(&mut state),
            WorkerEvent::Reply(Reply::Retry) => {
                let _ = sender.send(WorkerEvent::Request(Request::Refresh));
                continue;
            }
            WorkerEvent::Reply(reply) => server.replied(reply, &mut state),
            WorkerEvent::Connected(epoch, agent) if epoch == generation => {
                info!(server = endpoint.authority, "connected to Gotify stream");
                stream_agent = Some(agent);
                state.connected = true;
                state.error = None;
                trouble(None);
            }
            WorkerEvent::Arrived(epoch, raw) if epoch == generation => {
                server.arrived(raw, &mut state);
            }
            WorkerEvent::StreamEnded(epoch, reason) if epoch == generation => {
                warn!(cause = reason, "Gotify stream ended, reconnecting");
                trouble(Some((
                    "disconnected",
                    format!("{} · reconnecting", endpoint.authority),
                )));
                state.connected = false;
                state.error = Some(reason);
                stream_agent = None;
                let attempt_started = started;
                let pace = pace.clone();
                let tx = sender.clone();
                let _ = thread::spawn(move || {
                    if let Ok(mut pace) = pace.lock() {
                        pace.wait(attempt_started);
                    }
                    let _ = tx.send(WorkerEvent::Reconnect(epoch));
                });
            }
            WorkerEvent::Reconnect(epoch) if epoch == generation => {
                server.sync(&mut state);
                generation += 1;
                started = Instant::now();
                spawn_stream(generation, endpoint.clone(), server.token(), sender.clone());
            }
            _ => continue,
        }
        if !feed.publish(state.clone()) {
            if let Some(agent) = stream_agent.take() {
                agent.shutdown();
            }
            return Ended::Unwanted;
        }
    }
    Ended::Unwanted
}
