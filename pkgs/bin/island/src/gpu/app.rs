//! The wgpu frontend's Wayland connection and loop: outputs become islands, and whatever features,
//! the compositor and `island ctl` send is applied as it arrives.

use std::{
    os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd},
    pin::Pin,
    ptr::NonNull,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread,
    time::{Duration, Instant},
};

use futures::{
    Stream,
    channel::{mpsc::UnboundedReceiver, oneshot},
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, FrameCallbackData},
    delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers, RepeatInfo},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
};
use tracing::{error, info, warn};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle, delegate_noop,
    globals::{GlobalList, registry_queue_init},
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_surface},
};
use wayland_protocols::{
    ext::background_effect::v1::client::{
        ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1,
        ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
    },
    wp::{
        cursor_shape::v1::client::{
            wp_cursor_shape_device_v1::{Shape, WpCursorShapeDeviceV1},
            wp_cursor_shape_manager_v1::WpCursorShapeManagerV1,
        },
        fractional_scale::v1::client::{
            wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
            wp_fractional_scale_v1::{self, WpFractionalScaleV1},
        },
        idle_inhibit::zv1::client::{
            zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1,
            zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1,
        },
        viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
    },
};

use super::{
    cover::Cover,
    island::{Cursor, Island, Look, PIXELS_PER_NOTCH},
    node::SEMIBOLD,
    paint::Painter,
    renderer::Gpu,
};
use uuid::Uuid;

use crate::{
    bar,
    config::{self, Startup},
    control::{Asked, Control, Reply, Report, Row},
    features::{self, AnyLocal, Arrivals, Features, Frontend},
    glyphs::Glyphs,
    key::Key,
    profile,
    stage::{COLLAPSED_H, Grab, Panel, SURFACE_H, SURFACE_W, Stage, report},
    sys::{self, Samples},
    wm::Snapshot,
};

/// The compositor gives a scale in these parts of one.
const FRACTION: f32 = 120.0;
const REPEAT_AFTER: Duration = Duration::from_millis(400);
const REPEAT_EVERY: Duration = Duration::from_millis(33);
/// Acted on when let go: closing hands the keyboard to the window below, which would see them.
const ON_RELEASE: [&str; 2] = ["escape", "enter"];

struct Woken(OwnedFd);

impl Wake for Woken {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        let one = 1_u64.to_ne_bytes();
        // Full only if it is already waking the loop.
        unsafe { libc::write(self.0.as_raw_fd(), one.as_ptr().cast(), one.len()) };
    }
}

/// The scales the compositor would like each surface drawn at, as they arrive.
#[derive(Default)]
struct Scales(Vec<(wl_surface::WlSurface, f32)>);

impl Dispatch<WpFractionalScaleV1, wl_surface::WlSurface> for Scales {
    fn event(
        state: &mut Self,
        _: &WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        surface: &wl_surface::WlSurface,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            state.0.push((surface.clone(), scale as f32 / FRACTION));
        }
    }
}

delegate_noop!(Scales: WpFractionalScaleManagerV1);
delegate_noop!(Scales: WpViewporter);
delegate_noop!(Scales: WpViewport);
delegate_noop!(Scales: WpCursorShapeManagerV1);
delegate_noop!(Scales: WpCursorShapeDeviceV1);

pub struct EffectState;
delegate_noop!(EffectState: ignore ExtBackgroundEffectManagerV1);
delegate_noop!(EffectState: ignore ExtBackgroundEffectSurfaceV1);
delegate_noop!(EffectState: ignore ZwpIdleInhibitManagerV1);
delegate_noop!(EffectState: ignore ZwpIdleInhibitorV1);

/// A key held down, by its code as the shift held with it changes its name.
struct Repeat {
    code: u32,
    key: Key,
    at: Instant,
}

