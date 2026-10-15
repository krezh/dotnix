//! The animated energy border that marks running work: around a card, a button, or a step.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;

use crate::ui::animations_enabled;
use crate::ui::theme;

struct EnergyBorderGeometry {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
}

fn draw_energy_border(
    context: &gtk::cairo::Context,
    geometry: EnergyBorderGeometry,
    phase: f64,
    opacity: f64,
) {
    let EnergyBorderGeometry {
        x,
        y,
        width,
        height,
        radius,
    } = geometry;

    let scale = height / 30.0;
    let beam_len = 0.35_f64;
    // The fading tail is drawn as many short bands. Translucent bands drawn normally either
    // double their alpha where they overlap (beads) or leave a hairline where they only touch
    // (antialiased edges under-cover the shared pixel). So the bands overlap slightly and are
    // painted tail-first into a group with SOURCE, where each band replaces rather than adds
    // to its neighbour; the group is then composited once. Band count follows the beam's pixel
    // length, and each band is traced through sub-points so corners stay round.
    let beam_px = beam_len * theme::capsule_perimeter(width, height, radius);
    let bands = (beam_px / 4.0).clamp(24.0, 240.0) as usize;
    let band_len = beam_len / bands as f64;
    let samples = ((beam_px / bands as f64) / 1.5).ceil().max(1.0) as usize + 1;
    context.push_group();
    context.set_operator(gtk::cairo::Operator::Source);
    context.set_line_cap(gtk::cairo::LineCap::Butt);
    context.set_line_join(gtk::cairo::LineJoin::Round);
    for band in (0..bands).rev() {
        let fraction = 1.0 - band as f64 / bands as f64;
        let start = phase - band_len * band as f64;
        // Reach half a band into the already-drawn tail side so no seam is left uncovered.
        let end = phase - band_len * (band as f64 + 1.5);
        context.set_line_width((1.4 + 1.8 * fraction) * scale);
        theme::set_source_rgba(
            context,
            theme::SAPPHIRE,
            fraction * fraction * 0.95 * opacity,
        );
        for sample in 0..=samples {
            let u = start + (end - start) * (sample as f64 / samples as f64);
            let (px, py) = theme::capsule_point(x, y, width, height, radius, u);
            if sample == 0 {
                context.move_to(px, py);
            } else {
                context.line_to(px, py);
            }
        }
        let _ = context.stroke();
    }
    let _ = context.pop_group_to_source();
    context.set_operator(gtk::cairo::Operator::Over);
    let _ = context.paint();

    let (head_x, head_y) = theme::capsule_point(x, y, width, height, radius, phase);
    let (tail_x, tail_y) = theme::capsule_point(x, y, width, height, radius, phase - beam_len);
    context.arc(tail_x, tail_y, 0.7 * scale, 0.0, std::f64::consts::TAU);
    theme::set_source_rgba(context, theme::SAPPHIRE, 0.12 * opacity);
    let _ = context.fill();

    context.arc(head_x, head_y, 2.0 * scale, 0.0, std::f64::consts::TAU);
    theme::set_source_rgba(context, theme::SKY, 0.70 * opacity);
    let _ = context.fill();
    context.arc(head_x, head_y, 0.9 * scale, 0.0, std::f64::consts::TAU);
    theme::set_source_rgba(context, theme::TEXT, 0.80 * opacity);
    let _ = context.fill();
}

/// A filled capsule lit from the energy beam's head, with the beam running around its edge.
pub(crate) fn draw_energy_capsule(
    context: &gtk::cairo::Context,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
    phase: f64,
) {
    let scale = height / 30.0;
    theme::set_source_rgba(context, theme::BASE, 0.85);
    theme::rounded_rectangle(context, x, y, width, height, radius);
    let _ = context.fill();

    let (head_x, head_y) = theme::capsule_point(x, y, width, height, radius, phase);
    let _ = context.save();
    theme::rounded_rectangle(
        context,
        x + scale,
        y + scale,
        width - 2.0 * scale,
        height - 2.0 * scale,
        radius - scale,
    );
    context.clip();

    let gradient =
        gtk::cairo::RadialGradient::new(head_x, head_y, 0.0, head_x, head_y, 55.0 * scale);
    let (sr, sg, sb) = theme::SAPPHIRE.rgb();
    let (lr, lg, lb) = theme::LAVENDER.rgb();
    gradient.add_color_stop_rgba(0.0, lr, lg, lb, 0.32);
    gradient.add_color_stop_rgba(0.40, sr, sg, sb, 0.16);
    gradient.add_color_stop_rgba(1.0, sr, sg, sb, 0.0);
    let _ = context.set_source(&gradient);
    let _ = context.paint();
    let _ = context.restore();

    draw_energy_border(
        context,
        EnergyBorderGeometry {
            x,
            y,
            width,
            height,
            radius,
        },
        phase,
        1.0,
    );
}

/// A fade-out requested by `finish`; `started` is the frame time of its first frame.
#[derive(Clone, Copy)]
struct Fade {
    duration_ms: u32,
    started: Option<i64>,
}

