//! Shared motion: screen crossfades and slides, eased values, smooth scrolling, and fade-ins.
//! Each one jumps straight to its end state when animations are disabled.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use super::slide_bin::{Motion, SlideBin};
use super::widgets::{animations_enabled, clear};

const SCREEN_FADE: Duration = Duration::from_millis(200);
const SCREEN_SLIDE: Duration = Duration::from_millis(300);
const TWEEN: Duration = Duration::from_millis(240);
const GLIDE: Duration = Duration::from_millis(280);
const FADE_IN: Duration = Duration::from_millis(160);
/// How far the screen underneath moves, as a share of the width, and how much it darkens.
const SLIDE_DEPTH: f32 = 0.3;
const SLIDE_DIM: f32 = 0.35;

/// A gap between frames longer than this is a stall, e.g. while a new screen is built and laid
/// out. The animation clock pauses through it, so no part of the motion is skipped.
const STALL_US: i64 = 50_000;
/// One frame at 60 Hz, the time a stalled frame still counts for.
const FRAME_US: i64 = 16_667;

/// Cubic ease-out: fast start, gentle stop. For things arriving or settling into place.
fn ease_out(linear: f64) -> f64 {
    1.0 - (1.0 - linear).powi(3)
}

/// Cubic ease-in-out: starts and ends at rest. For slides, so a screen visibly enters from
/// outside the window and fully leaves it; ease-out would start it partway in.
pub(crate) fn ease_in_out(linear: f64) -> f64 {
    if linear < 0.5 {
        4.0 * linear.powi(3)
    } else {
        1.0 - (-2.0 * linear + 2.0).powi(3) / 2.0
    }
}

/// Calls `frame` with eased progress on every frame of `widget` until it reaches 1, or once with 1
/// when animations are off. `frame` returning `false` stops early. Frames start once `widget` is
/// realized, so a freshly created widget animates from its first frame. Callers capture widgets
/// weakly in `frame`: it lives in `widget`'s tick list until it finishes.
fn animate(
    widget: &impl IsA<gtk::Widget>,
    duration: Duration,
    frame: impl FnMut(f64) -> bool + 'static,
) {
    animate_with(widget, duration, ease_out, frame);
}

/// `animate` with an easing `curve` from linear to eased progress.
fn animate_with(
    widget: &impl IsA<gtk::Widget>,
    duration: Duration,
    curve: fn(f64) -> f64,
    mut frame: impl FnMut(f64) -> bool + 'static,
) {
    if !animations_enabled() {
        frame(1.0);
        return;
    }
    let duration = duration.as_micros() as f64;
    // When the animation started, shifted later by stalls, and when the previous frame ran.
    let timing = Cell::new(None::<(i64, i64)>);
    let frame = RefCell::new(frame);
    widget.add_tick_callback(move |_, clock| {
        let now = clock.frame_time();
        let (mut started, previous) = timing.get().unwrap_or((now, now));
        if now - previous > STALL_US {
            started += now - previous - FRAME_US;
        }
        timing.set(Some((started, now)));
        let linear = ((now - started) as f64 / duration).clamp(0.0, 1.0);
        if (frame.borrow_mut())(curve(linear)) && linear < 1.0 {
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });
}

/// Starts a screen change on the window's `SlideBin`: moves the current screen's widgets out of
/// `root` into the bin's previous-screen layer, leaving `root` empty for the next screen. `None`
/// when there is nothing to animate; `root` is then simply cleared.
fn begin_change(root: &gtk::Box, above: bool) -> Option<(SlideBin, u64)> {
    let bin = root.parent().and_downcast::<SlideBin>();
    if let Some(bin) = &bin {
        bin.settle();
    }
    // A window that has not been laid out yet has nothing on screen to animate away.
    let Some(bin) = bin.filter(|bin| {
        animations_enabled()
            && root.is_mapped()
            && root.first_child().is_some()
            && bin.width() > 0
            && bin.height() > 0
    }) else {
        clear(root);
        return None;
    };
    // The real widgets keep drawing exactly as they were; the copy of `root` keeps its classes,
    // so its padding and styling stay the same.
    let previous = gtk::Box::new(root.orientation(), root.spacing());
    let classes = root.css_classes();
    previous.set_css_classes(&classes.iter().map(glib::GString::as_str).collect::<Vec<_>>());
    while let Some(child) = root.first_child() {
        root.remove(&child);
        previous.append(&child);
    }
    // `root.width()` excludes its own padding; the screen fills the bin, so the bin's size is the
    // full size the moved screen had, padding included.
    let generation = bin.begin(&previous, bin.width(), bin.height(), above);
    Some((bin, generation))
}

/// Runs a screen change to its end, calling `place` with eased progress on every frame before
/// dropping the previous screen.
fn run_change(
    bin: &SlideBin,
    generation: u64,
    duration: Duration,
    curve: fn(f64) -> f64,
    place: impl Fn(&SlideBin, f32) + 'static,
) {
    place(bin, 0.0);
    let weak = bin.downgrade();
    animate_with(bin, duration, curve, move |progress| {
        let Some(bin) = weak.upgrade() else {
            return false;
        };
        if bin.generation() != generation {
            return false;
        }
        if progress >= 1.0 {
            bin.settle();
        } else {
            place(&bin, progress as f32);
        }
        true
    });
}

/// Clears `root` for the next screen while the current one fades out on top of it. `root` must
/// be the child of the window's `SlideBin`; otherwise this is a plain `clear`.
pub(crate) fn replace_screen(root: &gtk::Box) {
    let Some((bin, generation)) = begin_change(root, true) else {
        return;
    };
    run_change(&bin, generation, SCREEN_FADE, ease_out, |bin, progress| {
        bin.set_motion(Motion::default(), Motion::default(), f64::from(1.0 - progress));
    });
}

/// Which way a screen change slides.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slide {
    /// Into a detail screen: it slides in from the right over the current screen.
    Forward,
    /// Back out of it: it slides away to the right, uncovering the screen it came from.
    Back,
}

