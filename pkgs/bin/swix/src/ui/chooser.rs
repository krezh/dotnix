use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;

use crate::build::Target;
use crate::config::{Config, load_config};
use crate::repository::{self, RepositoryStatus};
use crate::state::{Operation, UiState};
use crate::ui::build_screen::start_build;
use crate::ui::common::{action_close_button, back_button, clear, fit_window, label};
use crate::ui::timeline::EnergyIndicator;

#[derive(Clone)]
struct RepositoryWidgets {
    status: gtk::Label,
    detail: gtk::Label,
    counter: gtk::Label,
    update: gtk::Button,
    refresh: gtk::Button,
    refresh_energy: EnergyIndicator,
}

pub(crate) fn back_to_chooser_button(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: &Rc<UiState>,
) -> gtk::Button {
    let tooltip = if state.appearance.keybinds {
        "Back to maintenance center (←)"
    } else {
        "Back to maintenance center"
    };
    let back = back_button("Back", tooltip);
    let window = window.clone();
    let root = root.clone();
    let callback_state = Rc::clone(state);
    back.connect_clicked(move |_| {
        show_chooser(&window, &root, Rc::clone(&callback_state), load_config());
    });
    state.set_back_button(&back);
    back
}

pub(crate) fn show_chooser(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    config: Result<Config, String>,
) {
    if !state.cancel() {
        return;
    }
    state.clear_actions();
    fit_window(window, (520, 222), (380, 212));
    clear(root);
    root.add_css_class("chooser-root");
    let Ok(config) = config else {
        let error = gtk::Label::new(config.err().as_deref());
        error.add_css_class("error");
        error.set_wrap(true);
        root.append(&error);
        return;
    };

    let chooser = gtk::Box::new(gtk::Orientation::Vertical, 8);
    chooser.add_css_class("chooser");
    chooser.set_vexpand(true);
    chooser.append(&chooser_header());

    let repository_panel = gtk::Box::new(gtk::Orientation::Vertical, 6);
    repository_panel.add_css_class("repository-panel");
    let repository_status = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let repository_icon = gtk::Image::from_icon_name("folder-remote-symbolic");
    repository_icon.add_css_class("repository-icon");
    repository_icon.set_pixel_size(18);
    repository_status.append(&repository_icon);
    let repository_copy = gtk::Box::new(gtk::Orientation::Vertical, 2);
    repository_copy.set_hexpand(true);
    let status = label("Checking repository…", &["repository-status"], 0.0);
    let detail = label(
        &config.flake_dir.display().to_string(),
        &["repository-detail"],
        0.0,
    );
    detail.set_ellipsize(gtk::pango::EllipsizeMode::End);
    repository_copy.append(&status);
    repository_copy.append(&detail);
    repository_status.append(&repository_copy);
    let counter = label("–", &["repository-counter"], 0.5);
    counter.set_tooltip_text(Some("Incoming commits"));
    counter.set_valign(gtk::Align::Center);
    repository_status.append(&counter);
    let refresh_icon = gtk::Image::from_icon_name("view-refresh-symbolic");
    refresh_icon.set_pixel_size(14);
    let refresh_energy = EnergyIndicator::new();
    let refresh = gtk::Button::new();
    refresh.set_child(Some(&refresh_icon));
    refresh.add_css_class("repository-refresh");
    refresh.set_tooltip_text(Some("Check the remote again"));
    refresh.set_sensitive(false);
    let refresh_overlay = gtk::Overlay::new();
    refresh_overlay.set_child(Some(&refresh_energy.root));
    refresh_overlay.add_overlay(&refresh);
    refresh.set_halign(gtk::Align::Fill);
    refresh.set_valign(gtk::Align::Fill);
    refresh_overlay.set_valign(gtk::Align::Center);
    repository_status.append(&refresh_overlay);
    repository_panel.append(&repository_status);

    let update = chooser_action(
        "Update repository",
        "Fast-forward or rebase",
        "software-update-available-symbolic",
        'P',
        &state,
    );
    update.set_sensitive(false);
    repository_panel.append(&update);
    chooser.append(&repository_panel);

    let actions = gtk::Box::new(gtk::Orientation::Vertical, 4);
    actions.add_css_class("chooser-list");
    let mut targets = vec![(Target::NixOs, "drive-harddisk-symbolic", 'N')];
    if config.home_flake.is_some() {
        targets.insert(0, (Target::HomeManager, "user-home-symbolic", 'M'));
    }
    let mut first_button = None;
    for (target, icon_name, key) in targets {
        let detail = format!("#{}", target.flake(&config).unwrap_or("User environment"));
        let button = chooser_action(target.name(), &detail, icon_name, key, &state);
        first_button.get_or_insert_with(|| button.clone());
        let window = window.clone();
        let root = root.clone();
        let callback_state = Rc::clone(&state);
        let config = config.clone();
        button.connect_clicked(move |_| {
            start_build(
                &window,
                &root,
                Rc::clone(&callback_state),
                config.clone(),
                target,
            );
        });
        actions.append(&button);
    }
    chooser.append(&actions);

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.add_css_class("chooser-footer");
    footer.set_halign(gtk::Align::End);
    footer.append(&action_close_button(
        window,
        Rc::clone(&state),
        "chooser-close-button",
        "chooser-close-label",
    ));
    chooser.append(&footer);
    root.append(&chooser);

    let widgets = Rc::new(RepositoryWidgets {
        status,
        detail,
        counter,
        update,
        refresh,
        refresh_energy,
    });
    connect_repository_actions(
        window,
        Rc::clone(&state),
        config.flake_dir.clone(),
        Rc::clone(&widgets),
    );
    start_repository_check(config.flake_dir, Rc::clone(&state), Rc::clone(&widgets));
    first_button
        .unwrap_or_else(|| widgets.update.clone())
        .grab_focus();
}

