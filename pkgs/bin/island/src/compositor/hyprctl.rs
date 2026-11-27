//! Asking Hyprland over its socket, each request answered within a second or not at all.

use std::{
    env,
    io::{self, Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    time::{Duration, Instant},
};

use serde::de::DeserializeOwned;

use crate::paths;

/// Hyprland answers well within this, unless it is stuck; nothing waits on it longer.
const ANSWER_WITHIN: Duration = Duration::from_secs(1);
/// Larger than any answer Hyprland gives, even with hundreds of windows.
const MOST: usize = 16 * 1024 * 1024;

/// Where Hyprland's sockets and lock are, for the instance island runs under.
pub fn socket_dir() -> Result<PathBuf, String> {
    let signature = env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .map_err(|_| "not running under Hyprland".to_owned())?;
    if signature.is_empty() || signature.contains('/') || signature == "." || signature == ".." {
        return Err("HYPRLAND_INSTANCE_SIGNATURE is not a folder's name".to_owned());
    }
    Ok(paths::runtime_root()
        .ok_or("XDG_RUNTIME_DIR is not set")?
        .join("hypr")
        .join(signature))
}

/// The socket its events come on.
pub fn events() -> io::Result<UnixStream> {
    UnixStream::connect(
        socket_dir()
            .map_err(io::Error::other)?
            .join(".socket2.sock"),
    )
}

fn left(deadline: Instant) -> io::Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|left| !left.is_zero())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "Hyprland took too long"))
}

pub fn request(command: &str) -> io::Result<String> {
    let deadline = Instant::now() + ANSWER_WITHIN;
    let path = socket_dir().map_err(io::Error::other)?.join(".socket.sock");
    let mut stream = UnixStream::connect(path)?;
    stream.set_write_timeout(Some(left(deadline)?))?;
    stream.write_all(command.as_bytes())?;
    let mut reply = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        stream.set_read_timeout(Some(left(deadline)?))?;
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) if reply.len() + count > MOST => {
                return Err(io::Error::other("Hyprland's answer is too long"));
            }
            Ok(count) => reply.extend_from_slice(&buffer[..count]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    String::from_utf8(reply).map_err(io::Error::other)
}

pub fn query<T: DeserializeOwned>(command: &str) -> io::Result<T> {
    serde_json::from_str(&request(command)?).map_err(io::Error::other)
}

pub fn lua_string(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

/// Runs a Lua dispatcher such as `hl.dsp.exec_cmd(...)`. Dispatchers are only descriptions until
/// handed to `hl.dispatch`, and Hyprland reports Lua failures in the reply, not as a socket error.
pub fn dispatch(dispatcher: &str) -> io::Result<()> {
    let reply = request(&format!("eval hl.dispatch({dispatcher})"))?;
    match reply.trim() {
        reply if reply.starts_with("error") => Err(io::Error::other(reply.to_owned())),
        _ => Ok(()),
    }
}
