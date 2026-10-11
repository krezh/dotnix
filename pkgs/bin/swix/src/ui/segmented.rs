use std::cell::Cell;
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;

use crate::ui::theme;
use crate::ui::{MorphLabel, animations_enabled};

/// How long option sizes animate after a label or visibility change, in microseconds: the label
/// crossfade and option reveal, with a frame to spare.
const RESIZE: i64 = 240_000;

/// Mutually exclusive toggle buttons over a pill that slides to the active one.
///
/// The pill is drawn behind the buttons, so their own backgrounds stay transparent; it eases to a
/// clicked option and snaps when the control is resized. Labels morph and options slide in and
/// out, with the pill following.
pub(crate) struct SegmentedControl {
    pub(crate) root: gtk::Overlay,
    pub(crate) options: Vec<gtk::ToggleButton>,
    labels: Vec<MorphLabel>,
    slots: Vec<gtk::Revealer>,
    thumb: gtk::DrawingArea,
    /// Frame time until which the resting pill is redrawn every frame; set while that runs.
    following: Rc<Cell<Option<i64>>>,
}

/// Thumb position and width, in the control's coordinates.
type ThumbSpan = (f64, f64);

impl SegmentedControl {
    pub(crate) fn new(labels: &[&str]) -> Self {
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let labels: Vec<MorphLabel> = labels
            .iter()
            .map(|text| {
                let label = MorphLabel::new(text, &[], 0.5);
                label.set_ellipsize(gtk::pango::EllipsizeMode::None);
                label
            })
            .collect();
        let options: Vec<gtk::ToggleButton> = labels
            .iter()
            .map(|label| {
                let option = gtk::ToggleButton::new();
                option.set_child(Some(&label.root));
                option.add_css_class("segmented-option");
                option
            })
            .collect();
        let slots: Vec<gtk::Revealer> = options
            .iter()
            .map(|option| {
                let slot = gtk::Revealer::new();
                slot.set_transition_type(gtk::RevealerTransitionType::SlideRight);
                slot.set_transition_duration(200);
                slot.set_reveal_child(true);
                slot.set_child(Some(option));
                buttons.append(&slot);
                slot
            })
            .collect();
        if let Some(first) = options.first() {
            for option in &options[1..] {
                option.set_group(Some(first));
            }
            first.set_active(true);
        }

        let thumb = gtk::DrawingArea::new();
        // `sliding` holds the pill's span only while it moves; at rest it is measured from the
        // active option at draw time, when layout is final (also covers first draw and resizes).
        let sliding: Rc<Cell<Option<ThumbSpan>>> = Rc::new(Cell::new(None));
        let drawn: Rc<Cell<Option<ThumbSpan>>> = Rc::new(Cell::new(None));
        let draw_sliding = Rc::clone(&sliding);
        let draw_drawn = Rc::clone(&drawn);
        let draw_options = options.clone();
        thumb.set_draw_func(move |thumb, context, _, height| {
            let span = draw_sliding
                .get()
                .or_else(|| active_span(thumb, &draw_options));
            draw_drawn.set(span);
            if let Some((x, width)) = span {
                theme::set_source_rgb(context, theme::SURFACE0);
                theme::rounded_rectangle(context, x, 0.0, width, f64::from(height), 6.0);
                let _ = context.fill();
            }
        });

        let root = gtk::Overlay::new();
        root.set_child(Some(&thumb));
        root.add_overlay(&buttons);
        root.set_measure_overlay(&buttons, true);

        let animation = Rc::new(Cell::new(0_u64));
        for option in &options {
            let thumb = thumb.downgrade();
            let options = options.clone();
            let sliding = Rc::clone(&sliding);
            let drawn = Rc::clone(&drawn);
            let animation = Rc::clone(&animation);
            option.connect_toggled(move |option| {
                if !option.is_active() {
                    return;
                }
                if let Some(thumb) = thumb.upgrade() {
                    let target = active_span(&thumb, &options);
                    slide_thumb(&thumb, drawn.get(), target, &sliding, &animation);
                }
            });
        }
        Self {
            root,
            options,
            labels,
            slots,
            thumb,
            following: Rc::default(),
        }
    }

    /// Morphs option `index`'s label to `text`.
    pub(crate) fn set_label(&self, index: usize, text: &str) {
        let label = &self.labels[index];
        if label.text() != text {
            label.set_text(text);
            self.follow_resize();
        }
    }

    /// Slides option `index` in or out.
    pub(crate) fn set_option_visible(&self, index: usize, visible: bool) {
        let slot = &self.slots[index];
        if slot.reveals_child() != visible {
            slot.set_reveal_child(visible);
            self.follow_resize();
        }
    }

    /// Keeps a resting pill on its option while option sizes animate: the pill is measured at
    /// draw time, and nothing else redraws it while the buttons above it move.
    fn follow_resize(&self) {
        let clock = self.thumb.frame_clock().filter(|_| animations_enabled());
        let Some(clock) = clock else {
            self.thumb.queue_draw();
            return;
        };
        let running = self.following.get().is_some();
        self.following.set(Some(clock.frame_time() + RESIZE));
        if running {
            return;
        }
        let following = Rc::clone(&self.following);
        self.thumb.add_tick_callback(move |thumb, clock| {
            thumb.queue_draw();
            if following.get().is_some_and(|until| clock.frame_time() < until) {
                glib::ControlFlow::Continue
            } else {
                following.set(None);
                glib::ControlFlow::Break
            }
        });
    }
}

fn active_span(thumb: &gtk::DrawingArea, options: &[gtk::ToggleButton]) -> Option<ThumbSpan> {
    let bounds = options
        .iter()
        .find(|option| option.is_active())?
        .compute_bounds(thumb)?;
    (bounds.width() > 0.0).then(|| (f64::from(bounds.x()), f64::from(bounds.width())))
}

fn slide_thumb(
    thumb: &gtk::DrawingArea,
    from: Option<ThumbSpan>,
    target: Option<ThumbSpan>,
    sliding: &Rc<Cell<Option<ThumbSpan>>>,
    animation: &Rc<Cell<u64>>,
) {
    let animation_id = animation.get().wrapping_add(1);
    animation.set(animation_id);
    let (Some(from), Some(target), true) = (from, target, animations_enabled()) else {
        sliding.set(None);
        thumb.queue_draw();
        return;
    };
    const DURATION: f64 = 0.22;
    let sliding = Rc::clone(sliding);
    let animation = Rc::clone(animation);
    let started = Cell::new(None);
    thumb.add_tick_callback(move |thumb, frame_clock| {
        if animation.get() != animation_id {
            return glib::ControlFlow::Break;
        }
        let now = frame_clock.frame_time();
        let started = started.get().unwrap_or_else(|| {
            started.set(Some(now));
            now
        });
        let progress = ((now - started) as f64 / 1_000_000.0 / DURATION).min(1.0);
        // Ease-out cubic: fast start, gentle landing on the new option.
        let eased = 1.0 - (1.0 - progress).powi(3);
        sliding.set((progress < 1.0).then_some((
            from.0 + (target.0 - from.0) * eased,
            from.1 + (target.1 - from.1) * eased,
        )));
        thumb.queue_draw();
        if progress >= 1.0 {
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}
