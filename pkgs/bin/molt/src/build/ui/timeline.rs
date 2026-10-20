use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;

use crate::build::BuildPhase;
use crate::ui::theme;
use crate::ui::{MorphLabel, animations_enabled, draw_energy_capsule, label};

const STEP_NAMES: [&str; 3] = ["Evaluate", "Build", "Compare"];
pub(crate) const STEP_WIDTH: f64 = 104.0;
pub(crate) const STEP_HEIGHT: f64 = 30.0;
const STEP_RADIUS: f64 = 7.0;
const CONNECTOR_WIDTH: f64 = 36.0;

/// The timeline's name for `phase`.
pub(crate) const fn step_name(phase: BuildPhase) -> &'static str {
    STEP_NAMES[phase.index()]
}
/// Room around the steps; the active step's glow fades out over this distance, though only the
/// part inside the step is drawn.
const GLOW_X: f64 = 14.0;
const GLOW_Y: f64 = 12.0;

pub(crate) struct BuildTimeline {
    pub(crate) root: gtk::Box,
    indicator: gtk::DrawingArea,
    current: Rc<Cell<f64>>,
    target: Rc<Cell<f64>>,
    pub(crate) complete: Rc<Cell<bool>>,
    /// The active step failed: drawn red, without glow or beam.
    failed: Rc<Cell<bool>>,
    pulse: Rc<Cell<f64>>,
    pulse_tick: RefCell<Option<gtk::TickCallbackId>>,
    animating: Rc<Cell<bool>>,
}

