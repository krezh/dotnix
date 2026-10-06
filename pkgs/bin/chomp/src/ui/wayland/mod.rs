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
    seat::{SeatState, keyboard::Modifiers, pointer::ThemedPointer},
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
    cli::{CaptureAction, Settings},
    render::{ModePaletteLayout, PaletteAction, Selection, SelectionHud, SelectionPurpose},
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

/// Work deferred until the compositor has presented a frame without chomp's UI.
pub(super) enum PendingCapture {
    /// Freeze the screen, then hand over to region selection.
    Freeze,
    /// Capture the screen or the active window, then exit.
    Image(CaptureMode),
}

/// How long to wait for the frame callbacks confirming the UI is off screen
/// before capturing anyway, so a compositor that withholds them cannot hang us.
const HIDE_TIMEOUT_MS: u64 = 120;

/// What the selector produced.
pub struct Selected {
    /// The selected region, for the modes that capture or record by coordinates.
    pub geometry: Option<String>,
    /// The mode picked in the selector, if it opened on the mode bar.
    pub mode: Option<CaptureMode>,
    /// The image the selector already captured, when it has one.
    pub image: Option<CapturedImage>,
    /// Ctrl was held, meaning copy to the clipboard instead of uploading.
    pub to_clipboard: bool,
    /// Target monitor name picked under the pointer for screen modes.
    pub target_monitor: Option<String>,
    /// Replay action chosen from the mode selector.
    pub replay_action: Option<crate::cli::ReplayAction>,
}

impl Selected {
    /// True when the user dismissed the selector without choosing anything.
    pub fn cancelled(&self) -> bool {
        self.geometry.is_none()
            && self.mode.is_none()
            && self.image.is_none()
            && self.replay_action.is_none()
    }
}

/// Main application state managing Wayland connection and surfaces
pub struct App {
    // Wayland state
    pub(super) conn: Connection,
    pub(super) screencopy: crate::compositor::Screencopy,
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
    pub(super) hovered_action: Option<PaletteAction>,
    pub(super) hovered_surface: Option<wl_surface::WlSurface>,

    // Pre-captured image for non-area modes (captured on the same connection as the UI
    // to avoid cross-connection ordering races with the compositor)
    pub(super) captured_image: Option<CapturedImage>,

    // True if a wl-screenrec recording is already running when the selector opens
    pub(super) is_recording: bool,
    pub(super) supports_window_capture: bool,

    // Mode-select entrance animation (slide up from bottom + fade).
    // `intro_progress` is in [0, 1]; 1.0 means the bar is at rest.
    pub(super) intro_progress: f64,
    pub(super) intro_start: Option<std::time::Instant>,
    pub(super) intro_duration: f64,
    pub(super) intro_done: bool,

    // Set when the capture was triggered with Ctrl held (copy to clipboard
    // instead of uploading). Returned from `run` to the caller.
    pub(super) to_clipboard: bool,

    // Current keyboard modifier state (updated via the modifier event).
    pub(super) modifiers: Modifiers,

    // Capture waiting for the UI to be off screen, and when the wait started.
    pub(super) pending_capture: Option<PendingCapture>,
    pub(super) pending_since: Option<std::time::Instant>,

    // Instant replay integration
    pub(super) replay_status: Option<crate::replay::ReplayStatus>,
    pub(super) replay_configured: bool,
    pub(super) chosen_replay_action: Option<crate::cli::ReplayAction>,

    // Multi-monitor refinement
    pub(super) target_monitor: Option<String>,
    pub(super) target_geometry: Option<String>,
    pub(super) active_palette_output: Option<usize>,
}

// ============================================================================
// App Implementation - Initialization & Setup
// ============================================================================

