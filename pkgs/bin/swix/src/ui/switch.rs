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
use crate::{Operation, UiState, label};

#[derive(Clone)]
pub(crate) struct SwitchAnimation {
    pub(crate) root: gtk::Box,
    canvas: gtk::DrawingArea,
    elapsed: gtk::Label,
    phase: Rc<Cell<f64>>,
}

impl SwitchAnimation {
    fn start(&self) {
        self.phase.set(0.0);
        self.elapsed.set_text("0.0s");
        self.root.set_visible(true);
        self.canvas.queue_draw();
    }

    fn update(&self, elapsed: Duration) {
        self.phase.set((elapsed.as_secs_f64() / 1.8).fract());
        let elapsed_text = format!("{:.1}s", elapsed.as_secs_f64());
        if self.elapsed.text().as_str() != elapsed_text.as_str() {
            self.elapsed.set_text(&elapsed_text);
        }
        self.canvas.queue_draw();
    }

    fn hide(&self) {
        self.root.set_visible(false);
    }
}

pub(crate) fn switch_animation(target: &str, flake: &str) -> SwitchAnimation {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.add_css_class("switch-activity");
    root.set_visible(false);

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
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

    let phase = Rc::new(Cell::new(0.0));
    let canvas = gtk::DrawingArea::new();
    canvas.set_content_height(44);
    canvas.set_hexpand(true);
    let draw_phase = Rc::clone(&phase);
    canvas.set_draw_func(move |_, context, width, height| {
        let start = 14.0;
        let end = f64::from(width) - 14.0;
        let center = f64::from(height) / 2.0;
        let distance = (end - start).max(1.0);
        let phase = draw_phase.get();

        context.set_line_width(3.0);
        context.set_source_rgba(0.40, 0.56, 0.68, 0.32);
        context.move_to(start, center);
        context.line_to(end, center);
        let _ = context.stroke();

        for (index, position) in [start, start + distance / 2.0, end].into_iter().enumerate() {
            let radius = if index == 1 {
                7.0 + (phase * std::f64::consts::TAU).sin().abs() * 2.0
            } else {
                7.0
            };
            context.set_source_rgba(
                if index == 0 { 0.62 } else { 0.47 },
                if index == 0 { 0.89 } else { 0.77 },
                if index == 0 { 0.66 } else { 0.83 },
                if index == 2 { 0.58 } else { 0.92 },
            );
            context.arc(position, center, radius, 0.0, std::f64::consts::TAU);
            let _ = context.fill();
        }

        let travel = ((phase - 0.08) / 0.84).clamp(0.0, 1.0);
        let fade = (phase / 0.1).min(1.0) * ((1.0 - phase) / 0.1).min(1.0);
        for trail in 0..4 {
            let trail_phase = travel - f64::from(trail) * 0.035;
            if trail_phase < 0.0 {
                continue;
            }
            let alpha = (0.9 - f64::from(trail) * 0.18) * fade;
            context.set_source_rgba(0.76, 0.89, 1.0, alpha);
            context.arc(
                start + distance * trail_phase,
                center,
                4.5 - f64::from(trail) * 0.65,
                0.0,
                std::f64::consts::TAU,
            );
            let _ = context.fill();
        }
    });
    root.append(&canvas);

    let stages = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    stages.set_homogeneous(true);
    stages.append(&label("Reviewed closure", &["switch-stage", "ready"], 0.0));
    stages.append(&label(
        "Activation signal",
        &["switch-stage", "active"],
        0.5,
    ));
    stages.append(&label("Live system", &["switch-stage", "pending"], 1.0));
    root.append(&stages);

    SwitchAnimation {
        root,
        canvas,
        elapsed,
        phase,
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
    pub(crate) success: gtk::Box,
    pub(crate) error: gtk::Label,
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
    let success = &view.success;
    let error = &view.error;
    let weak_button = button.downgrade();
    let weak_label = label.downgrade();
    let activity = activity.clone();
    let success = success.clone();
    let error = error.clone();
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
        if let Some(back) = state.back_button.borrow().as_ref() {
            back.set_sensitive(false);
        }
        button.set_sensitive(false);
        button.remove_css_class("switch-complete");
        label.set_text("Activating");
        activity.start();
        success.set_visible(false);
        error.set_visible(false);
        error.set_text("");
        let report = report.clone();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = activate(&report, &cancellation);
            let _ = sender.send(result);
        });
        let button = button.clone();
        let label = label.clone();
        let activity = activity.clone();
        let success = success.clone();
        let error = error.clone();
        let started = Instant::now();
        glib::timeout_add_local(Duration::from_millis(16), move || {
            activity.update(started.elapsed());
            match receiver.try_recv() {
                Ok(Ok(())) => {
                    if !state.finish(generation) {
                        return glib::ControlFlow::Break;
                    }
                    activity.hide();
                    label.set_text("Switched");
                    button.remove_css_class("switch-locked");
                    button.add_css_class("switch-complete");
                    success.set_visible(true);
                    if let Some(back) = state.back_button.borrow().as_ref() {
                        back.set_sensitive(true);
                    }
                    glib::ControlFlow::Break
                }
                Ok(Err(message)) => {
                    if !state.finish(generation) {
                        return glib::ControlFlow::Break;
                    }
                    activity.hide();
                    label.set_text("Switch failed");
                    button.remove_css_class("switch-locked");
                    button.set_tooltip_text(Some(&message));
                    button.set_sensitive(true);
                    error.set_text(&message);
                    error.set_visible(true);
                    if let Some(back) = state.back_button.borrow().as_ref() {
                        back.set_sensitive(true);
                    }
                    glib::ControlFlow::Break
                }
                Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(TryRecvError::Disconnected) => {
                    if state.finish(generation) {
                        activity.hide();
                        label.set_text("Switch failed");
                        button.remove_css_class("switch-locked");
                        button.set_sensitive(true);
                        error.set_text("activation worker stopped without a result");
                        error.set_visible(true);
                        if let Some(back) = state.back_button.borrow().as_ref() {
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
