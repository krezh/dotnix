//! Compositor-agnostic window management for snap-to-window functionality
//!
//! This module provides a trait-based backend system that supports multiple Wayland compositors.
//! Each compositor can implement the `WindowBackend` trait to provide window information.
//!
//! ## Supported Compositors
//!
//! - **Hyprland**: Uses IPC socket for window detection (auto-detected via HYPRLAND_INSTANCE_SIGNATURE)
//! - **Other compositors**: Falls back to NullBackend (no window snapping)
//!
//! ## Fullscreen Window Support
//!
//! Unlike the original implementation, this version allows snapping to fullscreen windows.
//! The Layer::Overlay surface in wayland.rs ensures the selection overlay appears above
//! fullscreen applications, making it possible to select windows that are in fullscreen mode.
//!
//! ## Architecture
//!
//! - `WindowInfo`: Common window representation across all backends
//! - `WindowBackend`: Trait for compositor-specific implementations
//! - `WindowManager`: High-level interface that auto-detects and uses the appropriate backend
//! - `HyprlandBackend`: Hyprland IPC implementation
//! - `NullBackend`: Fallback when no supported compositor is detected

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

use crate::selection::Rect;

// ============================================================================
// Common Types
// ============================================================================

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub rect: Rect,
    pub monitor_id: i32,
    pub workspace_id: i32,
    pub mapped: bool,
    pub hidden: bool,
    pub fullscreen: bool,
}

impl WindowInfo {
    /// Checks if this window should be considered for snap targeting.
    pub fn is_snappable(&self) -> bool {
        // Allow fullscreen windows to be snapped to
        self.mapped && !self.hidden
    }
}

// ============================================================================
// Backend Trait
// ============================================================================

pub trait WindowBackend {
    /// Fetch all visible windows from the compositor
    fn fetch_windows(&mut self) -> Result<Vec<WindowInfo>>;

    /// Get the current cursor position
    fn get_cursor_position(&self) -> Result<(i32, i32)>;

    /// Get active workspace per monitor
    fn get_active_workspaces(&self) -> HashMap<i32, i32>;

    /// Check if this backend is available (compositor detection)
    fn is_available() -> bool
    where
        Self: Sized;
}

// ============================================================================
// Window Manager
// ============================================================================

pub struct WindowManager {
    backend: Box<dyn WindowBackend>,
    pub windows: Vec<WindowInfo>,
    pub active_workspaces: HashMap<i32, i32>,
}

impl WindowManager {
    /// Create a new WindowManager with auto-detected backend
    pub fn new() -> Result<Self> {
        let backend = Self::detect_backend()?;
        let mut manager = Self {
            backend,
            windows: Vec::new(),
            active_workspaces: HashMap::new(),
        };
        manager.refresh()?;
        Ok(manager)
    }

    /// Detect which compositor backend to use
    fn detect_backend() -> Result<Box<dyn WindowBackend>> {
        if HyprlandBackend::is_available() {
            log::info!("Detected Hyprland compositor, using IPC backend");
            return Ok(Box::new(HyprlandBackend::new()?));
        }

        log::info!("No specific compositor detected, window snapping disabled");
        Ok(Box::new(NullBackend))
    }

    /// Refresh window list and workspaces
    pub fn refresh(&mut self) -> Result<()> {
        self.windows = self.backend.fetch_windows()?;
        self.active_workspaces = self.backend.get_active_workspaces();

        log::info!("WindowManager: fetched {} windows", self.windows.len());
        log::info!(
            "WindowManager: active workspaces per monitor: {:?}",
            self.active_workspaces
        );

        let snappable: Vec<_> = self.snappable_windows().collect();
        log::info!(
            "WindowManager: {} snappable windows on active workspaces",
            snappable.len()
        );
        for (i, w) in snappable.iter().enumerate() {
            log::debug!(
                "  Window {}: monitor {} workspace {} at ({}, {}) size {}x{} fullscreen={}",
                i,
                w.monitor_id,
                w.workspace_id,
                w.rect.x,
                w.rect.y,
                w.rect.width,
                w.rect.height,
                w.fullscreen
            );
        }

        Ok(())
    }

    /// Get current cursor position
    pub fn get_cursor_position(&self) -> Result<(i32, i32)> {
        self.backend.get_cursor_position()
    }

    /// Filter for windows that are snappable on their monitor's active workspace
    fn snappable_windows(&self) -> impl Iterator<Item = &WindowInfo> {
        self.windows.iter().filter(|w| {
            w.is_snappable()
                && self
                    .active_workspaces
                    .get(&w.monitor_id)
                    .map(|ws_id| w.workspace_id == *ws_id)
                    .unwrap_or(false)
        })
    }

    /// Calculate distance from point to rectangle edge
    #[inline]
    fn distance_to_rect(x: i32, y: i32, rect: &Rect) -> i32 {
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

        ((dx * dx + dy * dy) as f64).sqrt() as i32
    }

