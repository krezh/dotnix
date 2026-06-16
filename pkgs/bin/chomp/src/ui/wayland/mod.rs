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
    seat::{SeatState, pointer::ThemedPointer},
    shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell},
    shm::{Shm, slot::SlotPool},
};
use wayland_client::{
    Connection, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_output, wl_surface},
};

use crate::{
    capture::{CaptureMode, CapturedImage},
    cli::Settings,
    render::{Renderer, Selection},
};
use std::collections::HashMap;
use std::time::Duration;

mod capture;
mod handlers;
mod input;
mod output;
mod rendering;
mod utils;

use input::InputState;
use output::OutputSurface;
use utils::*;

#[derive(PartialEq)]
pub(super) enum UiPhase {
    ModeSelect,
    RegionSelect,
}

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
    pub(super) settings: Settings,

    // Input state
    pub(super) input: InputState,

    // Loop control
    pub(super) exit: bool,
    pub(super) loop_signal: LoopSignal,
    pub(super) needs_redraw: bool,

    // Selection result (for area modes)
    pub(super) selection_geometry: Option<String>,

    // Error from selection completion, returned from run()
    pub(super) completion_error: Option<anyhow::Error>,

    // Mode selector state
    pub(super) phase: UiPhase,
    pub(super) chosen_mode: Option<CaptureMode>,

    // Pre-captured image for non-area modes (captured on the same connection as the UI
    // to avoid cross-connection ordering races with the compositor)
    pub(super) captured_image: Option<CapturedImage>,

    // True if a wl-screenrec recording is already running when the selector opens
    pub(super) is_recording: bool,
}

// ============================================================================
// App Implementation - Initialization & Setup
// ============================================================================

impl App {
    pub fn run(
        settings: Settings,
    ) -> Result<(Option<String>, Option<CaptureMode>, Option<CapturedImage>)> {
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

        let outputs: HashMap<_, _> = output_state
            .outputs()
            .filter_map(|output| {
                output_state
                    .info(&output)
                    .map(|info| (output, info.clone()))
            })
            .collect();

        let phase = if settings.mode.is_none() {
            UiPhase::ModeSelect
        } else {
            UiPhase::RegionSelect
        };

        let is_recording = crate::capture::VideoRecorder::new()
            .is_recording()
            .map(|(r, _)| r)
            .unwrap_or(false);

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
            settings,
            input: InputState::new(),
            exit: false,
            loop_signal,
            needs_redraw: false,
            selection_geometry: None,
            completion_error: None,
            phase,
            chosen_mode: None,
            captured_image: None,
            is_recording,
        };

        event_queue.blocking_dispatch(&mut app)?;

        WaylandSource::new(conn.clone(), event_queue)
            .insert(event_loop.handle())
            .context("Failed to insert wayland source")?;

        app.create_layer_surfaces(&qh)?;

        // Capture frozen backgrounds before any buffer is attached to any surface.
        // At this point layer surfaces exist (surface.commit() with no buffer), so
        // they are not yet visible — the compositor cannot include them in a rendered
        // frame.  Capturing here works for both ModeSelect (mode dialog launched first)
        // and RegionSelect (--mode image-area), avoiding the race where capture happens
        // after the overlay is already on screen.
        if app.settings.freeze {
            app.capture_frozen_screens()?;
        }

        loop {
            event_loop.dispatch(Some(Duration::from_millis(IDLE_FRAME_TIMEOUT_MS)), &mut app)?;

            if app.needs_redraw {
                app.needs_redraw = false;
                app.redraw_all(&qh);
            }

            if app.exit {
                break;
            }
        }

        if let Some(e) = app.completion_error {
            return Err(e);
        }

