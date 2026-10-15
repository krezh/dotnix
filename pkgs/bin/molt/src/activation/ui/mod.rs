mod animation;

use std::cell::Cell;
use std::fs;
use std::rc::Rc;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use gtk::glib;
use gtk::prelude::*;
use molt::protocol::ActivationMode;

use crate::activation::activate;
use crate::app::mini::{MiniEnd, MiniStatus};
use crate::build::Target;
use crate::report::Report;
use crate::app::state::{Operation, UiState};
use crate::ui::{EnergyOverlay, MorphLabel, WeakMorphLabel};

pub(crate) use animation::{SwitchAnimation, smootherstep, switch_animation};

pub(crate) const COMPLETION_MORPH_MS: u32 = 700;

/// User-facing wording for one activation mode.
#[derive(Clone, Copy)]
struct ModeText {
    idle: &'static str,
    /// The minimized bar's step capsule while running.
    step: &'static str,
    running: &'static str,
    done: &'static str,
    failed: &'static str,
    tooltip: &'static str,
    cross_host_tooltip: &'static str,
    note: &'static str,
    complete_title: &'static str,
    failed_title: &'static str,
    conduit_running: &'static str,
    conduit_done: &'static str,
}

const fn mode_text(mode: ActivationMode) -> ModeText {
    match mode {
        ActivationMode::Switch => ModeText {
            idle: "Switch now",
            step: "Switch",
            running: "Activating",
            done: "Switched",
            failed: "Switch failed",
            tooltip: "Activate this configuration now",
            cross_host_tooltip: "This configuration targets another host; click twice to activate it",
            note: "Applying configuration and running activation scripts...",
            complete_title: "Configuration Activated",
            failed_title: "Switch Failed",
            conduit_running: "ACTIVATION IN PROGRESS",
            conduit_done: "SYSTEM ACTIVE",
        },
        ActivationMode::Boot => ModeText {
            idle: "Apply on reboot",
            step: "Boot",
            running: "Applying",
            done: "Applied",
            failed: "Apply failed",
            tooltip: "Make this configuration the boot default without activating it now",
            cross_host_tooltip: "This configuration targets another host; click twice to apply it on reboot",
            note: "Installing the boot entry; the running system is left untouched...",
            complete_title: "Applied — active after the next reboot",
            failed_title: "Apply Failed",
            conduit_running: "INSTALLING BOOT ENTRY",
            conduit_done: "ACTIVE AFTER REBOOT",
        },
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
    mode: ActivationMode,
    requires_confirmation: bool,
    confirmed: Rc<Cell<bool>>,
    key_down: Rc<Cell<bool>>,
    activate: Rc<dyn Fn() -> bool>,
}

impl SwitchConfirmation {
    /// Acts like a click: arms the cross-host confirmation instead when one is required.
    pub(crate) fn activate(&self) {
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
            label.set_text(mode_text(self.mode).idle);
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
        let text = mode_text(self.mode);
        self.confirmed.set(false);
        self.key_down.set(false);
        if let Some(button) = self.button.upgrade() {
            button.remove_css_class("switch-confirming");
            if self.requires_confirmation {
                button.set_tooltip_text(Some(text.cross_host_tooltip));
            }
        }
        if let Some(label) = self.label.upgrade() {
            label.set_text(text.idle);
        }
    }
}

/// A footer button that activates the reviewed configuration in one `ActivationMode`.
pub(crate) struct SwitchAction {
    pub(crate) button: gtk::Button,
    pub(crate) energy: EnergyOverlay,
    label: MorphLabel,
    mode: ActivationMode,
}

pub(crate) fn switch_action(mode: ActivationMode, requires_confirmation: bool) -> SwitchAction {
    let text = mode_text(mode);
    let button = gtk::Button::new();
    button.add_css_class("switch-button");
    button.add_css_class(if mode == ActivationMode::Boot {
        "surface-button"
    } else {
        "accent-button"
    });
    if requires_confirmation {
        button.add_css_class("switch-danger");
        button.set_tooltip_text(Some(text.cross_host_tooltip));
    } else {
        button.set_tooltip_text(Some(text.tooltip));
    }
    button.set_valign(gtk::Align::Center);
    let label = MorphLabel::new(text.idle, &["switch-button-label"], 0.5);
    label.root.set_transition_duration(COMPLETION_MORPH_MS);
    button.set_child(Some(&label.root));
    let energy = EnergyOverlay::new(&button, 7.0);
    SwitchAction {
        button,
        energy,
        label,
        mode,
    }
}

/// Where an activation reports progress, and the other actions it locks while running.
pub(crate) struct SwitchView {
    pub(crate) activity: SwitchAnimation,
    pub(crate) surface: gtk::Stack,
    pub(crate) siblings: Vec<gtk::Button>,
    /// Shown while the activation runs.
    pub(crate) minimize: gtk::Button,
}

pub(crate) fn connect_switch(
    action: &SwitchAction,
    view: &SwitchView,
    report: Report,
    state: Rc<UiState>,
    requires_confirmation: bool,
) -> SwitchConfirmation {
    let mode = action.mode;
    let text = mode_text(mode);
    let button = &action.button;
    let label = &action.label;
    let weak_button = button.downgrade();
    let weak_label = label.downgrade();
    let activity = view.activity.clone();
    let surface = view.surface.clone();
    let siblings = view.siblings.clone();
    let minimize = view.minimize.clone();
    let button_energy = action.energy.clone();
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
        for sibling in &siblings {
            sibling.set_sensitive(false);
        }
        label.set_text(text.running);
        button_energy.start();
        activity.start(mode);
        surface.set_visible_child(&activity.root);
        minimize.set_visible(true);
        let mini_title = format!("{} {}", text.running, report.target.name());
        let publish = move |state: &UiState, elapsed: Duration| {
            if let Some(mini) = state.mini.get() {
                mini.running(&MiniStatus {
                    step: text.step,
                    title: &mini_title,
                    note: text.note,
                    elapsed,
                    progress: None,
                });
            }
        };
        publish(&state, Duration::ZERO);
        let done_note = format!("{} #{}", report.target.name(), report.flake);
        let minimize = minimize.clone();
        let end = move |state: &UiState, end: MiniEnd, note: &str, elapsed: Duration| {
            minimize.set_visible(false);
            if let Some(mini) = state.mini.get() {
                let (step, title, action) = match end {
                    MiniEnd::Done => (text.done, text.complete_title, "Open"),
                    MiniEnd::Failed => (text.step, text.failed_title, "Details"),
                };
                let status = MiniStatus {
                    step,
                    title,
                    note,
                    elapsed,
                    progress: None,
                };
                mini.finish(end, &status, action);
            }
        };
        let switch_report = report.clone();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = activate(&switch_report, mode, &cancellation);
            let _ = sender.send(result);
        });
        let button = button.clone();
        let label = label.clone();
        let activity = activity.clone();
        let button_energy = button_energy.clone();
        let siblings = siblings.clone();
        // A switch also makes the configuration the boot default, so applying on reboot
        // afterwards would be redundant; after a boot apply, switching now is still useful.
        let unlock_siblings = move |succeeded: bool| {
            if !(succeeded && mode == ActivationMode::Switch) {
                for sibling in &siblings {
                    sibling.set_sensitive(true);
                }
            }
        };
        let started = Instant::now();
        glib::timeout_add_local(Duration::from_millis(100), move || {
            activity.update(started.elapsed());
            publish(&state, started.elapsed());
            match receiver.try_recv() {
                Ok(Ok(())) => {
                    if !state.finish(generation) {
                        return glib::ControlFlow::Break;
                    }
                    activity.complete(started.elapsed());
                    end(&state, MiniEnd::Done, &done_note, started.elapsed());
                    button_energy.finish(COMPLETION_MORPH_MS);
                    label.set_text(text.done);
                    button.remove_css_class("switch-locked");
                    button.add_css_class("switch-complete");
                    unlock_siblings(true);
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
                    let note = message.lines().next().unwrap_or_default();
                    end(&state, MiniEnd::Failed, note, started.elapsed());
                    button_energy.stop();
                    label.set_text(text.failed);
                    button.remove_css_class("switch-locked");
                    button.set_tooltip_text(Some(&message));
                    button.set_sensitive(true);
                    unlock_siblings(false);
                    let back = state.back_button.borrow().clone();
                    if let Some(back) = back {
                        back.set_sensitive(true);
                    }
                    glib::ControlFlow::Break
                }
                Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(TryRecvError::Disconnected) => {
                    if state.finish(generation) {
                        let message = "activation worker stopped without a result";
                        activity.fail(message);
                        end(&state, MiniEnd::Failed, message, started.elapsed());
                        button_energy.stop();
                        label.set_text(text.failed);
                        button.remove_css_class("switch-locked");
                        button.set_sensitive(true);
                        unlock_siblings(false);
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
        mode,
        requires_confirmation,
        confirmed: Rc::new(Cell::new(false)),
        key_down: Rc::new(Cell::new(false)),
        activate,
    };
    let clicked_confirmation = confirmation.clone();
    button.connect_clicked(move |_| clicked_confirmation.activate());

    confirmation
}

#[cfg(test)]
mod tests {
    use super::*;

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
