use std::cell::Cell;
use std::fs;
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use gtk::glib;
use gtk::prelude::*;

use crate::activation::activate;
use crate::build::Target;
use crate::report::Report;
use crate::state::{Operation, UiState};
use crate::theme;
use crate::ui::common::{MorphIcon, MorphLabel, WeakMorphLabel, animations_enabled};
use crate::ui::timeline::EnergyOverlay;

pub(crate) const COMPLETION_MORPH_MS: u32 = 700;

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
    target: String,
    flake: String,
}

impl SwitchAnimation {
    fn start(&self) {
        self.state.set(SwitchVisualState::Activating);
        self.phase.set(0.0);
        self.active.set(true);
        self.morph.set(0.0);
        self.icon.set_icon_name("system-run-symbolic");
        self.icon.remove_css_class("switch-success-icon");
        self.icon.remove_css_class("switch-error-icon");
        self.icon.add_css_class("switch-activity-icon");
        self.title
            .set_text(&format!("Activating {} #{}", self.target, self.flake));
        self.elapsed.set_text_immediate("0.0s");
        self.elapsed.remove_css_class("complete");
        self.elapsed.remove_css_class("error");
        self.note
            .set_text("Applying configuration and running activation scripts...");
        self.note_reveal.set_reveal_child(true);
        self.root.remove_css_class("switch-complete-card");
        self.root.remove_css_class("switch-error-card");
        self.elapsed_energy.start();
        self.canvas.queue_draw();
    }

    fn update(&self, elapsed: Duration) {
        let elapsed_text = format!("{:.1}s", elapsed.as_secs_f64());
        if self.elapsed.text().as_str() != elapsed_text.as_str() {
            self.elapsed.set_text_immediate(&elapsed_text);
        }
    }

