//! The minimized popup: a small bar at the top centre that keeps a running build or activation in
//! view while its full screen waits underneath. Clicking it restores the full screen.
//!
//! The window's root morphs between the screen frame and the bar. Screens publish their status to
//! the bar as they run, whether it is showing or not, so it is current the moment it appears.
//! Window sizes requested while minimized are kept and applied on restore.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use gtk4_layer_shell::{KeyboardMode, LayerShell};

use crate::activation::ui::{COMPLETION_MORPH_MS, smootherstep};
use crate::build::ui::progress::clock;
use crate::build::ui::timeline::{STEP_HEIGHT, STEP_WIDTH, StepLook, draw_step, step_glow};
use crate::config::Appearance;
use crate::ui::{
    MorphBin, MorphLabel, Tween, animations_enabled, centered_icon, ease_in_out, label,
    set_window_size, theme,
};

const BAR_WIDTH: i32 = 480;
/// Room around the step capsule for the energy beam's stroke and head.
const STEP_MARGIN: f64 = 4.0;
/// Seconds per lap of the beam around the step, matching the build timeline.
const BEAM_LAP: f64 = 1.4;
/// Seconds per sweep of the progress line while the work cannot be measured.
const SWEEP: f64 = 1.4;
/// After an operation ends, the progress line holds its final colour this long, then fades out
/// over the same time.
const PROGRESS_HOLD_MS: f64 = 900.0;

/// How a minimized operation ended.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MiniEnd {
    Done,
    Failed,
}

/// What the bar shows.
pub(crate) struct MiniStatus<'a> {
    /// The step capsule's name: the phase running, or the outcome once ended.
    pub(crate) step: &'a str,
    pub(crate) title: &'a str,
    pub(crate) note: &'a str,
    pub(crate) elapsed: Duration,
    /// Share of the work done; `None` while it cannot be measured.
    pub(crate) progress: Option<f64>,
}

#[derive(Clone)]
pub(crate) struct Mini {
    inner: Rc<Inner>,
}

struct Inner {
    window: glib::WeakRef<gtk::ApplicationWindow>,
    root: MorphBin,
    bar: gtk::Overlay,
    step: gtk::DrawingArea,
    title: MorphLabel,
    note: gtk::Label,
    elapsed: gtk::Label,
    action: gtk::Button,
    action_label: gtk::Label,
    restore_button: gtk::Button,
    progress: gtk::DrawingArea,
    fraction: Tween,
    minimized: Cell<bool>,
    /// The window size screens asked for: preferred, then minimum.
    screen_size: Cell<((i32, i32), (i32, i32))>,
    /// The running step's name, kept to crossfade from once the operation ends.
    running_step: RefCell<String>,
    ended_step: RefCell<String>,
    end: Cell<Option<MiniEnd>>,
    measured: Cell<bool>,
    /// Frame time the end was first drawn at, which the end's motion runs from.
    ended_at: Cell<Option<i64>>,
    /// `0..=1` from the running capsule to the ended one.
    morph: Cell<f64>,
    /// `0..=1` opacity of the progress line once ended.
    progress_alpha: Cell<f64>,
    beam: Cell<f64>,
    sweep: Cell<f64>,
    tick: RefCell<Option<gtk::TickCallbackId>>,
}

impl Mini {
    /// Builds the bar and makes the window's child a morph between `screen` and the bar.
    pub(crate) fn new(
        window: &gtk::ApplicationWindow,
        screen: &impl IsA<gtk::Widget>,
        appearance: &Appearance,
    ) -> Self {
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 14);
        content.add_css_class("mini-content");

        let step = gtk::DrawingArea::new();
        step.set_content_width((STEP_WIDTH + 2.0 * STEP_MARGIN) as i32);
        step.set_content_height((STEP_HEIGHT + 2.0 * STEP_MARGIN) as i32);
        step.set_valign(gtk::Align::Center);
        content.append(&step);

        let texts = gtk::Box::new(gtk::Orientation::Vertical, 2);
        texts.set_hexpand(true);
        texts.set_valign(gtk::Align::Center);
        let title = MorphLabel::new("", &["mini-title"], 0.0);
        title.root.set_transition_duration(COMPLETION_MORPH_MS);
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        texts.append(&title.root);
        let note = label("", &["mini-note"], 0.0);
        note.set_ellipsize(gtk::pango::EllipsizeMode::End);
        texts.append(&note);
        content.append(&texts);

