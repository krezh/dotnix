//! Wayland client implementation for screen region selection
//!
//! This module handles:
//! - Layer shell surface creation for fullscreen overlay (works above fullscreen windows)
//! - Pointer and keyboard event handling
//! - Multi-monitor rendering and synchronization
//! - Compositor-agnostic window snapping with smooth spring animations
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
    animation::SpringAnimation,
    cli::Args,
    render::Renderer,
    selection::{Rect, Selection},
    windows::WindowManager,
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
    pub(super) window_manager: Option<WindowManager>,

    // Input state
    pub(super) input: InputState,

    // Loop control
    pub(super) exit: bool,
    pub(super) loop_signal: LoopSignal,
    pub(super) last_frame_time: Instant,
    pub(super) needs_redraw: bool,
}

// ============================================================================
// App Implementation - Initialization & Setup
// ============================================================================

impl App {
    pub fn run(args: Args) -> Result<()> {
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

        // Initialize window manager if snapping is enabled
        let window_manager = if !args.no_snap {
            match WindowManager::new() {
                Ok(wm) => Some(wm),
                Err(e) => {
                    log::warn!(
                        "Failed to initialize window manager: {}. Snapping disabled.",
                        e
                    );
                    None
                }
            }
        } else {
            None
        };

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
            window_manager,
            input: InputState::new(),
            exit: false,
            loop_signal,
            last_frame_time: Instant::now(),
            needs_redraw: false,
        };

        // Dispatch any pending events
        event_queue.blocking_dispatch(&mut app)?;

        // Set up Wayland event source
        WaylandSource::new(conn.clone(), event_queue)
            .insert(event_loop.handle())
            .context("Failed to insert wayland source")?;

        // Create layer surfaces for each output
        app.create_layer_surfaces(&qh)?;

        app.initialize_snap_animation();
        app.input.snap_initialized = true;

        loop {
            let timeout = if let Some(ref anim) = app.input.snap_animation {
                if anim.is_settled() {
                    Some(Duration::from_millis(IDLE_FRAME_TIMEOUT_MS))
                } else {
                    Some(Duration::from_millis(ANIMATION_FRAME_TIMEOUT_MS))
                }
            } else {
                None
            };

            event_loop.dispatch(timeout, &mut app)?;

            if app.input.snap_animation.is_some() {
                app.update_animation();
            }

            // Render if needed (throttled by frame rate limiting in redraw_all)
            if app.needs_redraw {
                app.needs_redraw = false;
                app.redraw_all();
            }

            if app.exit {
                break;
            }
        }

        Ok(())
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

                // Calculate frame time for this specific output
                let fps = self.args.fps.unwrap_or(0);
                let frame_time = if fps > 0 {
                    // User specified FPS - use it for all monitors
                    fps_to_duration(fps)
                } else {
                    // Auto-detect: use this monitor's refresh rate
                    let refresh_rate = info
                        .modes
                        .iter()
                        .find(|m| m.current)
                        .map(|m| m.refresh_rate)
                        .unwrap_or(DEFAULT_REFRESH_RATE_MHZ);

                    let fps = (refresh_rate / REFRESH_RATE_DIVIDER) as u32;
                    log::info!(
                        "Output {:?}: {}x{} at ({}, {}) @ {}Hz",
                        info.name,
                        width,
                        height,
                        x,
                        y,
                        fps
                    );
                    fps_to_duration(fps)
                };

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
                    last_render: Instant::now(),
                    frame_time,
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

    // ------------------------------------------------------------------------
    // Animation & Snap Target Management
    // ------------------------------------------------------------------------

