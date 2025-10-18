use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

use crate::selection::Rect;

#[derive(Debug, Deserialize, Clone)]
pub struct WorkspaceInfo {
    pub id: i32,
    #[allow(dead_code)]
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HyprlandWindow {
    pub at: [i32; 2],
    pub size: [i32; 2],
    pub workspace: WorkspaceInfo,
    pub monitor: i32,
    pub mapped: bool,
    pub hidden: bool,
    #[serde(default)]
    pub fullscreen: i32,
}

impl HyprlandWindow {
    /// Convert to a Rect
    pub fn to_rect(&self) -> Rect {
        Rect::new(self.at[0], self.at[1], self.size[0], self.size[1])
    }

    /// Check if this window should be considered for snapping
    pub fn is_snappable(&self) -> bool {
        self.mapped && !self.hidden && self.fullscreen == 0
    }
}

pub struct WindowManager {
    pub windows: Vec<HyprlandWindow>,
    pub active_workspaces: HashMap<i32, i32>, // monitor_id -> workspace_id
}

impl WindowManager {
    /// Helper function to send a command to Hyprland socket and get response
    fn socket_command(command: &str) -> Result<String> {
        // Get the Hyprland socket path
        let runtime_dir = env::var("XDG_RUNTIME_DIR")
            .or_else(|_| env::var("TMPDIR"))
            .unwrap_or_else(|_| "/tmp".to_string());

        let instance_sig = env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .context("HYPRLAND_INSTANCE_SIGNATURE not set - is Hyprland running?")?;

        let socket_path = format!("{}/hypr/{}/.socket.sock", runtime_dir, instance_sig);

        // Connect to the socket
        let mut stream = UnixStream::connect(&socket_path).context(format!(
            "Failed to connect to Hyprland socket at {}",
            socket_path
        ))?;

        // Send the command (with 'j/' prefix for JSON responses)
        let cmd = if command.starts_with("j/") {
            command.to_string()
        } else {
            format!("j/{}", command)
        };

        stream
            .write_all(cmd.as_bytes())
            .context("Failed to write to Hyprland socket")?;

        // Read the response
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .context("Failed to read from Hyprland socket")?;

        Ok(response)
    }

    /// Create a new WindowManager and fetch current windows
    pub fn new() -> Result<Self> {
        let windows = Self::fetch_windows()?;
        let active_workspaces = Self::fetch_active_workspaces()?;

        log::info!(
            "WindowManager: fetched {} windows from Hyprland",
            windows.len()
        );
        log::info!(
            "WindowManager: active workspaces per monitor: {:?}",
            active_workspaces
        );

        // Log snappable windows on active workspaces for debugging
        let snappable: Vec<_> = windows
            .iter()
            .filter(|w| {
                w.is_snappable()
                    && active_workspaces
                        .get(&w.monitor)
                        .map(|ws_id| w.workspace.id == *ws_id)
                        .unwrap_or(false)
            })
            .collect();
        log::info!(
            "WindowManager: {} snappable windows on active workspaces",
            snappable.len()
        );
        for (i, w) in snappable.iter().enumerate() {
            let r = w.to_rect();
            log::debug!(
                "  Window {}: monitor {} workspace {} at ({}, {}) size {}x{}",
                i,
                w.monitor,
                w.workspace.id,
                r.x,
                r.y,
                r.width,
                r.height
            );
        }

        Ok(Self {
            windows,
            active_workspaces,
        })
    }

    /// Get current cursor position from Hyprland via socket IPC
    pub fn get_cursor_position() -> Result<(i32, i32)> {
        let response = Self::socket_command("cursorpos")?;

        #[derive(Deserialize)]
        struct CursorPos {
            x: i32,
            y: i32,
        }

        let pos: CursorPos = serde_json::from_str(&response)
            .context("Failed to parse cursor position JSON response")?;

        log::debug!(
            "Got cursor position from Hyprland socket: ({}, {})",
            pos.x,
            pos.y
        );
        Ok((pos.x, pos.y))
    }

    /// Fetch all windows from Hyprland via socket IPC
    fn fetch_windows() -> Result<Vec<HyprlandWindow>> {
        let response = Self::socket_command("clients")?;

        let windows: Vec<HyprlandWindow> =
            serde_json::from_str(&response).context("Failed to parse clients JSON response")?;

        Ok(windows)
    }

    /// Fetch the active workspaces per monitor from Hyprland via socket IPC
    fn fetch_active_workspaces() -> Result<HashMap<i32, i32>> {
        let response = Self::socket_command("monitors")?;

        #[derive(Deserialize)]
        struct MonitorInfo {
            id: i32,
            #[serde(rename = "activeWorkspace")]
            active_workspace: WorkspaceInfo,
        }

        let monitors: Vec<MonitorInfo> =
            serde_json::from_str(&response).context("Failed to parse monitors JSON response")?;

        let mut active_workspaces = HashMap::new();
        for monitor in monitors {
            active_workspaces.insert(monitor.id, monitor.active_workspace.id);
        }

        Ok(active_workspaces)
    }

    /// Find the window at a specific point (x, y) on the active workspace of its monitor
    pub fn find_window_at_point(&self, x: i32, y: i32) -> Option<&HyprlandWindow> {
        self.windows
            .iter()
            .filter(|w| {
                w.is_snappable()
                    && self
                        .active_workspaces
                        .get(&w.monitor)
                        .map(|ws_id| w.workspace.id == *ws_id)
                        .unwrap_or(false)
            })
            .find(|w| {
                let rect = w.to_rect();
                x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
            })
    }

    /// Find the nearest window within a threshold distance on the active workspace of its monitor
    pub fn find_nearest_window(&self, x: i32, y: i32, threshold: i32) -> Option<&HyprlandWindow> {
        // First check if we're directly inside a window
        if let Some(window) = self.find_window_at_point(x, y) {
            return Some(window);
        }

        // Find the nearest window within threshold on active workspaces
        let mut nearest: Option<(&HyprlandWindow, i32)> = None;

        for window in self.windows.iter().filter(|w| {
            w.is_snappable()
                && self
                    .active_workspaces
                    .get(&w.monitor)
                    .map(|ws_id| w.workspace.id == *ws_id)
                    .unwrap_or(false)
        }) {
            let rect = window.to_rect();

            // Calculate distance to window rectangle
            let dx = if x < rect.x {
                rect.x - x
            } else if x > rect.x + rect.width {
                x - (rect.x + rect.width)
            } else {
                0
            };

            let dy = if y < rect.y {
                rect.y - y
            } else if y > rect.y + rect.height {
                y - (rect.y + rect.height)
            } else {
                0
            };

            let distance = ((dx * dx + dy * dy) as f64).sqrt() as i32;

            if distance <= threshold {
                if let Some((_, nearest_dist)) = nearest {
                    if distance < nearest_dist {
                        nearest = Some((window, distance));
                    }
                } else {
                    nearest = Some((window, distance));
                }
            }
        }

        nearest.map(|(w, _)| w)
    }
}
