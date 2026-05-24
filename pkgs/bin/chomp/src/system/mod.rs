//! System integration domain
//!
//! Handles system-level integrations like clipboard operations.

pub mod annotate;
pub mod clipboard;

pub use annotate::annotate;
pub use clipboard::{copy_image, copy_text};
