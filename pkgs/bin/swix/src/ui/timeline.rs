use std::cell::Cell;
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;

use crate::build::BuildPhase;
use crate::theme;
use crate::ui::common::{animations_enabled, label};

fn draw_energy_border(
    context: &gtk::cairo::Context,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
    phase: f64,
) {
    let scale = height / 30.0;
    let beam_len = 0.35_f64;
    let steps = 64;
    context.set_line_cap(gtk::cairo::LineCap::Butt);
    for step in 0..steps {
        let u0 = phase - beam_len * (step as f64 / steps as f64);
        let u1 = phase - beam_len * ((step as f64 + 1.25) / steps as f64);
        let p0 = theme::capsule_point(x, y, width, height, radius, u0);
        let p1 = theme::capsule_point(x, y, width, height, radius, u1);
        let fraction = 1.0 - step as f64 / steps as f64;

        context.set_line_width((1.4 + 1.8 * fraction) * scale);
        theme::set_source_rgba(context, theme::SAPPHIRE, fraction * fraction * 0.95);
        context.move_to(p0.0, p0.1);
        context.line_to(p1.0, p1.1);
        let _ = context.stroke();
    }

    let (head_x, head_y) = theme::capsule_point(x, y, width, height, radius, phase);
    let (tail_x, tail_y) = theme::capsule_point(x, y, width, height, radius, phase - beam_len);
    context.arc(tail_x, tail_y, 0.7 * scale, 0.0, std::f64::consts::TAU);
    theme::set_source_rgba(context, theme::SAPPHIRE, 0.12);
    let _ = context.fill();

    context.arc(head_x, head_y, 2.0 * scale, 0.0, std::f64::consts::TAU);
    theme::set_source_rgba(context, theme::SKY, 0.70);
    let _ = context.fill();
    context.arc(head_x, head_y, 0.9 * scale, 0.0, std::f64::consts::TAU);
    theme::set_source_rgba(context, theme::TEXT, 0.80);
    let _ = context.fill();
}

fn draw_energy_capsule(
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
    let _ = context.fill_preserve();

    context.set_line_width(1.2 * scale);
    theme::set_source_rgba(context, theme::SURFACE1, 0.35);
    let _ = context.stroke();

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
    let (sr, sg, sb) = theme::SAPPHIRE;
    let (lr, lg, lb) = theme::LAVENDER;
    gradient.add_color_stop_rgba(0.0, lr, lg, lb, 0.32);
    gradient.add_color_stop_rgba(0.40, sr, sg, sb, 0.16);
    gradient.add_color_stop_rgba(1.0, sr, sg, sb, 0.0);
    let _ = context.set_source(&gradient);
    let _ = context.paint();
    let _ = context.restore();

    draw_energy_border(context, x, y, width, height, radius, phase);
}

#[derive(Clone)]
pub(crate) struct EnergyIndicator {
    pub(crate) root: gtk::DrawingArea,
    active: Rc<Cell<bool>>,
    phase: Rc<Cell<f64>>,
}

impl EnergyIndicator {
    pub(crate) fn new() -> Self {
        let root = gtk::DrawingArea::new();
        root.set_can_target(false);
        root.set_content_width(28);
        root.set_content_height(28);
        let active = Rc::new(Cell::new(false));
        let phase = Rc::new(Cell::new(0.12));
        let draw_active = Rc::clone(&active);
        let draw_phase = Rc::clone(&phase);
        root.set_draw_func(move |_, context, width, height| {
            if draw_active.get() {
                draw_energy_capsule(
                    context,
                    2.0,
                    2.0,
                    f64::from(width) - 4.0,
                    f64::from(height) - 4.0,
                    4.0,
                    draw_phase.get(),
                );
            }
        });
        Self {
            root,
            active,
            phase,
        }
    }

    pub(crate) fn start(&self) {
        if self.active.replace(true) {
            return;
        }
        self.root.queue_draw();
        if !animations_enabled() {
            return;
        }
        let active = Rc::clone(&self.active);
        let phase = Rc::clone(&self.phase);
        self.root.add_tick_callback(move |indicator, frame_clock| {
            if !active.get() {
                return glib::ControlFlow::Break;
            }
            let seconds = frame_clock.frame_time() as f64 / 1_000_000.0;
            phase.set((seconds / 1.4).fract());
            indicator.queue_draw();
            glib::ControlFlow::Continue
        });
    }

    pub(crate) fn stop(&self) {
        self.active.set(false);
        self.root.queue_draw();
    }
}

pub(crate) struct BuildTimeline {
    pub(crate) root: gtk::Box,
    steps: Vec<gtk::Label>,
    indicator: gtk::DrawingArea,
    current: Rc<Cell<f64>>,
    target: Rc<Cell<f64>>,
    pub(crate) complete: Rc<Cell<bool>>,
    pulse: Rc<Cell<f64>>,
    animating: Rc<Cell<bool>>,
}