    fn complete(&self, elapsed: Duration) {
        self.state.set(SwitchVisualState::Complete);
        self.root.remove_css_class("switch-error-card");
        self.root.add_css_class("switch-complete-card");
        self.icon.set_icon_name("object-select-symbolic");
        self.icon.remove_css_class("switch-activity-icon");
        self.icon.remove_css_class("switch-error-icon");
        self.icon.add_css_class("switch-success-icon");
        self.title.set_text("Configuration Activated");
        self.elapsed
            .set_text(&format!("✓ in {:.1}s", elapsed.as_secs_f64()));
        self.elapsed.remove_css_class("error");
        self.elapsed.add_css_class("complete");
        self.elapsed_energy.finish(COMPLETION_MORPH_MS);
        self.note_reveal.set_reveal_child(false);
        let morph = Rc::clone(&self.morph);
        let active = Rc::clone(&self.active);
        let canvas = self.canvas.clone();
        if !animations_enabled() {
            morph.set(1.0);
            active.set(false);
            canvas.queue_draw();
            return;
        }
        let start_time = Rc::new(Cell::new(0_i64));
        self.canvas.add_tick_callback(move |_, frame_clock| {
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

    fn fail(&self, message: &str) {
        self.state.set(SwitchVisualState::Error);
        self.active.set(false);
        self.root.remove_css_class("switch-complete-card");
        self.root.add_css_class("switch-error-card");
        self.icon.set_icon_name("dialog-warning-symbolic");
        self.icon.remove_css_class("switch-activity-icon");
        self.icon.remove_css_class("switch-success-icon");
        self.icon.add_css_class("switch-error-icon");
        self.title.set_text("Switch Failed");
        self.elapsed.set_text("Failed");
        self.elapsed.remove_css_class("complete");
        self.elapsed.add_css_class("error");
        self.note.set_text(message);
        self.note_reveal.set_reveal_child(true);
        self.elapsed_energy.stop();
        self.canvas.queue_draw();
    }
}

pub(crate) fn switch_animation(target: &str, flake: &str, font: &str) -> SwitchAnimation {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.add_css_class("switch-activity");
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
    let canvas = gtk::DrawingArea::new();
    canvas.add_css_class("switch-activity-conduit");
    canvas.set_content_height(28);
    canvas.set_hexpand(true);
    let draw_phase = Rc::clone(&phase);
    let draw_state = Rc::clone(&state);
    let draw_morph = Rc::clone(&morph);
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
        let current_state = draw_state.get();
        let morph = draw_morph.get().clamp(0.0, 1.0);

        if current_state == SwitchVisualState::Error {
            theme::set_source_rgba(context, theme::BASE, 0.85);
            theme::rounded_rectangle(context, x_cap, y_cap, w_cap, h_cap, r);
            let _ = context.fill_preserve();
            context.set_line_width(1.5);
            theme::set_source_rgba(context, theme::RED, 0.80);
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
                theme::set_source_rgb(context, theme::RED);
                context.move_to(tx, ty);
                let _ = context.show_text(text);
            }
            return;
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
        let (tr, tg, tb) = theme::TEAL;
        let (sr, sg, sb) = theme::SKY;
        let (gr, gg, gb) = theme::GREEN;
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
                theme::set_source_rgba(context, color, fraction * fraction * 0.95 * moving_alpha);
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
            let text = "SYSTEM ACTIVE";
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
            let text = "ACTIVATION IN PROGRESS";
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
        "Applying configuration and running activation scripts...",
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
        target: target.to_owned(),
        flake: flake.to_owned(),
    }
}

pub(crate) fn current_hostname() -> Option<String> {
    let hostname = fs::read_to_string("/proc/sys/kernel/hostname").ok()?;
    let hostname = hostname.trim();
    (!hostname.is_empty()).then(|| hostname.to_owned())
}

pub(crate) fn requires_host_switch_confirmation(
    target: Target,
    flake: &str,
    current_hostname: Option<&str>,
) -> bool {
    target == Target::NixOs
        && current_hostname.is_none_or(|hostname| {
            !hostname
                .split('.')
                .next()
                .is_some_and(|hostname| hostname.eq_ignore_ascii_case(flake))
        })
}

pub(crate) fn switch_confirmation_allows_activation(
    requires_confirmation: bool,
    confirmed: &Cell<bool>,
) -> bool {
    if requires_confirmation && !confirmed.replace(true) {
        return false;
    }
    confirmed.set(false);
    true
}

#[derive(Clone)]
pub(crate) struct SwitchConfirmation {
    button: glib::WeakRef<gtk::Button>,
    label: WeakMorphLabel,
    requires_confirmation: bool,
    confirmed: Rc<Cell<bool>>,
    key_down: Rc<Cell<bool>>,
    activate: Rc<dyn Fn() -> bool>,
}

impl SwitchConfirmation {
    fn activate(&self) {
        let Some(button) = self.button.upgrade() else {
            return;
        };
        let Some(label) = self.label.upgrade() else {
            return;
        };
        if !button.is_sensitive() {
            return;
        }
        if !switch_confirmation_allows_activation(self.requires_confirmation, &self.confirmed) {
            label.set_text("Are you sure?");
            button.add_css_class("switch-confirming");
            button.set_tooltip_text(Some("Click again to activate this configuration"));
            return;
        }

        if self.requires_confirmation {
            button.remove_css_class("switch-confirming");
            button.add_css_class("switch-locked");
        }
        if !(self.activate)() {
            button.remove_css_class("switch-locked");
            button.set_sensitive(true);
            label.set_text("Switch");
        }
    }

    pub(crate) fn key_press(&self) {
        if !self.key_down.replace(true) {
            self.activate();
        }
    }

    pub(crate) fn key_release(&self) {
        self.key_down.set(false);
    }

    pub(crate) fn reset(&self) {
        self.confirmed.set(false);
        self.key_down.set(false);
        if let Some(button) = self.button.upgrade() {
            button.remove_css_class("switch-confirming");
            if self.requires_confirmation {
                button.set_tooltip_text(Some(
                    "This configuration targets another host; click twice to activate it",
                ));
            }
        }
        if let Some(label) = self.label.upgrade() {
            label.set_text("Switch");
        }
    }
}

pub(crate) struct SwitchView {
    pub(crate) button: gtk::Button,
    pub(crate) label: MorphLabel,
    pub(crate) activity: SwitchAnimation,
    pub(crate) surface: gtk::Stack,
    pub(crate) button_energy: EnergyOverlay,
}

pub(crate) fn connect_switch(
    view: &SwitchView,
    report: Report,
    state: Rc<UiState>,
    requires_confirmation: bool,
) -> SwitchConfirmation {
    let button = &view.button;
    let label = &view.label;
    let activity = &view.activity;
    let surface = &view.surface;
    let button_energy = &view.button_energy;
    let weak_button = button.downgrade();
    let weak_label = label.downgrade();
    let activity = activity.clone();
    let surface = surface.clone();
    let button_energy = button_energy.clone();
    let weak_state = Rc::downgrade(&state);
    let activate = Rc::new(move || {
        let Some(state) = weak_state.upgrade() else {
            return false;
        };
        let Some(button) = weak_button.upgrade() else {
            return false;
        };
        let Some(label) = weak_label.upgrade() else {
            return false;
        };
        let Some((generation, cancellation)) = state.begin(Operation::Switching) else {
            return false;
        };
        let back = state.back_button.borrow().clone();
        if let Some(back) = back {
            back.set_sensitive(false);
        }
        button.set_sensitive(false);
        button.remove_css_class("switch-complete");
        label.set_text("Activating");
        button_energy.start();
        activity.start();
        surface.set_visible_child(&activity.root);
        let switch_report = report.clone();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = activate(&switch_report, &cancellation);
            let _ = sender.send(result);
        });
        let button = button.clone();
        let label = label.clone();
        let activity = activity.clone();
        let button_energy = button_energy.clone();
        // no separate success/error boxes
        let started = Instant::now();
        glib::timeout_add_local(Duration::from_millis(100), move || {
            activity.update(started.elapsed());
            match receiver.try_recv() {
                Ok(Ok(())) => {
                    if !state.finish(generation) {
                        return glib::ControlFlow::Break;
                    }
                    activity.complete(started.elapsed());
                    button_energy.finish(COMPLETION_MORPH_MS);
                    label.set_text("Switched");
                    button.remove_css_class("switch-locked");
                    button.add_css_class("switch-complete");
                    let back = state.back_button.borrow().clone();
                    if let Some(back) = back {
                        back.set_sensitive(true);
                    }
                    glib::ControlFlow::Break
                }
                Ok(Err(message)) => {
                    if !state.finish(generation) {
                        return glib::ControlFlow::Break;
                    }
                    activity.fail(&message);
                    button_energy.stop();
                    label.set_text("Switch failed");
                    button.remove_css_class("switch-locked");
                    button.set_tooltip_text(Some(&message));
                    button.set_sensitive(true);
                    let back = state.back_button.borrow().clone();
                    if let Some(back) = back {
                        back.set_sensitive(true);
                    }
                    glib::ControlFlow::Break
                }
                Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(TryRecvError::Disconnected) => {
                    if state.finish(generation) {
                        activity.fail("activation worker stopped without a result");
                        button_energy.stop();
                        label.set_text("Switch failed");
                        button.remove_css_class("switch-locked");
                        button.set_sensitive(true);
                        let back = state.back_button.borrow().clone();
                        if let Some(back) = back {
                            back.set_sensitive(true);
                        }
                    }
                    glib::ControlFlow::Break
                }
            }
        });
        true
    });

