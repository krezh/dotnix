//! The activation card: conduit canvas and morphing status shown while a switch runs.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use swix::protocol::ActivationMode;

use crate::ui::theme;
use crate::ui::{EnergyOverlay, MorphIcon, MorphLabel, animations_enabled};

use super::{COMPLETION_MORPH_MS, mode_text};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SwitchVisualState {
    Idle,
    Activating,
    Complete,
    Error,
}

#[derive(Clone)]
pub(crate) struct SwitchAnimation {
    pub(crate) root: gtk::Box,
    canvas: gtk::DrawingArea,
    icon: MorphIcon,
    title: MorphLabel,
    elapsed: MorphLabel,
    elapsed_energy: EnergyOverlay,
    note: MorphLabel,
    note_reveal: gtk::Revealer,
    phase: Rc<Cell<f64>>,
    active: Rc<Cell<bool>>,
    state: Rc<Cell<SwitchVisualState>>,
    morph: Rc<Cell<f64>>,
    mode: Rc<Cell<ActivationMode>>,
    target: String,
    flake: String,
}

impl SwitchAnimation {
    pub(super) fn start(&self, mode: ActivationMode) {
        let text = mode_text(mode);
        self.mode.set(mode);
        self.state.set(SwitchVisualState::Activating);
        self.phase.set(0.0);
        self.active.set(true);
        self.morph.set(0.0);
        self.icon.set_icon_name("system-run-symbolic");
        self.icon.remove_css_class("switch-success-icon");
        self.icon.remove_css_class("switch-error-icon");
        self.icon.add_css_class("switch-activity-icon");
        self.title.set_text(&match mode {
            ActivationMode::Switch => format!("Activating {} #{}", self.target, self.flake),
            ActivationMode::Boot => format!("Applying {} #{} on reboot", self.target, self.flake),
        });
        self.elapsed.set_text_immediate("0.0s");
        self.elapsed.remove_css_class("complete");
        self.elapsed.remove_css_class("error");
        self.note.set_text(text.note);
        self.note_reveal.set_reveal_child(true);
        self.root.remove_css_class("switch-complete-card");
        self.root.remove_css_class("switch-error-card");
        self.elapsed_energy.start();
        self.canvas.queue_draw();
    }

    pub(super) fn update(&self, elapsed: Duration) {
        let elapsed_text = format!("{:.1}s", elapsed.as_secs_f64());
        if self.elapsed.text().as_str() != elapsed_text.as_str() {
            self.elapsed.set_text_immediate(&elapsed_text);
        }
    }

    pub(super) fn complete(&self, elapsed: Duration) {
        self.state.set(SwitchVisualState::Complete);
        self.root.remove_css_class("switch-error-card");
        self.root.add_css_class("switch-complete-card");
        self.icon.set_icon_name("object-select-symbolic");
        self.icon.remove_css_class("switch-activity-icon");
        self.icon.remove_css_class("switch-error-icon");
        self.icon.add_css_class("switch-success-icon");
        self.title.set_text(mode_text(self.mode.get()).complete_title);
        self.elapsed
            .set_text(&format!("✓ in {:.1}s", elapsed.as_secs_f64()));
        self.elapsed.remove_css_class("error");
        self.elapsed.add_css_class("complete");
        self.elapsed_energy.finish(COMPLETION_MORPH_MS);
        self.note_reveal.set_reveal_child(false);
        self.morph_out();
    }

    pub(super) fn fail(&self, message: &str) {
        self.state.set(SwitchVisualState::Error);
        self.root.remove_css_class("switch-complete-card");
        self.root.add_css_class("switch-error-card");
        self.icon.set_icon_name("dialog-warning-symbolic");
        self.icon.remove_css_class("switch-activity-icon");
        self.icon.remove_css_class("switch-success-icon");
        self.icon.add_css_class("switch-error-icon");
        self.title.set_text(mode_text(self.mode.get()).failed_title);
        self.elapsed.set_text("Failed");
        self.elapsed.remove_css_class("complete");
        self.elapsed.add_css_class("error");
        self.note.set_text(message);
        self.note_reveal.set_reveal_child(true);
        self.elapsed_energy.stop();
        self.morph_out();
    }

