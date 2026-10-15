//! The window's screen container. During a screen change it also holds the previous screen's own
//! widgets, and moves, dims, or fades the two screens against each other.

use std::cell::{Cell, RefCell};

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, graphene, gsk};

/// Width of the shadow cast by the screen on top, at its leading edge.
const SHADOW: f32 = 28.0;
const SHADOW_ALPHA: f32 = 0.4;

/// Where a screen is drawn during a screen change.
#[derive(Clone, Copy, Default)]
pub(crate) struct Motion {
    /// Horizontal offset in pixels.
    pub(crate) offset: f32,
    /// `0..=1` darkening.
    pub(crate) dim: f32,
    /// Cast a shadow along the left edge. The screen under one with a shadow is cut off at that
    /// edge: screens have no background of their own, so it would otherwise show through.
    pub(crate) shadow: bool,
}

/// The screen being left, kept on screen until the change finishes.
struct Previous {
    widget: gtk::Widget,
    /// Its size before the change, kept while the window resizes for the new screen.
    width: i32,
    height: i32,
    /// Drawn over the current screen rather than under it.
    above: bool,
    motion: Motion,
    opacity: f64,
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub(crate) struct SlideBin {
        pub(super) child: RefCell<Option<gtk::Widget>>,
        pub(super) motion: Cell<Motion>,
        pub(super) previous: RefCell<Option<Previous>>,
        /// Bumped by every screen change, so a newer change takes over from an older one.
        pub(super) generation: Cell<u64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SlideBin {
        const NAME: &'static str = "MoltSlideBin";
        type Type = super::SlideBin;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for SlideBin {
        fn dispose(&self) {
            if let Some(previous) = self.previous.take() {
                previous.widget.unparent();
            }
            if let Some(child) = self.child.take() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for SlideBin {
        fn compute_expand(&self, hexpand: &mut bool, vexpand: &mut bool) {
            if let Some(child) = self.child.borrow().as_ref() {
                *hexpand = child.compute_expand(gtk::Orientation::Horizontal);
                *vexpand = child.compute_expand(gtk::Orientation::Vertical);
            }
        }

        fn request_mode(&self) -> gtk::SizeRequestMode {
            self.child
                .borrow()
                .as_ref()
                .map_or(gtk::SizeRequestMode::ConstantSize, WidgetExt::request_mode)
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            self.child
                .borrow()
                .as_ref()
                .map_or((0, 0, -1, -1), |child| child.measure(orientation, for_size))
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            // Only transforms change while moving, so neither screen is laid out again.
            if let Some(child) = self.child.borrow().as_ref() {
                let offset = graphene::Point::new(self.motion.get().offset, 0.0);
                child.allocate(width, height, baseline, Some(gsk::Transform::new().translate(&offset)));
            }
            if let Some(previous) = self.previous.borrow().as_ref() {
                // GTK requires a measure before every allocate; the results are cached.
                previous.widget.measure(gtk::Orientation::Horizontal, -1);
                previous.widget.measure(gtk::Orientation::Vertical, previous.width);
                let offset = graphene::Point::new(previous_x(previous, width), 0.0);
                previous.widget.allocate(
                    previous.width,
                    previous.height,
                    -1,
                    Some(gsk::Transform::new().translate(&offset)),
                );
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let (width, height) = (widget.width(), widget.height());
            let child = self.child.borrow();
            let draw_child = |snapshot: &gtk::Snapshot| {
                if let Some(child) = child.as_ref() {
                    widget.snapshot_child(child, snapshot);
                    decorate(snapshot, self.motion.get(), width as f32, height as f32);
                }
            };
            let previous = self.previous.borrow();
            let Some(previous) = previous.as_ref() else {
                draw_child(snapshot);
                return;
            };
            let draw_previous = |snapshot: &gtk::Snapshot| {
                let faded = previous.opacity < 1.0;
                if faded {
                    snapshot.push_opacity(previous.opacity);
                }
                widget.snapshot_child(&previous.widget, snapshot);
                let at = Motion {
                    offset: previous_x(previous, width),
                    ..previous.motion
                };
                decorate(snapshot, at, previous.width as f32, previous.height as f32);
                if faded {
                    snapshot.pop();
                }
            };
            let clip_at = |edge: f32| {
                graphene::Rect::new(0.0, 0.0, edge.clamp(0.0, width as f32), height as f32)
            };
            if previous.above {
                let clip = previous.motion.shadow;
                if clip {
                    snapshot.push_clip(&clip_at(previous_x(previous, width)));
                }
                draw_child(snapshot);
                if clip {
                    snapshot.pop();
                }
                draw_previous(snapshot);
            } else {
                let clip = self.motion.get().shadow;
                if clip {
                    snapshot.push_clip(&clip_at(self.motion.get().offset));
                }
                draw_previous(snapshot);
                if clip {
                    snapshot.pop();
                }
                draw_child(snapshot);
            }
        }
    }

    /// The previous screen's left edge: centred in the current width, then moved.
    fn previous_x(previous: &Previous, width: i32) -> f32 {
        (width - previous.width) as f32 / 2.0 + previous.motion.offset
    }
}

/// Darkens a screen drawn at `motion` and casts its shadow.
fn decorate(snapshot: &gtk::Snapshot, motion: Motion, width: f32, height: f32) {
    if motion.dim > 0.0 {
        snapshot.append_color(
            &gdk::RGBA::new(0.0, 0.0, 0.0, motion.dim),
            &graphene::Rect::new(motion.offset, 0.0, width, height),
        );
    }
    if motion.shadow && motion.offset > 0.0 {
        let start = motion.offset - SHADOW;
        snapshot.append_linear_gradient(
            &graphene::Rect::new(start, 0.0, SHADOW, height),
            &graphene::Point::new(start, 0.0),
            &graphene::Point::new(motion.offset, 0.0),
            &[
                gsk::ColorStop::new(0.0, gdk::RGBA::new(0.0, 0.0, 0.0, 0.0)),
                gsk::ColorStop::new(0.6, gdk::RGBA::new(0.0, 0.0, 0.0, SHADOW_ALPHA * 0.3)),
                gsk::ColorStop::new(1.0, gdk::RGBA::new(0.0, 0.0, 0.0, SHADOW_ALPHA)),
            ],
        );
    }
}

glib::wrapper! {
    pub(crate) struct SlideBin(ObjectSubclass<imp::SlideBin>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SlideBin {
    pub(crate) fn new(child: &impl IsA<gtk::Widget>) -> Self {
        let bin = glib::Object::new::<Self>();
        child.set_parent(&bin);
        bin.imp().child.replace(Some(child.clone().upcast()));
        bin
    }

    /// Starts a screen change: `previous` holds the screen being left, which was
    /// `width` x `height`. Returns the change's generation.
    pub(super) fn begin(&self, previous: &impl IsA<gtk::Widget>, width: i32, height: i32, above: bool) -> u64 {
        self.settle();
        // The previous screen is only drawn; clicks and keys go to the new one.
        previous.set_can_target(false);
        previous.set_can_focus(false);
        previous.set_parent(self);
        self.imp().previous.replace(Some(Previous {
            widget: previous.clone().upcast(),
            width,
            height,
            above,
            motion: Motion::default(),
            opacity: 1.0,
        }));
        self.imp().generation.get()
    }

    pub(super) fn generation(&self) -> u64 {
        self.imp().generation.get()
    }

    /// Places the current and the previous screen for one frame.
    pub(super) fn set_motion(&self, current: Motion, previous: Motion, previous_opacity: f64) {
        self.imp().motion.set(current);
        if let Some(entry) = self.imp().previous.borrow_mut().as_mut() {
            entry.motion = previous;
            entry.opacity = previous_opacity;
        }
        self.queue_allocate();
        self.queue_draw();
    }

    /// Ends any screen change: drops the previous screen and puts the current one in place.
    pub(super) fn settle(&self) {
        self.imp().generation.set(self.imp().generation.get().wrapping_add(1));
        if let Some(previous) = self.imp().previous.take() {
            previous.widget.unparent();
        }
        self.set_motion(Motion::default(), Motion::default(), 1.0);
    }
}