struct App {
    registry_state: RegistryState,
    output_state: OutputState,
    seat_state: SeatState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    conn: Connection,
    blur_manager: Option<ExtBackgroundEffectManagerV1>,
    blur_handle: QueueHandle<EffectState>,
    idle_manager: Option<ZwpIdleInhibitManagerV1>,
    scale_handle: QueueHandle<Scales>,
    viewporter: Option<WpViewporter>,
    fractional: Option<WpFractionalScaleManagerV1>,
    cursors: Option<WpCursorShapeManagerV1>,
    /// Sets the pointer's look.
    cursor: Option<WpCursorShapeDeviceV1>,
    /// The last time the pointer came onto something of ours, and how it looks.
    cursor_at: Option<(u32, Option<Cursor>)>,
    /// How long a key is held before it repeats, and how often then; `None` if keys do not repeat.
    key_repeat: Option<(Duration, Duration)>,
    gpu: Option<Gpu>,
    painter: Painter,
    islands: Vec<Island>,
    pointer: Option<wl_pointer::WlPointer>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    modifiers: Modifiers,
    /// A key held down, and when it next repeats.
    repeat: Option<Repeat>,
    /// A key that acts when let go.
    held: Option<Key>,
    covers: Vec<Cover>,
    /// Which island holds the keyboard or the clicks, and which.
    grabbed: Option<(wl_output::WlOutput, Grab)>,
    glyphs: Glyphs,
    features: Rc<Features>,
    config: Startup,
    system: Rc<config::System>,
    samples: Samples,
    workspaces: Option<Snapshot>,
    new_outputs: Vec<wl_output::WlOutput>,
}

impl App {
    fn island_of(&mut self, surface: &wl_surface::WlSurface) -> Option<&mut Island> {
        self.islands
            .iter_mut()
            .find(|island| island.layer.wl_surface() == surface)
    }

