//! Compositor integration domain

pub mod backend;
pub mod protocol;

pub use protocol::{Screencopy, get_outputs};

use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compositor {
    Hyprland,
    Niri,
    Unknown,
}

impl Compositor {
    pub fn name(self) -> &'static str {
        match self {
            Self::Hyprland => "Hyprland",
            Self::Niri => "Niri",
            Self::Unknown => "Unknown",
        }
    }
    pub fn supports_window_capture(self) -> bool {
        matches!(self, Self::Hyprland | Self::Niri)
    }
}

pub fn detect_compositor() -> Compositor {
    if hyprland_socket_exists() {
        Compositor::Hyprland
    } else if env_socket_exists("NIRI_SOCKET") {
        Compositor::Niri
    } else {
        Compositor::Unknown
    }
}

pub fn get_active_window() -> Result<String> {
    match detect_compositor() {
        Compositor::Hyprland => backend::hyprland::get_active_window(),
        Compositor::Niri => backend::niri::get_active_window(),
        Compositor::Unknown => {
            anyhow::bail!("Active-window capture is unavailable for the current compositor")
        }
    }
}

pub fn get_active_monitor() -> Result<String> {
    match detect_compositor() {
        Compositor::Hyprland => backend::hyprland::get_active_monitor(),
        Compositor::Niri => backend::niri::get_active_monitor(),
        Compositor::Unknown => {
            anyhow::bail!("Active-output detection is unavailable for the current compositor")
        }
    }
}

fn env_socket_exists(variable: &str) -> bool {
    std::env::var_os(variable).is_some_and(|path| Path::new(&path).exists())
}

fn hyprland_socket_exists() -> bool {
    let (Some(runtime), Some(signature)) = (
        std::env::var_os("XDG_RUNTIME_DIR"),
        std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE"),
    ) else {
        return false;
    };
    Path::new(&runtime)
        .join("hypr")
        .join(signature)
        .join(".socket.sock")
        .exists()
}
