//! The home screen's repository card: checking the flake's git remote and pulling changes.

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use crate::app::state::{Operation, UiState};
use crate::home::repository::{self, RepositoryStatus};
use crate::ui::{EnergyOverlay, MorphLabel, label};

use super::{HomeCardWidgets, set_card_status, single_line};

#[derive(Clone)]
pub(super) struct RepositoryCheckWidgets {
    pub(super) button: gtk::Button,
    pub(super) label: MorphLabel,
    pub(super) card: HomeCardWidgets,
}

#[derive(Clone)]
pub(super) struct RepositoryUpdateWidgets {
    button: gtk::Button,
    label: MorphLabel,
    pub(super) reveal: gtk::Revealer,
    energy: EnergyOverlay,
}

#[derive(Clone)]
pub(super) struct RepositoryWidgets {
    pub(super) check: RepositoryCheckWidgets,
    pub(super) update: RepositoryUpdateWidgets,
    pub(super) busy: Rc<Cell<bool>>,
}

pub(super) fn repository_update_action(state: &Rc<UiState>) -> RepositoryUpdateWidgets {
    let button = gtk::Button::new();
    button.add_css_class("home-card-action");
    button.add_css_class("surface-button");
    button.add_css_class("repository-update-button");
    button.set_tooltip_text(Some(if state.appearance.keybinds {
        "Fast-forward or rebase incoming changes (P)"
    } else {
        "Fast-forward or rebase incoming changes"
    }));
    let reveal = gtk::Revealer::new();
    reveal.set_transition_type(gtk::RevealerTransitionType::SlideLeft);
    reveal.set_transition_duration(180);

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    content.set_halign(gtk::Align::Center);
    content.set_valign(gtk::Align::Center);
    let update_label = MorphLabel::new("Pull", &["home-card-action-label"], 0.5);
    content.append(&update_label.root);
    if state.appearance.keybinds {
        content.append(&label("P", &["keycap", "keycap-subtle"], 0.5));
    }
    button.set_child(Some(&content));
    let energy = EnergyOverlay::new(&button, 6.0);
    reveal.set_child(Some(&energy.root));
    state.home_buttons.borrow_mut().push(button.clone());
    state.register_home_action('P', &button);

    RepositoryUpdateWidgets {
        button,
        label: update_label,
        reveal,
        energy,
    }
}

pub(super) fn connect_repository_actions(
    window: &gtk::ApplicationWindow,
    state: Rc<UiState>,
    flake_dir: PathBuf,
    widgets: Rc<RepositoryWidgets>,
) {
    let check_state = Rc::clone(&state);
    let check_dir = flake_dir.clone();
    let check_widgets = Rc::downgrade(&widgets);
    widgets.check.button.connect_clicked(move |_| {
        let Some(check_widgets) = check_widgets.upgrade() else {
            return;
        };
        start_repository_check(check_dir.clone(), Rc::clone(&check_state), check_widgets);
    });

    let update_window = window.clone();
    let update_state = Rc::clone(&state);
    let update_widgets = Rc::downgrade(&widgets);
    widgets.update.button.connect_clicked(move |_| {
        let Some(update_widgets) = update_widgets.upgrade() else {
            return;
        };
        start_repository_update(
            &update_window,
            flake_dir.clone(),
            Rc::clone(&update_state),
            update_widgets,
        );
    });
}

pub(super) fn start_repository_check(
    flake_dir: PathBuf,
    state: Rc<UiState>,
    widgets: Rc<RepositoryWidgets>,
) {
    if widgets.busy.replace(true) {
        return;
    }
    let cancellation = Arc::new(AtomicBool::new(false));
    state.set_view_cancellation(&cancellation);
    widgets.check.label.set_text("Checking…");
    widgets.check.button.set_sensitive(false);
    widgets.check.card.detail.set_text("");
    widgets.check.card.detail.root.set_tooltip_text(None);
    set_card_status(&widgets.check.card, "Checking", None);
    widgets.check.card.badge_container.start();
    let (sender, receiver) = mpsc::channel();
    let worker_cancellation = Arc::clone(&cancellation);
    thread::spawn(move || {
        let result = repository::check(&flake_dir, true, &worker_cancellation);
        let _ = sender.send(result);
    });
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(status)) => {
                widgets.busy.set(false);
                widgets.check.card.badge_container.stop();
                render_repository_status(&widgets, &status);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                widgets.busy.set(false);
                widgets.check.card.badge_container.stop();
                render_repository_check_error(&widgets, &error);
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                widgets.busy.set(false);
                widgets.check.card.badge_container.stop();
                render_repository_check_error(
                    &widgets,
                    "The repository check stopped without a result",
                );
                glib::ControlFlow::Break
            }
        }
    });
}

