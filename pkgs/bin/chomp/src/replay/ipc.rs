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
    Save { output: Option<PathBuf> },
    Stop,
    Status,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    WaitingForTarget,
    Buffering,
    RetainingAfterExit,
    Suspended,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayStatus {
    pub state: ServiceState,
    pub target: Option<String>,
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
