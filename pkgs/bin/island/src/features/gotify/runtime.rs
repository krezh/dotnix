use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        mpsc::{self, Sender},
    },
    thread,
    time::Instant,
};

use tracing::{debug, info, warn};
use ureq::Agent;

use super::{
    client,
    settings::Settings,
    state::{Endpoint, Entry, RawMessage, State},
};
use crate::{
    features::Feed,
    supervisor::{Ended, Reconnect},
};

const MAX_MESSAGES: usize = 50;

pub enum Request {
    Refresh,
    Delete(u64),
    Clear,
}

enum WorkerEvent {
    Request(Request),
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
        let worker_tx = sender.clone();
        feed.supervise(move |feed| run(feed, &settings, &worker_tx, &receiver));
    }
    Gotify { sender }
}

fn spawn_stream(
    generation: u64,
    endpoint: Endpoint,
    token: String,
    sender: Sender<WorkerEvent>,
) {
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
            if sender.send(WorkerEvent::Connected(generation, agent)).is_err() {
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

fn sync_messages(
    api: &Agent,
    endpoint: &Endpoint,
    token: &str,
    min_priority: i64,
    apps: &mut HashMap<u64, String>,
) -> Result<Vec<Entry>, String> {
    if let Ok(fetched_apps) = client::fetch_applications(api, endpoint, token) {
        *apps = fetched_apps;
    }
    let messages = client::fetch_messages(api, endpoint, token, apps)?;
    Ok(messages
        .into_iter()
        .filter(|entry| entry.priority >= min_priority)
        .collect())
}

fn run(
    feed: &Feed<State>,
    settings: &Settings,
    sender: &Sender<WorkerEvent>,
    receiver: &mpsc::Receiver<WorkerEvent>,
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

    let api = client::api_agent();
    let mut apps = HashMap::new();
    let mut state = State {
        server: Some(endpoint.display_url()),
        busy: true,
        ..State::default()
    };
    if !feed.publish(state.clone()) {
        return Ended::Unwanted;
    }

    match sync_messages(&api, &endpoint, &token, settings.min_priority, &mut apps) {
        Ok(messages) => state.messages = messages,
        Err(error) => state.error = Some(error),
    }
    state.busy = false;

    let mut generation: u64 = 1;
    let mut started = Instant::now();
    let pace: Arc<Mutex<Reconnect>> = Arc::default();
    let mut stream_agent: Option<Agent> = None;
    spawn_stream(
        generation,
        endpoint.clone(),
        token.clone(),
        sender.clone(),
    );
    if !feed.publish(state.clone()) {
        if let Some(agent) = stream_agent.take() {
            agent.shutdown();
        }
        return Ended::Unwanted;
    }

    while let Ok(event) = receiver.recv() {
        match event {
            WorkerEvent::Request(Request::Refresh) => {
                state.busy = true;
                if !feed.publish(state.clone()) {
                    return Ended::Unwanted;
                }
                let token = settings.read_token().unwrap_or_else(|_| token.clone());
                match sync_messages(&api, &endpoint, &token, settings.min_priority, &mut apps) {
                    Ok(messages) => {
                        state.messages = messages;
                        state.error = None;
                    }
                    Err(error) => state.error = Some(error),
                }
                state.busy = false;
                if !state.connected {
                    if let Some(agent) = stream_agent.take() {
                        agent.shutdown();
                    }
                    generation += 1;
                    started = Instant::now();
                    spawn_stream(
                        generation,
                        endpoint.clone(),
                        token,
                        sender.clone(),
                    );
                }
            }
            WorkerEvent::Request(Request::Delete(id)) => {
                match client::delete_message(&api, &endpoint, &token, id) {
                    Ok(()) => state.messages.retain(|msg| msg.id != id),
                    Err(error) => state.error = Some(error),
                }
            }
            WorkerEvent::Request(Request::Clear) => {
                match client::delete_all_messages(&api, &endpoint, &token) {
                    Ok(()) => state.messages.clear(),
                    Err(error) => state.error = Some(error),
                }
            }
            WorkerEvent::Connected(epoch, agent) if epoch == generation => {
                info!(server = endpoint.authority, "connected to Gotify stream");
                stream_agent = Some(agent);
                state.connected = true;
                state.error = None;
            }
            WorkerEvent::Arrived(epoch, raw) if epoch == generation => {
                if raw.priority < settings.min_priority {
                    debug!(
                        id = raw.id,
                        priority = raw.priority,
                        min_priority = settings.min_priority,
                        "ignoring Gotify message below minimum priority"
                    );
                    continue;
                }
                if raw.appid != 0
                    && !apps.contains_key(&raw.appid)
                    && let Ok(fetched) = client::fetch_applications(&api, &endpoint, &token)
                {
                    apps = fetched;
                }
                let entry = Entry::from_raw(raw, &apps);
                debug!(
                    id = entry.id,
                    app = entry.app,
                    priority = entry.priority,
                    "Gotify message arrived"
                );
                if let Err(error) = client::notify(&entry) {
                    warn!("cannot notify Gotify message: {error}");
                } else if settings.auto_delete {
                    if let Err(error) = client::delete_message(&api, &endpoint, &token, entry.id) {
                        warn!(id = entry.id, "{error}");
                    } else {
                        continue;
                    }
                }
                state.messages.retain(|existing| existing.id != entry.id);
                state.messages.insert(0, entry);
                state.messages.truncate(MAX_MESSAGES);
            }
            WorkerEvent::StreamEnded(epoch, reason) if epoch == generation => {
                warn!(cause = reason, "Gotify stream ended, reconnecting");
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
                let token = settings.read_token().unwrap_or_else(|_| token.clone());
                if let Ok(messages) =
                    sync_messages(&api, &endpoint, &token, settings.min_priority, &mut apps)
                {
                    state.messages = messages;
                }
                generation += 1;
                started = Instant::now();
                spawn_stream(
                    generation,
                    endpoint.clone(),
                    token,
                    sender.clone(),
                );
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