        Ok((app.selection_geometry, app.chosen_mode, app.captured_image))
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
                    Some("chomp-selection"),
                    Some(output),
                );

                layer_surface.set_anchor(Anchor::TOP | Anchor::LEFT);
                layer_surface.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
                layer_surface.set_exclusive_zone(-1);
                layer_surface.set_size(width as u32, height as u32);
                layer_surface.set_margin(0, 0, 0, 0);

                surface.commit();

                let pool_size = (width * height * 4 * 2) as usize;
                let pool = SlotPool::new(pool_size, &self.shm_state).ok();

                let renderer = create_renderer(width, height, &self.settings);

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

        if let Some(first) = self.output_surfaces.first() {
            self.renderer =
                create_renderer(first.width as i32, first.height as i32, &self.settings)
                    .ok_or_else(|| anyhow::anyhow!("Failed to create renderer"))?
                    .into();
        }

        Ok(())
    }

    /// Captures frozen screenshots of all outputs for freeze mode.
    pub(super) fn capture_frozen_screens(&mut self) -> Result<()> {
        use crate::compositor::protocol::capture_output;

        log::info!(
            "Capturing frozen screenshots for {} outputs",
            self.output_surfaces.len()
        );

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
                    log::warn!(
                        "Failed to capture frozen screen: {}. Continuing without freeze for this output.",
                        e
                    );
                }
            }
        }

        Ok(())
    }

    /// Captures the active window (cropped) or active monitor on the UI connection
    /// for non-area image modes.
    ///
    /// Falls back to the output at (0,0) or the first output when the compositor
    /// query for the active monitor fails.
    pub(super) fn pre_capture(&self, mode: CaptureMode) -> Result<CapturedImage> {
        use crate::compositor::protocol::capture_output;
        use crate::compositor::protocol::outputs::OutputInfo as OutputGeometry;

        let outputs_list: Vec<OutputGeometry> = self
            .outputs
            .iter()
            .filter_map(|(output, info)| {
                let (x, y) = info.logical_position?;
                let (w, h) = info.logical_size?;
                Some((
                    output.clone(),
                    info.name.clone().unwrap_or_default(),
                    x,
                    y,
                    w as u32,
                    h as u32,
                ))
            })
            .collect();

        match mode {
            CaptureMode::ImageWindow => {
                let geometry = crate::compositor::get_active_window()?;
                let rect = crate::render::Rect::from_geometry_string(&geometry)?;
                crate::capture::capture_region(&self.conn, &outputs_list, rect)
            }
            CaptureMode::ImageScreen => {
                let by_name = crate::compositor::get_active_monitor()
                    .ok()
                    .and_then(|name| {
                        outputs_list
                            .iter()
                            .find(|(_, n, ..)| n == &name)
                            .map(|(o, ..)| o.clone())
                    });
                let output = by_name
                    .or_else(|| {
                        outputs_list
                            .iter()
                            .find(|(_, _, x, y, ..)| (*x, *y) == (0, 0))
                            .map(|(o, ..)| o.clone())
                    })
                    .or_else(|| outputs_list.first().map(|(o, ..)| o.clone()))
                    .context("No outputs available")?;
                capture_output(&self.conn, &output)
            }
            _ => unreachable!(),
        }
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
        let is_mode_select = self.phase == UiPhase::ModeSelect;
        rendering::draw_output(
            &mut self.output_surfaces[index],
            &self.selection,
            is_mode_select,
            &self.settings.keybinds,
            &self.settings.mode_select,
            self.is_recording,
            qh,
        )
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
        if self.phase == UiPhase::ModeSelect {
            if pressed {
                self.cancel_selection();
            }
            return;
        }
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
            let outputs_map: Vec<(wl_output::WlOutput, String)> = self
                .outputs
                .iter()
                .map(|(output, info)| (output.clone(), info.name.clone().unwrap_or_default()))
                .collect();

            match capture::complete_selection(
                &self.conn,
                &mut self.output_surfaces,
                &outputs_map,
                &self.settings,
                rect,
            ) {
                Ok(geometry) => {
                    self.selection_geometry = geometry;
                }
                Err(e) => {
                    log::error!("Selection completion failed: {}", e);
                    self.completion_error = Some(e);
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
