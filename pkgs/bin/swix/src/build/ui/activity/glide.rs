//! A vertical box that animates its own layout: children glide from where they were drawn to their
//! new slot, and children that start being laid out fade in. Motion is applied while drawing, so
//! an animation frame only redraws and never lays the list out again.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use gtk::glib;
use gtk::graphene;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::ui::animations_enabled;

/// Matches the shared `glide` and `fade_in` motions, in microseconds.
const GLIDE: f64 = 280_000.0;
const FADE: f64 = 160_000.0;
/// Farthest a child glides, in pixels; past this it would only streak across the list, so it
/// fades in at its new slot instead.
const MAX_GLIDE: f64 = 400.0;

#[derive(Clone, Copy, Debug, Default)]
struct Motion {
    /// Offset from the slot when the glide started, and that frame time.
    glide: Option<(f64, i64)>,
    /// Frame time the fade-in started.
    fade: Option<i64>,
}

/// Ease-out cubic progress through `duration` since `started`.
fn eased(started: i64, now: i64, duration: f64) -> f64 {
    let linear = ((now - started) as f64 / duration).clamp(0.0, 1.0);
    1.0 - (1.0 - linear).powi(3)
}

impl Motion {
    fn appear(now: i64) -> Self {
        Self {
            glide: None,
            fade: Some(now),
        }
    }

    /// Glides in from `distance` below (negative: above) the slot, or fades in if that is too far.
    fn travel(distance: f64, now: i64) -> Self {
        if distance.abs() > MAX_GLIDE {
            return Self::appear(now);
        }
        Self {
            glide: Some((distance, now)),
            fade: None,
        }
    }

    fn offset(&self, now: i64) -> f64 {
        self.glide
            .map_or(0.0, |(from, started)| from * (1.0 - eased(started, now, GLIDE)))
    }

    fn opacity(&self, now: i64) -> f64 {
        self.fade.map_or(1.0, |started| eased(started, now, FADE))
    }

    fn finished(&self, now: i64) -> bool {
        self.offset(now) == 0.0 && self.opacity(now) >= 1.0
    }
}

fn children(widget: &gtk::Widget) -> impl Iterator<Item = gtk::Widget> {
    std::iter::successors(widget.first_child(), gtk::Widget::next_sibling)
}

