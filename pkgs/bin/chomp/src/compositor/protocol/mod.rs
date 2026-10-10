//! Generic Wayland protocols
//!
//! Compositor-agnostic Wayland protocol implementations that work across all compositors.

pub mod hyprland_toplevel;
pub mod outputs;
mod screencopy;
mod shm;
mod video_capture;

pub use outputs::get_outputs;
pub use screencopy::Screencopy;
pub use video_capture::VideoCapture;
