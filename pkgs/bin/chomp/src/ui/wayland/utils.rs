//! Utility functions for Wayland module

use crate::cli::Settings;
use crate::render::{RenderConfig, Renderer};

// Event loop timeout
pub const IDLE_FRAME_TIMEOUT_MS: u64 = 33; // ~30 FPS when idle

/// Creates a renderer with the specified dimensions and styling configuration.
///
/// Returns `None` if renderer creation fails due to invalid color values or other configuration errors.
pub fn create_renderer(width: i32, height: i32, settings: &Settings) -> Option<Renderer> {
    let config = RenderConfig::new(
        &settings.border_color,
        settings.border_thickness,
        settings.border_rounding,
        settings.dim_opacity,
        settings.font_family.clone(),
        settings.font_size,
        settings.font_weight,
    )
    .ok()?;

    Some(Renderer::new(width, height, config))
}