    fn open(&mut self, qh: &QueueHandle<Self>, output: wl_output::WlOutput) {
        let info = self.output_state.info(&output);
        let scale = info.as_ref().map_or(1, |info| info.scale_factor);
        let surface = self.compositor.create_surface(qh);
        let namespace = profile::namespace();
        let layer = self.layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Overlay,
            Some(namespace),
            Some(&output),
        );
        layer.set_anchor(Anchor::TOP);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.set_size(SURFACE_W as u32, SURFACE_H as u32);
        layer.set_exclusive_zone((self.config.top + COLLAPSED_H) as i32);
        let stage = Stage::new(
            self.config.layout.clone(),
            self.features.clone(),
            self.workspaces.clone(),
        );
        let look = Look {
            top: self.config.top,
            corner: self.config.corner,
            opacity: self.config.opacity,
            shadow: self.config.shadow,
            glow: self.config.glow,
            border: self.config.border,
            samples: self.samples.clone(),
            system: self.system.clone(),
        };
        let mut island = Island::new(stage, layer, output, scale as f32, look);
        let surface = island.layer.wl_surface().clone();
        match (&self.viewporter, &self.fractional) {
            (Some(viewporter), Some(fractional)) => {
                let viewport = viewporter.get_viewport(&surface, &self.scale_handle, ());
                viewport.set_destination(SURFACE_W as i32, SURFACE_H as i32);
                let preferred =
                    fractional.get_fractional_scale(&surface, &self.scale_handle, surface.clone());
                island.fractional = Some((viewport, preferred));
            }
            _ => surface.set_buffer_scale(scale),
        }
        if let Some(name) = info.and_then(|info| info.name) {
            island.set_output_name(&name);
        }
        island.blur = self.blur_manager.as_ref().map(|manager| {
            manager.get_background_effect(island.layer.wl_surface(), &self.blur_handle, ())
        });
        island.layer.commit();
        info!("opened an island");
        self.islands.push(island);
    }

    fn start_drawing(&mut self, index: usize) {
        let Some(display) = NonNull::new(self.conn.backend().display_ptr().cast()) else {
            return;
        };
        let island = &mut self.islands[index];
        let Some(window) = NonNull::new(island.layer.wl_surface().id().as_ptr().cast()) else {
            return;
        };
        let size = [SURFACE_W, SURFACE_H];
        let surface = match &self.gpu {
            Some(gpu) => gpu.surface(window),
            None => Gpu::new(display, window, size).map(|(gpu, surface)| {
                self.gpu = Some(gpu);
                surface
            }),
        };
        match surface {
            Ok(surface) => {
                if let Some(gpu) = &self.gpu {
                    let (width, height) = island.buffer_size();
                    gpu.configure(&surface, width, height);
                }
                island.surface = Some(surface);
                island.dirty = true;
            }
            Err(error) => error!("{error}"),
        }
    }

    /// The island holding the keyboard or the clicks.
    fn holder(&mut self) -> Option<&mut Island> {
        let (held, _) = self.grabbed.as_ref()?;
        self.islands
            .iter_mut()
            .find(|island| island.output == *held)
    }

    /// The island with something open, so `island ctl` acts on what is showing, or else the one
    /// on the focused monitor.
    fn target(&mut self) -> Option<&mut Island> {
        let focused = self
            .workspaces
            .as_ref()
            .and_then(Snapshot::focused)
            .map(|monitor| Uuid::new_v5(&Uuid::NAMESPACE_DNS, monitor.name.as_bytes()));
        let at = self
            .grabbed
            .as_ref()
            .and_then(|(held, _)| {
                self.islands
                    .iter()
                    .position(|island| island.output == *held)
            })
            .or_else(|| {
                let focused = focused?;
                self.islands
                    .iter()
                    .position(|island| island.stage.display == Some(focused))
            })
            .unwrap_or(0);
        self.islands.get_mut(at)
    }

    fn key(&mut self, key: &Key) {
        if let Some(island) = self.holder() {
            island.key(key);
        }
    }

    /// Keeps the pointer's look to what it is over.
    fn sync_cursor(&mut self) {
        let Some((serial, shown)) = self.cursor_at else {
            return;
        };
        let wanted = self
            .islands
            .iter()
            .map(Island::cursor)
            .find(|cursor| *cursor != Cursor::Default)
            .unwrap_or(Cursor::Default);
        if shown == Some(wanted) {
            return;
        }
        if let Some(device) = &self.cursor {
            device.set_shape(
                serial,
                match wanted {
                    Cursor::Default => Shape::Default,
                    Cursor::Pointer => Shape::Pointer,
                    Cursor::Grab => Shape::Grab,
                    Cursor::Grabbing => Shape::Grabbing,
                },
            );
        }
        self.cursor_at = Some((serial, Some(wanted)));
    }

    fn sync_covers(&mut self, qh: &QueueHandle<Self>) {
        let held = self.grabbed.as_ref().map(|(held, _)| held.clone());
        let grabbing: Vec<(wl_output::WlOutput, Grab)> = self
            .islands
            .iter()
            .filter(|island| island.grab() != Grab::None)
            .map(|island| (island.output.clone(), island.grab()))
            .collect();
        let taken = grabbing
            .iter()
            .find(|(output, _)| Some(output) != held.as_ref())
            .or_else(|| grabbing.first())
            .cloned();
        if let (Some(held), Some((output, _))) = (&held, &taken)
            && held != output
            && let Some(island) = self
                .islands
                .iter_mut()
                .find(|island| island.output == *held)
        {
            island.dismiss();
        }
        if taken != self.grabbed {
            self.grabbed = taken.clone();
            self.covers.clear();
            if let Some((held, grab)) = &taken {
                let outputs: Vec<wl_output::WlOutput> = self
                    .islands
                    .iter()
                    .map(|island| island.output.clone())
                    .collect();
                for output in outputs {
                    let keys = *grab == Grab::Keyboard && output == *held;
                    self.cover(qh, output, keys);
                }
            }
        }
        let holes = self
            .holder()
            .map(|island| (island.output.clone(), island.holes()));
        if let Some((output, holes)) = holes {
            for cover in &mut self.covers {
                let own: &[[i32; 4]] = if cover.output == output { &holes } else { &[] };
                if cover.size.0 > 0 {
                    cover.take_pointer_around(own, &self.compositor);
                }
            }
        }
    }

    fn cover(&mut self, qh: &QueueHandle<Self>, output: wl_output::WlOutput, keys: bool) {
        let scale = self
            .output_state
            .info(&output)
            .map_or(1, |info| info.scale_factor);
        let surface = self.compositor.create_surface(qh);
        let name = if keys {
            "island-keyboard"
        } else {
            "island-catcher"
        };
        let layer = self.layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Overlay,
            Some(name),
            Some(&output),
        );
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(if keys {
            KeyboardInteractivity::Exclusive
        } else {
            KeyboardInteractivity::None
        });
        layer.wl_surface().set_buffer_scale(scale);
        layer.commit();
        self.covers.push(Cover::new(layer, output, scale));
    }

    fn open_panel(
        &mut self,
        name: &str,
        view: Option<&str>,
        toggle: bool,
    ) -> Result<Reply, String> {
        let Some(panel) = Panel::parse(name) else {
            return Err(format!("No panel named {name}"));
        };
        let island = self.target().ok_or("No island is showing")?;
        let open = island.open_panel(panel, view, toggle);
        Ok(report::opened(name, view, island.stage.output(), open).into())
    }

    fn answer(&mut self, asked: Asked) {
        let Asked { control, answer } = asked;
        let reply: Result<Reply, String> = match control {
            Control::Open(name, view) => self.open_panel(&name, view.as_deref(), false),
            Control::Toggle(name, view) => self.open_panel(&name, view.as_deref(), true),
            Control::Close => {
                for island in &mut self.islands {
                    island.dismiss();
                }
                Ok(Report::did(Row::said("Closed what was open")).into())
            }
            Control::Feature(name, args) => self
                .features
                .get(name)
                .ok_or_else(|| format!("The {name} feature is not running"))
                .map(|feature| {
                    let args: Vec<&str> = args.iter().map(String::as_str).collect();
                    feature.command(&args)
                }),
            Control::Status => Ok(report::status(self.workspaces.as_ref(), &self.features).into()),
            Control::Log(_) => Ok(Report::default().into()),
        };
        match reply {
            Ok(Reply::Now(report)) => {
                let _ = answer.send(Ok(report));
            }
            Ok(Reply::Later(report)) => {
                thread::spawn(move || {
                    let report = pollster::block_on(report)
                        .unwrap_or_else(|_| Err("island stopped before answering".to_owned()));
                    let _ = answer.send(report);
                });
            }
            Err(error) => {
                let _ = answer.send(Err(error));
            }
        }
    }

    fn set_workspaces(&mut self, snapshot: Snapshot) {
        self.workspaces = Some(snapshot.clone());
        for island in &mut self.islands {
            island.set_workspaces(snapshot.clone());
        }
    }
}

