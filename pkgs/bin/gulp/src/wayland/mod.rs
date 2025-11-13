//! Wayland client implementation for screen region selection
//!
//! This module handles:
//! - Layer shell surface creation for fullscreen overlay (works above fullscreen windows)
//! - Pointer and keyboard event handling
//! - Multi-monitor rendering and synchronization
//! - Compositor-agnostic window snapping with smooth spring animations
//! - Frame-rate limiting per monitor
//!
//! The window snapping system uses a trait-based backend approach that supports:
//! - Hyprland (via IPC socket)
//! - Other compositors can be added by implementing the WindowBackend trait
//!
//! Module organization:
//! 1. **Helper Functions** - Utility functions for parsing and rendering
//! 2. **Data Structures** - `OutputSurface` and `App` structs
//! 3. **App Implementation**:
//!    - Initialization & Setup
//!    - Animation & Snap Target Management
//!    - Rendering
//!    - Event Handling
//! 4. **Wayland Protocol Handlers** - All trait implementations

use anyhow::{Context, Result};
use smithay_client_toolkit::reexports::calloop::{EventLoop, LoopSignal};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::{OutputInfo, OutputState},
    registry::RegistryState,
    seat::{
        pointer::ThemedPointer,
        SeatState,
    },
    shell::wlr_layer::{
        Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface,
    },
    shm::{slot::SlotPool, Shm},
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_shm, wl_surface},
    Connection, QueueHandle,
};