fn frame_time(widget: &gtk::Widget) -> i64 {
    widget.frame_clock().map_or(0, |clock| clock.frame_time())
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub(crate) struct GlideBox {
        pub(super) spacing: Cell<i32>,
        /// Each laid-out child's y from the last layout.
        pub(super) slots: RefCell<HashMap<gtk::Widget, i32>>,
        pub(super) motions: RefCell<HashMap<gtk::Widget, Motion>>,
        /// Where children moved in from another box were drawn, in the given widget's coordinates.
        /// Weak: that widget is usually an ancestor, which owns this box.
        pub(super) arrivals: RefCell<HashMap<gtk::Widget, (glib::WeakRef<gtk::Widget>, f64)>>,
        /// Whether the next layout animates; off until the box is first laid out and after it
        /// is unmapped, since then it appears as a whole and its parent animates that.
        pub(super) animate: Cell<bool>,
        pub(super) ticking: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GlideBox {
        const NAME: &'static str = "SwixGlideBox";
        type Type = super::GlideBox;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("glidebox");
        }
    }

    impl ObjectImpl for GlideBox {
        fn dispose(&self) {
            self.slots.borrow_mut().clear();
            self.motions.borrow_mut().clear();
            self.arrivals.borrow_mut().clear();
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for GlideBox {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let (mut minimum, mut natural, mut count) = (0, 0, 0);
            for child in children(self.obj().upcast_ref()).filter(|child| child.should_layout()) {
                if orientation == gtk::Orientation::Vertical {
                    let (min, nat, _, _) = child.measure(orientation, for_size);
                    minimum += min;
                    natural += nat;
                    count += 1;
                } else {
                    let (min, nat, _, _) = child.measure(orientation, -1);
                    minimum = minimum.max(min);
                    natural = natural.max(nat);
                }
            }
            let gaps = self.spacing.get() * (count - 1).max(0);
            (minimum + gaps, natural + gaps, -1, -1)
        }

        fn size_allocate(&self, width: i32, height: i32, _baseline: i32) {
            let widget = self.obj();
            let widget: &gtk::Widget = widget.upcast_ref();
            let spacing = self.spacing.get();
            let laid_out = children(widget)
                .filter(|child| child.should_layout())
                .map(|child| {
                    let (min, nat, _, _) = child.measure(gtk::Orientation::Vertical, width);
                    (child, min, nat)
                })
                .collect::<Vec<_>>();
            let gaps = spacing * (laid_out.len() as i32 - 1).max(0);
            let fits = laid_out.iter().map(|(_, _, nat)| nat).sum::<i32>() + gaps <= height;
            let animate = self.animate.replace(true) && animations_enabled();
            let now = frame_time(widget);
            let mut slots = self.slots.borrow_mut();
            let mut motions = self.motions.borrow_mut();
            let mut arrivals = self.arrivals.borrow_mut();
            let mut placed = HashMap::with_capacity(laid_out.len());
            let mut y = 0;
            for (child, min, nat) in laid_out {
                let child_height = if fits { nat } else { min };
                child.size_allocate(&gtk::Allocation::new(0, y, width, child_height), -1);
                let previous = slots.get(&child).copied();
                let arrival = arrivals.remove(&child);
                if !animate {
                    motions.remove(&child);
                } else if let Some((frame, origin)) =
                    arrival.and_then(|(frame, origin)| Some((frame.upgrade()?, origin)))
                {
                    let slot = widget
                        .compute_point(&frame, &graphene::Point::new(0.0, y as f32))
                        .map_or(origin, |point| f64::from(point.y()));
                    motions.insert(child.clone(), Motion::travel(origin - slot, now));
                } else if let Some(previous) = previous.filter(|previous| *previous != y) {
                    let current = motions.get(&child).copied().unwrap_or_default();
                    let drawn = f64::from(previous) + current.offset(now);
                    let mut motion = Motion::travel(drawn - f64::from(y), now);
                    motion.fade = motion.fade.or(current.fade);
                    motions.insert(child.clone(), motion);
                } else if previous.is_none() {
                    motions.insert(child.clone(), Motion::appear(now));
                }
                placed.insert(child, y);
                y += child_height + spacing;
            }
            *slots = placed;
            arrivals.clear();
            motions.retain(|child, motion| slots.contains_key(child) && !motion.finished(now));
            let animating = !motions.is_empty();
            drop((slots, motions, arrivals));
            if animating {
                self.tick();
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let motions = self.motions.borrow();
            if motions.is_empty() {
                drop(motions);
                self.parent_snapshot(snapshot);
                return;
            }
            let widget = self.obj();
            let now = frame_time(widget.upcast_ref());
            // Only children that can show in the scrolled view are drawn, so a frame costs the
            // visible rows rather than the whole list. Offsets (this box's and any gliding
            // ancestor's) never exceed `MAX_GLIDE`, which widens the drawn band.
            let visible = widget
                .ancestor(gtk::Viewport::static_type())
                .and_then(|viewport| viewport.compute_bounds(&*widget))
                .map(|bounds| {
                    (
                        f64::from(bounds.y()) - MAX_GLIDE,
                        f64::from(bounds.y() + bounds.height()) + MAX_GLIDE,
                    )
                });
            let slots = self.slots.borrow();
            for child in children(widget.upcast_ref()) {
                let motion = motions.get(&child);
                if let (Some((top, bottom)), Some(slot)) = (visible, slots.get(&child)) {
                    let y = f64::from(*slot);
                    if y > bottom + MAX_GLIDE {
                        break;
                    }
                    let drawn = y + motion.map_or(0.0, |motion| motion.offset(now));
                    if drawn > bottom || drawn + f64::from(child.height()) < top {
                        continue;
                    }
                }
                let Some(motion) = motion else {
                    widget.snapshot_child(&child, snapshot);
                    continue;
                };
                let opacity = motion.opacity(now);
                snapshot.save();
                snapshot.translate(&graphene::Point::new(0.0, motion.offset(now) as f32));
                if opacity < 1.0 {
                    snapshot.push_opacity(opacity);
                }
                widget.snapshot_child(&child, snapshot);
                if opacity < 1.0 {
                    snapshot.pop();
                }
                snapshot.restore();
            }
        }

        fn unmap(&self) {
            self.parent_unmap();
            self.animate.set(false);
            self.motions.borrow_mut().clear();
        }
    }

    impl GlideBox {
        /// Redraws every frame until all motion has finished.
        fn tick(&self) {
            if self.ticking.replace(true) {
                return;
            }
            self.obj().add_tick_callback(|widget, clock| {
                let imp = widget.imp();
                let now = clock.frame_time();
                imp.motions
                    .borrow_mut()
                    .retain(|_, motion| !motion.finished(now));
                widget.queue_draw();
                if imp.motions.borrow().is_empty() {
                    imp.ticking.set(false);
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        }
    }
}

glib::wrapper! {
    /// A vertical box whose children glide to new slots and fade in when they appear.
    pub(crate) struct GlideBox(ObjectSubclass<imp::GlideBox>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl GlideBox {
    pub(crate) fn new(spacing: i32) -> Self {
        let widget = glib::Object::new::<Self>();
        widget.imp().spacing.set(spacing);
        widget
    }

    pub(crate) fn append(&self, child: &impl IsA<gtk::Widget>) {
        child.insert_before(self, None::<&gtk::Widget>);
    }

    pub(crate) fn prepend(&self, child: &impl IsA<gtk::Widget>) {
        child.insert_after(self, None::<&gtk::Widget>);
    }

    pub(crate) fn remove(&self, child: &impl IsA<gtk::Widget>) {
        let child = child.upcast_ref::<gtk::Widget>();
        let imp = self.imp();
        imp.slots.borrow_mut().remove(child);
        imp.motions.borrow_mut().remove(child);
        imp.arrivals.borrow_mut().remove(child);
        child.unparent();
    }

    /// Where `child` is drawn, including any glide in progress, in `frame`'s coordinates.
    pub(crate) fn drawn_y(
        &self,
        child: &impl IsA<gtk::Widget>,
        frame: &impl IsA<gtk::Widget>,
    ) -> Option<f64> {
        if !child.is_mapped() {
            return None;
        }
        let child = child.upcast_ref::<gtk::Widget>();
        let point = child.compute_point(frame, &graphene::Point::zero())?;
        let offset = self
            .imp()
            .motions
            .borrow()
            .get(child)
            .map_or(0.0, |motion| motion.offset(frame_time(child)));
        Some(f64::from(point.y()) + offset)
    }

    /// Makes `child`, just moved here from another box, glide from `origin` (a [`Self::drawn_y`]
    /// in `frame`'s coordinates) instead of fading in.
    pub(crate) fn arrive(
        &self,
        child: &impl IsA<gtk::Widget>,
        frame: &impl IsA<gtk::Widget>,
        origin: f64,
    ) {
        self.imp()
            .arrivals
            .borrow_mut()
            .insert(child.clone().upcast(), (frame.upcast_ref().downgrade(), origin));
    }
}
