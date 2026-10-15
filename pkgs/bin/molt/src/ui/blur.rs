use gdk4_wayland::{WaylandDisplay, WaylandSurface, prelude::*};
use gtk::gdk;
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

pub(crate) fn surface_key(surface: &gdk::Surface) -> Option<usize> {
    let ptr = surface.downcast_ref::<WaylandSurface>()?.wl_surface_raw()?;
    Some(ptr.as_ptr() as usize)
}

pub(crate) struct Blur {
    conn: Connection,
    queue: EventQueue<State>,
    compositor: WlCompositor,
    effect: ExtBackgroundEffectSurfaceV1,
    sent: Option<[i32; 4]>,
}

impl Blur {
    pub(crate) fn attach(surface: &gdk::Surface) -> Option<Self> {
        let display = surface.display().downcast::<WaylandDisplay>().ok()?;
        let display_ptr = display.wl_display_raw()?;
        let surface_ptr = surface.downcast_ref::<WaylandSurface>()?.wl_surface_raw()?;

        let backend = unsafe { Backend::from_foreign_display(display_ptr.as_ptr().cast()) };
        let conn = Connection::from_backend(backend);
        let (globals, mut queue) = registry_queue_init::<State>(&conn).ok()?;
        let qh = queue.handle();
        let compositor: WlCompositor = globals.bind(&qh, 1..=6, ()).ok()?;
        let manager: ExtBackgroundEffectManagerV1 = globals.bind(&qh, 1..=1, ()).ok()?;
        let surface_id =
            unsafe { ObjectId::from_ptr(WlSurface::interface(), surface_ptr.as_ptr().cast()) }
                .ok()?;
        let wl_surface = WlSurface::from_id(&conn, surface_id).ok()?;

        let mut state = State::default();
        queue.roundtrip(&mut state).ok()?;
        if !state.can_blur {
            manager.destroy();
            return None;
        }
        let effect = manager.get_background_effect(&wl_surface, &qh, ());
        manager.destroy();

        Some(Self {
            conn,
            queue,
            compositor,
            effect,
            sent: None,
        })
    }

    pub(crate) fn set_shape(&mut self, x: f32, y: f32, width: f32, height: f32, radius: f32) {
        let key = [x, y, width, height].map(|value| value.round() as i32);
        if self.sent == Some(key) {
            return;
        }
        self.sent = Some(key);

        let region = self.compositor.create_region(&self.queue.handle(), ());
        for [x, y, w, h] in region_rects(key, radius) {
            region.add(x, y, w, h);
        }
        self.effect.set_blur_region(Some(&region));
        region.destroy();
        let _ = self.conn.flush();
    }
}

impl Drop for Blur {
    fn drop(&mut self) {
        self.effect.destroy();
        let _ = self.conn.flush();
    }
}

fn region_rects([x, y, w, h]: [i32; 4], radius: f32) -> Vec<[i32; 4]> {
    if w <= 0 || h <= 0 {
        return Vec::new();
    }
    let r = radius.clamp(0.0, w.min(h) as f32 / 2.0);
    let rows = r.ceil() as i32;
    let mut rects = Vec::with_capacity(2 * rows as usize + 1);
    for row in 0..rows {
        let from_centre = r - (row as f32 + 0.5);
        let reach = (r * r - from_centre * from_centre).max(0.0).sqrt();
        let inset = (r - reach).round() as i32;
        let width = w - 2 * inset;
        rects.push([x + inset, y + row, width, 1]);
        rects.push([x + inset, y + h - 1 - row, width, 1]);
    }
    if h > 2 * rows {
        rects.push([x, y + rows, w, h - 2 * rows]);
    }
    rects
}

#[cfg(test)]
mod tests {
    use super::region_rects;

    #[test]
    fn region_covers_the_rounded_rectangle_without_overlap() {
        let rects = region_rects([10, 20, 100, 60], 15.0);
        let area: i32 = rects.iter().map(|[_, _, w, h]| w * h).sum();
        let expected = 100.0 * 60.0 - 4.0 * (1.0 - std::f32::consts::FRAC_PI_4) * 225.0;
        assert!((area as f32 - expected).abs() < 40.0, "area {area}, expected ≈{expected}");
        let top = rects.iter().find(|[_, y, _, _]| *y == 20).unwrap();
        let middle = rects.iter().find(|[_, y, _, h]| *y == 35 && *h > 1).unwrap();
        assert!(top[0] > middle[0] && top[2] < middle[2]);
        let mut rows: Vec<i32> = rects
            .iter()
            .flat_map(|[_, y, _, h]| *y..*y + *h)
            .collect();
        rows.sort_unstable();
        assert_eq!(rows, (20..80).collect::<Vec<_>>());
    }
}