impl BuildTimeline {
    pub(crate) fn new(font: &str) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.add_css_class("build-timeline");
        root.set_halign(gtk::Align::Center);
        root.set_valign(gtk::Align::Center);
        root.set_vexpand(false);
        let current = Rc::new(Cell::new(0.0_f64));
        let target = Rc::new(Cell::new(0.0_f64));
        let complete = Rc::new(Cell::new(false));
        let failed = Rc::new(Cell::new(false));
        let pulse = Rc::new(Cell::new(0.0_f64));
        let indicator = gtk::DrawingArea::new();
        let steps_width = 3.0 * STEP_WIDTH + 2.0 * CONNECTOR_WIDTH;
        indicator.set_content_width((steps_width + 2.0 * GLOW_X) as i32);
        indicator.set_content_height((STEP_HEIGHT + 2.0 * GLOW_Y) as i32);
        let draw_position = Rc::clone(&current);
        let draw_target = Rc::clone(&target);
        let draw_complete = Rc::clone(&complete);
        let draw_failed = Rc::clone(&failed);
        let draw_pulse = Rc::clone(&pulse);
        let draw_font = font.to_owned();
        let glow = step_glow();
        indicator.set_draw_func(move |_, context, width, height| {
            let offset_x = ((f64::from(width) - steps_width) / 2.0).max(0.0);
            let y = ((f64::from(height) - STEP_HEIGHT) / 2.0).max(0.0);
            let center_y = y + STEP_HEIGHT / 2.0;
            let position = draw_position.get().clamp(0.0, 2.0);
            let active = draw_target.get().round().clamp(0.0, 2.0) as usize;
            let is_complete = draw_complete.get();
            let is_failed = draw_failed.get();
            context.select_font_face(
                &draw_font,
                gtk::cairo::FontSlant::Normal,
                gtk::cairo::FontWeight::Bold,
            );
            context.set_font_size(12.0);

            for i in 0..2 {
                let start = offset_x + (i as f64 + 1.0) * STEP_WIDTH + i as f64 * CONNECTOR_WIDTH;
                let progress = if is_complete {
                    1.0
                } else {
                    (position - i as f64).clamp(0.0, 1.0)
                };
                let travelling = !is_complete && !is_failed && active == i + 1 && progress > 0.01;
                draw_connector(context, start, center_y, progress, travelling);
            }

            for (i, name) in STEP_NAMES.iter().enumerate() {
                let x = offset_x + i as f64 * (STEP_WIDTH + CONNECTOR_WIDTH);
                let look = if is_complete || i < active {
                    StepLook::Done
                } else if i == active && is_failed {
                    StepLook::Failed
                } else if i == active && (active == 0 || position >= active as f64 - 0.15) {
                    StepLook::Active
                } else {
                    StepLook::Pending
                };
                draw_step(context, x, y, name, look, draw_pulse.get(), glow.as_ref());
            }
        });
        root.append(&indicator);
        let timeline = Self {
            root,
            indicator,
            current,
            target,
            complete,
            failed,
            pulse,
            pulse_tick: RefCell::new(None),
            animating: Rc::new(Cell::new(false)),
        };
        timeline.set_phase(BuildPhase::Evaluate);
        timeline
    }

    pub(crate) fn start_pulse(&self) {
        if !animations_enabled() {
            return;
        }
        let pulse = Rc::clone(&self.pulse);
        let tick = self
            .indicator
            .add_tick_callback(move |indicator, frame_clock| {
                let seconds = frame_clock.frame_time() as f64 / 1_000_000.0;
                pulse.set((seconds / 1.4).fract());
                indicator.queue_draw();
                glib::ControlFlow::Continue
            });
        self.pulse_tick.replace(Some(tick));
    }

    /// Marks the current step failed and stops the beam.
    pub(crate) fn set_failed(&self) {
        self.failed.set(true);
        self.current.set(self.target.get());
        if let Some(tick) = self.pulse_tick.take() {
            tick.remove();
        }
        self.indicator.queue_draw();
    }

    pub(crate) fn set_complete(&self) {
        self.complete.set(true);
        self.current.set(2.0);
        self.target.set(2.0);
        self.indicator.queue_draw();
    }

    pub(crate) fn set_phase(&self, phase: BuildPhase) {
        self.complete.set(false);
        self.target.set(phase.index() as f64);
        self.indicator.queue_draw();
        if (self.target.get() - self.current.get()).abs() < 0.001 {
            return;
        }
        if !animations_enabled() {
            self.current.set(self.target.get());
            self.indicator.queue_draw();
            return;
        }
        if self.animating.replace(true) {
            return;
        }
        let current = Rc::clone(&self.current);
        let target = Rc::clone(&self.target);
        let animating = Rc::clone(&self.animating);
        let previous_frame = Cell::new(0_i64);
        self.indicator
            .add_tick_callback(move |indicator, frame_clock| {
                let frame_time = frame_clock.frame_time();
                let previous = previous_frame.replace(frame_time);
                if previous != 0 {
                    let elapsed = (frame_time - previous).clamp(0, 50_000) as f64 / 1_000_000.0;
                    current.set(advance_timeline(current.get(), target.get(), elapsed));
                    indicator.queue_draw();
                }
                if (target.get() - current.get()).abs() < 0.001 {
                    current.set(target.get());
                    animating.set(false);
                    indicator.queue_draw();
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
    }
}

/// How one step capsule is drawn.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepLook {
    /// Passed: green tint, green check and name, like the rails between passed steps.
    Done,
    /// Failed: red tint, red outline, and a cross.
    Failed,
    /// Running: glow and the energy beam at `pulse`.
    Active,
    /// Not reached yet: mantle with dimmed text.
    Pending,
}

/// Draws one step capsule named `name` with its top-left at `x, y`. The caller selects the font.
pub(crate) fn draw_step(
    context: &gtk::cairo::Context,
    x: f64,
    y: f64,
    name: &str,
    look: StepLook,
    pulse: f64,
    glow: Option<&gtk::cairo::ImageSurface>,
) {
    let Ok(extents) = context.text_extents(name) else {
        return;
    };
    let center_y = y + STEP_HEIGHT / 2.0;
    let text_y = y + (STEP_HEIGHT - extents.height()) / 2.0 - extents.y_bearing();
    let centered_x = x + (STEP_WIDTH - extents.width()) / 2.0 - extents.x_bearing();
    match look {
        StepLook::Done => {
            theme::set_source_rgba(context, theme::GREEN, 0.14);
            theme::rounded_rectangle(context, x, y, STEP_WIDTH, STEP_HEIGHT, STEP_RADIUS);
            let _ = context.fill();
            let check_width = 9.0;
            let gap = 7.0;
            let left = x + (STEP_WIDTH - check_width - gap - extents.width()) / 2.0;
            context.set_line_width(1.6);
            context.set_line_cap(gtk::cairo::LineCap::Round);
            context.set_line_join(gtk::cairo::LineJoin::Round);
            theme::set_source_rgb(context, theme::GREEN);
            context.move_to(left, center_y);
            context.line_to(left + 3.0, center_y + 3.0);
            context.line_to(left + check_width, center_y - 3.5);
            let _ = context.stroke();
            context.move_to(left + check_width + gap - extents.x_bearing(), text_y);
            let _ = context.show_text(name);
        }
        StepLook::Failed => {
            draw_failed_step(context, x, y, center_y, text_y, name, &extents);
        }
        StepLook::Active => {
            if let Some(glow) = glow {
                draw_step_glow(context, glow, x, y);
            }
            draw_energy_capsule(context, x, y, STEP_WIDTH, STEP_HEIGHT, STEP_RADIUS, pulse);
            theme::set_source_rgb(context, theme::TEXT);
            context.move_to(centered_x, text_y);
            let _ = context.show_text(name);
        }
        StepLook::Pending => {
            theme::set_source_rgb(context, theme::MANTLE);
            theme::rounded_rectangle(context, x, y, STEP_WIDTH, STEP_HEIGHT, STEP_RADIUS);
            let _ = context.fill();
            theme::set_source_rgb(context, theme::OVERLAY1);
            context.move_to(centered_x, text_y);
            let _ = context.show_text(name);
        }
    }
}

/// A rail between two steps: surface1, filled green as the timeline passes through it, with an
/// energy pulse riding the fill while the next step is being reached.
fn draw_connector(
    context: &gtk::cairo::Context,
    start: f64,
    center_y: f64,
    progress: f64,
    travelling: bool,
) {
    context.set_line_cap(gtk::cairo::LineCap::Butt);
    context.set_line_width(1.5);
    theme::set_source_rgb(context, theme::SURFACE1);
    context.move_to(start, center_y);
    context.line_to(start + CONNECTOR_WIDTH, center_y);
    let _ = context.stroke();
    if progress <= 0.0 {
        return;
    }
    let end = start + CONNECTOR_WIDTH * progress;
    theme::set_source_rgb(context, theme::GREEN);
    context.move_to(start, center_y);
    context.line_to(end, center_y);
    let _ = context.stroke();
    if travelling && progress < 0.99 {
        context.arc(end, center_y, 4.0, 0.0, std::f64::consts::TAU);
        theme::set_source_rgba(context, theme::SAPPHIRE, 0.30);
        let _ = context.fill();
        context.arc(end, center_y, 2.2, 0.0, std::f64::consts::TAU);
        theme::set_source_rgba(context, theme::SKY, 0.95);
        let _ = context.fill();
    }
}

/// A failed step: red-tinted capsule with a red outline and a cross before its name.
fn draw_failed_step(
    context: &gtk::cairo::Context,
    x: f64,
    y: f64,
    center_y: f64,
    text_y: f64,
    name: &str,
    extents: &gtk::cairo::TextExtents,
) {
    theme::rounded_rectangle(context, x, y, STEP_WIDTH, STEP_HEIGHT, STEP_RADIUS);
    theme::set_source_rgba(context, theme::RED, 0.14);
    let _ = context.fill_preserve();
    context.set_line_width(1.5);
    theme::set_source_rgb(context, theme::RED);
    let _ = context.stroke();
    let cross = 7.0;
    let gap = 7.0;
    let left = x + (STEP_WIDTH - cross - gap - extents.width()) / 2.0;
    context.set_line_width(1.6);
    context.set_line_cap(gtk::cairo::LineCap::Round);
    context.move_to(left, center_y - cross / 2.0);
    context.line_to(left + cross, center_y + cross / 2.0);
    context.move_to(left + cross, center_y - cross / 2.0);
    context.line_to(left, center_y + cross / 2.0);
    let _ = context.stroke();
    theme::set_source_rgb(context, theme::RED);
    context.move_to(left + cross + gap - extents.x_bearing(), text_y);
    let _ = context.show_text(name);
}

/// Soft lavender-to-sapphire glow inside the active step, rendered once per timeline. Its
/// contours are squircles (superellipses) to match the step's shape; cairo gradients can only
/// draw circles and ellipses, so it is computed per pixel.
pub(crate) fn step_glow() -> Option<gtk::cairo::ImageSurface> {
    /// Superellipse exponent: 2 would be an ellipse.
    const SQUIRCLE: f64 = 4.0;
    let (radius_x, radius_y) = (STEP_WIDTH / 2.0 + GLOW_X, STEP_HEIGHT / 2.0 + GLOW_Y);
    let mut surface = gtk::cairo::ImageSurface::create(
        gtk::cairo::Format::ARgb32,
        STEP_WIDTH as i32,
        STEP_HEIGHT as i32,
    )
    .ok()?;
    let stride = usize::try_from(surface.stride()).ok()?;
    let mut data = surface.data().ok()?;
    for row in 0..STEP_HEIGHT as usize {
        let dy = ((row as f64 + 0.5 - STEP_HEIGHT / 2.0) / radius_y).abs();
        for column in 0..STEP_WIDTH as usize {
            let dx = ((column as f64 + 0.5 - STEP_WIDTH / 2.0) / radius_x).abs();
            let t = (dx.powf(SQUIRCLE) + dy.powf(SQUIRCLE)).powf(SQUIRCLE.recip());
            let ((r, g, b), alpha) = glow_color(t);
            let channel = |value: f64| (value * alpha * 255.0).round() as u32;
            let pixel = channel(1.0) << 24 | channel(r) << 16 | channel(g) << 8 | channel(b);
            let offset = row * stride + column * 4;
            data[offset..offset + 4].copy_from_slice(&pixel.to_ne_bytes());
        }
    }
    drop(data);
    Some(surface)
}

/// The glow's colour and opacity `t` of the way from the step's centre to where it fades out.
fn glow_color(t: f64) -> ((f64, f64, f64), f64) {
    const MID: f64 = 0.45;
    let (lavender, sapphire) = (theme::LAVENDER.rgb(), theme::SAPPHIRE.rgb());
    if t < MID {
        let u = t / MID;
        let mix = |from: f64, to: f64| from + (to - from) * u;
        let color = (
            mix(lavender.0, sapphire.0),
            mix(lavender.1, sapphire.1),
            mix(lavender.2, sapphire.2),
        );
        (color, mix(0.32, 0.14))
    } else {
        (sapphire, 0.14 * (1.0 - (t - MID) / (1.0 - MID)).max(0.0))
    }
}

fn draw_step_glow(context: &gtk::cairo::Context, glow: &gtk::cairo::ImageSurface, x: f64, y: f64) {
    let _ = context.save();
    theme::rounded_rectangle(context, x, y, STEP_WIDTH, STEP_HEIGHT, STEP_RADIUS);
    context.clip();
    if context.set_source_surface(glow, x, y).is_ok() {
        let _ = context.paint();
    }
    let _ = context.restore();
}

/// Title and subtitle on the left, the step timeline on the right: the header of every build phase.
pub(crate) struct PhaseHeader {
    pub(crate) root: gtk::Box,
    pub(crate) title: MorphLabel,
    /// The subtitle's row, where a screen can add a pill after it.
    pub(crate) subtitle_row: gtk::Box,
}

pub(crate) fn phase_header(title: &str, subtitle: &str, timeline: &BuildTimeline) -> PhaseHeader {
    let root = gtk::Box::new(gtk::Orientation::Horizontal, 24);
    root.add_css_class("phase-header");
    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);
    let title = MorphLabel::new(title, &["phase-title"], 0.0);
    text.append(&title.root);
    let subtitle_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    subtitle_row.append(&label(subtitle, &["phase-subtitle"], 0.0));
    text.append(&subtitle_row);
    root.append(&text);
    timeline.root.set_halign(gtk::Align::End);
    root.append(&timeline.root);
    PhaseHeader {
        root,
        title,
        subtitle_row,
    }
}

fn advance_timeline(current: f64, target: f64, elapsed: f64) -> f64 {
    let diff = target - current;
    if diff.abs() < 0.001 {
        return target;
    }
    let sign = diff.signum();
    let dist = diff.abs();
    let seg_progress = if sign >= 0.0 {
        (current % 1.0 + 1.0) % 1.0
    } else {
        (1.0 - (current % 1.0 + 1.0) % 1.0) % 1.0
    };
    let ease = (seg_progress * std::f64::consts::PI).sin().powi(2);
    let speed = 2.0 + 3.2 * ease;
    let step = speed * elapsed;
    if step >= dist {
        target
    } else {
        current + sign * step
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_animation_retargets_while_moving() {
        let first_step = advance_timeline(0.0, 1.0, 0.05);
        assert!(first_step > 0.0 && first_step < 1.0);

        let retargeted = advance_timeline(first_step, 2.0, 0.05);
        assert!(retargeted > first_step && retargeted < 2.0);

        let settled = (0..100).fold(retargeted, |position, _| {
            advance_timeline(position, 2.0, 0.05)
        });
        assert_eq!(settled, 2.0);
    }
}