/// Clears `root` for the next screen and slides between the current screen and the new one. The
/// screen underneath moves only part of the way and darkens, so the one on top reads as a layer
/// above it.
pub(crate) fn slide_screen(root: &gtk::Box, slide: Slide) {
    let Some((bin, generation)) = begin_change(root, slide == Slide::Back) else {
        return;
    };
    let width = bin.width() as f32;
    // `on_top` is the screen in front at `offset`; `under` sits behind it, pushed left and dimmed
    // by `depth`. Forward the new screen arrives on top; back the old one leaves on top.
    run_change(&bin, generation, SCREEN_SLIDE, ease_in_out, move |bin, progress| {
        let (offset, depth) = match slide {
            Slide::Forward => (width * (1.0 - progress), progress),
            Slide::Back => (width * progress, 1.0 - progress),
        };
        let on_top = Motion {
            offset,
            dim: 0.0,
            shadow: true,
        };
        let under = Motion {
            offset: -SLIDE_DEPTH * width * depth,
            dim: SLIDE_DIM * depth,
            shadow: false,
        };
        match slide {
            Slide::Forward => bin.set_motion(on_top, under, 1.0),
            Slide::Back => bin.set_motion(under, on_top, 1.0),
        }
    });
}

/// Callback run with the tween's value on every frame.
type OnFrame = Rc<RefCell<Option<Box<dyn Fn(f64)>>>>;

/// A number that eases to each new target, redrawing `widget` (and calling `on_frame`) on every
/// frame in between. Targets set while `widget` is not on screen apply at once.
#[derive(Clone)]
pub(crate) struct Tween {
    widget: glib::WeakRef<gtk::Widget>,
    value: Rc<Cell<f64>>,
    target: Rc<Cell<f64>>,
    /// Bumped by every change, so a newer animation takes over from an older one.
    generation: Rc<Cell<u64>>,
    on_frame: OnFrame,
}

impl Tween {
    pub(crate) fn new(widget: &impl IsA<gtk::Widget>, value: f64) -> Self {
        Self {
            widget: widget.upcast_ref::<gtk::Widget>().downgrade(),
            value: Rc::new(Cell::new(value)),
            target: Rc::new(Cell::new(value)),
            generation: Rc::new(Cell::new(0)),
            on_frame: Rc::default(),
        }
    }

