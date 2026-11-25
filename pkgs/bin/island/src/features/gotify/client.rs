use std::{
    collections::HashMap,
    fmt,
    io::{Read, Write},
    sync::{Arc, Mutex},
    time::Duration,
};

use super::state::{Endpoint, Entry, PagedMessages, RawApp, RawMessage};
use crate::sync;
use serde::de::DeserializeOwned;
use tungstenite::{
    HandshakeError, Message, WebSocket, client::IntoClientRequest, http::HeaderValue,
    protocol::WebSocketConfig,
};
use ureq::{
    Agent, Timeout,
    config::Config,
    unversioned::{
        resolver::DefaultResolver,
        transport::{
            ConnectionDetails, Connector, DefaultConnector, NextTimeout, Transport,
            TransportAdapter,
        },
    },
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const READ_TIMEOUT: Duration = Duration::from_secs(90);
const WRITE_TIMEOUT: Duration = Duration::from_secs(15);
const API_TIMEOUT: Duration = Duration::from_secs(10);
/// Far more than any one Gotify message.
const MAX_MESSAGE: usize = 1024 * 1024;
const MESSAGE_LIMIT: usize = 200;
const MOST_PAGES: usize = 50;

pub fn api_agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(API_TIMEOUT))
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .build()
        .into()
}

fn get_json<T: DeserializeOwned>(agent: &Agent, url: &str, token: &str) -> Result<T, String> {
    let mut response = agent
        .get(url)
        .header("X-Gotify-Key", token)
        .call()
        .map_err(|error| format!("request failed: {error}"))?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("cannot read response body: {error}"))?;
    serde_json::from_str(&body).map_err(|error| format!("invalid Gotify response: {error}"))
}

pub fn fetch_applications(
    agent: &Agent,
    endpoint: &Endpoint,
    token: &str,
) -> Result<HashMap<u64, String>, String> {
    let apps: Vec<RawApp> = get_json(agent, &endpoint.applications_url(), token)?;
    Ok(apps.into_iter().map(|app| (app.id, app.name)).collect())
}

pub fn fetch_messages(
    agent: &Agent,
    endpoint: &Endpoint,
    token: &str,
    apps: &HashMap<u64, String>,
) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    let mut since = None;
    for _ in 0..MOST_PAGES {
        let url = endpoint.messages_url(MESSAGE_LIMIT, since);
        let page: PagedMessages = get_json(agent, &url, token)?;
        since = page.paging.next_since();
        entries.extend(
            page.messages
                .into_iter()
                .map(|raw| Entry::from_raw(raw, apps)),
        );
        if since.is_none() {
            break;
        }
    }
    Ok(entries)
}

pub fn delete_message(
    agent: &Agent,
    endpoint: &Endpoint,
    token: &str,
    id: u64,
) -> Result<(), String> {
    agent
        .delete(&endpoint.delete_url(id))
        .header("X-Gotify-Key", token)
        .call()
        .map(drop)
        .map_err(|error| format!("cannot delete message {id}: {error}"))
}

pub fn delete_all_messages(agent: &Agent, endpoint: &Endpoint, token: &str) -> Result<(), String> {
    agent
        .delete(&endpoint.delete_all_url())
        .header("X-Gotify-Key", token)
        .call()
        .map(drop)
        .map_err(|error| format!("cannot clear messages: {error}"))
}

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Option<TransportAdapter>>>);

impl fmt::Debug for Capture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Capture")
    }
}

impl Capture {
    fn take(&self) -> Option<TransportAdapter> {
        sync::lock(&self.0).take()
    }
}

impl Connector<Box<dyn Transport>> for Capture {
    type Out = Box<dyn Transport>;

    fn connect(
        &self,
        details: &ConnectionDetails,
        chained: Option<Box<dyn Transport>>,
    ) -> Result<Option<Self::Out>, ureq::Error> {
        if let Some(transport) = chained {
            *sync::lock(&self.0) = Some(TransportAdapter::new(
                transport,
                details.current_time.clone(),
            ));
        }
        Ok(None)
    }
}

/// Gotify's `/stream`, over the connection ureq opened, so it goes through ureq's TLS.
pub type Socket = WebSocket<TransportAdapter>;

pub fn connect_stream(endpoint: &Endpoint, token: &str) -> Result<(Agent, Socket), String> {
    let capture = Capture::default();
    let connector = DefaultConnector::default().chain(capture.clone());
    let config = Config::builder()
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_resolve(Some(CONNECT_TIMEOUT))
        .build();
    let agent = Agent::with_parts(config, connector, DefaultResolver::default());
    let result = agent.get(&endpoint.stream_url()).call();
    let Some(mut stream) = capture.take() else {
        return match result {
            Ok(_) => Err(format!("cannot open transport to {}", endpoint.authority)),
            Err(error) => Err(format!("cannot connect to {}: {error}", endpoint.authority)),
        };
    };
    stream.set_timeout(NextTimeout {
        after: CONNECT_TIMEOUT.into(),
        reason: Timeout::Connect,
        per_read: Some(CONNECT_TIMEOUT.into()),
        per_write: Some(CONNECT_TIMEOUT.into()),
    });
    let mut socket = handshake(stream, endpoint, token)?;
    socket.get_mut().set_timeout(NextTimeout {
        after: ureq::unversioned::transport::time::Duration::NotHappening,
        reason: Timeout::Global,
        per_read: Some(READ_TIMEOUT.into()),
        per_write: Some(WRITE_TIMEOUT.into()),
    });
    Ok((agent, socket))
}

/// Upgrades `stream` to Gotify's `/stream`, signed in with `token`.
pub fn handshake<S: Read + Write>(
    stream: S,
    endpoint: &Endpoint,
    token: &str,
) -> Result<WebSocket<S>, String> {
    let mut request = format!("ws://{}{}", endpoint.authority, endpoint.stream_path())
        .into_client_request()
        .map_err(|error| format!("cannot ask for Gotify's stream: {error}"))?;
    let key = HeaderValue::from_str(token).map_err(|_| "the Gotify token is not a valid header")?;
    request.headers_mut().insert("X-Gotify-Key", key);
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE))
        .max_frame_size(Some(MAX_MESSAGE));
    tungstenite::client::client_with_config(request, stream, Some(config))
        .map(|(socket, _)| socket)
        .map_err(|error| match error {
            HandshakeError::Failure(error) => format!("Gotify refused the stream: {error}"),
            HandshakeError::Interrupted(_) => "Gotify took too long to open the stream".to_owned(),
        })
}

/// The next message; pings are answered as the stream is read.
pub fn read_message<S: Read + Write>(socket: &mut WebSocket<S>) -> Result<RawMessage, String> {
    loop {
        let payload = match socket.read() {
            Ok(Message::Text(text)) => text.as_bytes().to_vec(),
            Ok(Message::Binary(bytes)) => bytes.to_vec(),
            Ok(Message::Close(_)) => return Err("Gotify closed the stream".to_owned()),
            Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_)) => continue,
            Err(error) => return Err(format!("stream read failed: {error}")),
        };
        return serde_json::from_slice(&payload)
            .map_err(|error| format!("invalid Gotify message: {error}"));
    }
}

#[cfg(test)]
#[path = "client_test.rs"]
mod tests;
