use gpui::Window;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum, delegate_noop,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{
        wl_compositor::WlCompositor, wl_region::WlRegion, wl_registry::WlRegistry,
        wl_surface::WlSurface,
    },
};
use wayland_protocols::{
    ext::background_effect::v1::client::{
        ext_background_effect_manager_v1::{self, Capability, ExtBackgroundEffectManagerV1},
        ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
    },
    wp::idle_inhibit::zv1::client::{
        zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1,
        zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1,
    },
};

#[derive(Default)]
struct State {
    can_blur: bool,
}

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtBackgroundEffectManagerV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ExtBackgroundEffectManagerV1,
        event: ext_background_effect_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let ext_background_effect_manager_v1::Event::Capabilities { flags } = event else {
            return;
        };
        state.can_blur = matches!(flags, WEnum::Value(flags) if flags.contains(Capability::Blur));
    }
}

delegate_noop!(State: WlCompositor);
delegate_noop!(State: WlRegion);
delegate_noop!(State: ExtBackgroundEffectSurfaceV1);
delegate_noop!(State: ZwpIdleInhibitManagerV1);
delegate_noop!(State: ZwpIdleInhibitorV1);

pub struct Effects {
    conn: Connection,
    queue: EventQueue<State>,
    surface: WlSurface,
    compositor: WlCompositor,
    blur: Option<ExtBackgroundEffectSurfaceV1>,
    idle: Option<ZwpIdleInhibitManagerV1>,
    inhibitor: Option<ZwpIdleInhibitorV1>,
    sent: Option<Vec<[i32; 4]>>,
}

impl Effects {
    pub fn attach(window: &Window) -> Option<Self> {
        let RawDisplayHandle::Wayland(display) = window.display_handle().ok()?.as_raw() else {
            return None;
        };
        let raw_window = HasWindowHandle::window_handle(window).ok()?;
        let RawWindowHandle::Wayland(surface) = raw_window.as_raw() else {
            return None;
        };

        let backend = unsafe { Backend::from_foreign_display(display.display.as_ptr().cast()) };
        let conn = Connection::from_backend(backend);
        let (globals, mut queue) = registry_queue_init::<State>(&conn).ok()?;
        let qh = queue.handle();
        let compositor: WlCompositor = globals.bind(&qh, 1..=6, ()).ok()?;
        let surface_id =
            unsafe { ObjectId::from_ptr(WlSurface::interface(), surface.surface.as_ptr().cast()) }
                .ok()?;
        let surface = WlSurface::from_id(&conn, surface_id).ok()?;

        let manager: Option<ExtBackgroundEffectManagerV1> = globals.bind(&qh, 1..=1, ()).ok();
        let mut state = State::default();
        queue.roundtrip(&mut state).ok()?;
        let blur = manager.and_then(|manager| {
            let effect = state
                .can_blur
                .then(|| manager.get_background_effect(&surface, &qh, ()));
            manager.destroy();
            effect
        });
        let idle = globals.bind(&qh, 1..=1, ()).ok();

        Some(Self {
            conn,
            queue,
            surface,
            compositor,
            blur,
            idle,
            inhibitor: None,
            sent: None,
        })
    }

    pub fn set_blur(&mut self, rects: Vec<[i32; 4]>) {
        let Some(blur) = &self.blur else {
            return;
        };
        if self.sent.as_ref() == Some(&rects) {
            return;
        }

        let region = self.compositor.create_region(&self.queue.handle(), ());
        for [x, y, w, h] in &rects {
            region.add(*x, *y, *w, *h);
        }
        self.sent = Some(rects);
        blur.set_blur_region(Some(&region));
        region.destroy();
        let _ = self.conn.flush();
    }

    pub fn set_inhibit(&mut self, inhibit: bool) {
        if inhibit == self.inhibitor.is_some() {
            return;
        }
        match (&self.idle, self.inhibitor.take()) {
            (_, Some(inhibitor)) => inhibitor.destroy(),
            (Some(idle), None) => {
                self.inhibitor =
                    Some(idle.create_inhibitor(&self.surface, &self.queue.handle(), ()));
            }
            (None, None) => return,
        }
        let _ = self.conn.flush();
    }
}
