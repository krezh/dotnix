//! What leaving the session asks of Hyprland: its process, windows and layers, closing windows and
//! exiting, each answered within a second or not at all, and the events that say a window went.

use std::{
    env, fs,
    io::{self, Read, Write},
    os::{fd::AsRawFd, unix::net::UnixStream},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use serde::{Deserialize, de::DeserializeOwned};

use crate::paths;

const TIMEOUT: Duration = Duration::from_secs(1);
const MOST: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize)]
pub struct Client {
    pub address: String,
    pub class: String,
    pub title: String,
    pub pid: i32,
}

#[derive(Deserialize)]
pub struct Layer {
    pub address: String,
    pub namespace: String,
    pub pid: i32,
}

#[derive(Deserialize)]
pub struct Layers {
    pub levels: std::collections::HashMap<String, Vec<Layer>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    #[serde(default)]
    config_provider: Option<String>,
}

pub struct Hyprland {
    directory: PathBuf,
    /// Its config is Lua, which names dispatchers differently.
    lua: bool,
    pub pid: i32,
}

impl Hyprland {
    pub fn find() -> Result<Self, String> {
        let signature = env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .map_err(|_| "not running under Hyprland".to_owned())?;
        if signature.is_empty() || signature.contains('/') || signature == "." || signature == ".."
        {
            return Err("HYPRLAND_INSTANCE_SIGNATURE is not a folder's name".to_owned());
        }
        let directory = paths::runtime_root()
            .ok_or("XDG_RUNTIME_DIR is not set")?
            .join("hypr")
            .join(signature);
        let lock = fs::read_to_string(directory.join("hyprland.lock"))
            .map_err(|error| format!("cannot read hyprland.lock: {error}"))?;
        let pid = parse_lock(&lock)?;
        let mut found = Self {
            directory,
            lua: false,
            pid,
        };
        let status: Status = found.query("j/status")?;
        found.lua = match status.config_provider.as_deref() {
            Some("lua") => true,
            Some("hyprlang") | None => false,
            Some(provider) => return Err(format!("Hyprland's config is in {provider}, unknown")),
        };
        Ok(found)
    }

    fn socket(&self) -> PathBuf {
        self.directory.join(".socket.sock")
    }

    pub fn query<T: DeserializeOwned>(&self, command: &str) -> Result<T, String> {
        let reply = request(&self.socket(), command)?;
        serde_json::from_str(&reply).map_err(|error| {
            format!("Hyprland answered {command} with what cannot be read: {error}")
        })
    }

    pub fn clients(&self) -> Result<Vec<Client>, String> {
        let clients: Vec<Client> = self.query("j/clients")?;
        let mut seen = std::collections::HashSet::new();
        for client in &clients {
            check_address(&client.address)?;
            if !seen.insert(&client.address) {
                return Err(format!("Hyprland named window {} twice", client.address));
            }
        }
        Ok(clients)
    }

    pub fn layers(&self) -> Result<std::collections::HashMap<String, Layers>, String> {
        self.query("j/layers")
    }

    fn dispatch(&self, dispatcher: &str) -> Result<(), String> {
        let command = format!("/dispatch {dispatcher}");
        let reply = request(&self.socket(), &command)?;
        if reply.trim() != "ok" {
            return Err(format!("Hyprland refused {command}: {}", reply.trim()));
        }
        Ok(())
    }

    pub fn close(&self, address: &str) -> Result<(), String> {
        check_address(address)?;
        self.dispatch(&if self.lua {
            format!("hl.dsp.window.close({{ window = 'address:{address}' }})")
        } else {
            format!("closewindow address:{address}")
        })
    }

    pub fn exit(&self) -> Result<(), String> {
        self.dispatch(if self.lua { "hl.dsp.exit()" } else { "exit" })
    }

    /// Hyprland's events as they come, read only to know something changed.
    pub fn events(&self) -> Result<Events, String> {
        let stream = UnixStream::connect(self.directory.join(".socket2.sock"))
            .map_err(|error| format!("cannot hear Hyprland's events: {error}"))?;
        stream
            .set_nonblocking(true)
            .map_err(|error| format!("cannot hear Hyprland's events: {error}"))?;
        Ok(Events(stream))
    }
}

pub struct Events(UnixStream);

impl Events {
    pub fn fd(&self) -> std::os::fd::RawFd {
        self.0.as_raw_fd()
    }

    /// Reads what has come; false once Hyprland has hung up.
    pub fn drain(&mut self) -> bool {
        let mut buffer = [0_u8; 8192];
        loop {
            match self.0.read(&mut buffer) {
                Ok(0) => return false,
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return true,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => return false,
            }
        }
    }
}

pub fn parse_lock(lock: &str) -> Result<i32, String> {
    let mut lines = lock.lines();
    let pid = lines
        .next()
        .and_then(|line| line.parse::<i32>().ok())
        .filter(|pid| *pid > 1)
        .ok_or("hyprland.lock names no process")?;
    if lines.next().is_none_or(str::is_empty) || lines.any(|line| !line.is_empty()) {
        return Err("hyprland.lock holds more than it should".to_owned());
    }
    Ok(pid)
}

/// Hyprland's windows are named by a nonzero number in hex, which goes into a dispatcher as is.
pub fn check_address(address: &str) -> Result<(), String> {
    let valid = address.strip_prefix("0x").is_some_and(|hex| {
        !hex.is_empty()
            && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
            && u64::from_str_radix(hex, 16).is_ok_and(|value| value > 0)
    });
    if valid {
        Ok(())
    } else {
        Err(format!("Hyprland named a window {address:?}"))
    }
}

fn left(deadline: Instant) -> io::Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|left| !left.is_zero())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "Hyprland took too long"))
}

fn request(path: &Path, command: &str) -> Result<String, String> {
    let ask = || -> io::Result<String> {
        let deadline = Instant::now() + TIMEOUT;
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
    };
    ask().map_err(|error| format!("cannot ask Hyprland {command}: {error}"))
}

#[cfg(test)]
#[path = "hyprland_test.rs"]
mod tests;
