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
use crate::theme;
use crate::{Operation, UiState, label};

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
    icon: gtk::Image,
    title: gtk::Label,
    elapsed: gtk::Label,
    note: gtk::Label,
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
        self.icon.set_icon_name(Some("system-run-symbolic"));
        self.icon.remove_css_class("switch-success-icon");
        self.icon.remove_css_class("switch-error-icon");
        self.icon.add_css_class("switch-activity-icon");
        self.title.set_text(&format!("Activating {} #{}", self.target, self.flake));
        self.elapsed.set_text("0.0s");
        self.elapsed.remove_css_class("complete");
        self.elapsed.remove_css_class("error");
        self.note.set_text("Applying configuration and running activation scripts...");
        self.root.remove_css_class("switch-complete-card");
        self.root.remove_css_class("switch-error-card");
        self.root.set_visible(true);
        self.canvas.queue_draw();
    }

    fn update(&self, elapsed: Duration) {
        let elapsed_text = format!("{:.1}s", elapsed.as_secs_f64());
        if self.elapsed.text().as_str() != elapsed_text.as_str() {
            self.elapsed.set_text(&elapsed_text);
        }
    }

    fn complete(&self, elapsed: Duration) {
        self.state.set(SwitchVisualState::Complete);
        self.root.remove_css_class("switch-error-card");
        self.root.add_css_class("switch-complete-card");
        self.icon.set_icon_name(Some("object-select-symbolic"));
        self.icon.remove_css_class("switch-activity-icon");
        self.icon.remove_css_class("switch-error-icon");
        self.icon.add_css_class("switch-success-icon");
        self.title.set_text("Configuration Activated");
        self.elapsed.set_text(&format!("✓ in {:.1}s", elapsed.as_secs_f64()));
        self.elapsed.remove_css_class("error");
        self.elapsed.add_css_class("complete");
        self.note.set_visible(false);
        let morph = Rc::clone(&self.morph);
        let canvas = self.canvas.clone();
        let start_time = Rc::new(Cell::new(0_i64));
        self.canvas.add_tick_callback(move |_, frame_clock| {
            let now = frame_clock.frame_time();
            let start = start_time.get();
            if start == 0 {
                start_time.set(now);
                return glib::ControlFlow::Continue;
            }
            let progress = ((now - start) as f64 / 350_000.0).clamp(0.0, 1.0);
            morph.set(progress);
            canvas.queue_draw();
            if progress >= 1.0 {
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
        self.icon.set_icon_name(Some("dialog-warning-symbolic"));
        self.icon.remove_css_class("switch-activity-icon");
        self.icon.remove_css_class("switch-success-icon");
        self.icon.add_css_class("switch-error-icon");
        self.title.set_text("Switch Failed");
        self.elapsed.set_text("Failed");
        self.elapsed.remove_css_class("complete");
        self.elapsed.add_css_class("error");
        self.note.set_text(message);
        self.note.set_visible(true);
        self.canvas.queue_draw();
    }
}

pub(crate) fn switch_animation(target: &str, flake: &str, font: &str) -> SwitchAnimation {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.add_css_class("switch-activity");
    root.set_visible(false);

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    heading.add_css_class("switch-activity-header");
    heading.set_valign(gtk::Align::Center);

    let icon = gtk::Image::from_icon_name("system-run-symbolic");
    icon.add_css_class("switch-activity-icon");
    icon.set_pixel_size(20);
    icon.set_valign(gtk::Align::Center);
    heading.append(&icon);

    let title = label(
        &format!("Activating {target} #{flake}"),
        &["switch-activity-title"],
        0.0,
    );
    title.set_hexpand(true);
    heading.append(&title);

    let elapsed = label("0.0s", &["switch-activity-elapsed"], 1.0);
    heading.append(&elapsed);
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
        let r = 12.0_f64;
        let y_cap = center_y - h_cap / 2.0;
        let phase = draw_phase.get();
        let current_state = draw_state.get();
        let morph = draw_morph.get().clamp(0.0, 1.0);

        if current_state == SwitchVisualState::Error {
            theme::set_source_rgba(context, theme::BASE, 0.85);
            theme::rounded_rectangle(context, 0.0, y_cap, width, h_cap, r);
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
        theme::rounded_rectangle(context, 0.0, y_cap, width, h_cap, r);
        let _ = context.fill_preserve();
        context.set_line_width(1.2);
        theme::set_source_rgba(context, theme::SURFACE1, 0.35);
        let _ = context.stroke();

        // 2. Interior spotlight (interpolates from circulating head to full ambient glow)
        let (head_x, head_y) = theme::capsule_point(0.0, y_cap, width, h_cap, r, phase);
        let spot_x = head_x + (width / 2.0 - head_x) * morph;
        let spot_y = head_y + (center_y - head_y) * morph;
        let spot_radius = 60.0 + (width / 2.0 - 60.0).max(0.0) * morph;

        let _ = context.save();
        theme::rounded_rectangle(context, 1.0, y_cap + 1.0, width - 2.0, h_cap - 2.0, r - 1.0);
        context.clip();

        let gradient = gtk::cairo::RadialGradient::new(spot_x, spot_y, 0.0, spot_x, spot_y, spot_radius);
        let (tr, tg, tb) = theme::TEAL;
        let (sr, sg, sb) = theme::SKY;
        let (gr, gg, gb) = theme::GREEN;
        let r_mid = tr + (gr - tr) * morph;
        let g_mid = tg + (gg - tg) * morph;
        let b_mid = tb + (gb - tb) * morph;
        gradient.add_color_stop_rgba(0.0, sr + (gr - sr) * morph, sg + (gg - sg) * morph, sb + (gb - sb) * morph, 0.30 * (1.0 - morph) + 0.20 * morph);
        gradient.add_color_stop_rgba(0.50, r_mid, g_mid, b_mid, 0.16 * (1.0 - morph) + 0.08 * morph);
        gradient.add_color_stop_rgba(1.0, r_mid, g_mid, b_mid, 0.0);
        let _ = context.set_source(&gradient);
        let _ = context.paint();
        let _ = context.restore();

        // 3. Circulating beam morphs into full radiant border ring:
        if morph >= 1.0 {
            context.set_line_width(1.8);
            theme::set_source_rgba(context, theme::GREEN, 0.85);
            theme::rounded_rectangle(context, 0.0, y_cap, width, h_cap, r);
            let _ = context.stroke();
        } else {
            let beam_len = 0.30_f64 + 0.70_f64 * morph;
            let steps = 64;
            context.set_line_cap(gtk::cairo::LineCap::Butt);
            for s in 0..steps {
                let u0 = phase - beam_len * (s as f64 / steps as f64);
                let u1 = phase - beam_len * ((s as f64 + 1.25) / steps as f64);
                let p0 = theme::capsule_point(0.0, y_cap, width, h_cap, r, u0);
                let p1 = theme::capsule_point(0.0, y_cap, width, h_cap, r, u1);

                let frac = 1.0 - (s as f64 / steps as f64);
                let alpha = (frac * frac * 0.95) * (1.0 - morph) + 0.85 * morph;
                let w = (1.4 + 1.8 * frac) * (1.0 - morph) + 1.8 * morph;

                context.set_line_width(w);
                let col = (
                    tr + (gr - tr) * morph,
                    tg + (gg - tg) * morph,
                    tb + (gb - tb) * morph,
                );
                theme::set_source_rgba(context, col, alpha);
                context.move_to(p0.0, p0.1);
                context.line_to(p1.0, p1.1);
                let _ = context.stroke();
            }

            if morph < 0.95 {
                let tail_u = phase - beam_len;
                let (tail_x, tail_y) = theme::capsule_point(0.0, y_cap, width, h_cap, r, tail_u);
                context.arc(tail_x, tail_y, 0.7, 0.0, std::f64::consts::TAU);
                theme::set_source_rgba(context, theme::TEAL, 0.12 * (1.0 - morph));
                let _ = context.fill();

                context.arc(head_x, head_y, 2.0, 0.0, std::f64::consts::TAU);
                theme::set_source_rgba(context, theme::SKY, 0.75 * (1.0 - morph));
                let _ = context.fill();
                context.arc(head_x, head_y, 0.9, 0.0, std::f64::consts::TAU);
                theme::set_source_rgba(context, theme::TEXT, 0.85 * (1.0 - morph));
                let _ = context.fill();
            }
        }

        // 4. Conduit text:
        context.select_font_face(
            &draw_font,
            gtk::cairo::FontSlant::Normal,
            gtk::cairo::FontWeight::Bold,
        );

        if morph > 0.0 {
            context.set_font_size(11.5);
            let active_text = "LIVE SYSTEM ACTIVE";
            if let Ok(ext) = context.text_extents(active_text) {
                let chk_w = 11.0_f64;
                let gap = 7.0_f64;
                let total_w = chk_w + gap + ext.width();
                let start_x = (width - total_w) / 2.0;
                let chk_center = start_x + chk_w / 2.0;

                context.set_line_width(2.0);
                context.set_line_cap(gtk::cairo::LineCap::Round);
                context.set_line_join(gtk::cairo::LineJoin::Round);

                theme::set_source_rgba(context, theme::CRUST, 0.85 * morph);
                context.move_to(chk_center - 3.8, center_y + 0.8);
                context.line_to(chk_center - 1.0, center_y + 3.8);
                context.line_to(chk_center + 4.2, center_y - 3.2);
                let _ = context.stroke();

                theme::set_source_rgba(context, theme::GREEN, morph);
                context.move_to(chk_center - 3.8, center_y - 0.2);
                context.line_to(chk_center - 1.0, center_y + 2.8);
                context.line_to(chk_center + 4.2, center_y - 4.2);
                let _ = context.stroke();

                let tx = start_x + chk_w + gap - ext.x_bearing();
                let ty = y_cap + (h_cap - ext.height()) / 2.0 - ext.y_bearing();
                theme::set_source_rgba(context, theme::CRUST, 0.85 * morph);
                context.move_to(tx + 1.0, ty + 1.0);
                let _ = context.show_text(active_text);
                theme::set_source_rgba(context, theme::GREEN, morph);
                context.move_to(tx, ty);
                let _ = context.show_text(active_text);
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

    let tick_phase = Rc::clone(&phase);
    let tick_active = Rc::clone(&active);
    canvas.add_tick_callback(move |canvas, frame_clock| {
        if !tick_active.get() {
            return glib::ControlFlow::Continue;
        }
        let seconds = frame_clock.frame_time() as f64 / 1_000_000.0;
        tick_phase.set((seconds / 2.0).fract());
        canvas.queue_draw();
        glib::ControlFlow::Continue
    });
    root.append(&canvas);

    let note = label(
        "Applying configuration and running activation scripts...",
        &["switch-activity-note"],
        0.0,
    );
    root.append(&note);

    SwitchAnimation {
        root,
        canvas,
        icon,
        title,
        elapsed,
        note,
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
    label: glib::WeakRef<gtk::Label>,
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
    pub(crate) label: gtk::Label,
    pub(crate) activity: SwitchAnimation,
    pub(crate) summary: gtk::Box,
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
    let summary = &view.summary;
    let weak_button = button.downgrade();
    let weak_label = label.downgrade();
    let activity = activity.clone();
    let summary = summary.clone();
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
        activity.start();
        summary.set_visible(false);
        let switch_report = report.clone();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = activate(&switch_report, &cancellation);
            let _ = sender.send(result);
        });
        let button = button.clone();
        let label = label.clone();
        let activity = activity.clone();
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