    /// Runs `callback` with the value on every frame, e.g. to write a counting-up label.
    pub(crate) fn on_frame(&self, callback: impl Fn(f64) + 'static) {
        self.on_frame.replace(Some(Box::new(callback)));
    }

    pub(crate) fn value(&self) -> f64 {
        self.value.get()
    }

    pub(crate) fn set(&self, target: f64) {
        if self.target.get() == target {
            return;
        }
        let Some(widget) = self.widget.upgrade().filter(WidgetExt::is_mapped) else {
            self.jump(target);
            return;
        };
        self.target.set(target);
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        let from = self.value.get();
        let (value, current, weak, on_frame) = (
            Rc::clone(&self.value),
            Rc::clone(&self.generation),
            self.widget.clone(),
            Rc::clone(&self.on_frame),
        );
        animate(&widget, TWEEN, move |progress| {
            let Some(widget) = weak.upgrade() else {
                return false;
            };
            if current.get() != generation {
                return false;
            }
            let now = from + (target - from) * progress;
            value.set(now);
            if let Some(callback) = &*on_frame.borrow() {
                callback(now);
            }
            widget.queue_draw();
            true
        });
    }

    /// Moves to `value` without animating, e.g. for the first real reading.
    pub(crate) fn jump(&self, value: f64) {
        self.generation.set(self.generation.get().wrapping_add(1));
        self.target.set(value);
        self.value.set(value);
        if let Some(callback) = &*self.on_frame.borrow() {
            callback(value);
        }
        if let Some(widget) = self.widget.upgrade() {
            widget.queue_draw();
        }
    }
}

thread_local! {
    /// The latest glide started on each adjustment, keyed by its address.
    static GLIDES: RefCell<HashMap<usize, u64>> = RefCell::default();
}

fn glide_generation(adjustment: &gtk::Adjustment, bump: bool) -> u64 {
    let key = adjustment.as_ptr() as usize;
    GLIDES.with_borrow_mut(|glides| {
        let generation = glides.entry(key).or_default();
        if bump {
            *generation = generation.wrapping_add(1);
        }
        *generation
    })
}

/// Eases `adjustment` to `target`, driven by `clock`'s frame clock. A newer glide on the same
/// adjustment takes over, and the glide stops as soon as anything else moves the adjustment, such
/// as the user scrolling. The returned flag is true while the glide runs.
pub(crate) fn glide(
    clock: &impl IsA<gtk::Widget>,
    adjustment: &gtk::Adjustment,
    target: f64,
) -> Rc<Cell<bool>> {
    let lower = adjustment.lower();
    let target = target.clamp(lower, (adjustment.upper() - adjustment.page_size()).max(lower));
    let generation = glide_generation(adjustment, true);
    if !clock.is_mapped() {
        adjustment.set_value(target);
        return Rc::new(Cell::new(false));
    }
    let running = Rc::new(Cell::new(true));
    let from = adjustment.value();
    let written = Cell::new(from);
    let (adjustment, flag) = (adjustment.downgrade(), Rc::clone(&running));
    animate(clock, GLIDE, move |progress| {
        let Some(adjustment) = adjustment.upgrade() else {
            flag.set(false);
            return false;
        };
        if glide_generation(&adjustment, false) != generation || adjustment.value() != written.get()
        {
            flag.set(false);
            return false;
        }
        adjustment.set_value(from + (target - from) * progress);
        // Read back: the adjustment clamps, and a clamped value must not read as another scroll.
        written.set(adjustment.value());
        if progress >= 1.0 {
            flag.set(false);
        }
        true
    });
    running
}

/// Fades `widget` in from transparent; call it right after adding the widget.
pub(crate) fn fade_in(widget: &impl IsA<gtk::Widget>) {
    let target = widget.clone().upcast::<gtk::Widget>();
    target.set_opacity(0.0);
    let fading = target.downgrade();
    animate(&target, FADE_IN, move |progress| {
        let Some(fading) = fading.upgrade() else {
            return false;
        };
        fading.set_opacity(progress);
        true
    });
}
