//! Lock updates: updating, pinning, and restoring flake.lock in the background.

use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use crate::app::state::Operation;
use crate::inputs::{self, LockProgress, LockUpdate};
use crate::ui::compact_size;

use super::history::start_history;
use super::{InputsView, short_revision};

pub(super) enum InputOperationResult {
    Updated(LockUpdate),
    Restored,
}

pub(super) fn set_transaction_actions(view: &InputsView, visible: bool) {
    view.transaction_actions.set_reveal_child(visible);
    if visible {
        view.status.remove_css_class("error");
        view.show_status("flake.lock updated · build the configuration or undo every Swix change");
    }
}

pub(super) fn start_latest_update(
    window: &gtk::ApplicationWindow,
    view: Rc<InputsView>,
    input: Option<String>,
) {
    let message = input.as_ref().map_or_else(
        || "Updating every input…".to_owned(),
        |name| format!("Updating {name}…"),
    );
    start_operation(
        window,
        view,
        message,
        move |directory, cancellation, progress| {
            inputs::update(directory, input.as_deref(), cancellation, progress)
                .map(InputOperationResult::Updated)
        },
    );
}

pub(super) fn start_manual_revision(window: &gtk::ApplicationWindow, view: Rc<InputsView>) {
    let Some(input) = view.current_input() else {
        return;
    };
    let revision = view.revision_entry.text().to_string();
    let flake_ref = match inputs::revision_ref(&input, &revision) {
        Ok(reference) => reference,
        Err(error) => {
            view.show_error(&error);
            return;
        }
    };
    let input_name = input.name;
    let message = format!("Resolving {input_name} at {}…", revision.trim());
    start_operation(
        window,
        view,
        message,
        move |directory, cancellation, progress| {
            inputs::update_at_revision(directory, &input_name, &flake_ref, cancellation, progress)
                .map(InputOperationResult::Updated)
        },
    );
}

pub(super) fn apply_history_revision(
    window: &gtk::ApplicationWindow,
    view: Rc<InputsView>,
    index: usize,
) {
    let Some(input) = view.current_input() else {
        return;
    };
    let Some(record) = view.revisions.borrow().get(index).cloned() else {
        return;
    };
    let input_name = input.name;
    let message = format!(
        "Setting {input_name} to {}…",
        short_revision(&record.revision)
    );
    start_operation(
        window,
        view,
        message,
        move |directory, cancellation, progress| {
            inputs::update_at_revision(
                directory,
                &input_name,
                &record.flake_ref,
                cancellation,
                progress,
            )
            .map(InputOperationResult::Updated)
        },
    );
}

/// Runs `operation` off the main thread. Its third argument reports fetch progress, which the
/// status line shows next to `message`.
pub(super) fn start_operation(
    window: &gtk::ApplicationWindow,
    view: Rc<InputsView>,
    message: String,
    operation: impl FnOnce(
        &std::path::Path,
        &AtomicBool,
        &dyn Fn(LockProgress),
    ) -> Result<InputOperationResult, String>
    + Send
    + 'static,
) {
    let Some((generation, cancellation)) = view.state.begin(Operation::Updating) else {
        return;
    };
    view.state.cancel_view();
    view.set_busy(true, &message);
    let directory = view.directory.clone();
    let (sender, receiver) = mpsc::channel();
    let (progress_sender, progress_receiver) = mpsc::channel();
    thread::spawn(move || {
        let progress = |progress: LockProgress| {
            let _ = progress_sender.send(progress);
        };
        let result = operation(&directory, &cancellation, &progress);
        let _ = sender.send(result);
    });
    let callback_window = window.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        if let Some(progress) = progress_receiver.try_iter().last()
            && let Some(text) = progress_text(&message, progress)
        {
            view.status.set_text_immediate(&text);
        }
        match receiver.try_recv() {
            Ok(result) => {
                view.set_busy(false, "");
                if view.state.finish(generation) {
                    if view.state.close_pending.replace(false) {
                        callback_window.close();
                    } else {
                        match result {
                            Ok(InputOperationResult::Updated(update)) => {
                                let changed = update.changed();
                                let pending = match view.state.input_undo.take() {
                                    Some(previous) => previous.followed_by(update),
                                    None => update,
                                };
                                let undoable = pending.changed();
                                view.state.input_undo.replace(undoable.then_some(pending));
                                view.state.reports.borrow_mut().clear();
                                match inputs::load(&view.directory) {
                                    Ok((_, refreshed)) => {
                                        view.set_inputs(refreshed);
                                        view.revision_entry.set_text("");
                                        start_history(Rc::clone(&view));
                                        set_transaction_actions(&view, undoable);
                                        if !changed {
                                            view.show_status("flake.lock is already current");
                                        } else if !undoable {
                                            view.show_status(
                                                "flake.lock is back to how Swix found it",
                                            );
                                        }
                                    }
                                    Err(error) => view.show_error(&error),
                                }
                            }
                            Ok(InputOperationResult::Restored) => {
                                view.state.input_undo.replace(None);
                                set_transaction_actions(&view, false);
                                view.state.reports.borrow_mut().clear();
                                match inputs::load(&view.directory) {
                                    Ok((_, refreshed)) => {
                                        view.set_inputs(refreshed);
                                        start_history(Rc::clone(&view));
                                        view.show_status(
                                            "flake.lock restored to before Swix's changes",
                                        );
                                    }
                                    Err(error) => view.show_error(&error),
                                }
                            }
                            Err(error) => view.show_error(&error),
                        }
                    }
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                view.set_busy(false, "");
                if view.state.finish(generation) {
                    view.show_error("The input operation stopped without a result");
                }
                glib::ControlFlow::Break
            }
        }
    });
}

/// `Setting nixpkgs to 494ce7f… · 31 MiB downloaded`, then `… · adding to the Nix store` once the
/// transfer ends and Nix unpacks it silently.
fn progress_text(message: &str, progress: LockProgress) -> Option<String> {
    if progress.downloaded == 0 {
        return None;
    }
    let size = compact_size(progress.downloaded as f64);
    Some(if progress.downloading {
        format!("{message} · {size} downloaded")
    } else {
        format!("{message} · {size} downloaded · adding to the Nix store")
    })
}