    let confirmation = SwitchConfirmation {
        button: button.downgrade(),
        label: label.downgrade(),
        requires_confirmation,
        confirmed: Rc::new(Cell::new(false)),
        key_down: Rc::new(Cell::new(false)),
        activate,
    };
    let clicked_confirmation = confirmation.clone();
    button.connect_clicked(move |_| clicked_confirmation.activate());

    confirmation
}

fn smootherstep(t: f64) -> f64 {
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

    #[test]
    fn only_cross_host_nixos_switches_require_confirmation() {
        assert!(!requires_host_switch_confirmation(
            Target::NixOs,
            "thor",
            Some("thor")
        ));
        assert!(!requires_host_switch_confirmation(
            Target::NixOs,
            "thor",
            Some("thor.example")
        ));
        assert!(requires_host_switch_confirmation(
            Target::NixOs,
            "odin",
            Some("thor")
        ));
        assert!(requires_host_switch_confirmation(
            Target::NixOs,
            "odin",
            None
        ));
        assert!(!requires_host_switch_confirmation(
            Target::HomeManager,
            "alice",
            Some("thor")
        ));
    }

    #[test]
    fn cross_host_switch_requires_two_actions() {
        let confirmed = Cell::new(false);
        assert!(!switch_confirmation_allows_activation(true, &confirmed));
        assert!(confirmed.get());
        assert!(switch_confirmation_allows_activation(true, &confirmed));
        assert!(!confirmed.get());
        assert!(switch_confirmation_allows_activation(false, &confirmed));
    }
}
