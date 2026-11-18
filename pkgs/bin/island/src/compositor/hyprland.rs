use std::{
    collections::{BTreeSet, HashMap},
    env,
    io::{self, BufRead, BufReader, Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    time::Duration,
};

use serde::Deserialize;
use tracing::{info, trace, warn};

use super::{Compositor, windows};
use crate::{
    features::Feed,
    paths, profile,
    supervisor::Ended,
    wm::{Monitor, Place, Snapshot, Window, Workspace},
};

#[derive(Deserialize)]
struct RawMonitor {
    #[serde(default)]
    id: i64,
    name: String,
    focused: bool,
    #[serde(rename = "activeWorkspace")]
    active_workspace: RawWorkspaceRef,
    x: i32,
    y: i32,
    width: f32,
    height: f32,
    scale: f32,
    /// wl_output transform: odd values rotate by 90° or 270°.
    #[serde(default)]
    transform: u8,
}

impl RawMonitor {
    fn logical_size(&self) -> (i32, i32) {
        let (width, height) = if self.transform % 2 == 1 {
            (self.height, self.width)
        } else {
            (self.width, self.height)
        };
        let scale = self.scale.max(0.1);
        ((width / scale).round() as i32, (height / scale).round() as i32)
    }
}

#[derive(Deserialize)]
struct RawWorkspaceRef {
    #[serde(default)]
    id: Option<i64>,
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct RawWorkspace {
    #[serde(default)]
    id: Option<i64>,
    monitor: String,
    windows: u32,
    #[serde(default)]
    hasfullscreen: bool,
}

#[derive(Deserialize)]
struct RawClient {
    address: String,
    workspace: RawWorkspaceRef,
    /// Fullscreen mode bits: 1 is maximized, 2 is real fullscreen.
    #[serde(default)]
    fullscreen: u8,
    #[serde(default)]
    class: String,
    #[serde(default)]
    title: String,
    /// The monitor's id.
    #[serde(default)]
    monitor: i64,
    #[serde(default)]
    at: [i32; 2],
    #[serde(default)]
    size: [i32; 2],
    #[serde(default)]
    floating: bool,
    #[serde(default = "mapped")]
    mapped: bool,
    #[serde(default)]
    hidden: bool,
    #[serde(rename = "stableId", default)]
    stable_id: String,
    /// 0 for the window focused last, counting back.
    #[serde(rename = "focusHistoryID", default)]
    focus_history: i64,
}

fn mapped() -> bool {
    true
}

fn bare(address: &str) -> &str {
    address.trim_start_matches("0x")
}

fn window(address: &str) -> String {
    lua_string(&format!("address:0x{}", bare(address)))
}

/// Hyprland's windows, most recently focused first, none of them urgent yet.
fn query_windows() -> io::Result<Vec<Window>> {
    let mut clients: Vec<RawClient> = query("j/clients")?;
    let monitors: Vec<RawMonitor> = query("j/monitors")?;
    clients.retain(|client| client.mapped && !client.hidden);
    clients.sort_by_key(|client| client.focus_history);
    Ok(clients
        .into_iter()
        .filter_map(|client| {
            let id = client.workspace.id?;
            let place = if id < 0 {
                let name = client.workspace.name.trim_start_matches("special:");
                Place::Special(if name.is_empty() { "special" } else { name }.to_owned())
            } else if id > 0 {
                Place::Workspace(id)
            } else {
                return None;
            };
            let monitor = monitors
                .iter()
                .find(|monitor| monitor.id == client.monitor)?;
            Some(Window {
                urgent: false,
                address: client.address,
                id: client.stable_id,
                class: client.class,
                title: client.title,
                place,
                monitor: monitor.name.clone(),
                x: client.at[0] - monitor.x,
                y: client.at[1] - monitor.y,
                width: client.size[0],
                height: client.size[1],
                floating: client.floating,
            })
        })
        .collect())
}

/// One output's layer surfaces, keyed by layer ("0" background … "3" overlay).
#[derive(Deserialize)]
struct RawLayers {
    levels: HashMap<String, Vec<RawLayer>>,
}

#[derive(Deserialize)]
struct RawLayer {
    x: i32,
    y: i32,
    namespace: String,
    pid: u32,
}

fn socket_dir() -> Option<PathBuf> {
    let signature = env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    Some(paths::runtime_root()?.join("hypr").join(signature))
}

/// Hyprland answers well within this, unless it is stuck; nothing waits on it longer.
const ANSWER_WITHIN: Duration = Duration::from_secs(1);

fn request(command: &str) -> io::Result<String> {
    let path = socket_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not running under Hyprland"))?
        .join(".socket.sock");
    let mut stream = UnixStream::connect(path)?;
    stream.set_write_timeout(Some(ANSWER_WITHIN))?;
    stream.set_read_timeout(Some(ANSWER_WITHIN))?;
    stream.write_all(command.as_bytes())?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply)?;
    Ok(reply)
}

fn query<T: for<'de> Deserialize<'de>>(command: &str) -> io::Result<T> {
    serde_json::from_str(&request(command)?).map_err(io::Error::other)
}

fn lua_string(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

/// Runs a Lua dispatcher such as `hl.dsp.exec_cmd(...)`. Dispatchers are only descriptions until
/// handed to `hl.dispatch`, and Hyprland reports Lua failures in the reply, not as a socket error.
fn dispatch(dispatcher: &str) -> io::Result<()> {
    let reply = request(&format!("eval hl.dispatch({dispatcher})"))?;
    match reply.trim() {
        reply if reply.starts_with("error") => Err(io::Error::other(reply.to_owned())),
        _ => Ok(()),
    }
}

fn snapshot(urgent: &BTreeSet<i64>) -> io::Result<Snapshot> {
    let raw_monitors: Vec<RawMonitor> = query("j/monitors")?;
    let raw_workspaces: Vec<RawWorkspace> = query("j/workspaces")?;
    // `hasfullscreen` is also set for maximized windows, which leave the bar visible.
    let fullscreen: BTreeSet<i64> = if raw_workspaces
        .iter()
        .any(|workspace| workspace.hasfullscreen)
    {
        query::<Vec<RawClient>>("j/clients")?
            .into_iter()
            .filter(|client| client.fullscreen & 2 != 0)
            .filter_map(|client| client.workspace.id)
            .collect()
    } else {
        BTreeSet::new()
    };
    let monitors = raw_monitors
        .into_iter()
        .map(|monitor| {
            let mut workspaces: Vec<Workspace> = raw_workspaces
                .iter()
                .filter(|workspace| workspace.monitor == monitor.name)
                .filter_map(|workspace| {
                    let id = workspace.id?;
                    if id <= 0 {
                        return None;
                    }
                    Some(Workspace {
                        id,
                        occupied: workspace.windows > 0,
                        urgent: urgent.contains(&id),
                    })
                })
                .collect();
            workspaces.sort_by_key(|workspace| workspace.id);
            let active = monitor.active_workspace.id;
            let (width, height) = monitor.logical_size();
            Monitor {
                fullscreen: active.is_some_and(|active| fullscreen.contains(&active)),
                x: monitor.x,
                y: monitor.y,
                width,
                height,
                name: monitor.name,
                focused: monitor.focused,
                active,
                workspaces,
            }
        })
        .collect();
    Ok(Snapshot { monitors })
}

/// Events after which the windows are read again: they come, go or move.
fn moves_windows(event: &str) -> bool {
    const EVENTS: [&str; 12] = [
        "openwindow",
        "closewindow",
        "movewindowv2",
        "changefloatingmode",
        "pin",
        "moveintogroup",
        "moveoutofgroup",
        "moveworkspacev2",
        "renameworkspace",
        "changeworkspaceid",
        "monitoraddedv2",
        "monitorremovedv2",
    ];
    EVENTS.contains(&event)
}

fn affects_workspaces(event: &str) -> bool {
    const EVENTS: [&str; 15] = [
        "fullscreen",
        "workspace",
        "workspacev2",
        "focusedmon",
        "focusedmonv2",
        "createworkspacev2",
        "destroyworkspacev2",
        "moveworkspacev2",
        "renameworkspace",
        "changeworkspaceid",
        "monitoraddedv2",
        "monitorremovedv2",
        "openwindow",
        "closewindow",
        "movewindowv2",
    ];
    EVENTS.contains(&event)
}

/// What following Hyprland's events keeps: the windows, which ask for attention, and the
/// workspaces last published.
struct Follower<'a> {
    feed: &'a Feed<Snapshot>,
    windows: Vec<Window>,
    /// Addresses without `0x`: a window stays urgent until it is focused or its workspace shown.
    urgent: BTreeSet<String>,
    last: Snapshot,
}

impl Follower<'_> {
    fn read_windows(&mut self) {
        match query_windows() {
            Ok(windows) => self.windows = windows,
            Err(error) => warn!("cannot read the windows: {error}"),
        }
    }

    fn knows(&self, address: &str) -> Option<usize> {
        self.windows
            .iter()
            .position(|window| bare(&window.address) == address)
    }

    fn share_windows(&mut self) {
        for window in &mut self.windows {
            window.urgent = self.urgent.contains(bare(&window.address));
        }
        windows::set(&self.windows);
    }

    fn urgent_workspaces(&self) -> BTreeSet<i64> {
        self.windows
            .iter()
            .filter(|window| self.urgent.contains(bare(&window.address)))
            .filter_map(|window| match window.place {
                Place::Workspace(id) => Some(id),
                Place::Special(_) => None,
            })
            .collect()
    }

    /// Publishes the workspaces if they changed; whether they are still wanted.
    fn publish(&mut self) -> bool {
        let Ok(mut current) = snapshot(&self.urgent_workspaces()) else {
            return true;
        };
        let shown: Vec<i64> = current
            .monitors
            .iter()
            .filter_map(|monitor| monitor.active)
            .collect();
        let windows = &self.windows;
        let seen = |address: &String| {
            windows.iter().any(|window| {
                bare(&window.address) == address
                    && matches!(window.place, Place::Workspace(id) if shown.contains(&id))
            })
        };
        let before = self.urgent.len();
        self.urgent.retain(|address| !seen(address));
        if self.urgent.len() != before {
            self.share_windows();
            let urgent = self.urgent_workspaces();
            for workspace in current
                .monitors
                .iter_mut()
                .flat_map(|monitor| &mut monitor.workspaces)
            {
                workspace.urgent = urgent.contains(&workspace.id);
            }
        }
        if current == self.last {
            return true;
        }
        trace!(snapshot = ?current, "workspaces changed");
        self.last = current.clone();
        self.feed.publish(current)
    }

    /// Takes in one event; whether the workspaces are still wanted.
    fn event(&mut self, event: &str, data: &str) -> bool {
        let mut workspaces = affects_workspaces(event);
        match event {
            "activewindowv2" if !data.is_empty() => {
                workspaces |= self.urgent.remove(bare(data));
                match self.knows(data) {
                    Some(at) => self.windows[..=at].rotate_right(1),
                    None => self.read_windows(),
                }
            }
            "windowtitlev2" => {
                let (address, title) = data.split_once(',').unwrap_or((data, ""));
                match self.knows(address) {
                    Some(at) => title.clone_into(&mut self.windows[at].title),
                    None => self.read_windows(),
                }
            }
            "urgent" => {
                self.urgent.insert(bare(data).to_owned());
                if self.knows(bare(data)).is_none() {
                    self.read_windows();
                }
                workspaces = true;
            }
            _ if moves_windows(event) => {
                if event == "closewindow" {
                    self.urgent.remove(bare(data));
                }
                self.read_windows();
            }
            _ if !workspaces => return true,
            _ => {}
        }
        self.share_windows();
        !workspaces || self.publish()
    }
}

