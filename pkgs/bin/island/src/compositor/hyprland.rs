use std::{
    collections::{BTreeSet, HashMap},
    env,
    io::{self, BufRead, BufReader},
    os::unix::net::UnixStream,
};

use serde::Deserialize;
use tracing::{info, trace, warn};

use super::{
    Compositor,
    hyprctl::{self, dispatch, lua_string, query},
    windows,
};
use crate::{
    features::Feed,
    profile,
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
        (
            (width / scale).round() as i32,
            (height / scale).round() as i32,
        )
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
fn windows_of(clients: &[RawClient], monitors: &[RawMonitor]) -> Vec<Window> {
    let mut clients: Vec<&RawClient> = clients
        .iter()
        .filter(|client| client.mapped && !client.hidden)
        .collect();
    clients.sort_by_key(|client| client.focus_history);
    clients
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
                address: client.address.clone(),
                id: client.stable_id.clone(),
                class: client.class.clone(),
                title: client.title.clone(),
                place,
                monitor: monitor.name.clone(),
                x: client.at[0] - monitor.x,
                y: client.at[1] - monitor.y,
                width: client.size[0],
                height: client.size[1],
                floating: client.floating,
            })
        })
        .collect()
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

/// `clients` when already read, so they are not asked for twice.
fn snapshot(
    urgent: &BTreeSet<i64>,
    raw_monitors: Vec<RawMonitor>,
    clients: Option<&[RawClient]>,
) -> io::Result<Snapshot> {
    let raw_workspaces: Vec<RawWorkspace> = query("j/workspaces")?;
    // `hasfullscreen` is also set for maximized windows, which leave the bar visible.
    let fullscreen: BTreeSet<i64> = if raw_workspaces
        .iter()
        .any(|workspace| workspace.hasfullscreen)
    {
        let queried;
        let clients = match clients {
            Some(clients) => clients,
            None => {
                queried = query::<Vec<RawClient>>("j/clients")?;
                &queried
            }
        };
        clients
            .iter()
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

/// What the events read so far ask for, done once those already waiting are in too.
#[derive(Clone, Copy, Default)]
struct Wanted {
    share: bool,
    windows: bool,
    workspaces: bool,
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

    /// Reads what `wanted` asks for, each thing once; whether the workspaces are still wanted.
    fn catch_up(&mut self, wanted: Wanted) -> bool {
        let read = wanted.windows || wanted.workspaces;
        let monitors = if read {
            match query::<Vec<RawMonitor>>("j/monitors") {
                Ok(monitors) => Some(monitors),
                Err(error) => {
                    warn!("cannot read the monitors: {error}");
                    None
                }
            }
        } else {
            None
        };
        let clients = match &monitors {
            Some(monitors) if wanted.windows => match query::<Vec<RawClient>>("j/clients") {
                Ok(clients) => {
                    self.windows = windows_of(&clients, monitors);
                    Some(clients)
                }
                Err(error) => {
                    warn!("cannot read the windows: {error}");
                    None
                }
            },
            _ => None,
        };
        if wanted.share || read {
            self.share_windows();
        }
        match monitors {
            Some(monitors) if wanted.workspaces => self.publish(monitors, clients.as_deref()),
            _ => true,
        }
    }

    /// Publishes the workspaces if they changed; whether they are still wanted.
    fn publish(&mut self, monitors: Vec<RawMonitor>, clients: Option<&[RawClient]>) -> bool {
        let Ok(mut current) = snapshot(&self.urgent_workspaces(), monitors, clients) else {
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

    /// Takes in one event, adding to what is `wanted`.
    fn event(&mut self, event: &str, data: &str, wanted: &mut Wanted) {
        let mut workspaces = affects_workspaces(event);
        match event {
            "activewindowv2" if !data.is_empty() => {
                workspaces |= self.urgent.remove(bare(data));
                match self.knows(data) {
                    Some(at) => self.windows[..=at].rotate_right(1),
                    None => wanted.windows = true,
                }
            }
            "windowtitlev2" => {
                let (address, title) = data.split_once(',').unwrap_or((data, ""));
                match self.knows(address) {
                    Some(at) => title.clone_into(&mut self.windows[at].title),
                    None => wanted.windows = true,
                }
            }
            "urgent" => {
                self.urgent.insert(bare(data).to_owned());
                if self.knows(bare(data)).is_none() {
                    wanted.windows = true;
                }
                workspaces = true;
            }
            _ if moves_windows(event) => {
                if event == "closewindow" {
                    self.urgent.remove(bare(data));
                }
                wanted.windows = true;
            }
            _ if !workspaces => return,
            _ => {}
        }
        wanted.share = true;
        wanted.workspaces |= workspaces;
    }
}

fn follow(events: UnixStream, feed: &Feed<Snapshot>) -> Ended {
    let mut follower = Follower {
        feed,
        windows: Vec::new(),
        urgent: BTreeSet::new(),
        last: Snapshot::default(),
    };
    let everything = Wanted {
        share: true,
        windows: true,
        workspaces: true,
    };
    if !follower.catch_up(everything) {
        return Ended::Unwanted;
    }
    info!("following Hyprland's events");
    let mut events = BufReader::new(events);
    let mut line = Vec::new();
    let mut wanted = Wanted::default();
    loop {
        line.clear();
        match events.read_until(b'\n', &mut line) {
            Ok(0) => return Ended::Failed("the event socket closed".to_owned()),
            Ok(_) => {}
            Err(error) => return Ended::Failed(error.to_string()),
        }
        // Window titles in the stream can be any bytes.
        let text = String::from_utf8_lossy(line.strip_suffix(b"\n").unwrap_or(&line));
        let (event, data) = text.split_once(">>").unwrap_or((&text, ""));
        follower.event(event, data, &mut wanted);
        // A burst, such as the several events of one workspace switch, is read in once it is all in.
        if events.buffer().is_empty() && !follower.catch_up(std::mem::take(&mut wanted)) {
            return Ended::Unwanted;
        }
    }
}

pub struct Hyprland;

impl Compositor for Hyprland {
    const NAME: &'static str = "hyprland";

    fn detect() -> bool {
        env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
    }

    fn start(feed: Feed<Snapshot>) -> Option<Self> {
        let mut events = Some(hyprctl::events().ok()?);
        feed.supervise(
            move |feed| match events.take().map_or_else(hyprctl::events, Ok) {
                Ok(events) => follow(events, feed),
                Err(error) => Ended::Failed(error.to_string()),
            },
        );
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