    fn initialize_snap_animation(&mut self) {
        if let Some(ref window_manager) = self.window_manager {
            // Get actual cursor position from compositor backend
            let (px, py) = match window_manager.get_cursor_position() {
                Ok((x, y)) => {
                    log::info!("Got cursor position from backend: ({}, {})", x, y);
                    (x as f64, y as f64)
                }
                Err(e) => {
                    log::warn!(
                        "Failed to get cursor position from backend: {}. Using fallback.",
                        e
                    );
                    // Fallback to stored pointer position or screen center
                    if self.input.pointer_position != (0.0, 0.0) {
                        self.input.pointer_position
                    } else {
                        (1280.0, 720.0)
                    }
                }
            };

            if let Some(window) = window_manager.find_nearest_window(px as i32, py as i32, 50) {
                let rect = window.rect;
                log::info!("Initial snap target: {}", rect.describe());

                if !self.args.no_animation {
                    // Start animation from cursor position
                    let start_rect = Rect::new(px as i32, py as i32, 1, 1);
                    let mut anim = SpringAnimation::new(start_rect);
                    anim.set_target(rect);
                    self.input.snap_animation = Some(anim);
                }
                self.selection.set_snap_target(Some(rect));
            }
        }
    }

    /// Updates spring animation state and marks surfaces for redraw if needed.
    ///
    /// Uses delta time for frame-rate independent animation physics.
    fn update_animation(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f64();
        self.last_frame_time = now;

        if let Some(ref mut anim) = self.input.snap_animation {
            anim.update(dt);
            let animated_rect = anim.current();
            self.selection.set_animated_snap_target(Some(animated_rect));

            if !anim.is_settled() {
                self.needs_redraw = true;
            }
        }
    }

    /// Redraws all monitors with independent per-monitor FPS throttling.
    ///
    /// Throttles each monitor independently based on its refresh rate to reduce CPU and GPU usage.
    fn redraw_all(&mut self) {
        let now = Instant::now();
        let len = self.output_surfaces.len();

        for i in 0..len {
            if !self.output_surfaces[i].configured {
                continue;
            }

            // Per-monitor frame rate limiting
            let elapsed = now.duration_since(self.output_surfaces[i].last_render);
            if elapsed >= self.output_surfaces[i].frame_time {
                self.output_surfaces[i].last_render = now;
                let _ = self.draw_index(i);
            }
        }
    }

    // ------------------------------------------------------------------------
    // Rendering
    // ------------------------------------------------------------------------

    pub(super) fn draw_index(&mut self, index: usize) -> Result<()> {
        rendering::draw_output(&mut self.output_surfaces[index], &self.selection)
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

        if !self.input.snap_initialized && self.window_manager.is_some() {
            self.input.snap_initialized = true;
            self.initialize_snap_animation();
        }

        if self.input.mouse_pressed {
            if let Some((start_x, start_y)) = self.input.selection_start {
                self.selection
                    .update_drag(start_x, start_y, global_x as i32, global_y as i32);
                self.needs_redraw = true;
            }
        } else if let Some(ref window_manager) = self.window_manager {
            let snap_target = window_manager
                .find_nearest_window(global_x as i32, global_y as i32, 50)
                .map(|w| w.rect);

            if snap_target != self.selection.get_snap_target() {
                if let Some(rect) = snap_target {
                    log::info!("Snap target: {}", rect.describe());

                    if !self.args.no_animation {
                        if let Some(ref mut anim) = self.input.snap_animation {
                            anim.set_target(rect);
                        } else {
                            let start_rect = Rect::new(global_x as i32, global_y as i32, 1, 1);
                            let mut anim = SpringAnimation::new(start_rect);
                            anim.set_target(rect);
                            self.input.snap_animation = Some(anim);
                        }
                    }
                    self.needs_redraw = true;
                } else {
                    log::info!("Snap target: None");
                    self.needs_redraw = true;
                }

                self.selection.set_snap_target(snap_target);
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
            } else if let Some(snap_rect) = self.selection.get_snap_target() {
                log::info!("Using snap target on click: {}", snap_rect.describe());
                self.selection.start_selection(snap_rect.x, snap_rect.y);
                self.selection.update_drag(
                    snap_rect.x,
                    snap_rect.y,
                    snap_rect.x + snap_rect.width,
                    snap_rect.y + snap_rect.height,
                );
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
            let _ = capture::complete_selection(
                &self.conn,
                &mut self.output_surfaces,
                &outputs_map,
                &self.args,
                rect,
            );

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