fn follow(events: UnixStream, feed: &Feed<Snapshot>) -> Ended {
    let mut follower = Follower {
        feed,
        windows: Vec::new(),
        urgent: BTreeSet::new(),
        last: Snapshot::default(),
    };
    follower.read_windows();
    follower.share_windows();
    if !follower.publish() {
        return Ended::Unwanted;
    }
    info!("following Hyprland's events");
    for line in BufReader::new(events).split(b'\n') {
        let line = match line {
            Ok(line) => line,
            Err(error) => return Ended::Failed(error.to_string()),
        };
        // Window titles in the stream can be any bytes.
        let line = String::from_utf8_lossy(&line);
        let (event, data) = line.split_once(">>").unwrap_or((&line, ""));
        if !follower.event(event, data) {
            return Ended::Unwanted;
        }
    }
    Ended::Failed("the event socket closed".to_owned())
}

pub struct Hyprland;

impl Compositor for Hyprland {
    const NAME: &'static str = "hyprland";

    fn detect() -> bool {
        env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
    }

    fn start(feed: Feed<Snapshot>) -> Option<Self> {
        let path = socket_dir()?.join(".socket2.sock");
        let mut events = Some(UnixStream::connect(&path).ok()?);
        feed.supervise(move |feed| {
            match events.take().map_or_else(|| UnixStream::connect(&path), Ok) {
                Ok(events) => follow(events, feed),
                Err(error) => Ended::Failed(error.to_string()),
            }
        });
        Some(Self)
    }

