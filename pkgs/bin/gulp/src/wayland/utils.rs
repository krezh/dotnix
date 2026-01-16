//! Utility functions for Wayland module

use std::time::Duration;

use crate::cli::Args;
use crate::render::{Renderer, RenderConfig};

// Frame timing constants
pub const IDLE_FRAME_TIMEOUT_MS: u64 = 33; // ~30 FPS when idle
pub const ANIMATION_FRAME_TIMEOUT_MS: u64 = 8; // ~120 FPS when animating
pub const DEFAULT_REFRESH_RATE_MHZ: i32 = 60000;
pub const REFRESH_RATE_DIVIDER: i32 = 1000;

/// Converts FPS to frame duration in microseconds.
pub fn fps_to_duration(fps: u32) -> Duration {
    Duration::from_micros(1_000_000 / fps.max(1) as u64)
}

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
        args.font_weight.as_ref()?,
    )
    .ok()?;

    Some(Renderer::new(width, height, config))
}
