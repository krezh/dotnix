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
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputInfo, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
        pointer::{
            CursorIcon, PointerEvent, PointerEventKind, PointerHandler, ThemeSpec, ThemedPointer,
        },
        Capability, SeatHandler, SeatState,
    },
    shell::wlr_layer::{
        Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
        LayerSurfaceConfigure,
    },
    shm::{slot::SlotPool, Shm, ShmHandler},
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, QueueHandle,
};

use crate::{
    animation::SpringAnimation,
    ocr,
    render::Renderer,
    selection::{Rect, Selection},
    windows::WindowManager,
    Args,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

// ============================================================================
// Helper Functions
// ============================================================================


/// Convert FPS to frame Duration
#[inline]
fn fps_to_duration(fps: u32) -> Duration {
    Duration::from_micros(1_000_000 / fps.max(1) as u64)
}

/// Creates a renderer with the given dimensions and styling from Args
///
/// Returns None if renderer creation fails (e.g., invalid color values)
/// Note: Args should be merged with config before calling this, ensuring all Options are Some
#[inline]
fn create_renderer(width: i32, height: i32, args: &Args) -> Option<Renderer> {
    Renderer::new(
        width,
        height,
        args.border_color.as_ref()?,
        args.border_thickness?,
        args.border_rounding?,
        args.dim_opacity?,
        args.font_family.as_ref()?.clone(),
        args.font_size?,
        args.font_weight.as_ref()?,
    )
    .ok()
}

// ============================================================================
// Data Structures
// ============================================================================

/// Represents a single monitor's overlay surface
struct OutputSurface {
    _output: wl_output::WlOutput,
    layer_surface: LayerSurface,
    surface: wl_surface::WlSurface,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    configured: bool,
    pool: Option<SlotPool>,
    renderer: Option<Renderer>,
    last_render: Instant,
    frame_time: Duration,
}

/// Main application state managing Wayland connection and surfaces
pub struct App {
    // Wayland state
    conn: Connection,
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm_state: Shm,
    layer_shell: LayerShell,
    themed_pointer: Option<ThemedPointer>,

    // Application state
    outputs: HashMap<wl_output::WlOutput, OutputInfo>,
    output_surfaces: Vec<OutputSurface>,
    renderer: Option<Renderer>,
    selection: Selection,
    args: Args,
    window_manager: Option<WindowManager>,
    snap_animation: Option<SpringAnimation>,
    snap_initialized: bool,

    // Input state
    pointer_position: (f64, f64),
    mouse_pressed: bool,
    selection_start: Option<(i32, i32)>,
    current_surface: Option<wl_surface::WlSurface>,
    _active_output_index: Option<usize>,

    // Loop control
    exit: bool,
    loop_signal: LoopSignal,
    last_frame_time: Instant,
    needs_redraw: bool,
}

// ============================================================================
// App Implementation - Initialization & Setup
// ============================================================================

impl App {
    pub fn new(args: Args) -> Result<()> {
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
            snap_animation: None,
            snap_initialized: false,
            pointer_position: (0.0, 0.0),
            mouse_pressed: false,
            selection_start: None,
            current_surface: None,
            _active_output_index: None,
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

        // Initialize snap animation with actual cursor position from Hyprland
        app.initialize_snap_animation();
        app.snap_initialized = true;

        // Run the event loop
        loop {
            // Dynamic FPS based on animation state
            let timeout = if let Some(ref anim) = app.snap_animation {
                if anim.is_settled() {
                    // Animation settled - use lower FPS to save CPU/power
                    Some(Duration::from_millis(33)) // ~30 FPS when idle
                } else {
                    // Animation active - use high FPS for smooth motion
                    Some(Duration::from_millis(8)) // ~120 FPS when animating
                }
            } else {
                // No animation - block indefinitely
                None
            };

            event_loop.dispatch(timeout, &mut app)?;

            // Update animations if active
            if app.snap_animation.is_some() {
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
                        .unwrap_or(60000); // Default to 60Hz

                    let fps = (refresh_rate / 1000) as u32;
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
                    if self.pointer_position != (0.0, 0.0) {
                        self.pointer_position
                    } else {
                        (1280.0, 720.0)
                    }
                }
            };

            if let Some(window) = window_manager.find_nearest_window(px as i32, py as i32, 50) {
                let rect = window.rect;
                log::info!(
                    "Initial snap target: {}x{} at ({}, {})",
                    rect.width,
                    rect.height,
                    rect.x,
                    rect.y
                );

                // Start animation from cursor position
                let start_rect = Rect::new(px as i32, py as i32, 1, 1);
                let mut anim = SpringAnimation::new(start_rect);
                anim.set_target(rect);
                self.snap_animation = Some(anim);
                self.selection.set_snap_target(Some(rect));
            }
        }
    }

    /// Updates spring animation state and marks for redraw
    ///
    /// Called each event loop iteration when animation is active.
    /// Uses delta time for frame-rate independent animation.
    fn update_animation(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f64();
        self.last_frame_time = now;

        if let Some(ref mut anim) = self.snap_animation {
            anim.update(dt);
            let animated_rect = anim.current();
            self.selection.set_animated_snap_target(Some(animated_rect));

            // Only request redraw if animation is still moving
            // This prevents unnecessary redraws once animation has settled
            if !anim.is_settled() {
                self.needs_redraw = true;
            }
        }
    }

    /// Redraws all monitors with per-monitor FPS throttling
    ///
    /// Each monitor is independently throttled based on its refresh rate
    /// to avoid unnecessary rendering and reduce CPU/GPU usage.
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

    /// Translate global rect to local output coordinates
    #[inline]
    fn translate_rect_to_local(rect: Rect, offset_x: i32, offset_y: i32) -> Rect {
        Rect::new(
            rect.x - offset_x,
            rect.y - offset_y,
            rect.width,
            rect.height,
        )
    }

    /// Create a local selection from a global rect
    #[inline]
    fn create_local_selection(
        global_rect: Rect,
        offset_x: i32,
        offset_y: i32,
    ) -> Selection {
        let local_rect = Self::translate_rect_to_local(global_rect, offset_x, offset_y);
        Selection::from_rect(local_rect)
    }

    fn draw_index(&mut self, index: usize) -> Result<()> {
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
            // Check if we need to render a snap target on this output
            // Use animated snap target if available, otherwise fall back to static
            let snap_rect = self
                .selection
                .get_animated_snap_target()
                .or_else(|| self.selection.get_snap_target());

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
            self.output_surfaces[index].x,
            self.output_surfaces[index].y,
            width,
            height
        );
        self.output_surfaces[index]
            .surface
            .attach(Some(buffer.wl_buffer()), 0, 0);
        self.output_surfaces[index]
            .surface
            .damage_buffer(0, 0, width, height);
        self.output_surfaces[index].surface.commit();

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Event Handling
    // ------------------------------------------------------------------------

    fn handle_pointer_move(&mut self, surface: &wl_surface::WlSurface, x: f64, y: f64) {
        // Find which output surface this is and convert to global coordinates
        let mut global_x = x;
        let mut global_y = y;

        for output_surface in &self.output_surfaces {
            if &output_surface.surface == surface {
                global_x = x + output_surface.x as f64;
                global_y = y + output_surface.y as f64;
                break;
            }
        }

        self.pointer_position = (global_x, global_y);

        // Initialize snap animation on first pointer movement
        if !self.snap_initialized && self.window_manager.is_some() {
            self.snap_initialized = true;
            self.initialize_snap_animation();
        }

        if self.mouse_pressed {
            if let Some((start_x, start_y)) = self.selection_start {
                self.selection
                    .update_drag(start_x, start_y, global_x as i32, global_y as i32);
                // Mark for redraw instead of drawing immediately
                self.needs_redraw = true;
            }
        } else {
            // Only check for window snapping when not actively selecting
            if let Some(ref window_manager) = self.window_manager {
                // Use nearest window within 50px threshold to handle fast cursor movement
                let snap_target = window_manager
                    .find_nearest_window(global_x as i32, global_y as i32, 50)
                    .map(|w| w.rect);

                // Only log and update animation when snap target changes
                if snap_target != self.selection.get_snap_target() {
                    if let Some(rect) = snap_target {
                        log::info!(
                            "Snap target: {}x{} at ({}, {})",
                            rect.width,
                            rect.height,
                            rect.x,
                            rect.y
                        );

                        // Initialize or update animation
                        if let Some(ref mut anim) = self.snap_animation {
                            // Changing target - animate to new position
                            anim.set_target(rect);
                        } else {
                            // First snap - start animation from a small rect at cursor position
                            let start_rect = Rect::new(global_x as i32, global_y as i32, 1, 1);
                            let mut anim = SpringAnimation::new(start_rect);
                            anim.set_target(rect);
                            self.snap_animation = Some(anim);
                        }
                        // Only request redraw when snap target changes
                        self.needs_redraw = true;
                    } else {
                        log::info!("Snap target: None");
                        // Target cleared - request one final redraw to clear the snap highlight
                        self.needs_redraw = true;
                    }

                    self.selection.set_snap_target(snap_target);
                }
                // Note: If snap target hasn't changed and animation is settled,
                // we don't request a redraw - this prevents flickering on mouse movement
            }
        }
    }

    fn handle_pointer_button(&mut self, pressed: bool) {
        if pressed {
            self.mouse_pressed = true;
            self.selection_start = Some((
                self.pointer_position.0 as i32,
                self.pointer_position.1 as i32,
            ));
            self.selection.start_selection(
                self.pointer_position.0 as i32,
                self.pointer_position.1 as i32,
            );
        } else {
            self.mouse_pressed = false;
            // Check if we have a valid dragged selection
            if self.selection.get_selection().is_some() {
                self.complete_selection();
            } else if let Some(snap_rect) = self.selection.get_snap_target() {
                // If we clicked without dragging but there's a snap target, use it
                log::info!(
                    "Using snap target on click: {}x{} at ({}, {})",
                    snap_rect.width,
                    snap_rect.height,
                    snap_rect.x,
                    snap_rect.y
                );
                // Set the selection to the snap target
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
        // Mark for redraw instead of drawing immediately
        self.needs_redraw = true;
    }

    fn complete_selection(&mut self) {
        if let Some(rect) = self.selection.get_selection() {
            // Hide all overlays so grim captures a clean screenshot
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

    fn cancel_selection(&mut self) {
        eprintln!("Selection cancelled by user");
        log::debug!("Selection cancelled by user");
        std::process::exit(1);
    }
}

// ============================================================================
// Wayland Protocol Handler Implementations
// ============================================================================

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        // Frame callbacks not used - we render immediately on changes
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wayland_client::protocol::wl_output::Transform,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        self.current_surface = Some(surface.clone());
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        log::debug!("New output detected");
        if let Some(info) = self.output_state.info(&output) {
            self.outputs.insert(output, info.clone());
        }
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        if let Some(info) = self.output_state.info(&output) {
            self.outputs.insert(output, info.clone());
        }
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        self.outputs.remove(&output);
    }
}

impl LayerShellHandler for App {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.exit = true;
        self.loop_signal.stop();
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        // Find the surface and mark it as configured
        for i in 0..self.output_surfaces.len() {
            if &self.output_surfaces[i].layer_surface == layer {
                self.output_surfaces[i].configured = true;
                self.output_surfaces[i].width = configure.new_size.0;
                self.output_surfaces[i].height = configure.new_size.1;

                // Render initial frame
                let _ = self.draw_index(i);
                break;
            }
        }
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.themed_pointer.is_none() {
            // Create a themed pointer with the system theme
            let surface = self.compositor_state.create_surface(qh);
            let themed_pointer = self.seat_state.get_pointer_with_theme(
                qh,
                &seat,
                self.shm_state.wl_shm(),
                surface,
                ThemeSpec::System,
            );
            if let Ok(pointer) = themed_pointer {
                self.themed_pointer = Some(pointer);
            }
        }

        if capability == Capability::Keyboard {
            let _ = self.seat_state.get_keyboard(qh, &seat, None);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        _capability: Capability,
    ) {
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            match event.kind {
                PointerEventKind::Enter { .. } => {
                    self.current_surface = Some(event.surface.clone());

                    // Set crosshair cursor using ThemedPointer
                    if let Some(themed_pointer) = &self.themed_pointer {
                        let _ = themed_pointer.set_cursor(conn, CursorIcon::Crosshair);
                    }
                }
                PointerEventKind::Leave { .. } => {}
                PointerEventKind::Motion { .. } => {
                    if let Some(surface) = self.current_surface.clone() {
                        self.handle_pointer_move(&surface, event.position.0, event.position.1);
                    }
                }
                PointerEventKind::Press { button, .. } => {
                    if button == 0x110 {
                        // BTN_LEFT
                        self.handle_pointer_button(true);
                    } else if button == 0x111 {
                        // BTN_RIGHT - cancel
                        self.cancel_selection();
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if button == 0x110 {
                        // BTN_LEFT
                        self.handle_pointer_button(false);
                    }
                }
                PointerEventKind::Axis { .. } => {}
            }
        }
    }
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl KeyboardHandler for App {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
        // We don't need to do anything on keyboard focus enter
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {
        // We don't need to do anything on keyboard focus leave
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        // Handle Escape key to cancel
        if event.keysym == Keysym::Escape {
            self.cancel_selection();
        }
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
        // We don't need to handle key releases
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: Modifiers,
        _raw_modifiers: smithay_client_toolkit::seat::keyboard::RawModifiers,
        _layout: u32,
    ) {
        // We don't need to track modifiers
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
        // We don't need to handle key repeats
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState];
}

delegate_compositor!(App);
delegate_output!(App);
delegate_shm!(App);
delegate_seat!(App);
delegate_pointer!(App);
delegate_keyboard!(App);
delegate_layer!(App);
delegate_registry!(App);
