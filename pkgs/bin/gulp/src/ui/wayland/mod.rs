//! Wayland client implementation for screen region selection
//!
//! This module handles:
//! - Layer shell surface creation for fullscreen overlay (works above fullscreen windows)
//! - Pointer and keyboard event handling
//! - Multi-monitor rendering and synchronization
//! - Frame-rate limiting per monitor

use anyhow::{Context, Result};
use smithay_client_toolkit::reexports::calloop::{EventLoop, LoopSignal};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::{OutputInfo, OutputState},
    registry::RegistryState,
    seat::{pointer::ThemedPointer, SeatState},
    shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell},
    shm::{slot::SlotPool, Shm},
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_surface},
    Connection, QueueHandle,
};

use crate::{
    cli::Args,
    render::{Renderer, Selection},
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

mod capture;
mod handlers;
mod input;
mod output;
mod rendering;
mod utils;

use input::InputState;
use output::OutputSurface;
use utils::*;

/// Main application state managing Wayland connection and surfaces
pub struct App {
    // Wayland state
    pub(super) conn: Connection,
    pub(super) registry_state: RegistryState,
    pub(super) seat_state: SeatState,
    pub(super) output_state: OutputState,
    pub(super) compositor_state: CompositorState,
    pub(super) shm_state: Shm,
    pub(super) layer_shell: LayerShell,
    pub(super) themed_pointer: Option<ThemedPointer>,

    // Application state
    pub(super) outputs: HashMap<wl_output::WlOutput, OutputInfo>,
    pub(super) output_surfaces: Vec<OutputSurface>,
    pub(super) renderer: Option<Renderer>,
    pub(super) selection: Selection,
    pub(super) args: Args,

    // Input state
    pub(super) input: InputState,

    // Loop control
    pub(super) exit: bool,
    pub(super) loop_signal: LoopSignal,
    pub(super) needs_redraw: bool,
    
    // Selection result (for area modes)
    pub(super) selection_geometry: Option<String>,
}

// ============================================================================
// App Implementation - Initialization & Setup
// ============================================================================

impl App {
    pub fn run(args: Args) -> Result<Option<String>> {
        let conn = Connection::connect_to_env().context("Failed to connect to Wayland")?;
        let (globals, mut event_queue) =
            registry_queue_init::<Self>(&conn).context("Failed to init registry")?;
        let qh: QueueHandle<Self> = event_queue.handle();

        let registry_state = RegistryState::new(&globals);
        let seat_state = SeatState::new(&globals, &qh);
        let output_state = OutputState::new(&globals, &qh);
        let compositor_state =
            CompositorState::bind(&globals, &qh).context("wl_compositor not available")?;
        let shm_state = Shm::bind(&globals, &qh).context("wl_shm not available")?;
        let layer_shell =
            LayerShell::bind(&globals, &qh).context("zwlr_layer_shell not available")?;

        let selection = Selection::new();

        // Create event loop
        let mut event_loop: EventLoop<Self> = EventLoop::try_new()?;
        let loop_signal = event_loop.get_signal();

        // Get initial outputs
        let outputs: HashMap<_, _> = output_state
            .outputs()
            .filter_map(|output| {
                output_state
                    .info(&output)
                    .map(|info| (output, info.clone()))
            })
            .collect();

        let mut app = Self {
            conn: conn.clone(),
            registry_state,
            seat_state,
            output_state,
            compositor_state,
            shm_state,
            layer_shell,
            themed_pointer: None,
            outputs,
            output_surfaces: Vec::new(),
            renderer: None,
            selection,
            args,
            input: InputState::new(),
            exit: false,
            loop_signal,
            needs_redraw: false,
            selection_geometry: None,
        };

        // Dispatch any pending events
        event_queue.blocking_dispatch(&mut app)?;

        // Set up Wayland event source
        WaylandSource::new(conn.clone(), event_queue)
            .insert(event_loop.handle())
            .context("Failed to insert wayland source")?;

        // Create layer surfaces for each output
        app.create_layer_surfaces(&qh)?;

        // Capture frozen screenshots if freeze mode is enabled
        let should_freeze = app.args.freeze.unwrap_or(true);
        if should_freeze {
            app.capture_frozen_screens()?;
        }

        loop {
            event_loop.dispatch(Some(Duration::from_millis(IDLE_FRAME_TIMEOUT_MS)), &mut app)?;

            // Render if needed (throttled by frame rate limiting in redraw_all)
            if app.needs_redraw {
                app.needs_redraw = false;
                app.redraw_all(&qh);
            }

            if app.exit {
                break;
            }
        }

        // Return geometry if captured (for area modes)
        Ok(app.selection_geometry)
    }

    fn create_layer_surfaces(&mut self, qh: &QueueHandle<Self>) -> Result<()> {
        for (output, info) in &self.outputs {
            if let Some((width, height)) = info.logical_size {
                let (x, y) = info.logical_position.unwrap_or((0, 0));
                let surface = self.compositor_state.create_surface(qh);

                let layer_surface = self.layer_shell.create_layer_surface(
                    qh,
                    surface.clone(),
                    Layer::Overlay,
                    Some("gulp-selection"),
                    Some(output),
                );

                layer_surface.set_anchor(Anchor::TOP | Anchor::LEFT);
                layer_surface.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
                layer_surface.set_exclusive_zone(-1);
                layer_surface.set_size(width as u32, height as u32);
                layer_surface.set_margin(0, 0, 0, 0);

                surface.commit();

                // Create a dedicated pool for this output with space for double buffering
                // Double the size to allow for two buffers (reduces flickering)
                let pool_size = (width * height * 4 * 2) as usize;
                let pool = SlotPool::new(pool_size, &self.shm_state).ok();

                // Create a dedicated renderer for this output
                let renderer = create_renderer(width, height, &self.args);

                log::info!(
                    "Output {:?}: {}x{} at ({}, {})",
                    info.name,
                    width,
                    height,
                    x,
                    y
                );

                self.output_surfaces.push(OutputSurface {
                    _output: output.clone(),
                    layer_surface,
                    surface,
                    width: width as u32,
                    height: height as u32,
                    x,
                    y,
                    configured: false,
                    pool,
                    renderer,
                    frozen_buffer: None,
                    last_had_selection: false,
                    needs_render: true,
                    frame_callback: None,
                    waiting_for_frame: false,
                });
            }
        }

        // Initialize renderer with first output dimensions
        if let Some(first) = self.output_surfaces.first() {
            self.renderer = create_renderer(first.width as i32, first.height as i32, &self.args)
                .ok_or_else(|| anyhow::anyhow!("Failed to create renderer"))?
                .into();
        }

        Ok(())
    }

    /// Captures frozen screenshots of all outputs for freeze mode.
    fn capture_frozen_screens(&mut self) -> Result<()> {
        use crate::compositor::protocol::capture_output;

        log::info!("Capturing frozen screenshots for {} outputs", self.output_surfaces.len());

        for output_surface in &mut self.output_surfaces {
            match capture_output(&self.conn, &output_surface._output) {
                Ok(captured_image) => {
                    log::debug!(
                        "Captured frozen screen for output: {}x{}",
                        captured_image.width,
                        captured_image.height
                    );
                    output_surface.frozen_buffer = Some(captured_image);
                }
                Err(e) => {
                    log::warn!("Failed to capture frozen screen: {}. Continuing without freeze for this output.", e);
                }
            }
        }

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Rendering
    // ------------------------------------------------------------------------

    /// Redraws all monitors using frame callbacks for vsync.
    ///
    /// Frame callbacks ensure rendering is synchronized with compositor refresh.
    fn redraw_all(&mut self, qh: &QueueHandle<Self>) {
        for i in 0..self.output_surfaces.len() {
            if !self.output_surfaces[i].configured {
                continue;
            }

            let _ = self.draw_index(i, qh);
        }
    }

    pub(super) fn draw_index(&mut self, index: usize, qh: &QueueHandle<Self>) -> Result<()> {
        rendering::draw_output(&mut self.output_surfaces[index], &self.selection, qh)
    }

    // ------------------------------------------------------------------------
    // Event Handling
    // ------------------------------------------------------------------------

    pub(super) fn handle_pointer_move(&mut self, surface: &wl_surface::WlSurface, x: f64, y: f64) {
        let mut global_x = x;
        let mut global_y = y;

        for output_surface in &self.output_surfaces {
            if &output_surface.surface == surface {
                global_x = x + output_surface.x as f64;
                global_y = y + output_surface.y as f64;
                break;
            }
        }

        self.input.pointer_position = (global_x, global_y);

        if self.input.mouse_pressed {
            if let Some((start_x, start_y)) = self.input.selection_start {
                self.selection
                    .update_drag(start_x, start_y, global_x as i32, global_y as i32);
                self.needs_redraw = true;
            }
        }
    }

    pub(super) fn handle_pointer_button(&mut self, pressed: bool) {
        if pressed {
            self.input.mouse_pressed = true;
            self.input.selection_start = Some((
                self.input.pointer_position.0 as i32,
                self.input.pointer_position.1 as i32,
            ));
            self.selection.start_selection(
                self.input.pointer_position.0 as i32,
                self.input.pointer_position.1 as i32,
            );
        } else {
            self.input.mouse_pressed = false;
            if self.selection.get_selection().is_some() {
                self.complete_selection();
            }
        }
        self.needs_redraw = true;
    }

    fn complete_selection(&mut self) {
        if let Some(rect) = self.selection.get_selection() {
            // Collect outputs map for capture module
            let outputs_map: Vec<(wl_output::WlOutput, String)> = self
                .outputs
                .iter()
                .map(|(output, info)| (output.clone(), info.name.clone().unwrap_or_default()))
                .collect();

            // Handle selection completion
            match capture::complete_selection(
                &self.conn,
                &mut self.output_surfaces,
                &outputs_map,
                &self.args,
                rect,
            ) {
                Ok(geometry) => {
                    self.selection_geometry = geometry;
                }
                Err(e) => {
                    log::error!("Selection completion failed: {}", e);
                }
            }

            self.exit = true;
            self.loop_signal.stop();
        }
    }

    pub(super) fn cancel_selection(&mut self) {
        eprintln!("Selection cancelled by user");
        log::debug!("Selection cancelled by user");
        std::process::exit(1);
    }
}