fn chooser_header() -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header.add_css_class("chooser-header");
    let icon = gtk::Image::from_icon_name("emblem-system-symbolic");
    icon.add_css_class("chooser-prompt-icon");
    icon.set_pixel_size(16);
    icon.set_valign(gtk::Align::Center);
    header.append(&icon);
    header.append(&label(
        "Switch Configuration",
        &["chooser-prompt-label"],
        0.0,
    ));
    header
}

fn chooser_action(
    title: &str,
    detail: &str,
    icon_name: &str,
    key: char,
    state: &Rc<UiState>,
) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("chooser-button");
    button.add_css_class("chooser-row");
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_valign(gtk::Align::Center);
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.add_css_class("chooser-row-icon");
    icon.set_pixel_size(18);
    row.append(&icon);
    row.append(&label(title, &["chooser-row-title"], 0.0));
    let detail = label(detail, &["chooser-row-detail"], 0.0);
    detail.set_hexpand(true);
    row.append(&detail);
    if state.appearance.keybinds {
        row.append(&label(
            &key.to_string(),
            &["keycap", "chooser-row-key"],
            0.5,
        ));
    }
    button.set_child(Some(&row));
    state.chooser_buttons.borrow_mut().push(button.clone());
    state.register_chooser_action(key, &button);
    button
}

fn connect_repository_actions(
    window: &gtk::ApplicationWindow,
    state: Rc<UiState>,
    flake_dir: PathBuf,
    widgets: Rc<RepositoryWidgets>,
) {
    let refresh_state = Rc::clone(&state);
    let refresh_dir = flake_dir.clone();
    let refresh_widgets = Rc::clone(&widgets);
    widgets.refresh.connect_clicked(move |_| {
        start_repository_check(
            refresh_dir.clone(),
            Rc::clone(&refresh_state),
            Rc::clone(&refresh_widgets),
        );
    });

    let update_window = window.clone();
    let update_state = Rc::clone(&state);
    let update_widgets = Rc::clone(&widgets);
    widgets.update.connect_clicked(move |_| {
        start_repository_update(
            &update_window,
            flake_dir.clone(),
            Rc::clone(&update_state),
            Rc::clone(&update_widgets),
        );
    });
}