use crate::{
    animation::SpringAnimation,
    ocr,
    render::{Renderer, RenderConfig},
    selection::{Rect, Selection},
    windows::WindowManager,
    Args,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

mod handlers;
mod input;

use input::InputState;

// Frame timing constants
const IDLE_FRAME_TIMEOUT_MS: u64 = 33; // ~30 FPS when idle
const ANIMATION_FRAME_TIMEOUT_MS: u64 = 8; // ~120 FPS when animating
const DEFAULT_REFRESH_RATE_MHZ: i32 = 60000;
const REFRESH_RATE_DIVIDER: i32 = 1000;

// ============================================================================
// Helper Functions
// ============================================================================


/// Converts FPS to frame duration in microseconds.
#[inline]
fn fps_to_duration(fps: u32) -> Duration {
    Duration::from_micros(1_000_000 / fps.max(1) as u64)
}

/// Creates a renderer with the specified dimensions and styling configuration.
///
/// Returns `None` if renderer creation fails due to invalid color values or other configuration errors.
/// Expects all `Option` fields in args to be populated after merging with config.
#[inline]
fn create_renderer(width: i32, height: i32, args: &Args) -> Option<Renderer> {
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

// ============================================================================
// Data Structures
// ============================================================================

/// Represents a single monitor's overlay surface
pub(super) struct OutputSurface {
    pub(super) _output: wl_output::WlOutput,
    pub(super) layer_surface: LayerSurface,
    pub(super) surface: wl_surface::WlSurface,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) configured: bool,
    pub(super) pool: Option<SlotPool>,
    pub(super) renderer: Option<Renderer>,
    pub(super) last_render: Instant,
    pub(super) frame_time: Duration,
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
                    Some("selection"),
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

                // Start animation from cursor position
                let start_rect = Rect::new(px as i32, py as i32, 1, 1);
                let mut anim = SpringAnimation::new(start_rect);
                anim.set_target(rect);
                self.input.snap_animation = Some(anim);
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

    /// Translates global rectangle coordinates to local output coordinates.
    #[inline]
    fn translate_rect_to_local(rect: Rect, offset_x: i32, offset_y: i32) -> Rect {
        Rect::new(
            rect.x - offset_x,
            rect.y - offset_y,
            rect.width,
            rect.height,
        )
    }

    /// Creates a local selection from a global rectangle by translating coordinates.
    #[inline]
    fn create_local_selection(
        global_rect: Rect,
        offset_x: i32,
        offset_y: i32,
    ) -> Selection {
        let local_rect = Self::translate_rect_to_local(global_rect, offset_x, offset_y);
        Selection::from_rect(local_rect)
    }

    pub(super) fn draw_index(&mut self, index: usize) -> Result<()> {
        if !self.output_surfaces[index].configured {
            return Ok(());
        }

        let width = self.output_surfaces[index].width as i32;
        let height = self.output_surfaces[index].height as i32;
        let offset_x = self.output_surfaces[index].x;
        let offset_y = self.output_surfaces[index].y;

        let stride = width * 4;

        // Extract what we need from the output surface to avoid borrow issues
        let (buffer, canvas, renderer) = {
            let output_surface = &mut self.output_surfaces[index];

            // Check if we have renderer and pool
            if output_surface.renderer.is_none() || output_surface.pool.is_none() {
                return Ok(());
            }

            let pool = output_surface.pool.as_mut().unwrap();

            // Use double buffering to prevent flickering
            // The pool automatically manages multiple buffer slots
            let (buf, canv) =
                match pool.create_buffer(width, height, stride, wl_shm::Format::Argb8888) {
                    Ok(buffer) => buffer,
                    Err(e) => {
                        log::warn!(
                            "Failed to create buffer for output {}: {}. Resizing pool.",
                            index,
                            e
                        );
                        // Pool might be exhausted, resize it
                        pool.resize((width * height * 4 * 2) as usize)?;
                        pool.create_buffer(width, height, stride, wl_shm::Format::Argb8888)?
                    }
                };

            log::debug!(
                "Output {} got buffer, canvas ptr: {:p}",
                index,
                canv.as_ptr()
            );

            (buf, canv, output_surface.renderer.as_ref().unwrap())
        };

        // Check if we need to render a selection on this output
        let has_selection = if let Some(rect) = self.selection.get_rect() {
            let output_rect = Rect::new(offset_x, offset_y, width, height);

            if rect.intersects(&output_rect) {
                // Calculate the intersection rectangle (clipped to this output)
                let clip_x = rect.x.max(offset_x);
                let clip_y = rect.y.max(offset_y);
                let clip_right = (rect.x + rect.width).min(offset_x + width);
                let clip_bottom = (rect.y + rect.height).min(offset_y + height);
                let clip_width = clip_right - clip_x;
                let clip_height = clip_bottom - clip_y;

                log::debug!(
                    "RENDERING SELECTION on output {} at ({},{}) {}x{} - global rect: ({},{}) {}x{}, clipped: ({},{}) {}x{}",
                    index,
                    offset_x,
                    offset_y,
                    width,
                    height,
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    clip_x,
                    clip_y,
                    clip_width,
                    clip_height
                );

                // Translate the ENTIRE global selection to output-local coordinates
                // This preserves the visual continuity across monitors
                let local_rect_x = rect.x - offset_x;
                let local_rect_y = rect.y - offset_y;

                log::debug!(
                    "Creating local selection for output {}: global rect ({},{}) {}x{} -> local rect ({},{}) {}x{}",
                    index, rect.x, rect.y, rect.width, rect.height,
                    local_rect_x, local_rect_y, rect.width, rect.height
                );

                // Create a selection with the full rectangle translated to local coords
                let local_selection =
                    Self::create_local_selection(rect, offset_x, offset_y);

                // Render directly to buffer - NO surface allocation!
                renderer.render_to_buffer(&local_selection, canvas)?;
                true
            } else {
                log::debug!("SKIPPING output {} - no intersection", index);
                false
            }
        } else {
            false
        };

        // If no selection on this output, render dimmed overlay only (or snap target if present)
        if !has_selection {
            let snap_rect = self.selection.get_current_snap_target();

            if let Some(snap_rect) = snap_rect {
                let local_snap = Self::translate_rect_to_local(snap_rect, offset_x, offset_y);

                log::debug!(
                    "RENDERING SNAP TARGET on output {}: global ({},{}) {}x{} -> local ({},{}) {}x{}",
                    index, snap_rect.x, snap_rect.y, snap_rect.width, snap_rect.height,
                    local_snap.x, local_snap.y, local_snap.width, local_snap.height
                );

                let mut local_selection = Selection::new();
                local_selection.set_animated_snap_target(Some(local_snap));

                // Render directly to buffer - NO surface allocation!
                renderer.render_to_buffer(&local_selection, canvas)?;
            } else {
                // Render dimmed overlay (whether there's a selection elsewhere or not)
                log::debug!("RENDERING DIMMED ONLY on output {}", index);
                let empty_selection = Selection::new();
                // Render directly to buffer - NO surface allocation!
                renderer.render_to_buffer(&empty_selection, canvas)?;
            }
        }

        // Attach and commit
        log::debug!(
            "Committing buffer to output {} surface at offset ({},{}) with damage {}x{}",
            index,
            offset_x,
            offset_y,
            width,
            height
        );
        let output = &self.output_surfaces[index];
        output.surface.attach(Some(buffer.wl_buffer()), 0, 0);
        output.surface.damage_buffer(0, 0, width, height);
        output.surface.commit();

        Ok(())
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

                    if let Some(ref mut anim) = self.input.snap_animation {
                        anim.set_target(rect);
                    } else {
                        let start_rect = Rect::new(global_x as i32, global_y as i32, 1, 1);
                        let mut anim = SpringAnimation::new(start_rect);
                        anim.set_target(rect);
                        self.input.snap_animation = Some(anim);
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
            // Hide all overlays before screencopy
            // Detach buffers from all surfaces to make them invisible
            for output_surface in &mut self.output_surfaces {
                output_surface.surface.attach(None, 0, 0);
                output_surface.surface.commit();
            }

            // Flush and wait for compositor to process
            let _ = self.conn.flush();

            // Roundtrip ensures compositor has processed the detachment
            let _ = self.conn.roundtrip();

            // Add small delay to ensure compositor has fully processed the changes
            std::thread::sleep(std::time::Duration::from_millis(100));

            if self.args.ocr {
                // OCR mode: capture and extract text
                // Collect outputs with their names and positions
                let outputs_list: Vec<(wl_output::WlOutput, String, i32, i32, u32, u32)> = self
                    .output_surfaces
                    .iter()
                    .map(|surf| {
                        let info = self.outputs.get(&surf._output);
                        let name = info.and_then(|i| i.name.clone()).unwrap_or_default();
                        (surf._output.clone(), name, surf.x, surf.y, surf.width, surf.height)
                    })
                    .collect();

                match ocr::capture_and_ocr(&self.conn, &outputs_list, rect) {
                    Ok(text) => {
                        println!("{}", text);
                    }
                    Err(e) => {
                        eprintln!("OCR failed: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                // Normal mode: output coordinates
                let output = self.format_output(rect.x, rect.y, rect.width, rect.height);
                println!("{}", output);
            }

            self.exit = true;
            self.loop_signal.stop();
        }
    }

    fn format_output(&self, x: i32, y: i32, width: i32, height: i32) -> String {
        "%x,%y %wx%h"
            .replace("%x", &x.to_string())
            .replace("%y", &y.to_string())
            .replace("%w", &width.to_string())
            .replace("%h", &height.to_string())
            .replace("%X", &(x + width).to_string())
            .replace("%Y", &(y + height).to_string())
    }

    pub(super) fn cancel_selection(&mut self) {
        eprintln!("Selection cancelled by user");
        log::debug!("Selection cancelled by user");
        std::process::exit(1);
    }
}