    /// Find the window at a specific point (x, y) on the active workspace of its monitor
    pub fn find_window_at_point(&self, x: i32, y: i32) -> Option<&WindowInfo> {
        self.snappable_windows().find(|w| {
            let rect = &w.rect;
            x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
        })
    }

    /// Find the nearest window within a threshold distance on the active workspace of its monitor
    pub fn find_nearest_window(&self, x: i32, y: i32, threshold: i32) -> Option<&WindowInfo> {
        if let Some(window) = self.find_window_at_point(x, y) {
            return Some(window);
        }

        self.snappable_windows()
            .filter_map(|window| {
                let rect = &window.rect;
                let distance = Self::distance_to_rect(x, y, rect);
                (distance <= threshold).then_some((window, distance))
            })
            .min_by_key(|(_, distance)| *distance)
            .map(|(w, _)| w)
    }
}

// ============================================================================
// Hyprland Backend (IPC-based)
// ============================================================================

#[derive(Debug, Deserialize, Clone)]
struct WorkspaceInfo {
    id: i32,
    #[allow(dead_code)]
    name: String,
}

#[derive(Debug, Deserialize, Clone)]
struct HyprlandWindow {
    at: [i32; 2],
    size: [i32; 2],
    workspace: WorkspaceInfo,
    monitor: i32,
    mapped: bool,
    hidden: bool,
    #[serde(default)]
    fullscreen: i32,
}

struct HyprlandBackend {
    runtime_dir: String,
    instance_sig: String,
}

impl HyprlandBackend {
    fn new() -> Result<Self> {
        let runtime_dir = env::var("XDG_RUNTIME_DIR")
            .or_else(|_| env::var("TMPDIR"))
            .unwrap_or_else(|_| "/tmp".to_string());

        let instance_sig = env::var("HYPRLAND_INSTANCE_SIGNATURE")
            .context("HYPRLAND_INSTANCE_SIGNATURE not set")?;

        Ok(Self {
            runtime_dir,
            instance_sig,
        })
    }

    fn socket_command(&self, command: &str) -> Result<String> {
        let socket_path = format!(
            "{}/hypr/{}/.socket.sock",
            self.runtime_dir, self.instance_sig
        );

        let mut stream = UnixStream::connect(&socket_path).context(format!(
            "Failed to connect to Hyprland socket at {}",
            socket_path
        ))?;

        let cmd = if command.starts_with("j/") {
            command.to_string()
        } else {
            format!("j/{}", command)
        };

        stream
            .write_all(cmd.as_bytes())
            .context("Failed to write to Hyprland socket")?;

        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .context("Failed to read from Hyprland socket")?;

        Ok(response)
    }
}

impl WindowBackend for HyprlandBackend {
    fn fetch_windows(&mut self) -> Result<Vec<WindowInfo>> {
        let response = self
            .socket_command("clients")
            .context("Failed to fetch window list from Hyprland")?;
        let windows: Vec<HyprlandWindow> =
            serde_json::from_str(&response).context("Failed to parse clients JSON response")?;

        Ok(windows
            .into_iter()
            .map(|w| WindowInfo {
                rect: Rect::new(w.at[0], w.at[1], w.size[0], w.size[1]),
                monitor_id: w.monitor,
                workspace_id: w.workspace.id,
                mapped: w.mapped,
                hidden: w.hidden,
                fullscreen: w.fullscreen != 0,
            })
            .collect())
    }

    fn get_cursor_position(&self) -> Result<(i32, i32)> {
        let response = self
            .socket_command("cursorpos")
            .context("Failed to fetch cursor position from Hyprland")?;

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

    fn get_active_workspaces(&self) -> HashMap<i32, i32> {
        let response = match self.socket_command("monitors") {
            Ok(r) => r,
            Err(e) => {
                log::warn!("Failed to fetch active workspaces: {}", e);
                return HashMap::new();
            }
        };

        #[derive(Deserialize)]
        struct MonitorInfo {
            id: i32,
            #[serde(rename = "activeWorkspace")]
            active_workspace: WorkspaceInfo,
        }

        let monitors: Vec<MonitorInfo> = match serde_json::from_str(&response) {
            Ok(m) => m,
            Err(e) => {
                log::warn!("Failed to parse monitors JSON: {}", e);
                return HashMap::new();
            }
        };

        let mut active_workspaces = HashMap::new();
        for monitor in monitors {
            active_workspaces.insert(monitor.id, monitor.active_workspace.id);
        }

        active_workspaces
    }

    fn is_available() -> bool {
        env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
    }
}

// ============================================================================
// Null Backend (fallback when no compositor is detected)
// ============================================================================

struct NullBackend;

impl WindowBackend for NullBackend {
    fn fetch_windows(&mut self) -> Result<Vec<WindowInfo>> {
        Ok(Vec::new())
    }

    fn get_cursor_position(&self) -> Result<(i32, i32)> {
        Ok((0, 0))
    }

    fn get_active_workspaces(&self) -> HashMap<i32, i32> {
        HashMap::new()
    }

    fn is_available() -> bool {
        true
    }
}
