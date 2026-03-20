//! System integration domain
//!
//! Handles system-level integrations like clipboard operations.

pub mod clipboard;

pub use clipboard::{copy_image, copy_text};
