//! Generic Wayland protocols
//!
//! Compositor-agnostic Wayland protocol implementations that work across all compositors.

pub mod outputs;
mod screencopy;
mod shm;

pub use outputs::get_outputs;
pub use screencopy::capture_output;
