//! Scrolled content without scrollbars: each edge with more content past it fades out.

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, graphene, gsk};

/// Longest fade, in pixels. A fade shortens as its edge nears the end of the content, so it
/// never cuts off the first or last row.
const FADE: f32 = 28.0;

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    pub(crate) struct FadeScroll {
        pub(super) scroller: RefCell<Option<gtk::ScrolledWindow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FadeScroll {
        const NAME: &'static str = "SwixFadeScroll";
        type Type = super::FadeScroll;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();
            klass.set_css_name("fadescroll");
        }
    }

    impl ObjectImpl for FadeScroll {
        fn dispose(&self) {
            if let Some(scroller) = self.scroller.take() {
                scroller.unparent();
            }
        }
    }

    impl WidgetImpl for FadeScroll {
        fn compute_expand(&self, hexpand: &mut bool, vexpand: &mut bool) {
            if let Some(scroller) = self.scroller.borrow().as_ref() {
                *hexpand = scroller.compute_expand(gtk::Orientation::Horizontal);
                *vexpand = scroller.compute_expand(gtk::Orientation::Vertical);
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let Some(scroller) = self.scroller.borrow().clone() else {
                return;
            };
            let (width, height) = (widget.width() as f32, widget.height() as f32);
            let masks = [
                (edge_fades(&scroller.vadjustment(), height), false),
                (edge_fades(&scroller.hadjustment(), width), true),
            ]
            .into_iter()
            .filter(|((start, end), _)| *start > 0.0 || *end > 0.0)
            .collect::<Vec<_>>();
            // Each mask records its gradient, then the masked content up to its second pop.
            for ((start, end), horizontal) in &masks {
                snapshot.push_mask(gsk::MaskMode::Alpha);
                append_fade(snapshot, width, height, *start, *end, *horizontal);
                snapshot.pop();
            }
            widget.snapshot_child(&scroller, snapshot);
            for _ in &masks {
                snapshot.pop();
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct FadeScroll(ObjectSubclass<imp::FadeScroll>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl FadeScroll {
    /// Wraps `scroller` and hides its scrollbars; it stays scrollable on every axis whose policy
    /// is not `Never`. Style backgrounds and borders on the wrapper: the scroller's own are faded
    /// with its content.
    pub(crate) fn new(scroller: &gtk::ScrolledWindow) -> Self {
        let widget = glib::Object::new::<Self>();
        let hide = |policy| match policy {
            gtk::PolicyType::Never => gtk::PolicyType::Never,
            _ => gtk::PolicyType::External,
        };
        let (horizontal, vertical) = scroller.policy();
        scroller.set_policy(hide(horizontal), hide(vertical));
        scroller.set_parent(&widget);
        for adjustment in [scroller.vadjustment(), scroller.hadjustment()] {
            let weak = widget.downgrade();
            let redraw = move |_: &gtk::Adjustment| {
                if let Some(widget) = weak.upgrade() {
                    widget.queue_draw();
                }
            };
            adjustment.connect_value_changed(redraw.clone());
            adjustment.connect_changed(redraw);
        }
        widget.imp().scroller.replace(Some(scroller.clone()));
        widget
    }
}

/// Fade lengths at the start and end of one axis; zero where the content already ends.
fn edge_fades(adjustment: &gtk::Adjustment, length: f32) -> (f32, f32) {
    let before = (adjustment.value() - adjustment.lower()) as f32;
    let after = (adjustment.upper() - adjustment.page_size() - adjustment.value()) as f32;
    let limit = FADE.min(length / 2.0);
    (before.clamp(0.0, limit), after.clamp(0.0, limit))
}

/// Draws the alpha mask: transparent at a fading edge, opaque everywhere else.
fn append_fade(
    snapshot: &gtk::Snapshot,
    width: f32,
    height: f32,
    start: f32,
    end: f32,
    horizontal: bool,
) {
    let length = if horizontal { width } else { height };
    let alpha = |faded: bool| gdk::RGBA::new(0.0, 0.0, 0.0, if faded { 0.0 } else { 1.0 });
    let stops = [
        gsk::ColorStop::new(0.0, alpha(start > 0.0)),
        gsk::ColorStop::new(start / length, alpha(false)),
        gsk::ColorStop::new(1.0 - end / length, alpha(false)),
        gsk::ColorStop::new(1.0, alpha(end > 0.0)),
    ];
    let to = if horizontal {
        graphene::Point::new(width, 0.0)
    } else {
        graphene::Point::new(0.0, height)
    };
    snapshot.append_linear_gradient(
        &graphene::Rect::new(0.0, 0.0, width, height),
        &graphene::Point::new(0.0, 0.0),
        &to,
        &stops,
    );
}