    /// Eases `morph` from the running conduit into the current end state, then stops the beam.
    fn morph_out(&self) {
        let morph = Rc::clone(&self.morph);
        let active = Rc::clone(&self.active);
        let canvas = self.canvas.clone();
        if !animations_enabled() {
            morph.set(1.0);
            active.set(false);
            canvas.queue_draw();
            return;
        }
        let state = Rc::clone(&self.state);
        let end_state = state.get();
        let start_time = Rc::new(Cell::new(0_i64));
        self.canvas.add_tick_callback(move |_, frame_clock| {
            // A newer activation took over.
            if state.get() != end_state {
                return glib::ControlFlow::Break;
            }
            let now = frame_clock.frame_time();
            let start = start_time.get();
            if start == 0 {
                start_time.set(now);
                return glib::ControlFlow::Continue;
            }
            let duration = f64::from(COMPLETION_MORPH_MS) * 1_000.0;
            let linear = ((now - start) as f64 / duration).clamp(0.0, 1.0);
            let progress = smootherstep(linear);
            morph.set(progress);
            canvas.queue_draw();
            if linear >= 1.0 {
                morph.set(1.0);
                active.set(false);
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }
}

pub(crate) fn switch_animation(target: &str, flake: &str, font: &str) -> SwitchAnimation {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.add_css_class("switch-activity");
    root.add_css_class("phase-panel");
    root.set_visible(true);

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    heading.add_css_class("switch-activity-header");
    heading.set_valign(gtk::Align::Center);

    let icon = MorphIcon::new("system-run-symbolic", 20, &["switch-activity-icon"]);
    icon.root.set_transition_duration(COMPLETION_MORPH_MS);
    icon.root.set_valign(gtk::Align::Center);
    heading.append(&icon.root);

    let title = MorphLabel::new(
        &format!("Activating {target} #{flake}"),
        &["switch-activity-title"],
        0.0,
    );
    title.root.set_transition_duration(COMPLETION_MORPH_MS);
    title.root.set_hexpand(true);
    heading.append(&title.root);

    let elapsed = MorphLabel::new("0.0s", &["switch-activity-elapsed"], 1.0);
    elapsed.root.set_transition_duration(COMPLETION_MORPH_MS);
    let elapsed_energy = EnergyOverlay::new(&elapsed.root, 5.0);
    heading.append(&elapsed_energy.root);
    root.append(&heading);

    let phase = Rc::new(Cell::new(0.0_f64));
    let active = Rc::new(Cell::new(false));
    let state = Rc::new(Cell::new(SwitchVisualState::Idle));
    let morph = Rc::new(Cell::new(0.0_f64));
    let mode = Rc::new(Cell::new(ActivationMode::Switch));
    let canvas = gtk::DrawingArea::new();
    canvas.add_css_class("switch-activity-conduit");
    canvas.set_content_height(28);
    canvas.set_hexpand(true);
    let draw_phase = Rc::clone(&phase);
    let draw_state = Rc::clone(&state);
    let draw_morph = Rc::clone(&morph);
    let draw_mode = Rc::clone(&mode);
    let draw_font = font.to_owned();
    canvas.set_draw_func(move |_, context, width, height| {
        let width = f64::from(width);
        let height = f64::from(height);
        let center_y = height / 2.0;
        let h_cap = 24.0_f64;
        let r = 5.0_f64;
        let y_cap = center_y - h_cap / 2.0;
        let x_cap = 2.0_f64;
        let w_cap = (width - 2.0 * x_cap).max(0.0);
        let phase = draw_phase.get();
        let morph = draw_morph.get().clamp(0.0, 1.0);
        let text = mode_text(draw_mode.get());
        // A failure fades the still-running conduit out under the error capsule.
        let (morph, error_fade) = if draw_state.get() == SwitchVisualState::Error {
            (0.0, morph)
        } else {
            (morph, 0.0)
        };
        let draw_error = |alpha: f64| {
            theme::set_source_rgba(context, theme::BASE, 0.85 * alpha);
            theme::rounded_rectangle(context, x_cap, y_cap, w_cap, h_cap, r);
            let _ = context.fill_preserve();
            context.set_line_width(1.5);
            theme::set_source_rgba(context, theme::RED, 0.80 * alpha);
            let _ = context.stroke();

            context.select_font_face(
                &draw_font,
                gtk::cairo::FontSlant::Normal,
                gtk::cairo::FontWeight::Bold,
            );
            context.set_font_size(11.0);
            let text = "ACTIVATION ENCOUNTERED AN ERROR";
            if let Ok(ext) = context.text_extents(text) {
                let tx = (width - ext.width()) / 2.0 - ext.x_bearing();
                let ty = y_cap + (h_cap - ext.height()) / 2.0 - ext.y_bearing();
                theme::set_source_rgba(context, theme::RED, alpha);
                context.move_to(tx, ty);
                let _ = context.show_text(text);
            }
        };
        if error_fade >= 1.0 {
            draw_error(1.0);
            return;
        }
        if error_fade > 0.0 {
            context.push_group();
        }

        // 1. Base conduit backdrop
        let bg_alpha = 0.85 + 0.10 * morph;
        theme::set_source_rgba(context, theme::BASE, bg_alpha);
        theme::rounded_rectangle(context, x_cap, y_cap, w_cap, h_cap, r);
        let _ = context.fill_preserve();
        context.set_line_width(1.2);
        theme::set_source_rgba(context, theme::SURFACE1, 0.35);
        let _ = context.stroke();

        // 2. Interior spotlight (interpolates from circulating head to full ambient glow)
        let (head_x, head_y) = theme::capsule_point(x_cap, y_cap, w_cap, h_cap, r, phase);
        let spot_x = head_x + (x_cap + w_cap / 2.0 - head_x) * morph;
        let spot_y = head_y + (center_y - head_y) * morph;
        let spot_radius = 60.0 + (w_cap / 2.0 - 60.0).max(0.0) * morph;

        let _ = context.save();
        theme::rounded_rectangle(
            context,
            x_cap + 1.0,
            y_cap + 1.0,
            w_cap - 2.0,
            h_cap - 2.0,
            r - 1.0,
        );
        context.clip();

        let gradient =
            gtk::cairo::RadialGradient::new(spot_x, spot_y, 0.0, spot_x, spot_y, spot_radius);
        let (tr, tg, tb) = theme::TEAL.rgb();
        let (sr, sg, sb) = theme::SKY.rgb();
        let (gr, gg, gb) = theme::GREEN.rgb();
        let r_mid = tr + (gr - tr) * morph;
        let g_mid = tg + (gg - tg) * morph;
        let b_mid = tb + (gb - tb) * morph;
        gradient.add_color_stop_rgba(
            0.0,
            sr + (gr - sr) * morph,
            sg + (gg - sg) * morph,
            sb + (gb - sb) * morph,
            0.30 * (1.0 - morph) + 0.20 * morph,
        );
        gradient.add_color_stop_rgba(
            0.50,
            r_mid,
            g_mid,
            b_mid,
            0.16 * (1.0 - morph) + 0.08 * morph,
        );
        gradient.add_color_stop_rgba(1.0, r_mid, g_mid, b_mid, 0.0);
        let _ = context.set_source(&gradient);
        let _ = context.paint();
        let _ = context.restore();

        let moving_alpha = 1.0 - morph;
        if morph > 0.0 {
            context.set_line_width(1.5);
            theme::set_source_rgba(context, theme::GREEN, 0.62 * morph);
            theme::rounded_rectangle(context, x_cap, y_cap, w_cap, h_cap, r);
            let _ = context.stroke();
        }

        if moving_alpha > 0.0 {
            let beam_len = 0.30_f64;
            let steps = 192;
            let color = (
                tr + (gr - tr) * morph,
                tg + (gg - tg) * morph,
                tb + (gb - tb) * morph,
            );
            context.set_line_cap(gtk::cairo::LineCap::Round);
            for step in 0..steps {
                let distance = beam_len * step as f64 / steps as f64;
                let fraction = 1.0 - step as f64 / steps as f64;
                let p0 = theme::capsule_point(x_cap, y_cap, w_cap, h_cap, r, phase - distance);
                let p1 = theme::capsule_point(
                    x_cap,
                    y_cap,
                    w_cap,
                    h_cap,
                    r,
                    phase - beam_len * (step as f64 + 1.1) / steps as f64,
                );
                context.set_line_width(1.4 + 1.8 * fraction);
                let (cr, cg, cb) = color;
                context.set_source_rgba(cr, cg, cb, fraction * fraction * 0.95 * moving_alpha);
                context.move_to(p0.0, p0.1);
                context.line_to(p1.0, p1.1);
                let _ = context.stroke();
            }

            context.arc(head_x, head_y, 2.0, 0.0, std::f64::consts::TAU);
            theme::set_source_rgba(context, theme::SKY, 0.75 * moving_alpha);
            let _ = context.fill();
            context.arc(head_x, head_y, 0.9, 0.0, std::f64::consts::TAU);
            theme::set_source_rgba(context, theme::TEXT, 0.85 * moving_alpha);
            let _ = context.fill();
        }

        context.select_font_face(
            &draw_font,
            gtk::cairo::FontSlant::Normal,
            gtk::cairo::FontWeight::Bold,
        );

        if morph > 0.0 {
            context.set_font_size(11.0);
            let text = text.conduit_done;
            if let Ok(ext) = context.text_extents(text) {
                let tx = (width - ext.width()) / 2.0 - ext.x_bearing();
                let ty = y_cap + (h_cap - ext.height()) / 2.0 - ext.y_bearing();
                theme::set_source_rgba(context, theme::GREEN, morph);
                context.move_to(tx, ty);
                let _ = context.show_text(text);
            }
        }

        if morph < 1.0 {
            context.set_font_size(11.0);
            let text = text.conduit_running;
            if let Ok(ext) = context.text_extents(text) {
                let tx = (width - ext.width()) / 2.0 - ext.x_bearing();
                let ty = y_cap + (h_cap - ext.height()) / 2.0 - ext.y_bearing();
                let alpha = (1.0 - morph).max(0.0);
                theme::set_source_rgba(context, theme::CRUST, 0.85 * alpha);
                context.move_to(tx + 1.0, ty + 1.0);
                let _ = context.show_text(text);
                theme::set_source_rgba(context, theme::TEAL, alpha);
                context.move_to(tx, ty);
                let _ = context.show_text(text);
            }
        }

        if error_fade > 0.0 {
            let _ = context.pop_group_to_source();
            let _ = context.paint_with_alpha(1.0 - error_fade);
            draw_error(error_fade);
        }
    });

    if animations_enabled() {
        let tick_phase = Rc::clone(&phase);
        let tick_active = Rc::clone(&active);
        canvas.add_tick_callback(move |canvas, frame_clock| {
            if tick_active.get() {
                let seconds = frame_clock.frame_time() as f64 / 1_000_000.0;
                tick_phase.set((seconds / 2.0).fract());
                canvas.queue_draw();
            }
            glib::ControlFlow::Continue
        });
    }
    root.append(&canvas);

    let note = MorphLabel::new(
        mode_text(ActivationMode::Switch).note,
        &["switch-activity-note"],
        0.0,
    );
    let note_reveal = gtk::Revealer::new();
    note_reveal.set_transition_type(gtk::RevealerTransitionType::Crossfade);
    note_reveal.set_transition_duration(COMPLETION_MORPH_MS);
    note_reveal.set_child(Some(&note.root));
    root.append(&note_reveal);

    SwitchAnimation {
        root,
        canvas,
        icon,
        title,
        elapsed,
        elapsed_energy,
        note,
        note_reveal,
        phase,
        active,
        state,
        morph,
        mode,
        target: target.to_owned(),
        flake: flake.to_owned(),
    }
}

pub(crate) fn smootherstep(t: f64) -> f64 {
    let c = t.clamp(0.0, 1.0);
    c * c * c * (c * (c * 6.0 - 15.0) + 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smootherstep_boundaries_and_monotonicity() {
        assert_eq!(smootherstep(0.0), 0.0);
        assert_eq!(smootherstep(1.0), 1.0);
        assert_eq!(smootherstep(-0.5), 0.0);
        assert_eq!(smootherstep(1.5), 1.0);
        assert_eq!(smootherstep(0.5), 0.5);

        let mut prev = 0.0;
        for i in 1..=100 {
            let t = i as f64 / 100.0;
            let val = smootherstep(t);
            assert!(val >= prev);
            prev = val;
        }
    }
}
