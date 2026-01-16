//! Per-output surface management and rendering

use smithay_client_toolkit::shm::slot::SlotPool;
use smithay_client_toolkit::shell::wlr_layer::LayerSurface;
use wayland_client::protocol::{wl_output, wl_surface};

use crate::render::Renderer;
use std::time::{Duration, Instant};

/// Represents a single monitor's overlay surface
pub struct OutputSurface {
    pub _output: wl_output::WlOutput,
    pub layer_surface: LayerSurface,
    pub surface: wl_surface::WlSurface,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub configured: bool,
    pub pool: Option<SlotPool>,
    pub renderer: Option<Renderer>,
    pub last_render: Instant,
    pub frame_time: Duration,
}
