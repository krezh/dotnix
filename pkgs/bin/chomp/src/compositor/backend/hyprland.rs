//! Hyprland window and monitor queries via Unix socket
//!
//! Uses direct socket communication instead of spawning hyprctl processes for better performance.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

use super::super::{detect_compositor, Compositor};

#[derive(Deserialize)]
struct HyprctlWindow {
    at: [i32; 2],
    size: [i32; 2],
}

#[derive(Deserialize)]
struct HyprctlWorkspace {
    monitor: String,
}

/// Sends a command to Hyprland's socket and returns the JSON response.
fn hyprctl_socket(command: &str) -> Result<String> {
    // Verify we're running under Hyprland
    let compositor = detect_compositor();
    if compositor != Compositor::Hyprland {
        anyhow::bail!(
            "Hyprland-specific features require Hyprland (detected: {}). Use generic Wayland protocols instead.",
            compositor.name()
        );
    }

    let signature = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .context("HYPRLAND_INSTANCE_SIGNATURE not set")?;
    
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .context("XDG_RUNTIME_DIR not set")?;
    
    let socket_path = format!("{}/hypr/{}/.socket.sock", runtime_dir, signature);
    
    let mut stream = UnixStream::connect(&socket_path)
        .context(format!("Failed to connect to Hyprland socket at {}", socket_path))?;
    
    // Send command with -j flag for JSON output
    let cmd = format!("-j/{}", command);
    stream.write_all(cmd.as_bytes())
        .context("Failed to write to Hyprland socket")?;
    
    // Read response
    let mut response = String::new();
    stream.read_to_string(&mut response)
        .context("Failed to read from Hyprland socket")?;
    
    Ok(response)
}

/// Gets the geometry of the active window as "x,y wxh" format.
pub fn get_active_window() -> Result<String> {
    let response = hyprctl_socket("activewindow")?;
    
    let window: HyprctlWindow = serde_json::from_str(&response)
        .context("Failed to parse activewindow response")?;

    Ok(format!(
        "{},{} {}x{}",
        window.at[0], window.at[1], window.size[0], window.size[1]
    ))
}

/// Gets the name of the active monitor.
pub fn get_active_monitor() -> Result<String> {
    let response = hyprctl_socket("activeworkspace")?;
    
    let workspace: HyprctlWorkspace = serde_json::from_str(&response)
        .context("Failed to parse activeworkspace response")?;

    Ok(workspace.monitor)
}
