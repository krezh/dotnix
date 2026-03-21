//! Rendering domain
//!
//! Handles Cairo-based rendering, pixel format conversion, and selection state management.

pub mod cairo;
pub mod pixel;
pub mod selection;

pub use cairo::{RenderConfig, Renderer};
pub use pixel::convert_argb_to_rgba;
pub use selection::{Rect, Selection};
