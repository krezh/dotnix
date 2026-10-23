use std::{
    collections::{BTreeSet, HashMap},
    env,
    io::{self, BufRead, BufReader, Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
};

use serde::Deserialize;
use tracing::{info, trace, warn};

use super::Compositor;
use crate::{
    features::Feed,
    shell::NAMESPACE,
    supervisor::Ended,
    wm::{Monitor, Snapshot, Workspace},
};

#[derive(Deserialize)]
struct RawMonitor {
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
    fn logical_width(&self) -> i32 {
        let width = if self.transform % 2 == 1 {
            self.height
        } else {
            self.width
        };
        (width / self.scale.max(0.1)).round() as i32
    }
}

#[derive(Deserialize)]
struct RawWorkspaceRef {
    id: i64,
}

#[derive(Deserialize)]
struct RawWorkspace {
    id: i64,
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
    let runtime = env::var_os("XDG_RUNTIME_DIR")?;
    Some(PathBuf::from(runtime).join("hypr").join(signature))
}

fn request(command: &str) -> io::Result<String> {
    let path = socket_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not running under Hyprland"))?
        .join(".socket.sock");
    let mut stream = UnixStream::connect(path)?;
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
            .map(|client| client.workspace.id)
            .collect()
    } else {
        BTreeSet::new()
    };
    let monitors = raw_monitors
        .into_iter()
        .map(|monitor| {
            let mut workspaces: Vec<Workspace> = raw_workspaces
                .iter()
                .filter(|workspace| workspace.monitor == monitor.name && workspace.id > 0)
                .map(|workspace| Workspace {
                    id: workspace.id,
                    occupied: workspace.windows > 0,
                    urgent: urgent.contains(&workspace.id),
                })
                .collect();
            workspaces.sort_by_key(|workspace| workspace.id);
            let active = monitor.active_workspace.id;
            Monitor {
                fullscreen: fullscreen.contains(&active),
                x: monitor.x,
                y: monitor.y,
                width: monitor.logical_width(),
                name: monitor.name,
                focused: monitor.focused,
                active: Some(active),
                workspaces,
            }
        })
        .collect();
    Ok(Snapshot { monitors })
}

fn workspace_of(address: &str) -> Option<i64> {
    let address = address.trim_start_matches("0x");
    let clients: Vec<RawClient> = query("j/clients").ok()?;
    clients
        .into_iter()
        .find(|client| client.address.trim_start_matches("0x") == address)
        .map(|client| client.workspace.id)
}

fn affects_workspaces(event: &str) -> bool {
    const EVENTS: [&str; 14] = [
        "fullscreen",
        "workspace",
        "workspacev2",
        "focusedmon",
        "focusedmonv2",
        "createworkspacev2",
        "destroyworkspacev2",
        "moveworkspacev2",
        "renameworkspace",
        "monitoraddedv2",
        "monitorremovedv2",
        "openwindow",
        "closewindow",
        "movewindowv2",
    ];
    EVENTS.contains(&event)
}

fn follow(events: UnixStream, feed: &Feed<Snapshot>) -> Ended {
    let mut urgent = BTreeSet::new();
    let mut last = Snapshot::default();
    let publish = |urgent: &mut BTreeSet<i64>, last: &mut Snapshot| {
        let Ok(mut current) = snapshot(urgent) else {
            return true;
        };
        for monitor in &mut current.monitors {
            let Some(active) = monitor.active else {
                continue;
            };
            if urgent.remove(&active) {
                for workspace in &mut monitor.workspaces {
                    workspace.urgent &= workspace.id != active;
                }
            }
        }
        if current == *last {
            return true;
        }
        trace!(snapshot = ?current, "workspaces changed");
        *last = current.clone();
        feed.publish(current)
    };
    if !publish(&mut urgent, &mut last) {
        return Ended::Unwanted;
    }
    info!("following Hyprland's events");
    for line in BufReader::new(events).lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => return Ended::Failed(error.to_string()),
        };
        let (event, data) = line.split_once(">>").unwrap_or((line.as_str(), ""));
        if event == "urgent" {
            match workspace_of(data) {
                Some(id) => urgent.insert(id),
                None => continue,
            };
        } else if !affects_workspaces(event) {
            continue;
        }
        if !publish(&mut urgent, &mut last) {
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
            .find(|layer| layer.namespace == NAMESPACE && layer.pid == pid)
            .map(|layer| (layer.x, layer.y))
    }
}