fn start_repository_check(flake_dir: PathBuf, state: Rc<UiState>, widgets: Rc<RepositoryWidgets>) {
    let cancellation = Arc::new(AtomicBool::new(false));
    state.set_view_cancellation(&cancellation);
    widgets.status.set_text("Checking repository…");
    widgets.detail.set_text(&flake_dir.display().to_string());
    widgets.counter.set_text("–");
    widgets.update.set_sensitive(false);
    set_refresh_busy(&widgets, true);
    let (sender, receiver) = mpsc::channel();
    let worker_cancellation = Arc::clone(&cancellation);
    thread::spawn(move || {
        let result = repository::check(&flake_dir, &worker_cancellation);
        let _ = sender.send(result);
    });
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(status)) => {
                render_repository_status(&widgets, &status);
                widgets.update.set_sensitive(status.incoming > 0);
                set_refresh_busy(&widgets, false);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                widgets.status.set_text("Could not check repository");
                widgets.detail.set_text(&single_line(&error));
                widgets.counter.set_text("!");
                set_refresh_busy(&widgets, false);
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                widgets.status.set_text("Repository check stopped");
                widgets.counter.set_text("!");
                set_refresh_busy(&widgets, false);
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
    let Some((generation, cancellation)) = state.begin(Operation::Updating) else {
        return;
    };
    state.cancel_view();
    for button in state.chooser_buttons.borrow().iter() {
        button.set_sensitive(false);
    }
    set_refresh_busy(&widgets, true);
    widgets.status.set_text("Updating repository…");
    widgets
        .detail
        .set_text("Fetching and integrating remote changes safely");
    widgets.counter.set_text("…");
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = repository::update(&flake_dir, &cancellation);
        let _ = sender.send(result);
    });
    let callback_window = window.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(result) => {
                if state.finish(generation) {
                    if state.close_pending.replace(false) {
                        callback_window.close();
                    } else {
                        match result {
                            Ok(status) => {
                                render_repository_status(&widgets, &status);
                                widgets.detail.set_text("Repository updated successfully");
                            }
                            Err(error) => {
                                widgets.status.set_text("Repository update failed");
                                widgets.detail.set_text(&single_line(&error));
                                widgets.counter.set_text("!");
                            }
                        }
                        set_refresh_busy(&widgets, false);
                        widgets.update.set_sensitive(false);
                        for button in state.chooser_buttons.borrow().iter() {
                            if button != &widgets.update {
                                button.set_sensitive(true);
                            }
                        }
                    }
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                if state.finish(generation) {
                    widgets.status.set_text("Repository update stopped");
                    widgets
                        .detail
                        .set_text("The update worker exited without a result");
                    widgets.counter.set_text("!");
                    set_refresh_busy(&widgets, false);
                }
                glib::ControlFlow::Break
            }
        }
    });
}

fn render_repository_status(widgets: &RepositoryWidgets, status: &RepositoryStatus) {
    widgets
        .status
        .set_text(&format!("{} repository", status.kind.name()));
    widgets.detail.set_text(&status.detail());
    widgets.counter.set_text(&status.incoming.to_string());
}

fn set_refresh_busy(widgets: &RepositoryWidgets, busy: bool) {
    if busy {
        widgets.refresh.add_css_class("energy-active");
        widgets.refresh_energy.start();
    } else {
        widgets.refresh_energy.stop();
        widgets.refresh.remove_css_class("energy-active");
    }
    widgets.refresh.set_sensitive(!busy);
}

fn single_line(message: &str) -> String {
    message.lines().next().unwrap_or(message).trim().to_owned()
}
