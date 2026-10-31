use std::{
    collections::HashMap,
    fmt,
    io::{self, Read, Write},
    sync::{Arc, Mutex},
    time::Duration,
};

use super::state::{Endpoint, Entry, PagedMessages, RawApp, RawMessage};
use serde::de::DeserializeOwned;
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
const MAX_HANDSHAKE: usize = 8 * 1024;
const MAX_FRAME: usize = 1024 * 1024;
const MESSAGE_LIMIT: usize = 50;

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
    let paged: PagedMessages = get_json(agent, &endpoint.messages_url(MESSAGE_LIMIT), token)?;
    Ok(paged
        .messages
        .into_iter()
        .map(|raw| Entry::from_raw(raw, apps))
        .collect())
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
        self.0.lock().ok()?.take()
    }
}

impl Connector<Box<dyn Transport>> for Capture {
    type Out = Box<dyn Transport>;

    fn connect(
        &self,
        details: &ConnectionDetails,
        chained: Option<Box<dyn Transport>>,
    ) -> Result<Option<Self::Out>, ureq::Error> {
        if let Some(transport) = chained
            && let Ok(mut slot) = self.0.lock()
        {
            *slot = Some(TransportAdapter::new(
                transport,
                details.current_time.clone(),
            ));
        }
        Ok(None)
    }
}

pub fn connect_stream(
    endpoint: &Endpoint,
    token: &str,
) -> Result<(Agent, TransportAdapter), String> {
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
    handshake(&mut stream, endpoint, token)?;
    stream.set_timeout(NextTimeout {
        after: ureq::unversioned::transport::time::Duration::NotHappening,
        reason: Timeout::Global,
        per_read: Some(READ_TIMEOUT.into()),
        per_write: Some(WRITE_TIMEOUT.into()),
    });
    Ok((agent, stream))
}

pub fn handshake(
    stream: &mut (impl Read + Write),
    endpoint: &Endpoint,
    token: &str,
) -> Result<(), String> {
    write!(
        stream,
        "GET {} HTTP/1.1\r\n\
         Host: {}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
         Sec-WebSocket-Version: 13\r\n\
         X-Gotify-Key: {}\r\n\r\n",
        endpoint.stream_path(),
        endpoint.authority,
        token,
    )
    .and_then(|()| stream.flush())
    .map_err(|error| format!("cannot send WebSocket upgrade: {error}"))?;

    let mut header = Vec::with_capacity(256);
    let mut byte = [0_u8; 1];
    while !header.ends_with(b"\r\n\r\n") {
        if header.len() >= MAX_HANDSHAKE {
            return Err("Gotify handshake headers exceeded limit".to_owned());
        }
        match stream.read(&mut byte) {
            Ok(0) => return Err("Gotify closed connection during handshake".to_owned()),
            Ok(_) => header.push(byte[0]),
            Err(error) => return Err(format!("cannot read handshake response: {error}")),
        }
    }
    let text = String::from_utf8_lossy(&header);
    let status = text.lines().next().unwrap_or_default().trim();
    if !status.starts_with("HTTP/1.1 101") {
        return Err(format!("Gotify rejected /stream upgrade: {status}"));
    }
    Ok(())
}

fn random_mask() -> [u8; 4] {
    let mut mask = [0_u8; 4];
    let ret = unsafe { libc::getrandom(mask.as_mut_ptr().cast(), mask.len(), 0) };
    if ret == mask.len() as isize {
        mask
    } else {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0x1234_5678, |d| d.subsec_nanos());
        nanos.to_ne_bytes()
    }
}

fn write_frame(stream: &mut impl Write, opcode: u8, payload: &[u8]) -> io::Result<()> {
    let mut frame = Vec::with_capacity(14 + payload.len());
    frame.push(0x80 | (opcode & 0x0f));
    let len = payload.len();
    if len < 126 {
        frame.push(0x80 | (len as u8));
    } else if len <= usize::from(u16::MAX) {
        frame.push(0x80 | 126);
        frame.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        frame.push(0x80 | 127);
        frame.extend_from_slice(&(len as u64).to_be_bytes());
    }
    let mask = random_mask();
    frame.extend_from_slice(&mask);
    for (index, byte) in payload.iter().enumerate() {
        frame.push(byte ^ mask[index & 3]);
    }
    stream.write_all(&frame)?;
    stream.flush()
}

pub fn read_message(stream: &mut (impl Read + Write)) -> Result<RawMessage, String> {
    let mut fragments = Vec::new();
    let payload = loop {
        let mut head = [0_u8; 2];
        stream
            .read_exact(&mut head)
            .map_err(|error| format!("stream read failed: {error}"))?;
        let fin = (head[0] & 0x80) != 0;
        let opcode = head[0] & 0x0f;
        let masked = (head[1] & 0x80) != 0;
        let len = match head[1] & 0x7f {
            126 => {
                let mut ext = [0_u8; 2];
                stream
                    .read_exact(&mut ext)
                    .map_err(|error| format!("stream read failed: {error}"))?;
                usize::from(u16::from_be_bytes(ext))
            }
            127 => {
                let mut ext = [0_u8; 8];
                stream
                    .read_exact(&mut ext)
                    .map_err(|error| format!("stream read failed: {error}"))?;
                usize::try_from(u64::from_be_bytes(ext))
                    .map_err(|_| "WebSocket frame exceeds limit".to_owned())?
            }
            len => usize::from(len),
        };
        if len > MAX_FRAME || fragments.len().saturating_add(len) > MAX_FRAME {
            return Err("WebSocket frame exceeds limit".to_owned());
        }
        let mut mask = [0_u8; 4];
        if masked {
            stream
                .read_exact(&mut mask)
                .map_err(|error| format!("stream read failed: {error}"))?;
        }
        let mut payload = vec![0_u8; len];
        stream
            .read_exact(&mut payload)
            .map_err(|error| format!("stream read failed: {error}"))?;
        if masked {
            for (index, byte) in payload.iter_mut().enumerate() {
                *byte ^= mask[index & 3];
            }
        }
        match opcode {
            0x1 | 0x2 => {
                if !fragments.is_empty() {
                    return Err("new message arrived before prior fragment finished".to_owned());
                }
                if fin {
                    break payload;
                }
                fragments.extend_from_slice(&payload);
            }
            0x0 => {
                if fragments.is_empty() {
                    return Err("unexpected continuation frame without initial fragment".to_owned());
                }
                fragments.extend_from_slice(&payload);
                if fin {
                    break std::mem::take(&mut fragments);
                }
            }
            0x9 => {
                write_frame(stream, 0xA, &payload)
                    .map_err(|error| format!("cannot send WebSocket pong: {error}"))?;
            }
            0xA => {}
            0x8 => {
                let _ = write_frame(stream, 0x8, &[]);
                return Err("Gotify closed the stream".to_owned());
            }
            other => return Err(format!("unexpected WebSocket opcode {other:#x}")),
        }
    };
    serde_json::from_slice(&payload).map_err(|error| format!("invalid Gotify message: {error}"))
}

#[cfg(test)]
#[path = "client_test.rs"]
mod tests;
