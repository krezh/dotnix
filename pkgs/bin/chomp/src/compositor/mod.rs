//! Compositor integration domain
//!
//! Provides both generic Wayland protocols and compositor-specific functionality.
//!
//! ## Structure
//!
//! - **protocol/** - Generic Wayland protocols (screencopy, outputs) that work everywhere
//! - **backend/** - Compositor-specific implementations (Hyprland, Niri, Sway, etc.)
//!
//! ## Adding New Compositor Support
//!
//! 1. Create `backend/yourcompositor.rs`
//! 2. Implement window/monitor query functions
//! 3. Add detection logic to `detect_compositor()`
//! 4. Export public API in this module

pub mod backend;
pub mod protocol;

// Re-export commonly used protocol functions
pub use protocol::{capture_output, get_outputs};

// Re-export compositor-specific functions (currently only Hyprland)
pub use backend::hyprland::{get_active_monitor, get_active_window};

/// Detected compositor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compositor {
    Hyprland,
    Niri,
    Sway,
    Unknown,
}

impl Compositor {
    /// Returns the human-readable name of the compositor.
    pub fn name(&self) -> &'static str {
        match self {
            Compositor::Hyprland => "Hyprland",
            Compositor::Niri => "Niri",
            Compositor::Sway => "Sway",
            Compositor::Unknown => "Unknown",
        }
    }
}

/// Detects the currently running compositor.
///
/// Detection is based on environment variables:
/// - `HYPRLAND_INSTANCE_SIGNATURE` → Hyprland
/// - `NIRI_SOCKET` → Niri  
/// - `SWAYSOCK` → Sway
pub fn detect_compositor() -> Compositor {
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        Compositor::Hyprland
    } else if std::env::var("NIRI_SOCKET").is_ok() {
        Compositor::Niri
    } else if std::env::var("SWAYSOCK").is_ok() {
        Compositor::Sway
    } else {
        Compositor::Unknown
    }
}