    fn focus_workspace(&self, id: i64) {
        if let Err(error) = dispatch(&format!("hl.dsp.focus({{workspace = {id}}})")) {
            warn!(workspace = id, "cannot switch workspace: {error}");
        }
    }

    fn focus_relative(&self, step: i32) {
        let target = format!("m{step:+}");
        if let Err(error) = dispatch(&format!(
            "hl.dsp.focus({{workspace = {}}})",
            lua_string(&target)
        )) {
            warn!(step, "cannot switch workspace: {error}");
        }
    }

    fn focus_window(&self, address: &str) {
        if let Err(error) = dispatch(&format!("hl.dsp.focus({{window = {}}})", window(address))) {
            warn!(window = address, "cannot focus the window: {error}");
        }
    }

    fn close_window(&self, address: &str) -> Result<(), String> {
        dispatch(&format!(
            "hl.dsp.window.close({{window = {}}})",
            window(address)
        ))
        .map_err(|error| format!("cannot close window {address}: {error}"))
    }

    fn top_gap(&self) -> Option<f32> {
        let option: serde_json::Value = query("j/getoption general:gaps_out").ok()?;
        option["css"]
            .as_str()
            .and_then(|css| css.split_whitespace().next())
            .and_then(|top| top.parse().ok())
            .or_else(|| option["int"].as_f64().map(|value| value as f32))
    }

    fn focused_class(&self) -> Option<String> {
        let window: serde_json::Value = query("j/activewindow").ok()?;
        window["class"]
            .as_str()
            .filter(|class| !class.is_empty())
            .map(str::to_owned)
    }

    fn focused_monitor(&self) -> Option<String> {
        let monitors: Vec<RawMonitor> = query("j/monitors").ok()?;
        monitors
            .into_iter()
            .find(|monitor| monitor.focused)
            .map(|monitor| monitor.name)
    }

    fn surface_origin(&self, monitor: &str) -> Option<(i32, i32)> {
        let mut outputs: HashMap<String, RawLayers> = query("j/layers").ok()?;
        let pid = std::process::id();
        outputs
            .remove(monitor)?
            .levels
            .into_values()
            .flatten()
            .find(|layer| layer.namespace == profile::namespace() && layer.pid == pid)
            .map(|layer| (layer.x, layer.y))
    }
}
