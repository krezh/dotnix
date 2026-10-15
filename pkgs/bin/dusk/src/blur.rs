use std::cell::RefCell;
use std::rc::Rc;

use gdk4_wayland::{WaylandDisplay, WaylandSurface, prelude::*};
use gtk::{gdk, prelude::*};
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum, delegate_noop,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{
        wl_compositor::WlCompositor, wl_region::WlRegion, wl_registry::WlRegistry,
        wl_surface::WlSurface,
    },
};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1::{self, Capability, ExtBackgroundEffectManagerV1},
    ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
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

struct Blur {
    connection: Connection,
    _queue: EventQueue<State>,
    effect: ExtBackgroundEffectSurfaceV1,
}

impl Blur {
    fn attach(surface: &gdk::Surface) -> Option<Self> {
        let display = surface.display().downcast::<WaylandDisplay>().ok()?;
        let display_ptr = display.wl_display_raw()?;
        let surface_ptr = surface.downcast_ref::<WaylandSurface>()?.wl_surface_raw()?;
        let backend = unsafe { Backend::from_foreign_display(display_ptr.as_ptr().cast()) };
        let connection = Connection::from_backend(backend);
        let (globals, mut queue) = registry_queue_init::<State>(&connection).ok()?;
        let handle = queue.handle();
        let compositor: WlCompositor = globals.bind(&handle, 1..=6, ()).ok()?;
        let manager: ExtBackgroundEffectManagerV1 = globals.bind(&handle, 1..=1, ()).ok()?;
        let id = unsafe { ObjectId::from_ptr(WlSurface::interface(), surface_ptr.as_ptr().cast()) }
            .ok()?;
        let surface = WlSurface::from_id(&connection, id).ok()?;
        let mut state = State::default();
        queue.roundtrip(&mut state).ok()?;
        if !state.can_blur {
            manager.destroy();
            return None;
        }
        let effect = manager.get_background_effect(&surface, &handle, ());
        manager.destroy();
        let region = compositor.create_region(&handle, ());
        // The compositor clips this region to the surface, including after output resizes.
        region.add(0, 0, i32::MAX, i32::MAX);
        effect.set_blur_region(Some(&region));
        region.destroy();
        let _ = connection.flush();
        Some(Self {
            connection,
            _queue: queue,
            effect,
        })
    }
}

impl Drop for Blur {
    fn drop(&mut self) {
        self.effect.destroy();
        let _ = self.connection.flush();
    }
}

pub fn install(window: &gtk::Window) {
    let blur = Rc::new(RefCell::new(None));
    let realized = Rc::clone(&blur);
    window.connect_realize(move |window| {
        *realized.borrow_mut() = window.surface().and_then(|surface| Blur::attach(&surface));
    });
    window.connect_unrealize(move |_| {
        blur.borrow_mut().take();
    });
}