impl App {
    pub fn run(settings: Settings) -> Result<Selected> {
        // Only allow one interactive overlay at a time; otherwise the overlays
        // stack on top of each other. If another instance is already running, exit.
        let _instance_lock = match utils::ensure_single_instance() {
            Ok(lock) => lock,
            Err(e) => {
                log::warn!("{}", e);
                eprintln!("chomp is already running; exiting to avoid a stacked overlay");
                std::process::exit(0);
            }
        };

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

        let is_mode_select = settings.request.mode.is_none() && !settings.request.is_ocr();
        let phase = if is_mode_select {
            UiPhase::ModeSelect
        } else {
            UiPhase::RegionSelect
        };

        let is_recording = crate::capture::recording(&settings)?.is_some();
        let to_clipboard = settings.request.to_clipboard();
        let supports_window_capture =
            crate::compositor::detect_compositor().supports_window_capture();
        let replay_status = is_mode_select.then(crate::replay::query_status).flatten();
        let replay_configured = is_mode_select && settings.replay.enabled;

        let mut app = Self {
            screencopy: crate::compositor::Screencopy::new(&conn)?,
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
            hovered_action: None,
            hovered_surface: None,
            captured_image: None,
            is_recording,
            supports_window_capture,
            replay_status,
            replay_configured,
            chosen_replay_action: None,
            target_monitor: None,
            target_geometry: None,
            active_palette_output: None,
            intro_progress: if is_mode_select { 0.0 } else { 1.0 },
            intro_start: None,
            intro_duration: 0.22,
            intro_done: !is_mode_select,
            to_clipboard,
            modifiers: Modifiers::default(),
            pending_capture: None,
            pending_since: None,
        };

        event_queue.blocking_dispatch(&mut app)?;

        WaylandSource::new(conn.clone(), event_queue)
            .insert(event_loop.handle())
            .context("Failed to insert wayland source")?;

        app.create_layer_surfaces(&qh)?;
        app.active_palette_output = app.active_output_index();

        // Capture the frozen background before any buffer is attached: the layer
        // surfaces exist but are not yet visible, so the compositor cannot include
        // them in a rendered frame. Only for a run that starts straight in region
        // selection — with the mode selector, the freeze is taken once a mode has
        // been picked, so it shows the screen as it is then rather than at launch.
        if app.settings.freeze && app.phase == UiPhase::RegionSelect {
            app.capture_frozen_screens()?;
        }

        loop {
            // Only the intro animation and the wait for a hidden UI need the loop
            // to wake on its own; otherwise Wayland events do the waking.
            let animating = app.phase == UiPhase::ModeSelect && !app.intro_done;
            let timeout = if animating || app.pending_capture.is_some() {
                Duration::from_millis(IDLE_FRAME_TIMEOUT_MS)
            } else {
                Duration::from_millis(IDLE_TIMEOUT_MS)
            };

            event_loop.dispatch(Some(timeout), &mut app)?;

            // While a capture is pending the UI is deliberately invisible: draw nothing
            // back onto the screen until the capture has been taken.
            if app.pending_capture.is_some() {
                if app.ui_hidden() {
                    app.run_pending_capture();
                }
                if app.exit {
                    break;
                }
                continue;
            }

            // Drive the mode-select slide-up entrance animation.
            if app.phase == UiPhase::ModeSelect && !app.intro_done {
                let start = *app.intro_start.get_or_insert_with(std::time::Instant::now);
                let t = (start.elapsed().as_secs_f64() / app.intro_duration).min(1.0);
                // Ease-out cubic for a snappy settle.
                app.intro_progress = 1.0 - (1.0 - t).powi(3);
                app.needs_redraw = true;
                for s in &mut app.output_surfaces {
                    s.needs_render = true;
                }
                if t >= 1.0 {
                    app.intro_done = true;
                    app.intro_progress = 1.0;
                }
            }

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

        Ok(Selected {
            geometry: app.selection_geometry.or(app.target_geometry),
            mode: app.chosen_mode,
            image: app.captured_image,
            to_clipboard: app.to_clipboard,
            target_monitor: app.target_monitor,
            replay_action: app.chosen_replay_action,
        })
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

                let renderer = Some(create_renderer(width, height, &self.settings)?);

                log::info!(
                    "Output {:?}: {}x{} at ({}, {})",
                    info.name,
                    width,
                    height,
                    x,
                    y
                );

                self.output_surfaces.push(OutputSurface {
                    name: info.name.clone().unwrap_or_default(),
                    output: output.clone(),
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
                    frozen_dimmed: None,
                    last_had_selection: false,
                    needs_render: true,
                    frame_callback: None,
                    waiting_for_frame: false,
                });
            }
        }

