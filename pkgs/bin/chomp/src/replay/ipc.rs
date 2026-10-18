use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

const PROTOCOL_VERSION: u16 = 1;
const MAX_MESSAGE_SIZE: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplayCommand {
    Start,
    Save {
        output: Option<PathBuf>,
        #[serde(default)]
        duration_seconds: Option<u32>,
    },
    Configure {
        duration_seconds: u32,
    },
    Stop,
    Status,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    #[default]
    WaitingForTarget,
    Buffering,
    RetainingAfterExit,
    Suspended,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ReplayStatus {
    pub state: ServiceState,
    pub target: Option<String>,
    #[serde(default)]
    pub target_app: Option<String>,
    #[serde(default)]
    pub target_title: Option<String>,
    #[serde(default)]
    pub duration_seconds: u32,
    pub buffered_millis: u64,
    pub buffered_bytes: u64,
    pub message: Option<String>,
}

impl ReplayStatus {
    /// True when the buffer has captured frames and the service can produce a replay clip.
    pub fn can_save(&self) -> bool {
        matches!(
            self.state,
            ServiceState::Buffering | ServiceState::RetainingAfterExit
        ) && self.buffered_millis > 0
    }

    /// Human-friendly representation of the active capture target application.
    pub fn active_app(&self) -> Option<String> {
        let app = self
            .target_app
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let title = self
            .target_title
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());

        match (app, title) {
            (Some(app), Some(title)) => {
                if app.eq_ignore_ascii_case(title) {
                    Some(title.to_string())
                } else {
                    Some(format!("{title} ({app})"))
                }
            }
            (Some(app), None) => Some(app.to_string()),
            (None, Some(title)) => Some(title.to_string()),
            (None, None) => {
                let target = self.target.as_deref()?.trim();
                if target.is_empty() {
                    return None;
                }
                let target_without_addr = if let Some(idx) = target.rfind(" (0x") {
                    &target[..idx]
                } else {
                    target
                };
                if let Some((class, title)) = target_without_addr.split_once(" — ") {
                    let class = class.trim();
                    let title = title.trim();
                    if class.eq_ignore_ascii_case(title) {
                        Some(title.to_string())
                    } else if !title.is_empty() && !class.is_empty() {
                        Some(format!("{title} ({class})"))
                    } else if !title.is_empty() {
                        Some(title.to_string())
                    } else {
                        Some(class.to_string())
                    }
                } else {
                    Some(target_without_addr.to_string())
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplayReply {
    Status(ReplayStatus),
    Saved {
        output: PathBuf,
        duration_millis: u64,
    },
    Stopped,
}

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    version: u16,
    command: ReplayCommand,
}

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    version: u16,
    result: std::result::Result<ReplayReply, String>,
}

pub fn socket_path() -> Result<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").context("XDG_RUNTIME_DIR is not set")?;
    Ok(PathBuf::from(runtime).join("chomp/replay.sock"))
}

pub fn request(command: ReplayCommand) -> Result<ReplayReply> {
    let read_timeout = response_timeout(&command);
    request_with_timeout(command, read_timeout, Some(Duration::from_secs(5)))
}

pub fn request_with_timeout(
    command: ReplayCommand,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
) -> Result<ReplayReply> {
    let path = socket_path()?;
    let mut stream = UnixStream::connect(&path).with_context(|| {
        format!(
            "Failed to connect to replay service at {}; start chomp-replay.service",
            path.display()
        )
    })?;
    stream.set_read_timeout(read_timeout)?;
    stream.set_write_timeout(write_timeout)?;
    write_message(
        &mut stream,
        &Request {
            version: PROTOCOL_VERSION,
            command,
        },
    )?;
    let response: Response = read_message(&mut stream)?;
    anyhow::ensure!(
        response.version == PROTOCOL_VERSION,
        "Replay service uses unsupported protocol version {}",
        response.version
    );
    response.result.map_err(anyhow::Error::msg)
}

fn response_timeout(command: &ReplayCommand) -> Option<Duration> {
    match command {
        ReplayCommand::Save { .. } => None,
        _ => Some(Duration::from_secs(30)),
    }
}

pub fn read_request(stream: &mut UnixStream) -> Result<ReplayCommand> {
    let request: Request = read_message(stream)?;
    anyhow::ensure!(
        request.version == PROTOCOL_VERSION,
        "Unsupported replay protocol version {}",
        request.version
    );
    Ok(request.command)
}

pub fn write_reply(
    stream: &mut UnixStream,
    result: std::result::Result<ReplayReply, String>,
) -> Result<()> {
    write_message(
        stream,
        &Response {
            version: PROTOCOL_VERSION,
            result,
        },
    )
}

fn write_message<T: Serialize>(stream: &mut UnixStream, value: &T) -> Result<()> {
    let payload = serde_json::to_vec(value).context("Failed to encode replay IPC message")?;
    anyhow::ensure!(
        payload.len() <= MAX_MESSAGE_SIZE,
        "Replay IPC message exceeds {} bytes",
        MAX_MESSAGE_SIZE
    );
    stream.write_all(&(payload.len() as u32).to_be_bytes())?;
    stream.write_all(&payload)?;
    Ok(())
}

fn read_message<T: for<'de> Deserialize<'de>>(stream: &mut UnixStream) -> Result<T> {
    let mut length = [0; 4];
    stream.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length) as usize;
    anyhow::ensure!(
        length <= MAX_MESSAGE_SIZE,
        "Replay IPC message exceeds {} bytes",
        MAX_MESSAGE_SIZE
    );
    let mut payload = vec![0; length];
    stream.read_exact(&mut payload)?;
    serde_json::from_slice(&payload).context("Failed to decode replay IPC message")
}

#[cfg(test)]
#[path = "ipc_test.rs"]
mod tests;