        let elapsed = label("0:00", &["mini-elapsed"], 0.5);
        elapsed.set_valign(gtk::Align::Center);
        content.append(&elapsed);

        let action_label = label("", &["mini-action-label"], 0.5);
        let action = gtk::Button::new();
        action.add_css_class("mini-action");
        action.set_child(Some(&action_label));
        action.set_valign(gtk::Align::Center);
        action.set_visible(false);
        content.append(&action);

        let restore_button = gtk::Button::new();
        restore_button.add_css_class("mini-restore");
        restore_button.add_css_class("surface-button");
        restore_button.set_tooltip_text(Some("Open Molt"));
        restore_button.set_valign(gtk::Align::Center);
        restore_button.set_child(Some(&centered_icon("view-fullscreen-symbolic", 14, 14)));
        content.append(&restore_button);

        let progress = gtk::DrawingArea::new();
        progress.add_css_class("mini-progress");
        progress.set_content_height(2);
        progress.set_valign(gtk::Align::End);
        progress.set_can_target(false);

        let bar = gtk::Overlay::new();
        bar.add_css_class("mini-bar");
        bar.set_size_request(BAR_WIDTH, -1);
        bar.set_child(Some(&content));
        bar.add_overlay(&progress);
        let root = MorphBin::new(screen, &bar, appearance.rounding as f32);
        root.add_css_class("updates-root");
        window.set_child(Some(&root));

        let fraction = Tween::new(&progress, 0.0);
        let inner = Rc::new(Inner {
            window: window.downgrade(),
            root,
            bar,
            step,
            title,
            note,
            elapsed,
            action,
            action_label,
            restore_button,
            progress,
            fraction,
            minimized: Cell::new(false),
            screen_size: Cell::new(((0, 0), (0, 0))),
            running_step: RefCell::default(),
            ended_step: RefCell::default(),
            end: Cell::new(None),
            measured: Cell::new(false),
            ended_at: Cell::new(None),
            morph: Cell::new(0.0),
            progress_alpha: Cell::new(1.0),
            beam: Cell::new(0.0),
            sweep: Cell::new(0.0),
            tick: RefCell::new(None),
        });
        connect(&inner, &appearance.sans_font);
        Self { inner }
    }

    pub(crate) fn is_minimized(&self) -> bool {
        self.inner.minimized.get()
    }

    /// Records the window size a screen asks for, applying it unless the window is minimized.
    pub(crate) fn fit(&self, preferred: (i32, i32), minimum: (i32, i32)) {
        self.inner.screen_size.set((preferred, minimum));
        if !self.is_minimized()
            && let Some(window) = self.inner.window.upgrade()
        {
            set_window_size(&window, preferred, minimum);
        }
    }

    pub(crate) fn minimize(&self) {
        let inner = &self.inner;
        if inner.minimized.replace(true) {
            return;
        }
        let Some(window) = inner.window.upgrade() else {
            return;
        };
        // Never steal the keyboard from whatever the user goes back to.
        window.set_keyboard_mode(KeyboardMode::None);
        inner.start_tick();
        // The window keeps its size throughout: resizing a layer surface makes compositors such
        // as Hyprland animate it a second time. Around the bar it is transparent and lets clicks
        // through to whatever is underneath.
        let weak = Rc::downgrade(inner);
        inner.root.morph_to(true, move || {
            let Some(inner) = weak.upgrade() else {
                return;
            };
            if let Some(window) = inner.window.upgrade()
                && let Some(bounds) = inner.bar.compute_bounds(&window)
            {
                let area = gtk::cairo::RectangleInt::new(
                    bounds.x() as i32,
                    bounds.y() as i32,
                    bounds.width().ceil() as i32,
                    bounds.height().ceil() as i32,
                );
                set_input_region(&window, Some(&area));
            }
        });
    }

    pub(crate) fn restore(&self) {
        restore(&self.inner);
    }

    /// Shows a running step. The first call after an operation ended starts the bar afresh.
    pub(crate) fn running(&self, status: &MiniStatus) {
        let inner = &self.inner;
        if inner.end.take().is_some() {
            inner.ended_at.set(None);
            inner.morph.set(0.0);
            inner.progress_alpha.set(1.0);
            inner.bar.remove_css_class("done");
            inner.bar.remove_css_class("failed");
            inner.action.set_visible(false);
            inner.restore_button.set_visible(true);
            inner.fraction.jump(0.0);
            inner.start_tick();
        }
        if *inner.running_step.borrow() != status.step {
            status.step.clone_into(&mut inner.running_step.borrow_mut());
            inner.step.queue_draw();
        }
        inner.show(status);
    }

    /// Shows how the operation ended, with `action` on the button that restores the window.
    pub(crate) fn finish(&self, end: MiniEnd, status: &MiniStatus, action: &str) {
        let inner = &self.inner;
        inner.end.set(Some(end));
        status.step.clone_into(&mut inner.ended_step.borrow_mut());
        inner.ended_at.set(None);
        inner.show(status);
        let (class, button_class, stale) = match end {
            MiniEnd::Done => ("done", "accent-button", "surface-button"),
            MiniEnd::Failed => ("failed", "surface-button", "accent-button"),
        };
        inner.bar.add_css_class(class);
        inner.action.remove_css_class(stale);
        inner.action.add_css_class(button_class);
        inner.action_label.set_text(action);
        inner.action.set_visible(true);
        inner.restore_button.set_visible(false);
        if inner.minimized.get() && animations_enabled() {
            inner.start_tick();
        } else {
            inner.morph.set(1.0);
            inner.progress_alpha.set(0.0);
            inner.step.queue_draw();
            inner.progress.queue_draw();
        }
    }
}

