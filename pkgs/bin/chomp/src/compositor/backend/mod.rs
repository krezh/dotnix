//! Compositor-specific backend implementations
//!
//! Each compositor may have its own IPC protocol or API for querying windows,
//! workspaces, and other compositor-specific features.

pub mod hyprland;
pub mod niri;
