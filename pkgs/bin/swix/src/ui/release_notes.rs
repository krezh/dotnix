use std::process::Command;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use crate::changelog::parse_changelog;
use crate::nix::run;
use crate::report::{Change, Report};
use crate::state::UiState;
use crate::ui::changelog::render as render_changelog;
use crate::ui::common::{back_button, clear, fit_window, label, navigation_footer, simple_header};
use crate::ui::report::show_report;

const CHANGELOG_TIMEOUT: Duration = Duration::from_secs(2 * 60);

pub(crate) fn show_changelog(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    change: &Change,
    state: Rc<UiState>,
    report: Report,
) {
    state.clear_actions();
    clear(root);
    fit_window(window, (900, 760), (360, 480));
    let flake_dir = report.flake_dir.clone();

    root.append(&simple_header(&format!("{} release notes", change.name)));

    let back_tooltip = if state.appearance.keybinds {
        "Back to report (←)"
    } else {
        "Back to report"
    };
    let back = back_button("Back to report", back_tooltip);
    let back_window = window.clone();
    let back_root = root.clone();
    let back_state = Rc::clone(&state);
    back.connect_clicked(move |_| {
        show_report(
            &back_window,
            &back_root,
            Rc::clone(&back_state),
            report.clone(),
        );
    });
    state.set_back_button(&back);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 18);
    content.add_css_class("changelog-content");
    let loading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    loading.add_css_class("changelog-loading");
    let spinner = gtk::Spinner::new();
    spinner.start();
    loading.append(&spinner);
    loading.append(&label(
        "Looking up release notes...",
        &["changelog-loading-label"],
        0.0,
    ));
    content.append(&loading);
    let scroller = gtk::ScrolledWindow::new();
    scroller.add_css_class("changelog-scroll");
    scroller.set_vexpand(true);
    scroller.set_child(Some(&content));
    state.set_scroll_adjustment(scroller.vadjustment());
    root.append(&scroller);
    root.append(&navigation_footer(
        &back,
        state.appearance.keybinds,
        &[
            ("↑", "Scroll Up"),
            ("↓", "Scroll Down"),
            ("PgUp / PgDn", "Page"),
            ("←", "Back"),
            ("Esc", "Close"),
        ],
    ));

    let cancellation = Arc::new(AtomicBool::new(false));
    state.set_view_cancellation(&cancellation);
    back.grab_focus();

    let name = change.name.clone();
    let version_spec = if change.old.is_empty() {
        change.new.clone()
    } else {
        format!("{}..{}", change.old, change.new)
    };
    let cache_key = format!("{}\0{name}\0{version_spec}", flake_dir.display());
    let changelog = state.changelog_cache.borrow().get(&cache_key).cloned();
    if let Some(changelog) = changelog {
        render_changelog(&content, &changelog);
        return;
    }
    let (sender, receiver) = mpsc::channel();
    let worker_cancellation = Arc::clone(&cancellation);
    thread::spawn(move || {
        let result = run(
            Command::new("nix-changelog")
                .env("NO_COLOR", "1")
                .arg("--json")
                .arg(name)
                .arg(version_spec)
                .current_dir(flake_dir),
            "nix-changelog",
            &worker_cancellation,
            CHANGELOG_TIMEOUT,
            8 * 1024 * 1024,
        )
        .and_then(|output| parse_changelog(&output.stdout));
        let _ = sender.send(result);
    });
    let cache_state = Rc::clone(&state);
    let callback_cancellation = Arc::clone(&cancellation);
    glib::timeout_add_local(Duration::from_millis(100), move || {
        if callback_cancellation.load(Ordering::Relaxed) {
            return glib::ControlFlow::Break;
        }
        match receiver.try_recv() {
            Ok(Ok(changelog)) => {
                render_changelog(&content, &changelog);
                cache_state
                    .changelog_cache
                    .borrow_mut()
                    .insert(cache_key.clone(), changelog);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                clear(&content);
                let message = label(&error, &["error", "changelog-error"], 0.0);
                message.set_wrap(true);
                message.set_selectable(true);
                content.append(&message);
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                clear(&content);
                let message = label(
                    "changelog worker stopped without a result",
                    &["error", "changelog-error"],
                    0.0,
                );
                message.set_wrap(true);
                content.append(&message);
                glib::ControlFlow::Break
            }
        }
    });
}