impl Frontend for App {
    fn islands(&self) -> usize {
        self.islands.len()
    }

    fn local(&self, island: usize, name: &str) -> Option<&dyn AnyLocal> {
        self.islands.get(island)?.stage.local(name)
    }

    fn update_local(
        &mut self,
        island: usize,
        name: &'static str,
        action: &mut dyn FnMut(&mut dyn AnyLocal),
    ) {
        if let Some(island) = self.islands.get_mut(island) {
            island.stage.update_local(name, action);
            island.local_changed(name);
        }
    }

    fn changed(&mut self, name: &'static str) {
        for island in &mut self.islands {
            island.feature_changed(name);
        }
    }

    fn live(&mut self, name: &'static str) {
        for island in &mut self.islands {
            island.feature_live(name);
        }
    }
}

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        scale: i32,
    ) {
        let gpu = self.gpu.as_ref();
        if let Some(island) = self
            .islands
            .iter_mut()
            .find(|island| island.layer.wl_surface() == surface)
            .filter(|island| island.fractional.is_none())
        {
            surface.set_buffer_scale(scale);
            island.set_scale(scale as f32, gpu);
            island.dirty = true;
        }
        if let Some(cover) = self
            .covers
            .iter_mut()
            .find(|cover| cover.layer.wl_surface() == surface)
        {
            cover.rescale(scale, gpu);
        }
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        _: u32,
    ) {
        if let Some(island) = self.island_of(surface) {
            island.turn.answered();
        }
    }

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: wl_output::WlOutput) {
        self.new_outputs.push(output);
    }

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn output_destroyed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        self.islands.retain(|island| island.output != output);
    }
}

impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        self.islands.retain(|island| &island.layer != layer);
        self.covers.retain(|cover| &cover.layer != layer);
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        if let Some(island) = self
            .islands
            .iter_mut()
            .find(|island| &island.layer == layer)
        {
            island.configured = true;
            island.dirty = true;
        }
        if let (Some(cover), Some(gpu)) = (
            self.covers.iter_mut().find(|cover| &cover.layer == layer),
            &self.gpu,
        ) {
            cover.size = configure.new_size;
            if cover.surface.is_none()
                && let Some(window) = NonNull::new(cover.layer.wl_surface().id().as_ptr().cast())
            {
                cover.surface = gpu.surface(window).ok();
            }
            cover.draw(gpu);
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
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            match self.seat_state.get_pointer(qh, &seat) {
                Ok(pointer) => {
                    self.cursor = self
                        .cursors
                        .as_ref()
                        .map(|manager| manager.get_pointer(&pointer, &self.scale_handle, ()));
                    self.pointer = Some(pointer);
                }
                Err(error) => warn!("no pointer: {error}"),
            }
        }
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            match self.seat_state.get_keyboard(qh, &seat, None) {
                Ok(keyboard) => self.keyboard = Some(keyboard),
                Err(error) => warn!("no keyboard: {error}"),
            }
        }
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer
            && let Some(pointer) = self.pointer.take()
        {
            if let Some(device) = self.cursor.take() {
                device.destroy();
            }
            self.cursor_at = None;
            pointer.release();
        }
        if capability == Capability::Keyboard
            && let Some(keyboard) = self.keyboard.take()
        {
            keyboard.release();
            self.repeat = None;
            self.held = None;
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if let PointerEventKind::Enter { serial } = event.kind {
                self.cursor_at = Some((serial, None));
            }
            if self
                .covers
                .iter()
                .any(|cover| cover.layer.wl_surface() == &event.surface)
            {
                if matches!(event.kind, PointerEventKind::Press { .. })
                    && let Some(island) = self.holder()
                {
                    island.dismiss();
                }
                continue;
            }
            let Some(island) = self.island_of(&event.surface) else {
                continue;
            };
            let at = (event.position.0 as f32, event.position.1 as f32);
            match &event.kind {
                PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                    island.pointer_moved(at);
                }
                PointerEventKind::Leave { .. } => island.pointer_left(),
                PointerEventKind::Press { button, .. } => island.press(*button),
                PointerEventKind::Release { button, .. } => island.release(*button),
                PointerEventKind::Axis {
                    vertical, source, ..
                } => {
                    let delta = if vertical.value120 != 0 {
                        vertical.value120 as f32 / 120.0
                    } else if vertical.discrete != 0 {
                        vertical.discrete as f32
                    } else {
                        vertical.absolute as f32 / PIXELS_PER_NOTCH
                    };
                    island.scroll(-delta, *source == Some(wl_pointer::AxisSource::Finger));
                }
            }
        }
    }
}

fn key_of(event: &KeyEvent, modifiers: &Modifiers) -> Key {
    let named = match event.keysym {
        Keysym::Escape => Some("escape"),
        Keysym::Return | Keysym::KP_Enter => Some("enter"),
        Keysym::BackSpace => Some("backspace"),
        Keysym::Tab | Keysym::ISO_Left_Tab => Some("tab"),
        Keysym::space => Some("space"),
        Keysym::Up => Some("up"),
        Keysym::Down => Some("down"),
        Keysym::Left => Some("left"),
        Keysym::Right => Some("right"),
        Keysym::Delete => Some("delete"),
        Keysym::Home => Some("home"),
        Keysym::End => Some("end"),
        Keysym::Page_Up => Some("pageup"),
        Keysym::Page_Down => Some("pagedown"),
        _ => None,
    };
    let typed = event
        .utf8
        .clone()
        .filter(|text| !text.chars().any(char::is_control));
    let name = named.map(str::to_owned).unwrap_or_else(|| {
        event
            .keysym
            .key_char()
            .map(|letter| letter.to_lowercase().to_string())
            .or_else(|| typed.clone())
            .unwrap_or_default()
    });
    Key {
        name,
        typed,
        control: modifiers.ctrl,
        shift: modifiers.shift,
        alt: modifiers.alt,
    }
}

impl KeyboardHandler for App {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
        self.repeat = None;
        self.held = None;
    }

    fn press_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        let key = key_of(&event, &self.modifiers);
        if key.name.is_empty() {
            return;
        }
        if ON_RELEASE.contains(&key.name.as_str()) {
            self.held.get_or_insert(key);
            return;
        }
        self.key(&key);
        if let Some((after, _)) = self.key_repeat {
            self.repeat = Some(Repeat {
                code: event.raw_code,
                key,
                at: Instant::now() + after,
            });
        }
    }

    fn repeat_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.repeat.take_if(|repeat| repeat.code == event.raw_code);
        let name = key_of(&event, &self.modifiers).name;
        if let Some(key) = self.held.take_if(|held| held.name == name) {
            self.key(&key);
        }
    }

    fn update_repeat_info(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        info: RepeatInfo,
    ) {
        self.key_repeat = match info {
            RepeatInfo::Repeat { rate, delay } => Some((
                Duration::from_millis(u64::from(delay)),
                Duration::from_secs(1) / rate.get(),
            )),
            RepeatInfo::Disable => None,
        };
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        modifiers: Modifiers,
        _: RawModifiers,
        _: u32,
    ) {
        self.modifiers = modifiers;
    }
}

