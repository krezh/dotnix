use anyhow::{Context, Result};
use serde::Deserialize;
use std::process::Command;

#[derive(Deserialize)]
struct Output {
    name: String,
    logical: LogicalOutput,
}

#[derive(Deserialize)]
struct LogicalOutput {
    x: i32,
    y: i32,
}

#[derive(Deserialize)]
struct Window {
    layout: WindowLayout,
}

#[derive(Deserialize)]
struct WindowLayout {
    window_size: [i32; 2],
    tile_pos_in_workspace_view: Option<[f64; 2]>,
    window_offset_in_tile: [f64; 2],
}

pub fn get_active_monitor() -> Result<String> {
    Ok(query::<Output>("focused-output")?.name)
}

pub fn get_active_window() -> Result<String> {
    let output = query::<Output>("focused-output")?;
    let window = query::<Window>("focused-window")?;
    let tile = window
        .layout
        .tile_pos_in_workspace_view
        .context("Niri did not report the focused window position")?;
    let x = output.logical.x + (tile[0] + window.layout.window_offset_in_tile[0]).round() as i32;
    let y = output.logical.y + (tile[1] + window.layout.window_offset_in_tile[1]).round() as i32;
    Ok(format!(
        "{},{} {}x{}",
        x, y, window.layout.window_size[0], window.layout.window_size[1]
    ))
}

fn query<T: serde::de::DeserializeOwned>(request: &str) -> Result<T> {
    let output = Command::new("niri")
        .args(["msg", "--json", request])
        .output()
        .context("Failed to run niri IPC client")?;
    anyhow::ensure!(
        output.status.success(),
        "niri msg {} failed: {}",
        request,
        String::from_utf8_lossy(&output.stderr).trim()
    );
    serde_json::from_slice(&output.stdout)
        .with_context(|| format!("Failed to parse niri {} response", request))
}
