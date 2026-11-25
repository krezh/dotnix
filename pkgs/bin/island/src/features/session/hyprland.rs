//! What leaving the session asks of Hyprland: its process, windows and layers, closing windows
//! (through the [`compositor`]) and exiting, and the events that say a window went.

use std::{
    fs,
    io::{self, Read},
    os::{fd::AsRawFd, unix::net::UnixStream},
};

use serde::{Deserialize, de::DeserializeOwned};

use crate::compositor::{self, hyprctl};

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

pub struct Hyprland {
    pub pid: i32,
}

impl Hyprland {
    pub fn find() -> Result<Self, String> {
        let lock = fs::read_to_string(hyprctl::socket_dir()?.join("hyprland.lock"))
            .map_err(|error| format!("cannot read hyprland.lock: {error}"))?;
        let pid = parse_lock(&lock)?;
        Ok(Self { pid })
    }

    pub fn query<T: DeserializeOwned>(&self, command: &str) -> Result<T, String> {
        hyprctl::query(command).map_err(|error| format!("cannot ask Hyprland {command}: {error}"))
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

    pub fn close(&self, address: &str) -> Result<(), String> {
        check_address(address)?;
        compositor::backend()
            .ok_or("island follows no compositor")?
            .close_window(address)
    }

    pub fn exit(&self) -> Result<(), String> {
        const EXIT: &str = "/dispatch hl.dsp.exit()";
        let reply = hyprctl::request(EXIT)
            .map_err(|error| format!("cannot ask Hyprland {EXIT}: {error}"))?;
        if reply.trim() != "ok" {
            return Err(format!("Hyprland refused {EXIT}: {}", reply.trim()));
        }
        Ok(())
    }

    /// Hyprland's events as they come, read only to know something changed.
    pub fn events(&self) -> Result<Events, String> {
        let stream = hyprctl::events()
            .and_then(|stream| stream.set_nonblocking(true).map(|()| stream))
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

#[cfg(test)]
#[path = "hyprland_test.rs"]
mod tests;
