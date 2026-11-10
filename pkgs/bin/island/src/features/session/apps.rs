//! The apps open as the session ends, and closing them: their windows asked to close, then what
//! has no window sent SIGTERM, and every one of them SIGKILL if forced. Nothing is signalled that
//! is not still the process found, nor anything island itself runs under.

use std::{
    collections::{HashMap, HashSet},
    os::fd::RawFd,
    time::{Duration, Instant},
};

use super::{
    hyprland::{Client, Hyprland, check_address},
    process::{self, Identity, Process},
};

/// How often windows still open are asked again.
pub const ASK_AGAIN: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Open {
    /// A window's address, or a process's number for one without a window.
    pub key: String,
    pub address: String,
    pub class: String,
    pub title: String,
    pub pid: i32,
    pub window: bool,
    identity: Option<Identity>,
}

pub struct Apps {
    hyprland: Hyprland,
    open: Vec<Open>,
    processes: HashMap<Identity, Process>,
    compositor: Process,
    termed: HashSet<Identity>,
    asked_at: Option<Instant>,
    forced: bool,
}

impl Apps {
    /// Every window, every layer's process, and every other process Hyprland started, but for
    /// island and whatever it runs under, and Xwayland.
    pub fn find() -> Result<Self, String> {
        let hyprland = Hyprland::find()?;
        let compositor = Process::open(hyprland.pid)?.ok_or("Hyprland is no longer running")?;
        let mut kept = process::ancestors()?;
        kept.insert(hyprland.pid);
        let clients = hyprland.clients()?;
        let layers = hyprland.layers()?;
        let children = process::children_of(hyprland.pid)?;
        let mut open: Vec<Open> = clients
            .iter()
            .filter(|client| !kept.contains(&client.pid))
            .map(|client| Open {
                key: client.address.clone(),
                address: client.address.clone(),
                class: client.class.clone(),
                title: client.title.clone(),
                pid: client.pid,
                window: true,
                identity: None,
            })
            .collect();
        let windowed: HashSet<i32> = clients
            .iter()
            .filter(|client| client.pid > 0)
            .map(|client| client.pid)
            .collect();
        let mut alone = HashSet::new();
        for layer in layers
            .into_values()
            .flat_map(|monitor| monitor.levels.into_values())
            .flatten()
        {
            check_address(&layer.address)?;
            if layer.pid <= 0 {
                return Err(format!("layer {} names no process", layer.namespace));
            }
            if !kept.contains(&layer.pid)
                && !windowed.contains(&layer.pid)
                && alone.insert(layer.pid)
            {
                open.push(Open {
                    key: format!("pid-{}", layer.pid),
                    address: layer.address,
                    class: layer.namespace,
                    title: String::new(),
                    pid: layer.pid,
                    window: false,
                    identity: None,
                });
            }
        }
        for child in children {
            let pid = child.identity.pid;
            if child.name != "Xwayland"
                && !kept.contains(&pid)
                && !windowed.contains(&pid)
                && alone.insert(pid)
            {
                open.push(Open {
                    key: format!("pid-{pid}"),
                    address: String::new(),
                    class: child.name,
                    title: String::new(),
                    pid,
                    window: false,
                    identity: None,
                });
            }
        }
        let mut processes = HashMap::new();
        let mut identities = HashMap::new();
        for app in &mut open {
            if app.pid <= 0 {
                continue;
            }
            let identity = match identities.get(&app.pid) {
                Some(identity) => *identity,
                None => {
                    let found = Process::open(app.pid)?;
                    let identity = found.as_ref().map(|found| found.identity);
                    if let Some(found) = found {
                        if process::stat(app.pid)?.is_some_and(|stat| stat.name == "Xwayland") {
                            kept.insert(app.pid);
                        }
                        processes.insert(found.identity, found);
                    }
                    identities.insert(app.pid, identity);
                    identity
                }
            };
            app.identity = identity;
        }
        open.retain(|app| !kept.contains(&app.pid) && (app.window || app.identity.is_some()));
        open.sort_by(|left, right| {
            (&left.class, &left.title, left.pid).cmp(&(&right.class, &right.title, right.pid))
        });
        if !compositor.alive()? {
            return Err("Hyprland exited while its apps were being found".to_owned());
        }
        Ok(Self {
            hyprland,
            open,
            processes,
            compositor,
            termed: HashSet::new(),
            asked_at: None,
            forced: false,
        })
    }

