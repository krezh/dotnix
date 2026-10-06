//! Rendering domain
//!
//! Handles Cairo rendering, pixel operations, and selection state.

pub mod cairo;
pub mod palette;
pub mod pixel;
pub mod selection;

pub use cairo::{FrozenFrame, RenderConfig, Renderer, SelectionHud, SelectionPurpose};
pub use palette::{ModePaletteLayout, PaletteAction};
pub use pixel::dim_argb;
pub use selection::{Rect, Selection};