impl Inner {
    fn show(&self, status: &MiniStatus) {
        self.title.set_text(status.title);
        self.note.set_text(status.note);
        self.elapsed.set_text(&clock(status.elapsed));
        self.measured.set(status.progress.is_some());
        if let Some(progress) = status.progress {
            self.fraction.set(progress.clamp(0.0, 1.0));
        }
        self.progress.queue_draw();
    }

    /// Drives the beam, the progress sweep, and the end's motion while the bar is showing.
    fn start_tick(self: &Rc<Self>) {
        if !self.minimized.get() || !animations_enabled() || self.tick.borrow().is_some() {
            return;
        }
        let weak = Rc::downgrade(self);
        let id = self.bar.add_tick_callback(move |_, frame_clock| {
            let Some(inner) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let now = frame_clock.frame_time();
            let seconds = now as f64 / 1_000_000.0;
            inner.beam.set((seconds / BEAM_LAP).fract());
            inner.sweep.set((seconds / SWEEP).fract());
            let mut settled = false;
            if inner.end.get().is_some() {
                let started = inner.ended_at.get().unwrap_or(now);
                inner.ended_at.set(Some(started));
                let ms = (now - started) as f64 / 1_000.0;
                inner
                    .morph
                    .set(smootherstep(ms / f64::from(COMPLETION_MORPH_MS)));
                inner
                    .progress_alpha
                    .set(1.0 - smootherstep((ms - PROGRESS_HOLD_MS) / PROGRESS_HOLD_MS));
                settled = ms >= 2.0 * PROGRESS_HOLD_MS;
            }
            inner.step.queue_draw();
            inner.progress.queue_draw();
            if settled {
                inner.tick.borrow_mut().take();
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
        self.tick.replace(Some(id));
    }

    fn stop_tick(&self) {
        if let Some(tick) = self.tick.take() {
            tick.remove();
        }
        // The end's motion does not replay on the next minimize.
        if self.end.get().is_some() {
            self.morph.set(1.0);
            self.progress_alpha.set(0.0);
        }
    }
}

fn restore(inner: &Rc<Inner>) {
    if !inner.minimized.replace(false) {
        return;
    }
    inner.stop_tick();
    let Some(window) = inner.window.upgrade() else {
        return;
    };
    set_input_region(&window, None);
    let (preferred, minimum) = inner.screen_size.get();
    set_window_size(&window, preferred, minimum);
    inner.root.morph_to(false, || {});
    window.present();
}

/// Limits where the window takes pointer input to `area`, in window coordinates; `None` takes it
/// across the whole window again.
fn set_input_region(window: &gtk::ApplicationWindow, area: Option<&gtk::cairo::RectangleInt>) {
    if let Some(surface) = window.surface() {
        let region = area.map(gtk::cairo::Region::create_rectangle);
        surface.set_input_region(region.as_ref());
    }
}

fn connect(inner: &Rc<Inner>, font: &str) {
    let glow = step_glow();
    let draw = Rc::downgrade(inner);
    let font = font.to_owned();
    inner.step.set_draw_func(move |_, context, _, _| {
        let Some(inner) = draw.upgrade() else {
            return;
        };
        context.select_font_face(
            &font,
            gtk::cairo::FontSlant::Normal,
            gtk::cairo::FontWeight::Bold,
        );
        context.set_font_size(12.0);
        let morph = if inner.end.get().is_some() {
            inner.morph.get()
        } else {
            0.0
        };
        if morph < 1.0 {
            let _ = context.save();
            context.push_group();
            draw_step(
                context,
                STEP_MARGIN,
                STEP_MARGIN,
                &inner.running_step.borrow(),
                StepLook::Active,
                inner.beam.get(),
                glow.as_ref(),
            );
            let _ = context.pop_group_to_source();
            let _ = context.paint_with_alpha(1.0 - morph);
            let _ = context.restore();
        }
        if let Some(end) = inner.end.get()
            && morph > 0.0
        {
            let look = match end {
                MiniEnd::Done => StepLook::Done,
                MiniEnd::Failed => StepLook::Failed,
            };
            let _ = context.save();
            context.push_group();
            draw_step(
                context,
                STEP_MARGIN,
                STEP_MARGIN,
                &inner.ended_step.borrow(),
                look,
                0.0,
                None,
            );
            let _ = context.pop_group_to_source();
            let _ = context.paint_with_alpha(morph);
            let _ = context.restore();
        }
    });

    let draw = Rc::downgrade(inner);
    inner
        .progress
        .set_draw_func(move |_, context, width, height| {
            let Some(inner) = draw.upgrade() else {
                return;
            };
            draw_progress(&inner, context, f64::from(width), f64::from(height));
        });

    for button in [&inner.action, &inner.restore_button] {
        let weak = Rc::downgrade(inner);
        button.connect_clicked(move |_| restore_weak(&weak));
    }
    let click = gtk::GestureClick::new();
    let weak = Rc::downgrade(inner);
    click.connect_released(move |_, _, _, _| restore_weak(&weak));
    inner.bar.add_controller(click);
}

fn restore_weak(weak: &Weak<Inner>) {
    if let Some(inner) = weak.upgrade() {
        restore(&inner);
    }
}

/// The hairline along the bar's bottom edge: the share done, a sweep while that is unknown, and
/// the outcome's colour once ended.
fn draw_progress(inner: &Inner, context: &gtk::cairo::Context, width: f64, height: f64) {
    let inset = 16.0;
    let track = (width - 2.0 * inset).max(0.0);
    let alpha = inner.progress_alpha.get();
    if alpha <= 0.0 || track <= 0.0 {
        return;
    }
    theme::set_source_rgba(context, theme::SURFACE0, 0.8 * alpha);
    theme::rounded_rectangle(context, inset, 0.0, track, height, height / 2.0);
    let _ = context.fill();
    let (color, start, length) = match inner.end.get() {
        Some(MiniEnd::Done) => (theme::GREEN, 0.0, 1.0),
        Some(MiniEnd::Failed) => (theme::RED, 0.0, inner.fraction.value()),
        None if inner.measured.get() => (theme::SAPPHIRE, 0.0, inner.fraction.value()),
        None => {
            let travel = ease_in_out(inner.sweep.get());
            (theme::SAPPHIRE, travel * 1.3 - 0.3, 0.3)
        }
    };
    let left = inset + track * start.max(0.0);
    let right = inset + track * (start + length).min(1.0);
    if right <= left {
        return;
    }
    theme::set_source_rgba(context, color, alpha);
    theme::rounded_rectangle(context, left, 0.0, right - left, height, height / 2.0);
    let _ = context.fill();
}