    pub fn open(&self) -> &[Open] {
        &self.open
    }

    pub fn hyprland(&self) -> &Hyprland {
        &self.hyprland
    }

    /// Every app has closed, or been killed.
    pub fn done(&self) -> bool {
        self.open.is_empty() || self.forced
    }

    /// What to wait on: each process still followed, and Hyprland's own.
    pub fn fds(&self) -> Vec<RawFd> {
        self.processes
            .values()
            .map(Process::fd)
            .chain([self.compositor.fd()])
            .collect()
    }

    /// When windows still open are next asked again.
    pub fn next_ask(&self) -> Option<Instant> {
        self.asked_at.map(|at| at + ASK_AGAIN)
    }

    /// Asks what is open to close, and sees what has: windows are asked again every `ASK_AGAIN`.
    pub fn close(&mut self) -> Result<(), String> {
        if !self.compositor.alive()? {
            return Err("Hyprland exited before its apps had closed".to_owned());
        }
        let clients = self.hyprland.clients()?;
        let mut living = HashSet::new();
        for (identity, found) in &self.processes {
            if found.alive()? {
                living.insert(*identity);
            }
        }
        self.processes
            .retain(|identity, _| living.contains(identity));
        self.open.retain(|app| still_open(app, &clients, &living));
        let again = self.asked_at.is_none_or(|at| at.elapsed() >= ASK_AGAIN);
        let windowed: HashSet<i32> = clients
            .iter()
            .filter(|client| client.pid > 0)
            .map(|client| client.pid)
            .collect();
        let mut signals = HashSet::new();
        for app in &self.open {
            if again
                && app.window
                && window_present(app, &clients)
                && app
                    .identity
                    .is_none_or(|identity| living.contains(&identity))
            {
                self.hyprland.close(&app.address)?;
            }
            if let Some(identity) = app.identity
                && should_term(identity, &living, &windowed, &self.termed, again)
            {
                signals.insert(identity);
            }
        }
        for identity in signals {
            if let Some(found) = self.processes.get(&identity) {
                found.signal(libc::SIGTERM)?;
            }
            self.termed.insert(identity);
        }
        if again {
            self.asked_at = Some(Instant::now());
        }
        Ok(())
    }

    /// Kills `key`'s process, or, without one, refuses rather than kill blind.
    pub fn kill(&mut self, key: &str) -> Result<(), String> {
        let Some(app) = self.open.iter().find(|app| app.key == key) else {
            return Ok(());
        };
        match app
            .identity
            .and_then(|identity| self.processes.get(&identity))
        {
            Some(found) => found.signal(libc::SIGKILL),
            None if app.window => Err(format!(
                "cannot force {} to quit: its process is unknown",
                app.class
            )),
            None => Ok(()),
        }
    }

    /// Every app is killed, so leaving need not wait for the last of them to go.
    pub fn forced(&mut self) {
        self.forced = true;
    }
}

pub fn still_open(app: &Open, clients: &[Client], living: &HashSet<Identity>) -> bool {
    app.identity
        .is_some_and(|identity| living.contains(&identity))
        || (app.window && window_present(app, clients))
}

pub fn window_present(app: &Open, clients: &[Client]) -> bool {
    clients
        .iter()
        .any(|client| client.address == app.address && client.pid == app.pid)
}

/// A process with no window left is sent SIGTERM once, and again each time windows are asked again.
pub fn should_term(
    identity: Identity,
    living: &HashSet<Identity>,
    windowed: &HashSet<i32>,
    termed: &HashSet<Identity>,
    again: bool,
) -> bool {
    living.contains(&identity)
        && !windowed.contains(&identity.pid)
        && (again || !termed.contains(&identity))
}

#[cfg(test)]
pub fn open_for_test(key: &str, pid: i32, window: bool, identity: Option<Identity>) -> Open {
    Open {
        key: key.to_owned(),
        address: key.to_owned(),
        class: "Editor".to_owned(),
        title: "Unsaved".to_owned(),
        pid,
        window,
        identity,
    }
}

#[cfg(test)]
#[path = "apps_test.rs"]
mod tests;
