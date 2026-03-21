//! Utility functions for Wayland module

use crate::cli::Args;
use crate::render::{Renderer, RenderConfig};

// Event loop timeout
pub const IDLE_FRAME_TIMEOUT_MS: u64 = 33; // ~30 FPS when idle

/// Creates a renderer with the specified dimensions and styling configuration.
///
/// Returns `None` if renderer creation fails due to invalid color values or other configuration errors.
/// Expects all `Option` fields in args to be populated after merging with config.
pub fn create_renderer(width: i32, height: i32, args: &Args) -> Option<Renderer> {
    let config = RenderConfig::new(
        args.border_color.as_ref()?,
        args.border_thickness?,
        args.border_rounding?,
        args.dim_opacity?,
        args.font_family.as_ref()?.clone(),
        args.font_size?,
        args.font_weight?,
    )
    .ok()?;

    Some(Renderer::new(width, height, config))
}