fn start_repository_update(
    window: &gtk::ApplicationWindow,
    flake_dir: PathBuf,
    state: Rc<UiState>,
    widgets: Rc<RepositoryWidgets>,
) {
    if widgets.busy.get() {
        return;
    }
    let Some((generation, cancellation)) = state.begin(Operation::Updating) else {
        return;
    };
    widgets.busy.set(true);
    state.cancel_view();
    widgets.update.label.set_text("Applying");
    widgets.update.button.remove_css_class("error");
    widgets.update.button.set_sensitive(false);
    widgets.update.energy.start();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = repository::update(&flake_dir, &cancellation);
        let _ = sender.send(result);
    });
    let callback_window = window.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(result) => {
                widgets.busy.set(false);
                widgets.update.energy.stop();
                if state.finish(generation) {
                    if state.close_pending.replace(false) {
                        callback_window.close();
                    } else {
                        match result {
                            Ok(status) => {
                                state.reports.borrow_mut().clear();
                                render_repository_status(&widgets, &status);
                            }
                            Err(error) => render_repository_update_error(&widgets, &error),
                        }
                    }
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                widgets.busy.set(false);
                widgets.update.energy.stop();
                if state.finish(generation) {
                    render_repository_update_error(
                        &widgets,
                        "The repository update stopped without a result",
                    );
                }
                glib::ControlFlow::Break
            }
        }
    });
}

fn render_repository_status(widgets: &RepositoryWidgets, status: &RepositoryStatus) {
    widgets.check.label.set_text("Check");
    widgets.check.button.set_sensitive(true);
    widgets
        .check
        .card
        .detail
        .set_text(&format!("{} · {}", status.kind.name(), status.upstream));
    widgets.check.card.detail.root.set_tooltip_text(Some(&format!(
        "{} incoming · {} local",
        status.incoming, status.local
    )));
    if status.incoming > 0 {
        let pending = if status.local > 0 {
            format!("{} incoming · {} local", status.incoming, status.local)
        } else {
            format!("{} incoming", status.incoming)
        };
        set_card_status(&widgets.check.card, &pending, Some("attention"));
        widgets.update.label.set_text("Pull");
        widgets.update.button.remove_css_class("error");
        widgets.update.button.set_sensitive(true);
        widgets
            .update
            .button
            .set_tooltip_text(Some("Fast-forward or rebase incoming changes"));
        widgets.update.reveal.set_reveal_child(true);
    } else {
        if status.local == 0 {
            set_card_status(&widgets.check.card, "Up to date", Some("success"));
        } else {
            set_card_status(&widgets.check.card, &format!("{} local", status.local), None);
        }
        widgets.update.reveal.set_reveal_child(false);
    }
}

fn render_repository_check_error(widgets: &RepositoryWidgets, error: &str) {
    widgets.check.label.set_text("Retry");
    widgets.check.button.set_sensitive(true);
    widgets.check.card.detail.set_text(&single_line(error));
    widgets.check.card.detail.root.set_tooltip_text(Some(error));
    set_card_status(&widgets.check.card, "Check failed", Some("error"));
    widgets.update.reveal.set_reveal_child(false);
}

fn render_repository_update_error(widgets: &RepositoryWidgets, error: &str) {
    widgets.update.label.set_text("Retry");
    widgets.update.button.add_css_class("error");
    widgets.update.button.set_sensitive(true);
    widgets
        .update
        .button
        .set_tooltip_text(Some(&single_line(error)));
    widgets.check.card.detail.set_text(&single_line(error));
    widgets.check.card.detail.root.set_tooltip_text(Some(error));
    set_card_status(&widgets.check.card, "Update failed", Some("error"));
    widgets.update.reveal.set_reveal_child(true);
}