#[derive(Clone)]
pub(crate) struct EnergyIndicator {
    pub(crate) root: gtk::DrawingArea,
    active: Rc<Cell<bool>>,
    phase: Rc<Cell<f64>>,
    opacity: Rc<Cell<f64>>,
    fade: Rc<Cell<Option<Fade>>>,
    /// The one tick callback animating the border, while it runs.
    tick: Rc<RefCell<Option<gtk::TickCallbackId>>>,
}

impl EnergyIndicator {
    pub(crate) fn new(border_radius: f64) -> Self {
        let root = gtk::DrawingArea::new();
        root.set_can_target(false);
        root.set_content_width(28);
        root.set_content_height(28);
        let active = Rc::new(Cell::new(false));
        let phase = Rc::new(Cell::new(0.12));
        let opacity = Rc::new(Cell::new(1.0));
        let draw_active = Rc::clone(&active);
        let draw_phase = Rc::clone(&phase);
        let draw_opacity = Rc::clone(&opacity);
        root.set_draw_func(move |_, context, width, height| {
            if draw_active.get() {
                draw_energy_border(
                    context,
                    EnergyBorderGeometry {
                        x: 1.0,
                        y: 1.0,
                        width: f64::from(width) - 2.0,
                        height: f64::from(height) - 2.0,
                        radius: (border_radius - 1.0).max(0.0),
                    },
                    draw_phase.get(),
                    draw_opacity.get(),
                );
            }
        });
        Self {
            root,
            active,
            phase,
            opacity,
            fade: Rc::new(Cell::new(None)),
            tick: Rc::new(RefCell::new(None)),
        }
    }

    pub(crate) fn start(&self) {
        self.opacity.set(1.0);
        self.fade.set(None);
        if self.active.replace(true) {
            return;
        }
        self.root.queue_draw();
        if !animations_enabled() {
            return;
        }
        let active = Rc::clone(&self.active);
        let phase = Rc::clone(&self.phase);
        let opacity = Rc::clone(&self.opacity);
        let fade = Rc::clone(&self.fade);
        let tick = Rc::clone(&self.tick);
        let id = self.root.add_tick_callback(move |indicator, frame_clock| {
            let now = frame_clock.frame_time();
            let seconds = now as f64 / 1_000_000.0;
            phase.set((seconds / 1.4).fract());
            indicator.queue_draw();
            let Some(current) = fade.get() else {
                return glib::ControlFlow::Continue;
            };
            let Some(started) = current.started else {
                fade.set(Some(Fade {
                    started: Some(now),
                    ..current
                }));
                return glib::ControlFlow::Continue;
            };
            let duration = f64::from(current.duration_ms) * 1_000.0;
            let linear = ((now - started) as f64 / duration).clamp(0.0, 1.0);
            let eased = linear * linear * (3.0 - 2.0 * linear);
            opacity.set(1.0 - eased);
            if linear < 1.0 {
                return glib::ControlFlow::Continue;
            }
            active.set(false);
            fade.set(None);
            opacity.set(1.0);
            tick.borrow_mut().take();
            glib::ControlFlow::Break
        });
        *self.tick.borrow_mut() = Some(id);
    }

    /// Fades the running border out over `duration_ms`; `start` cancels the fade.
    pub(crate) fn finish(&self, duration_ms: u32) {
        if !self.active.get() {
            return;
        }
        if !animations_enabled() || self.tick.borrow().is_none() {
            self.stop();
            return;
        }
        if self.fade.get().is_none() {
            self.fade.set(Some(Fade {
                duration_ms,
                started: None,
            }));
        }
    }

    pub(crate) fn stop(&self) {
        self.active.set(false);
        self.fade.set(None);
        self.opacity.set(1.0);
        if let Some(tick) = self.tick.take() {
            tick.remove();
        }
        self.root.queue_draw();
    }
}

#[derive(Clone)]
pub(crate) struct EnergyOverlay {
    pub(crate) root: gtk::Overlay,
    indicator: EnergyIndicator,
}

impl EnergyOverlay {
    pub(crate) fn new(child: &impl IsA<gtk::Widget>, border_radius: f64) -> Self {
        let root = gtk::Overlay::new();
        root.set_child(Some(child));
        let indicator = EnergyIndicator::new(border_radius);
        indicator.root.set_content_width(0);
        indicator.root.set_content_height(0);
        indicator.root.set_halign(gtk::Align::Fill);
        indicator.root.set_valign(gtk::Align::Fill);
        indicator.root.set_hexpand(true);
        indicator.root.set_vexpand(true);
        root.add_overlay(&indicator.root);
        Self { root, indicator }
    }

    pub(crate) fn start(&self) {
        self.indicator.start();
    }

    pub(crate) fn stop(&self) {
        self.indicator.stop();
    }

    pub(crate) fn finish(&self, duration_ms: u32) {
        self.indicator.finish(duration_ms);
    }
}
