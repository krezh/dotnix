use anyhow::{Context, Result};
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

const PROTOCOL_VERSION: u16 = 1;
const MAX_MESSAGE_SIZE: usize = 64 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingCommand {
    Status,
    Stop,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingReply {
    Status { output: PathBuf },
    Stopped { output: PathBuf },
}

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    version: u16,
    command: RecordingCommand,
}

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    version: u16,
    result: std::result::Result<RecordingReply, String>,
}

pub fn socket_path() -> Result<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").context("XDG_RUNTIME_DIR is not set")?;
    Ok(PathBuf::from(runtime).join("chomp/recording.sock"))
}

pub fn socket_exists() -> Result<bool> {
    Ok(socket_path()?.exists())
}

pub fn request(command: RecordingCommand) -> Result<RecordingReply> {
    let path = socket_path()?;
    let mut stream = UnixStream::connect(&path)
        .with_context(|| format!("Failed to connect to recorder at {}", path.display()))?;
    let read_timeout = match command {
        RecordingCommand::Status => Some(Duration::from_secs(5)),
        RecordingCommand::Stop => None,
    };
    stream.set_read_timeout(read_timeout)?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
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
        "Recorder uses unsupported protocol version {}",
        response.version
    );
    response.result.map_err(anyhow::Error::msg)
}

pub struct RecordingListener {
    listener: UnixListener,
    path: PathBuf,
}

impl RecordingListener {
    pub fn bind() -> Result<Self> {
        let path = socket_path()?;
        let directory = path
            .parent()
            .context("Recording socket has no parent directory")?;
        fs::create_dir_all(directory)
            .with_context(|| format!("Failed to create {}", directory.display()))?;
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
        if path.exists() {
            match UnixStream::connect(&path) {
                Ok(_) => anyhow::bail!("Another recording is already running"),
                Err(error)
                    if matches!(
                        error.kind(),
                        ErrorKind::ConnectionRefused | ErrorKind::NotFound
                    ) =>
                {
                    fs::remove_file(&path)
                        .with_context(|| format!("Failed to remove stale {}", path.display()))?;
                }
                Err(error) => return Err(error).context("Failed to inspect recording socket"),
            }
        }
        let listener = UnixListener::bind(&path)
            .with_context(|| format!("Failed to bind {}", path.display()))?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        listener.set_nonblocking(true)?;
        Ok(Self { listener, path })
    }

    pub fn poll(&self, output: &Path) -> Result<Option<UnixStream>> {
        let (mut stream, _) = match self.listener.accept() {
            Ok(client) => client,
            Err(error) if error.kind() == ErrorKind::WouldBlock => return Ok(None),
            Err(error) => return Err(error).context("Failed to accept recording client"),
        };
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        let result = self.handle(&mut stream, output);
        match result {
            Ok(Some(stream)) => Ok(Some(stream)),
            Ok(None) => Ok(None),
            Err(error) => {
                write_reply(&mut stream, Err(format!("{error:#}")))?;
                Ok(None)
            }
        }
    }

    fn handle(&self, stream: &mut UnixStream, output: &Path) -> Result<Option<UnixStream>> {
        let credentials =
            getsockopt(&*stream, PeerCredentials).context("Failed to inspect recording client")?;
        let current_uid = unsafe { libc::geteuid() };
        anyhow::ensure!(
            credentials.uid() == current_uid,
            "Recording client belongs to another user"
        );
        match read_request(stream)? {
            RecordingCommand::Status => {
                write_reply(
                    stream,
                    Ok(RecordingReply::Status {
                        output: output.to_path_buf(),
                    }),
                )?;
                Ok(None)
            }
            RecordingCommand::Stop => Ok(Some(stream.try_clone()?)),
        }
    }
}

impl Drop for RecordingListener {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn reply_stopped(stream: &mut UnixStream, result: &Result<()>, output: &Path) -> Result<()> {
    let reply = match result {
        Ok(()) => Ok(RecordingReply::Stopped {
            output: output.to_path_buf(),
        }),
        Err(error) => Err(format!("{error:#}")),
    };
    write_reply(stream, reply)
}

fn read_request(stream: &mut UnixStream) -> Result<RecordingCommand> {
    let request: Request = read_message(stream)?;
    anyhow::ensure!(
        request.version == PROTOCOL_VERSION,
        "Unsupported recording protocol version {}",
        request.version
    );
    Ok(request.command)
}

fn write_reply(
    stream: &mut UnixStream,
    result: std::result::Result<RecordingReply, String>,
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
    let payload = serde_json::to_vec(value).context("Failed to encode recording IPC message")?;
    anyhow::ensure!(
        payload.len() <= MAX_MESSAGE_SIZE,
        "Recording IPC message exceeds {} bytes",
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
        "Recording IPC message exceeds {} bytes",
        MAX_MESSAGE_SIZE
    );
    let mut payload = vec![0; length];
    stream.read_exact(&mut payload)?;
    serde_json::from_slice(&payload).context("Failed to decode recording IPC message")
}