        Ok(())
    }

    /// Captures frozen screenshots of all outputs for freeze mode.
    pub(super) fn capture_frozen_screens(&mut self) -> Result<()> {
        log::info!(
            "Capturing frozen screenshots for {} outputs",
            self.output_surfaces.len()
        );

        let dim_opacity = self.settings.dim_opacity;

        for output_surface in &mut self.output_surfaces {
            let transform = self
                .outputs
                .get(&output_surface.output)
                .map(|info| info.transform)
                .unwrap_or(wayland_client::protocol::wl_output::Transform::Normal);
            match self.screencopy.capture(&output_surface.output, transform) {
                Ok(captured_image) => {
                    log::debug!(
                        "Captured frozen screen for output: {}x{}",
                        captured_image.width,
                        captured_image.height
                    );
                    // Dim once here rather than per frame while dragging.
                    output_surface.frozen_dimmed =
                        Some(crate::render::dim_argb(&captured_image.data, dim_opacity));
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

    /// Hides the UI and queues `pending` to run once it is off screen.
    pub(super) fn hide_ui_for_capture(&mut self, pending: PendingCapture, qh: &QueueHandle<Self>) {
        self.pending_capture = Some(pending);
        self.pending_since = Some(std::time::Instant::now());

        for output_surface in &mut self.output_surfaces {
            if let Err(e) = rendering::draw_transparent(output_surface, qh) {
                log::warn!("Failed to hide overlay before capture: {}", e);
            }
        }
    }

    /// True once every surface has had its transparent frame presented, or the
    /// wait for those frame callbacks timed out.
    pub(super) fn ui_hidden(&self) -> bool {
        let timed_out = self
            .pending_since
            .is_none_or(|since| since.elapsed() >= Duration::from_millis(HIDE_TIMEOUT_MS));

        timed_out
            || !self
                .output_surfaces
                .iter()
                .any(|surf| surf.configured && surf.waiting_for_frame)
    }

    /// Runs the queued capture now that the UI is off screen.
    pub(super) fn run_pending_capture(&mut self) {
        let Some(pending) = self.pending_capture.take() else {
            return;
        };
        self.pending_since = None;

        // Reaching here on the timeout leaves frame callbacks outstanding, which
        // would keep `draw_output` from drawing anything ever again.
        for output_surface in &mut self.output_surfaces {
            output_surface.waiting_for_frame = false;
        }

        match pending {
            PendingCapture::Freeze => {
                if let Err(e) = self.capture_frozen_screens() {
                    log::warn!("Failed to capture freeze: {}", e);
                }
                self.enter_region_select();
            }
            PendingCapture::Image(mode) => {
                if matches!(mode, CaptureMode::ImageScreen | CaptureMode::ImageWindow) {
                    match self.pre_capture(mode) {
                        Ok(img) => self.captured_image = Some(img),
                        Err(e) => log::warn!("Pre-capture failed: {}", e),
                    }
                }
                self.exit = true;
                self.loop_signal.stop();
            }
        }
    }

    /// Leaves the mode selector for region selection, freezing the screen first
    /// when freeze is enabled.
    pub(super) fn begin_region_select(&mut self, qh: &QueueHandle<Self>) {
        if self.settings.freeze {
            self.hide_ui_for_capture(PendingCapture::Freeze, qh);
        } else {
            self.enter_region_select();
        }
    }

    /// Switches from the mode selector to region selection and refreshes the UI.
    pub(super) fn enter_region_select(&mut self) {
        use smithay_client_toolkit::seat::pointer::CursorIcon;

        self.phase = UiPhase::RegionSelect;

        for output_surface in &mut self.output_surfaces {
            output_surface.needs_render = true;
        }

        if let Some(themed_pointer) = &self.themed_pointer {
            let _ = themed_pointer.set_cursor(&self.conn, CursorIcon::Crosshair);
        }

        self.needs_redraw = true;
    }

    /// Captures the active window (cropped) or active monitor on the UI connection
    /// for non-area image modes.
    ///
    /// Falls back to the output at (0,0) or the first output when the compositor
    /// query for the active monitor fails.
    pub(super) fn pre_capture(&mut self, mode: CaptureMode) -> Result<CapturedImage> {
        let outputs_list = self.output_geometry();

        match mode {
            CaptureMode::ImageWindow => {
                let geometry = crate::compositor::get_active_window()?;
                let rect = crate::render::Rect::from_geometry_string(&geometry)?;
                crate::capture::capture_region(&mut self.screencopy, &outputs_list, rect)
            }
            CaptureMode::ImageScreen => {
                let by_target = self.target_monitor.as_deref().and_then(|target_name| {
                    outputs_list
                        .iter()
                        .find(|output| output.name == target_name)
                        .cloned()
                });
                let by_name = by_target.or_else(|| {
                    crate::compositor::get_active_monitor()
                        .ok()
                        .and_then(|name| {
                            outputs_list
                                .iter()
                                .find(|output| output.name == name)
                                .cloned()
                        })
                });
                let output = by_name
                    .or_else(|| {
                        outputs_list
                            .iter()
                            .find(|output| (output.logical.x, output.logical.y) == (0, 0))
                            .cloned()
                    })
                    .or_else(|| outputs_list.first().cloned())
                    .context("No outputs available")?;
                self.screencopy.capture(&output.output, output.transform)
            }
            _ => unreachable!(),
        }
    }

    /// Returns the geometry of every mapped overlay surface, with its output name.
    ///
    /// Taken from the surfaces rather than the outputs, because the size the
    /// compositor configured is what the selection was actually drawn against.
    pub(super) fn output_geometry(&self) -> Vec<crate::compositor::protocol::outputs::OutputInfo> {
        self.output_surfaces
            .iter()
            .map(|surface| {
                let info = self.outputs.get(&surface.output);
                crate::compositor::protocol::outputs::OutputInfo {
                    output: surface.output.clone(),
                    name: surface.name.clone(),
                    logical: crate::render::Rect::new(
                        surface.x,
                        surface.y,
                        surface.width as i32,
                        surface.height as i32,
                    ),
                    transform: info
                        .map(|info| info.transform)
                        .unwrap_or(wayland_client::protocol::wl_output::Transform::Normal),
                }
            })
            .collect()
    }
    /// Returns the index of the single output surface that should display the mode palette.
    pub(super) fn active_output_index(&self) -> Option<usize> {
        if let Some(current) = self.input.current_surface.as_ref() {
            if let Some(index) = self
                .output_surfaces
                .iter()
                .position(|surface| &surface.surface == current)
            {
                return Some(index);
            }
        }

        if let Ok(active_name) = crate::compositor::get_active_monitor() {
            if let Some(index) = self
                .output_surfaces
                .iter()
                .position(|surface| surface.name == active_name)
            {
                return Some(index);
            }
        }

        self.output_surfaces
            .iter()
            .position(|surface| (surface.x, surface.y) == (0, 0))
            .or_else(|| {
                self.output_surfaces
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, surface)| (surface.x, surface.y, surface.name.as_str()))
                    .map(|(index, _)| index)
            })
    }

    /// Returns the index of the single output surface that should display the region HUD.
    pub(super) fn region_hud_output_index(&self) -> Option<usize> {
        let target_point = self
            .selection
            .get_selection()
            .map(|rect| (rect.x, rect.y))
            .or(self.input.selection_start)
            .or_else(|| {
                self.input.current_surface.as_ref().map(|_| {
                    let (x, y) = self.input.pointer_position;
                    (x as i32, y as i32)
                })
            });

        target_point
            .and_then(|(px, py)| {
                self.output_surfaces.iter().position(|surface| {
                    px >= surface.x
                        && py >= surface.y
                        && px < surface.x + surface.width as i32
                        && py < surface.y + surface.height as i32
                })
            })
            .or_else(|| self.active_output_index())
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
        let is_palette_output = is_mode_select && (Some(index) == self.active_palette_output);
        let is_hud_output = !is_mode_select && (Some(index) == self.region_hud_output_index());

        let selection_hud = SelectionHud {
            purpose: if self.settings.request.is_ocr() {
                SelectionPurpose::Ocr
            } else if self
                .settings
                .request
                .mode
                .is_some_and(|mode| mode.is_video())
            {
                SelectionPurpose::Recording
            } else {
                SelectionPurpose::Screenshot
            },
            to_clipboard: self.to_clipboard,
            visible: is_hud_output,
        };
        let state = rendering::DrawState {
            selection: &self.selection,
            is_mode_select,
            is_palette_output,
            keybinds: &self.settings.keybinds,
            mode_select: &self.settings.mode_select,
            is_recording: self.is_recording,
            supports_window_capture: self.supports_window_capture,
            replay_status: self.replay_status.as_ref(),
            replay_configured: self.replay_configured,
            hovered_action: self
                .hovered_surface
                .as_ref()
                .filter(|surface| *surface == &self.output_surfaces[index].surface)
                .and(self.hovered_action),
            selection_hud,
            intro_progress: self.intro_progress,
        };
        rendering::draw_output(&mut self.output_surfaces[index], &state, qh)
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

        if self.phase == UiPhase::ModeSelect {
            let new_active_output = self.active_output_index();
            if new_active_output != self.active_palette_output {
                if let Some(prev) = self.active_palette_output {
                    if prev < self.output_surfaces.len() {
                        self.output_surfaces[prev].needs_render = true;
                    }
                }
                if let Some(curr) = new_active_output {
                    if curr < self.output_surfaces.len() {
                        self.output_surfaces[curr].needs_render = true;
                    }
                }
                self.active_palette_output = new_active_output;
                self.hovered_action = None;
                self.hovered_surface = None;
                self.needs_redraw = true;
            }

            let hovered_action = self.palette_action_at(surface, x, y);
            let surface_changed = self
                .hovered_surface
                .as_ref()
                .is_none_or(|hovered| hovered != surface);
            if self.hovered_action != hovered_action || surface_changed {
                self.hovered_action = hovered_action;
                self.hovered_surface = Some(surface.clone());
                self.needs_redraw = true;
                for output_surface in &mut self.output_surfaces {
                    output_surface.needs_render = true;
                }
            }

            if let Some(themed_pointer) = &self.themed_pointer {
                use smithay_client_toolkit::seat::pointer::CursorIcon;
                let icon = if hovered_action.is_some() {
                    CursorIcon::Pointer
                } else {
                    CursorIcon::Default
                };
                let _ = themed_pointer.set_cursor(&self.conn, icon);
            }
            return;
        }

        if self.input.mouse_pressed {
            if let Some((start_x, start_y)) = self.input.selection_start {
                self.selection
                    .update_drag(start_x, start_y, global_x as i32, global_y as i32);
                self.needs_redraw = true;
            }
        }
    }

    pub(super) fn handle_pointer_button(&mut self, pressed: bool, qh: &QueueHandle<Self>) {
        if self.phase == UiPhase::ModeSelect {
            if pressed {
                let action = self.input.current_surface.as_ref().and_then(|surface| {
                    let (global_x, global_y) = self.input.pointer_position;
                    self.output_surfaces
                        .iter()
                        .find(|output| &output.surface == surface)
                        .and_then(|output| {
                            self.palette_action_at(
                                surface,
                                global_x - output.x as f64,
                                global_y - output.y as f64,
                            )
                        })
                });
                if let Some(action) = action {
                    self.activate_palette_action(action, self.modifiers.ctrl, qh);
                } else {
                    self.cancel_selection();
                }
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

    fn palette_action_at(
        &self,
        surface: &wl_surface::WlSurface,
        x: f64,
        y: f64,
    ) -> Option<PaletteAction> {
        let active_index = self.active_palette_output?;
        let output = self.output_surfaces.get(active_index)?;
        if &output.surface != surface {
            return None;
        }

        let replay_state = crate::render::ReplayPaletteState {
            visible: self.replay_configured || self.replay_status.is_some(),
            can_save: self.replay_status.as_ref().map_or(false, |s| s.can_save()),
        };

        ModePaletteLayout::new(
            output.width as i32,
            output.height as i32,
            self.settings.mode_select.control_height,
            self.is_recording,
            self.supports_window_capture,
            replay_state,
            self.intro_progress,
        )
        .action_at(x, y)
    }

    pub(super) fn activate_palette_action(
        &mut self,
        action: PaletteAction,
        ctrl_held: bool,
        qh: &QueueHandle<Self>,
    ) {
        if action == PaletteAction::Ocr {
            self.settings.request.action = CaptureAction::Ocr;
            self.begin_region_select(qh);
            return;
        }

        if action == PaletteAction::StopRecording {
            self.chosen_mode = Some(CaptureMode::StopRecording);
            self.exit = true;
            self.loop_signal.stop();
            return;
        }

        if action == PaletteAction::SaveReplay {
            self.chosen_replay_action = Some(crate::cli::ReplayAction::Save);
            self.exit = true;
            self.loop_signal.stop();
            return;
        }

        if matches!(
            action,
            PaletteAction::ScreenshotScreen | PaletteAction::RecordScreen
        ) {
            if let Some(target_surf) = self
                .active_output_index()
                .and_then(|idx| self.output_surfaces.get(idx))
            {
                self.target_monitor = Some(target_surf.name.clone());
                self.target_geometry = Some(format!(
                    "{},{} {}x{}",
                    target_surf.x, target_surf.y, target_surf.width, target_surf.height
                ));
            }
        }

        let (mode, is_area) = match action {
            PaletteAction::ScreenshotArea => (CaptureMode::ImageArea, true),
            PaletteAction::ScreenshotScreen => (CaptureMode::ImageScreen, false),
            PaletteAction::ScreenshotWindow => (CaptureMode::ImageWindow, false),
            PaletteAction::RecordArea => (CaptureMode::VideoArea, true),
            PaletteAction::RecordScreen => (CaptureMode::VideoScreen, false),
            PaletteAction::RecordWindow => (CaptureMode::VideoWindow, false),
            PaletteAction::Ocr | PaletteAction::StopRecording | PaletteAction::SaveReplay => {
                unreachable!()
            }
        };

        self.chosen_mode = Some(mode);
        self.to_clipboard = ctrl_held && !mode.is_video();

        if is_area {
            self.settings.request.mode = Some(mode);
            self.begin_region_select(qh);
        } else {
            self.hide_ui_for_capture(PendingCapture::Image(mode), qh);
        }
    }

    fn complete_selection(&mut self) {
        if let Some(rect) = self.selection.get_selection() {
            let outputs_list = self.output_geometry();

            match capture::complete_selection(
                &self.conn,
                &mut self.screencopy,
                &mut self.output_surfaces,
                &outputs_list,
                &self.settings,
                rect,
            ) {
                Ok((geometry, cropped)) => {
                    self.selection_geometry = geometry;
                    self.captured_image = cropped;
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

    /// Abandons the capture and leaves the overlay.
    ///
    /// Cancelling is a normal outcome, not a failure: it unwinds through the
    /// event loop and `run` returns with nothing chosen, so chomp exits 0 and
    /// anything scripting it can tell cancel from error.
    pub(super) fn cancel_selection(&mut self) {
        log::debug!("Selection cancelled by user");

        self.chosen_mode = None;
        self.chosen_replay_action = None;
        self.target_monitor = None;
        self.target_geometry = None;
        self.selection_geometry = None;
        self.captured_image = None;
        self.pending_capture = None;
        self.exit = true;
        self.loop_signal.stop();
    }
}
#[cfg(test)]
/// Resolves which output should display the single mode palette.
///
/// Priority:
/// 1. Output containing the pointer index.
/// 2. Output matching the compositor's active monitor name.
/// 3. Deterministic fallback: output at (0, 0), or lowest (x, y, name).
pub(crate) fn resolve_palette_output(
    pointer_output: Option<usize>,
    compositor_active: Option<&str>,
    outputs: &[(String, i32, i32)],
) -> Option<usize> {
    if outputs.is_empty() {
        return None;
    }
    if let Some(idx) = pointer_output {
        if idx < outputs.len() {
            return Some(idx);
        }
    }
    if let Some(active_name) = compositor_active {
        if let Some(idx) = outputs.iter().position(|(name, _, _)| name == active_name) {
            return Some(idx);
        }
    }
    if let Some(idx) = outputs.iter().position(|(_, x, y)| *x == 0 && *y == 0) {
        return Some(idx);
    }
    outputs
        .iter()
        .enumerate()
        .min_by_key(|(_, (name, x, y))| (*x, *y, name.as_str()))
        .map(|(idx, _)| idx)
}

#[cfg(test)]
/// Resolves which output should display the region HUD.
///
/// Priority:
/// 1. Output containing the target point (selection origin, drag start, or pointer).
/// 2. Fallback palette output.
pub(crate) fn resolve_hud_output(
    target_point: Option<(i32, i32)>,
    outputs: &[(i32, i32, u32, u32)],
    fallback: Option<usize>,
) -> Option<usize> {
    if outputs.is_empty() {
        return None;
    }
    if let Some((px, py)) = target_point {
        if let Some(idx) = outputs
            .iter()
            .position(|&(x, y, w, h)| px >= x && py >= y && px < x + w as i32 && py < y + h as i32)
        {
            return Some(idx);
        }
    }
    fallback
}

#[cfg(test)]
#[path = "output_test.rs"]
mod tests;