delegate_registry!(App);

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

smithay_client_toolkit::delegate_dispatch2!(App);

fn event_fd() -> Result<OwnedFd, String> {
    // A new descriptor, which `OwnedFd` then owns.
    let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_CLOEXEC) };
    if fd < 0 {
        return Err("cannot make an eventfd".to_owned());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

fn drained(fd: &OwnedFd) {
    let mut count = [0_u8; 8];
    // Nothing to read is the same as having read it.
    unsafe { libc::read(fd.as_raw_fd(), count.as_mut_ptr().cast(), count.len()) };
}

fn drain<T>(
    stream: &mut (impl Stream<Item = T> + Unpin),
    cx: &mut Context<'_>,
    mut each: impl FnMut(T),
) {
    while let Poll::Ready(Some(item)) = Pin::new(&mut *stream).poll_next(cx) {
        each(item);
    }
}

/// Sleeps until there is something to do. Dispatching one queue moves what has been read for the
/// others into them, where the socket no longer shows it, so both are emptied first: a read made
/// by the graphics driver would otherwise leave an event waiting through the sleep.
fn wait(
    (queue, app): (&mut EventQueue<App>, &mut App),
    (scale_queue, scales): (&mut EventQueue<Scales>, &mut Scales),
    conn: &Connection,
    wake: &OwnedFd,
    until: Option<Instant>,
) {
    let _ = conn.flush();
    let caught = scale_queue.dispatch_pending(scales).unwrap_or(1)
        + queue.dispatch_pending(app).unwrap_or(1);
    if caught > 0 {
        return;
    }
    let Some(read) = queue.prepare_read() else {
        return;
    };
    let timeout = until.map_or(-1, |until| {
        until
            .saturating_duration_since(Instant::now())
            .as_nanos()
            .div_ceil(1_000_000)
            .min(i32::MAX as u128) as i32
    });
    let mut fds = [
        libc::pollfd {
            fd: read.connection_fd().as_fd().as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: wake.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        },
    ];
    // Both descriptors live past the call.
    unsafe { libc::poll(fds.as_mut_ptr(), 2, timeout) };
    if fds[0].revents & libc::POLLIN != 0 {
        let _ = read.read();
    }
    if fds[1].revents & libc::POLLIN != 0 {
        drained(wake);
    }
}

fn bind(
    globals: &GlobalList,
    handle: &QueueHandle<EffectState>,
) -> Option<ExtBackgroundEffectManagerV1> {
    globals.bind(handle, 1..=1, ()).ok()
}

/// Runs until the process ends.
pub fn run(
    config: Startup,
    system: config::System,
    mut controls: UnboundedReceiver<Asked>,
    mut workspaces: Option<UnboundedReceiver<Snapshot>>,
) -> Result<(), String> {
    let conn =
        Connection::connect_to_env().map_err(|error| format!("no Wayland display: {error}"))?;
    let (globals, mut queue) =
        registry_queue_init(&conn).map_err(|error| format!("no Wayland globals: {error}"))?;
    let qh = queue.handle();
    let compositor = CompositorState::bind(&globals, &qh)
        .map_err(|error| format!("no Wayland compositor: {error}"))?;
    let layer_shell =
        LayerShell::bind(&globals, &qh).map_err(|error| format!("no layer shell: {error}"))?;
    let blur_queue = conn.new_event_queue::<EffectState>();
    let blur_handle = blur_queue.handle();
    let blur_manager = bind(&globals, &blur_handle);
    let idle_manager = globals.bind(&blur_handle, 1..=1, ()).ok();
    let mut scale_queue = conn.new_event_queue::<Scales>();
    let scale_handle = scale_queue.handle();
    let viewporter = globals.bind(&scale_handle, 1..=1, ()).ok();
    let fractional = globals.bind(&scale_handle, 1..=1, ()).ok();
    let cursors = globals.bind(&scale_handle, 1..=1, ()).ok();
    let mut scales = Scales::default();

    let layout = config.layout.clone();
    let (started, mut arrivals): (Features, Arrivals) = features::start(
        |name| layout.widgets().any(|widget| widget.0 == name),
        config.glow > 0.0,
        &config.features,
    );

    let (sent_glyphs, mut resolved) = oneshot::channel();
    thread::spawn(move || {
        let specs = [features::glyphs(), bar::GLYPHS.to_vec()].concat();
        let _ = sent_glyphs.send(Glyphs::resolve(&specs));
    });

    let mut app = App {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        seat_state: SeatState::new(&globals, &qh),
        compositor,
        layer_shell,
        conn: conn.clone(),
        blur_manager,
        blur_handle,
        idle_manager,
        scale_handle,
        viewporter,
        fractional,
        cursors,
        cursor: None,
        cursor_at: None,
        key_repeat: Some((REPEAT_AFTER, REPEAT_EVERY)),
        gpu: None,
        painter: Painter::new(),
        islands: Vec::new(),
        pointer: None,
        keyboard: None,
        modifiers: Modifiers::default(),
        repeat: None,
        held: None,
        covers: Vec::new(),
        grabbed: None,
        glyphs: Glyphs::default(),
        features: Rc::new(started),
        config,
        system: Rc::new(system),
        samples: sys::watch(),
        workspaces: None,
        new_outputs: Vec::new(),
    };
    bar::measure_labels(&mut |title, size| app.painter.line(title, size, SEMIBOLD, 1.0).width);

    let wake = Arc::new(Woken(event_fd()?));
    let wake_fd = wake.0.try_clone().map_err(|error| error.to_string())?;
    let waker = Waker::from(wake);
    let mut cx = Context::from_waker(&waker);
    let mut glyphs_pending = true;

    loop {
        queue
            .dispatch_pending(&mut app)
            .map_err(|error| format!("the Wayland connection failed: {error}"))?;
        for output in std::mem::take(&mut app.new_outputs) {
            app.open(&qh, output);
        }
        for index in 0..app.islands.len() {
            if app.islands[index].configured && app.islands[index].surface.is_none() {
                app.start_drawing(index);
            }
        }

        drain(&mut arrivals, &mut cx, |arrival| arrival(&mut app));
        drain(&mut controls, &mut cx, |asked| app.answer(asked));
        if let Some(snapshots) = &mut workspaces {
            drain(snapshots, &mut cx, |snapshot| app.set_workspaces(snapshot));
        }
        if glyphs_pending && let Poll::Ready(glyphs) = Pin::new(&mut resolved).poll(&mut cx) {
            glyphs_pending = false;
            if let Ok(glyphs) = glyphs {
                app.glyphs = glyphs;
                for island in &mut app.islands {
                    island.forget_widgets();
                }
            }
        }

        let _ = scale_queue.dispatch_pending(&mut scales);
        for (surface, scale) in scales.0.drain(..) {
            let gpu = app.gpu.as_ref();
            if let Some(island) = app
                .islands
                .iter_mut()
                .find(|island| island.layer.wl_surface() == &surface)
            {
                island.set_scale(scale, gpu);
            }
        }

        let now = Instant::now();
        if let Some(mut repeat) = app.repeat.take() {
            if repeat.at <= now {
                app.key(&repeat.key);
                let every = app.key_repeat.map_or(REPEAT_EVERY, |(_, every)| every);
                repeat.at = now + every;
            }
            app.repeat = Some(repeat);
        }
        for island in &mut app.islands {
            island.poll_menu(&mut cx);
            island.tick(now);
        }
        if app.painter.reclaim(now) {
            for island in &mut app.islands {
                island.forget_atlas();
            }
        }
        if let Some(gpu) = &app.gpu {
            for island in &mut app.islands {
                if island.wants_frame() {
                    if island.turn.start() {
                        let surface = island.layer.wl_surface();
                        surface.frame(&qh, FrameCallbackData(surface.clone()));
                    }
                    island.frame(&app.glyphs, &mut app.painter, gpu, &app.compositor);
                }
            }
        }
        app.sync_covers(&qh);
        app.sync_cursor();
        if let Some(manager) = &app.idle_manager {
            let awake = app.features.iter().any(|feature| feature.inhibits_idle());
            for island in &mut app.islands {
                island.keep_awake(awake, manager, &app.blur_handle);
            }
        }
        let until = app
            .islands
            .iter()
            .filter_map(Island::next_wake)
            .chain(app.repeat.as_ref().map(|repeat| repeat.at))
            .chain(app.painter.reclaim_at())
            .min();
        wait(
            (&mut queue, &mut app),
            (&mut scale_queue, &mut scales),
            &conn,
            &wake_fd,
            until,
        );
    }
}