impl BuildTimeline {
    pub(crate) fn new(font: &str) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 4);
        root.add_css_class("build-timeline");
        root.set_width_request(440);
        root.set_halign(gtk::Align::Center);
        root.set_valign(gtk::Align::Center);
        root.set_vexpand(false);
        let current = Rc::new(Cell::new(0.0_f64));
        let target = Rc::new(Cell::new(0.0_f64));
        let complete = Rc::new(Cell::new(false));
        let pulse = Rc::new(Cell::new(0.0_f64));
        let indicator = gtk::DrawingArea::new();
        indicator.add_css_class("build-phase-indicator");
        indicator.set_hexpand(true);
        indicator.set_content_width(450);
        indicator.set_content_height(34);
        let draw_position = Rc::clone(&current);
        let draw_target = Rc::clone(&target);
        let draw_complete = Rc::clone(&complete);
        let draw_pulse = Rc::clone(&pulse);
        let draw_font = font.to_owned();
        indicator.set_draw_func(move |_, context, width, _| {
            let width = f64::from(width);
            let w_cap = 116.0_f64;
            let h_cap = 30.0_f64;
            let y_cap = 2.0_f64;
            let center_y = 17.0_f64;
            let w_conn = 46.0_f64;
            let total_caps_width = 3.0 * w_cap + 2.0 * w_conn;
            let offset_x = ((width - total_caps_width) / 2.0).max(5.0);
            let position = draw_position.get().clamp(0.0, 2.0);
            let active = draw_target.get().round().clamp(0.0, 2.0) as usize;
            let is_complete = draw_complete.get();
            let phase = draw_pulse.get();

            let cap_names = ["Evaluate", "Build", "Compare"];

            // 1. Draw inter-phase connecting rails
            for i in 0..2 {
                let conn_start = offset_x + (i as f64 + 1.0) * w_cap + i as f64 * w_conn;
                let conn_end = conn_start + w_conn;

                // Base rail track
                context.set_line_cap(gtk::cairo::LineCap::Butt);
                context.set_line_width(1.8);
                theme::set_source_rgba(context, theme::SURFACE1, 0.40);
                context.move_to(conn_start, center_y);
                context.line_to(conn_end, center_y);
                let _ = context.stroke();

                // Progress fill through connector
                let conn_progress = if is_complete {
                    1.0
                } else {
                    (position - i as f64).clamp(0.0, 1.0)
                };
                if conn_progress > 0.0 {
                    let fill_end = conn_start + w_conn * conn_progress;
                    context.set_line_cap(gtk::cairo::LineCap::Butt);
                    context.set_line_width(1.8);
                    theme::set_source_rgba(context, theme::GREEN, 0.85);
                    context.move_to(conn_start, center_y);
                    context.line_to(fill_end, center_y);
                    let _ = context.stroke();
                }

                // Active traveling energy pulse during transition
                if !is_complete
                    && active == i + 1
                    && position < (i + 1) as f64
                    && conn_progress > 0.01
                    && conn_progress < 0.99
                {
                    let transition_t = (position - i as f64).clamp(0.0, 1.0);
                    let pulse_x = conn_start + w_conn * transition_t;

                    context.arc(pulse_x, center_y, 4.0, 0.0, std::f64::consts::TAU);
                    theme::set_source_rgba(context, theme::SAPPHIRE, 0.30);
                    let _ = context.fill();

                    context.arc(pulse_x, center_y, 2.2, 0.0, std::f64::consts::TAU);
                    theme::set_source_rgba(context, theme::SKY, 0.95);
                    let _ = context.fill();

                    context.arc(pulse_x, center_y, 1.0, 0.0, std::f64::consts::TAU);
                    theme::set_source_rgba(context, theme::TEXT, 1.0);
                    let _ = context.fill();
                }

                // Docking terminal pins at connection points
                let is_start_complete = is_complete || position > i as f64;
                let is_end_complete = is_complete || position >= (i + 1) as f64;

                context.arc(conn_start, center_y, 2.0, 0.0, std::f64::consts::TAU);
                if is_start_complete {
                    theme::set_source_rgba(context, theme::GREEN, 0.85);
                } else {
                    theme::set_source_rgba(context, theme::SURFACE1, 0.50);
                }
                let _ = context.fill();

                context.arc(conn_end, center_y, 2.0, 0.0, std::f64::consts::TAU);
                if is_end_complete {
                    theme::set_source_rgba(context, theme::GREEN, 0.85);
                } else if active == i + 1 && position > i as f64 + 0.85 {
                    theme::set_source_rgba(context, theme::SKY, 0.90);
                } else {
                    theme::set_source_rgba(context, theme::SURFACE1, 0.50);
                }
                let _ = context.fill();
            }

            // 2. Draw three horizontal phase capsules
            for (i, name) in cap_names.iter().enumerate() {
                let cap_x = offset_x + i as f64 * (w_cap + w_conn);
                let is_capsule_complete = is_complete || i < active;
                let is_capsule_active = !is_complete
                    && (i == active)
                    && (active == 0 || position >= active as f64 - 0.15);

                // Select font and calculate text extents
                let font_name = draw_font.clone();
                context.select_font_face(
                    &font_name,
                    gtk::cairo::FontSlant::Normal,
                    gtk::cairo::FontWeight::Bold,
                );
                context.set_font_size(13.0);
                let Ok(ext) = context.text_extents(name) else {
                    continue;
                };

                if is_capsule_complete {
                    // Completed: subtle green background tint + green border + checkmark
                    theme::set_source_rgba(context, theme::GREEN, 0.12);
                    theme::rounded_rectangle(context, cap_x, y_cap, w_cap, h_cap, 5.0);
                    let _ = context.fill_preserve();

                    context.set_line_width(1.5);
                    theme::set_source_rgba(context, theme::GREEN, 0.80);
                    let _ = context.stroke();

                    // Checkmark indicator at left
                    let chk_x = cap_x + 18.0;
                    context.set_line_width(2.0);
                    context.set_line_cap(gtk::cairo::LineCap::Round);
                    context.set_line_join(gtk::cairo::LineJoin::Round);
                    theme::set_source_rgb(context, theme::GREEN);
                    context.move_to(chk_x - 3.2, center_y);
                    context.line_to(chk_x - 0.8, center_y + 2.5);
                    context.line_to(chk_x + 3.6, center_y - 2.8);
                    let _ = context.stroke();

                    // Centered text in remaining width
                    let tx = cap_x + 22.0 + (w_cap - 22.0 - ext.width()) / 2.0 - ext.x_bearing();
                    let ty = y_cap + (h_cap - ext.height()) / 2.0 - ext.y_bearing();
                    theme::set_source_rgb(context, theme::GREEN);
                    context.move_to(tx, ty);
                    let _ = context.show_text(name);
                } else if is_capsule_active {
                    draw_energy_capsule(context, cap_x, y_cap, w_cap, h_cap, 5.0, phase);

                    // 3. Crisp centered label text
                    let tx = cap_x + (w_cap - ext.width()) / 2.0 - ext.x_bearing();
                    let ty = y_cap + (h_cap - ext.height()) / 2.0 - ext.y_bearing();
                    theme::set_source_rgba(context, theme::CRUST, 0.85);
                    context.move_to(tx + 1.0, ty + 1.0);
                    let _ = context.show_text(name);
                    theme::set_source_rgb(context, theme::TEXT);
                    context.move_to(tx, ty);
                    let _ = context.show_text(name);
                } else {
                    // Pending: Dim capsule with muted centered label
                    theme::set_source_rgba(context, theme::MANTLE, 0.40);
                    theme::rounded_rectangle(context, cap_x, y_cap, w_cap, h_cap, 5.0);
                    let _ = context.fill_preserve();

                    context.set_line_width(1.0);
                    theme::set_source_rgba(context, theme::SURFACE1, 0.35);
                    let _ = context.stroke();

                    let tx = cap_x + (w_cap - ext.width()) / 2.0 - ext.x_bearing();
                    let ty = y_cap + (h_cap - ext.height()) / 2.0 - ext.y_bearing();
                    theme::set_source_rgb(context, theme::OVERLAY0);
                    context.move_to(tx, ty);
                    let _ = context.show_text(name);
                }
            }
        });
        root.append(&indicator);

        let mut steps = Vec::new();
        for text in ["Evaluate", "Build", "Compare"] {
            let step = label(text, &["build-phase"], 0.5);
            steps.push(step);
        }
        let timeline = Self {
            root,
            steps,
            indicator,
            current,
            target,
            complete,
            pulse,
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
        self.indicator
            .add_tick_callback(move |indicator, frame_clock| {
                let seconds = frame_clock.frame_time() as f64 / 1_000_000.0;
                pulse.set((seconds / 1.4).fract());
                indicator.queue_draw();
                glib::ControlFlow::Continue
            });
    }

    pub(crate) fn set_complete(&self) {
        self.complete.set(true);
        self.current.set(2.0);
        self.target.set(2.0);
        for step in &self.steps {
            step.remove_css_class("active");
            step.add_css_class("complete");
        }
        self.indicator.queue_draw();
    }

    pub(crate) fn set_phase(&self, phase: BuildPhase) {
        let active = phase.index();
        self.complete.set(false);
        for (index, step) in self.steps.iter().enumerate() {
            step.remove_css_class("complete");
            step.remove_css_class("active");
            if index < active {
                step.add_css_class("complete");
            } else if index == active {
                step.add_css_class("active");
            }
        }
        self.target.set(active as f64);
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
