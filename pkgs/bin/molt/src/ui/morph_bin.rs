//! The window's root: the current screen or a compact stand-in for it, and the morph between the
//! two. It draws the popup's translucent panel itself, as one rounded rectangle that moves from
//! the screen's bounds to the compact widget's while the two crossfade inside it. Only the
//! rectangle moves, so neither side is laid out again mid-morph and the window is never resized.

use std::cell::{Cell, RefCell};

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{graphene, gsk};

use super::animation::ease_in_out;
use super::blur::{self, Blur};
use super::theme;
use super::widgets::animations_enabled;

const MORPH_US: f64 = 420_000.0;
/// The panel: base at 75%, so compositor blur reads through it, and edgeless.
const PANEL_ALPHA: f32 = 0.75;

mod imp {
    use super::*;

    #[derive(Default)]
    pub(crate) struct MorphBin {
        pub(super) screen: RefCell<Option<gtk::Widget>>,
        pub(super) compact: RefCell<Option<gtk::Widget>>,
        pub(super) radius: Cell<f32>,
        /// `0` shows the screen, `1` the compact widget.
        pub(super) progress: Cell<f64>,
        pub(super) compact_target: Cell<bool>,
        pub(super) morphing: Cell<bool>,
        /// Bumped by every morph, so a newer one takes over from an older one.
        pub(super) generation: Cell<u64>,
        pub(super) blur: RefCell<Option<Blur>>,
        pub(super) blur_surface: Cell<Option<usize>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MorphBin {
        const NAME: &'static str = "MoltMorphBin";
        type Type = super::MorphBin;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for MorphBin {
        fn dispose(&self) {
            for child in [self.screen.take(), self.compact.take()]
                .into_iter()
                .flatten()
            {
                child.unparent();
            }
        }
    }

    impl MorphBin {
        /// The side the window is sized for: the compact widget only once the morph to it has
        /// settled.
        fn sizing_child(&self) -> Option<gtk::Widget> {
            if self.compact_target.get() && !self.morphing.get() {
                self.compact.borrow().clone()
            } else {
                self.screen.borrow().clone()
            }
        }

        fn update_blur(&self, panel: &graphene::Rect) {
            let widget = self.obj();
            let Some(native) = widget.native() else {
                return;
            };
            let Some(surface) = native.surface() else {
                return;
            };
            let key = blur::surface_key(&surface);
            if key != self.blur_surface.get() {
                self.blur_surface.set(key);
                self.blur.replace(Blur::attach(&surface));
            }
            let mut blur = self.blur.borrow_mut();
            let Some(blur) = blur.as_mut() else {
                return;
            };
            let Some(origin) =
                widget.compute_point(&native, &graphene::Point::new(panel.x(), panel.y()))
            else {
                return;
            };
            let (dx, dy) = native.surface_transform();
            blur.set_shape(
                origin.x() + dx as f32,
                origin.y() + dy as f32,
                panel.width(),
                panel.height(),
                self.radius.get(),
            );
        }
    }

    impl WidgetImpl for MorphBin {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            self.sizing_child()
                .map_or(gtk::SizeRequestMode::ConstantSize, |child| {
                    child.request_mode()
                })
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            self.sizing_child()
                .map_or((0, 0, -1, -1), |child| child.measure(orientation, for_size))
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            if let Some(screen) = self
                .screen
                .borrow()
                .as_ref()
                .filter(|child| child.is_visible())
            {
                screen.allocate(width, height, baseline, None);
            }
            if let Some(compact) = self
                .compact
                .borrow()
                .as_ref()
                .filter(|child| child.is_visible())
            {
                // Top centre: where the window's anchor keeps it once the window shrinks to it.
                let (_, natural_width, _, _) = compact.measure(gtk::Orientation::Horizontal, -1);
                let compact_width = natural_width.min(width);
                let (_, natural_height, _, _) =
                    compact.measure(gtk::Orientation::Vertical, compact_width);
                let x = (width - compact_width) / 2;
                compact.allocate(
                    compact_width,
                    natural_height.min(height),
                    -1,
                    Some(gsk::Transform::new().translate(&graphene::Point::new(x as f32, 0.0))),
                );
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let full = graphene::Rect::new(0.0, 0.0, widget.width() as f32, widget.height() as f32);
            let eased = ease_in_out(self.progress.get()) as f32;
            let screen = self.screen.borrow();
            let compact = self.compact.borrow();
            let screen = screen.as_ref().filter(|child| child.is_visible());
            let compact = compact.as_ref().filter(|child| child.is_visible());
            let compact_bounds = compact
                .and_then(|child| child.compute_bounds(&*widget))
                .unwrap_or(full);
            let panel = lerp_rect(&full, &compact_bounds, eased);
            let rounded = gsk::RoundedRect::from_rect(panel, self.radius.get());
            self.update_blur(&panel);

            snapshot.push_rounded_clip(&rounded);
            snapshot.append_color(&theme::BASE.with_alpha(PANEL_ALPHA), &panel);
            snapshot.pop();
            snapshot.push_rounded_clip(&rounded);
            // The screen is gone before the panel is half its size; the compact widget arrives
            // over the second half, so the two never stack into a muddle.
            let draw = |child: &gtk::Widget, opacity: f32| {
                if opacity <= 0.0 {
                    return;
                }
                let faded = opacity < 1.0;
                if faded {
                    snapshot.push_opacity(f64::from(opacity));
                }
                widget.snapshot_child(child, snapshot);
                if faded {
                    snapshot.pop();
                }
            };
            if let Some(screen) = screen {
                draw(screen, 1.0 - smoothstep(0.0, 0.5, eased));
            }
            if let Some(compact) = compact {
                draw(compact, smoothstep(0.45, 1.0, eased));
            }
            snapshot.pop();
        }
    }

    fn lerp_rect(from: &graphene::Rect, to: &graphene::Rect, t: f32) -> graphene::Rect {
        let lerp = |a: f32, b: f32| a + (b - a) * t;
        graphene::Rect::new(
            lerp(from.x(), to.x()),
            lerp(from.y(), to.y()),
            lerp(from.width(), to.width()),
            lerp(from.height(), to.height()),
        )
    }

    fn smoothstep(start: f32, end: f32, value: f32) -> f32 {
        let t = ((value - start) / (end - start)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }
}

glib::wrapper! {
    pub(crate) struct MorphBin(ObjectSubclass<imp::MorphBin>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl MorphBin {
    /// Shows `screen`, with `compact` ready to morph to. `radius` rounds the panel's corners.
    pub(crate) fn new(
        screen: &impl IsA<gtk::Widget>,
        compact: &impl IsA<gtk::Widget>,
        radius: f32,
    ) -> Self {
        let bin = glib::Object::new::<Self>();
        let imp = bin.imp();
        imp.radius.set(radius);
        screen.set_parent(&bin);
        compact.set_parent(&bin);
        compact.set_visible(false);
        imp.screen.replace(Some(screen.clone().upcast()));
        imp.compact.replace(Some(compact.clone().upcast()));
        bin
    }

    /// Morphs to the compact widget, or back to the screen, then calls `settled` unless a newer
    /// morph took over.
    pub(crate) fn morph_to(&self, compact: bool, settled: impl FnOnce() + 'static) {
        let imp = self.imp();
        let generation = imp.generation.get().wrapping_add(1);
        imp.generation.set(generation);
        imp.compact_target.set(compact);
        imp.morphing.set(true);
        for child in [&imp.screen, &imp.compact] {
            if let Some(child) = child.borrow().as_ref() {
                child.set_visible(true);
            }
        }
        self.queue_resize();
        let target = if compact { 1.0 } else { 0.0 };
        if !animations_enabled() || !self.is_mapped() {
            imp.progress.set(target);
            self.settle();
            settled();
            return;
        }
        let from = imp.progress.get();
        let started = Cell::new(None::<i64>);
        let settled = RefCell::new(Some(settled));
        self.add_tick_callback(move |bin, frame_clock| {
            let imp = bin.imp();
            if imp.generation.get() != generation {
                return glib::ControlFlow::Break;
            }
            let now = frame_clock.frame_time();
            let start = started.get().unwrap_or(now);
            started.set(Some(start));
            let linear = ((now - start) as f64 / MORPH_US).clamp(0.0, 1.0);
            imp.progress.set(from + (target - from) * linear);
            bin.queue_draw();
            if linear < 1.0 {
                return glib::ControlFlow::Continue;
            }
            bin.settle();
            if let Some(settled) = settled.take() {
                settled();
            }
            glib::ControlFlow::Break
        });
    }

    /// Ends a morph: only the side it went to stays shown and sizes the window.
    fn settle(&self) {
        let imp = self.imp();
        imp.morphing.set(false);
        let compact = imp.compact_target.get();
        if let Some(screen) = imp.screen.borrow().as_ref() {
            screen.set_visible(!compact);
        }
        if let Some(widget) = imp.compact.borrow().as_ref() {
            widget.set_visible(compact);
        }
        self.queue_resize();
        self.queue_draw();
    }
}
