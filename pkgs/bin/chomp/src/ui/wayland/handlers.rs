use smithay_client_toolkit::seat::pointer::CursorIcon;
use smithay_client_toolkit::{
    compositor::CompositorHandler,
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    shm::{Shm, ShmHandler},
};
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    protocol::{wl_callback, wl_output, wl_pointer, wl_seat, wl_surface},
};

use super::App;

/// Returns true if `keysym` matches the first character of the config key string.
fn key_matches(keysym: Keysym, key: &str) -> bool {
    key.chars()
        .next()
        .map_or(false, |c| Keysym::from_char(c) == keysym)
}

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
        self.input.current_surface = Some(surface.clone());
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
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        for i in 0..self.output_surfaces.len() {
            if &self.output_surfaces[i].layer_surface == layer {
                self.output_surfaces[i].configured = true;
                self.output_surfaces[i].width = configure.new_size.0;
                self.output_surfaces[i].height = configure.new_size.1;

                let _ = self.draw_index(i, qh);
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
            let surface = self.compositor_state.create_surface(qh);
            let themed_pointer = self.seat_state.get_pointer_with_theme(
                qh,
                &seat,
                self.shm_state.wl_shm(),
                surface,
                smithay_client_toolkit::seat::pointer::ThemeSpec::System,
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
        use super::UiPhase;
        for event in events {
            match event.kind {
                PointerEventKind::Enter { .. } => {
                    self.input.current_surface = Some(event.surface.clone());

                    if let Some(themed_pointer) = &self.themed_pointer {
                        let icon = if self.phase == UiPhase::ModeSelect {
                            CursorIcon::Default
                        } else {
                            CursorIcon::Crosshair
                        };
                        let _ = themed_pointer.set_cursor(conn, icon);
                    }
                }
                PointerEventKind::Leave { .. } => {}
                PointerEventKind::Motion { .. } => {
                    if let Some(surface) = self.input.current_surface.clone() {
                        self.handle_pointer_move(&surface, event.position.0, event.position.1);
                    }
                }
                PointerEventKind::Press { button, .. } => {
                    if button == 0x110 {
                        self.handle_pointer_button(true);
                    } else if button == 0x111 {
                        self.cancel_selection();
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if button == 0x110 {
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
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {
    }

    fn press_key(
        &mut self,
        conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        use super::UiPhase;
        use crate::capture::CaptureMode;

        if event.keysym == Keysym::Escape || event.keysym == Keysym::q {
            self.cancel_selection();
            return;
        }

        if self.phase != UiPhase::ModeSelect {
            return;
        }

        let kb = self.settings.keybinds.clone();

        // OCR: transition to region select without setting a capture mode
        if key_matches(event.keysym, &kb.ocr) {
            self.settings.ocr = true;
            self.phase = UiPhase::RegionSelect;
            if self.settings.freeze
                && self
                    .output_surfaces
                    .iter()
                    .any(|os| os.frozen_buffer.is_none())
            {
                if let Err(e) = self.capture_frozen_screens() {
                    log::warn!("Failed to capture freeze: {}", e);
                }
            }
            for os in &mut self.output_surfaces {
                os.needs_render = true;
            }
            if let Some(themed_pointer) = &self.themed_pointer {
                let _ = themed_pointer.set_cursor(conn, CursorIcon::Crosshair);
            }
            self.needs_redraw = true;
            return;
        }

        // Stop recording key — only active when a recording is running
        if self.is_recording && key_matches(event.keysym, &kb.stop_recording) {
            self.chosen_mode = Some(CaptureMode::StopRecording);
            self.exit = true;
            self.loop_signal.stop();
            return;
        }

        let result = if key_matches(event.keysym, &kb.screenshot_area) {
            Some((CaptureMode::ImageArea, true))
        } else if key_matches(event.keysym, &kb.screenshot_screen) {
            Some((CaptureMode::ImageScreen, false))
        } else if key_matches(event.keysym, &kb.screenshot_window) {
            Some((CaptureMode::ImageWindow, false))
        } else if key_matches(event.keysym, &kb.record_area) {
            Some((CaptureMode::VideoArea, true))
        } else if key_matches(event.keysym, &kb.record_screen) {
            Some((CaptureMode::VideoScreen, false))
        } else if key_matches(event.keysym, &kb.record_window) {
            Some((CaptureMode::VideoWindow, false))
        } else {
            None
        };

        let Some((mode, is_area)) = result else {
            return;
        };

        if is_area {
            self.chosen_mode = Some(mode);
            self.settings.mode = Some(mode);
            self.phase = UiPhase::RegionSelect;
            if self.settings.freeze {
                // Frozen backgrounds are pre-captured at startup before any buffer is
                // attached.  Only re-capture if that failed for some output.
                if self
                    .output_surfaces
                    .iter()
                    .any(|os| os.frozen_buffer.is_none())
                {
                    if let Err(e) = self.capture_frozen_screens() {
                        log::warn!("Failed to capture freeze: {}", e);
                    }
                }
            }
            for os in &mut self.output_surfaces {
                os.needs_render = true;
            }
            if let Some(themed_pointer) = &self.themed_pointer {
                let _ = themed_pointer.set_cursor(conn, CursorIcon::Crosshair);
            }
            self.needs_redraw = true;
        } else {
            // Unmap mode-select surfaces before the screencopy request.
            // Using the same connection guarantees ordering: the screencopy's
            // first internal roundtrip flushes these null-buffer commits,
            // ensuring the compositor removes the overlay before it captures.
            for os in &self.output_surfaces {
                os.surface.attach(None, 0, 0);
                os.surface.commit();
            }
            if matches!(mode, CaptureMode::ImageScreen | CaptureMode::ImageWindow) {
                match self.pre_capture(mode) {
                    Ok(img) => self.captured_image = Some(img),
                    Err(e) => log::warn!("Pre-capture failed: {}", e),
                }
            }
            self.chosen_mode = Some(mode);
            self.exit = true;
            self.loop_signal.stop();
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
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState];
}

// Frame callback handler for vsync synchronization
impl Dispatch<wl_callback::WlCallback, ()> for App {
    fn event(
        state: &mut Self,
        callback: &wl_callback::WlCallback,
        _event: wl_callback::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        for output_surface in &mut state.output_surfaces {
            if let Some(ref frame_callback) = output_surface.frame_callback {
                if frame_callback == callback {
                    output_surface.waiting_for_frame = false;
                    output_surface.frame_callback = None;
                    output_surface.needs_render = true;
                    state.needs_redraw = true;
                    break;
                }
            }
        }
    }
}

delegate_compositor!(App);
delegate_output!(App);
delegate_shm!(App);
delegate_seat!(App);
delegate_pointer!(App);
delegate_keyboard!(App);
delegate_layer!(App);
delegate_registry!(App);
