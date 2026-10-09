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

use crate::build::Target;
use crate::config::{Config, load_config};
use crate::inputs;
use crate::repository::{self, RepositoryStatus};
use crate::state::{Operation, UiState};
use crate::ui::build_screen::start_build;
use crate::ui::cleanup::show_cleanup;
use crate::ui::common::{
    MorphLabel, action_close_button, back_button, clear, fit_window, label, title,
};
use crate::ui::inputs::show_inputs;
use crate::ui::timeline::EnergyOverlay;

#[derive(Clone)]
struct HomeCardWidgets {
    root: gtk::Box,
    detail: gtk::Label,
    badge: MorphLabel,
    badge_container: EnergyOverlay,
}

#[derive(Clone)]
struct RepositoryCheckWidgets {
    button: gtk::Button,
    label: MorphLabel,
    card: HomeCardWidgets,
}

#[derive(Clone)]
struct RepositoryUpdateWidgets {
    button: gtk::Button,
    label: MorphLabel,
    reveal: gtk::Revealer,
    energy: EnergyOverlay,
}

#[derive(Clone)]
struct RepositoryWidgets {
    check: RepositoryCheckWidgets,
    update: RepositoryUpdateWidgets,
    busy: Rc<Cell<bool>>,
}

pub(crate) fn back_to_home_button(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: &Rc<UiState>,
) -> gtk::Button {
    let tooltip = if state.appearance.keybinds {
        "Back to Swix home (←)"
    } else {
        "Back to Swix home"
    };
    let back = back_button("Back", tooltip);
    let window = window.clone();
    let root = root.clone();
    let callback_state = Rc::clone(state);
    back.connect_clicked(move |_| {
        show_home(&window, &root, Rc::clone(&callback_state), load_config());
    });
    state.set_back_button(&back);
    back
}

pub(crate) fn show_home(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    config: Result<Config, String>,
) {
    if !state.cancel() {
        return;
    }
    state.clear_actions();
    clear(root);
    root.add_css_class("home-root");
    let Ok(config) = config else {
        let error = gtk::Label::new(config.err().as_deref());
        error.add_css_class("error");
        error.set_wrap(true);
        root.append(&error);
        return;
    };

    let preferred_width = 940 + if state.appearance.keybinds { 120 } else { 0 };
    fit_window(window, (preferred_width, 500), (720, 430));

    let home = gtk::Box::new(gtk::Orientation::Vertical, 18);
    home.add_css_class("home");
    home.set_vexpand(true);
    home.append(&home_header());

    let dashboard = gtk::Grid::new();
    dashboard.add_css_class("home-dashboard");
    dashboard.set_column_spacing(10);
    dashboard.set_row_spacing(10);
    dashboard.set_column_homogeneous(true);
    dashboard.set_vexpand(false);
    let mut card_index = 0;

    let mut targets = vec![(Target::NixOs, "drive-harddisk-symbolic", 'N')];
    if config.home_flake.is_some() {
        targets.insert(0, (Target::HomeManager, "user-home-symbolic", 'M'));
    }
    let mut first_button = None;
    for (target, icon_name, key) in targets {
        let detail = format!("#{}", target.flake(&config).unwrap_or("User environment"));
        let card = home_card(target.name(), &detail, icon_name);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions.add_css_class("home-card-actions");
        actions.set_homogeneous(true);
        let (button, _) = home_card_action("Build", Some(icon_name), key, &state);
        button.add_css_class("primary");
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
        card.root.append(&actions);
        dashboard.attach(&card.root, card_index % 3, card_index / 3, 1, 1);
        card_index += 1;
    }

    let repository_card = home_card("Repository", "", "folder-remote-symbolic");
    set_repository_badge(&repository_card, "Checking", None);
    let repository_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    repository_actions.add_css_class("home-card-actions");
    let (repository_check_button, repository_check_label) =
        home_card_action("Check", None, 'R', &state);
    repository_check_button.set_hexpand(true);
    repository_actions.append(&repository_check_button);
    let repository_update = repository_update_action(&state);
    repository_actions.append(&repository_update.reveal);
    repository_card.root.append(&repository_actions);
    dashboard.attach(&repository_card.root, card_index % 3, card_index / 3, 1, 1);
    card_index += 1;
    let repository_check = RepositoryCheckWidgets {
        button: repository_check_button,
        label: repository_check_label,
        card: repository_card,
    };

    let input_count = inputs::load(&config.flake_dir)
        .map(|(_, inputs)| format!("{} direct inputs", inputs.len()))
        .unwrap_or_else(|_| "Manage flake.lock".to_owned());
    let inputs_card = home_card("Inputs", &input_count, "view-list-symbolic");
    let inputs_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    inputs_actions.add_css_class("home-card-actions");
    inputs_actions.set_homogeneous(true);
    let (inputs_button, _) = home_card_action("Manage", Some("view-list-symbolic"), 'I', &state);
    inputs_button.add_css_class("primary");
    let inputs_window = window.clone();
    let inputs_root = root.clone();
    let inputs_state = Rc::clone(&state);
    let inputs_directory = config.flake_dir.clone();
    inputs_button.connect_clicked(move |_| {
        show_inputs(
            &inputs_window,
            &inputs_root,
            Rc::clone(&inputs_state),
            inputs_directory.clone(),
        );
    });
    inputs_actions.append(&inputs_button);
    inputs_card.root.append(&inputs_actions);
    dashboard.attach(&inputs_card.root, card_index % 3, card_index / 3, 1, 1);
    card_index += 1;

    let cleanup_card = home_card("Cleanup", "6 areas", "user-trash-symbolic");
    cleanup_card.detail.set_tooltip_text(Some(
        "Generations, journals, containers, build artifacts, caches, and trash",
    ));
    let cleanup_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    cleanup_actions.add_css_class("home-card-actions");
    cleanup_actions.set_homogeneous(true);
    let (cleanup_button, _) = home_card_action("Cleanup", Some("user-trash-symbolic"), 'C', &state);
    let cleanup_window = window.clone();
    let cleanup_root = root.clone();
    let cleanup_state = Rc::clone(&state);
    cleanup_button.connect_clicked(move |_| {
        show_cleanup(&cleanup_window, &cleanup_root, Rc::clone(&cleanup_state));
    });
    cleanup_actions.append(&cleanup_button);
    cleanup_card.root.append(&cleanup_actions);
    dashboard.attach(&cleanup_card.root, card_index % 3, card_index / 3, 1, 1);

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    footer.add_css_class("home-footer");
    footer.append(&action_close_button(
        window,
        Rc::clone(&state),
        "report-close-button",
        "report-close-label",
    ));

    home.append(&dashboard);
    let dashboard_spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    dashboard_spacer.set_vexpand(true);
    home.append(&dashboard_spacer);
    home.append(&footer);
    root.append(&home);

    let widgets = Rc::new(RepositoryWidgets {
        check: repository_check,
        update: repository_update,
        busy: Rc::new(Cell::new(false)),
    });
    connect_repository_actions(
        window,
        Rc::clone(&state),
        config.flake_dir.clone(),
        Rc::clone(&widgets),
    );
    start_repository_check(config.flake_dir, Rc::clone(&state), Rc::clone(&widgets));
    first_button
        .unwrap_or_else(|| widgets.check.button.clone())
        .grab_focus();
}

fn home_header() -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    header.add_css_class("home-header");
    header.set_valign(gtk::Align::Start);

    let heading = gtk::Box::new(gtk::Orientation::Vertical, 0);
    heading.set_hexpand(true);
    heading.append(&title("Swix"));
    heading.append(&label(
        "System configuration and maintenance",
        &["home-subtitle"],
        0.0,
    ));
    header.append(&heading);
    header
}

fn home_card(title_text: &str, detail_text: &str, icon_name: &str) -> HomeCardWidgets {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 4);
    root.add_css_class("home-card");
    root.set_hexpand(true);
    root.set_vexpand(false);

    let card_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    card_header.add_css_class("home-card-header");
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.add_css_class("home-card-icon");
    icon.set_pixel_size(18);
    icon.set_valign(gtk::Align::Center);
    card_header.append(&icon);
    card_header.append(&label(title_text, &["home-card-title"], 0.0));
    root.append(&card_header);

    let meta = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    meta.set_vexpand(true);
    let detail = label(detail_text, &["home-card-detail"], 0.0);
    detail.set_ellipsize(gtk::pango::EllipsizeMode::End);
    detail.set_single_line_mode(true);
    detail.set_hexpand(true);
    meta.append(&detail);

    let badge = MorphLabel::new("", &["home-card-badge"], 0.5);
    badge.root.set_valign(gtk::Align::Center);
    let badge_container = EnergyOverlay::new(&badge.root, 6.0);
    badge_container.root.set_visible(false);
    badge_container.root.set_halign(gtk::Align::End);
    badge_container.root.set_valign(gtk::Align::Center);
    meta.append(&badge_container.root);
    root.append(&meta);

    HomeCardWidgets {
        root,
        detail,
        badge,
        badge_container,
    }
}

fn home_card_action(
    text: &str,
    icon_name: Option<&str>,
    key: char,
    state: &Rc<UiState>,
) -> (gtk::Button, MorphLabel) {
    let button = gtk::Button::new();
    button.add_css_class("home-card-action");
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    content.set_halign(gtk::Align::Center);
    content.set_valign(gtk::Align::Center);
    if let Some(icon_name) = icon_name {
        let icon = gtk::Image::from_icon_name(icon_name);
        icon.set_pixel_size(14);
        content.append(&icon);
    }
    let action_label = MorphLabel::new(text, &["home-card-action-label"], 0.5);
    content.append(&action_label.root);
    if state.appearance.keybinds {
        content.append(&label(&key.to_string(), &["keycap", "keycap-subtle"], 0.5));
    }
    button.set_child(Some(&content));
    state.home_buttons.borrow_mut().push(button.clone());
    state.register_home_action(key, &button);
    (button, action_label)
}

fn repository_update_action(state: &Rc<UiState>) -> RepositoryUpdateWidgets {
    let button = gtk::Button::new();
    button.add_css_class("home-card-action");
    button.add_css_class("repository-update-button");
    button.set_tooltip_text(Some(if state.appearance.keybinds {
        "Fast-forward or rebase incoming changes (P)"
    } else {
        "Fast-forward or rebase incoming changes"
    }));
    let reveal = gtk::Revealer::new();
    reveal.set_transition_type(gtk::RevealerTransitionType::SlideLeft);
    reveal.set_transition_duration(180);

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 4);
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

fn connect_repository_actions(
    window: &gtk::ApplicationWindow,
    state: Rc<UiState>,
    flake_dir: PathBuf,
    widgets: Rc<RepositoryWidgets>,
) {
    let check_state = Rc::clone(&state);
    let check_dir = flake_dir.clone();
    let check_widgets = Rc::clone(&widgets);
    widgets.check.button.connect_clicked(move |_| {
        start_repository_check(
            check_dir.clone(),
            Rc::clone(&check_state),
            Rc::clone(&check_widgets),
        );
    });

    let update_window = window.clone();
    let update_state = Rc::clone(&state);
    let update_widgets = Rc::clone(&widgets);
    widgets.update.button.connect_clicked(move |_| {
        start_repository_update(
            &update_window,
            flake_dir.clone(),
            Rc::clone(&update_state),
            Rc::clone(&update_widgets),
        );
    });
}

fn start_repository_check(flake_dir: PathBuf, state: Rc<UiState>, widgets: Rc<RepositoryWidgets>) {
    if widgets.busy.replace(true) {
        return;
    }
    let cancellation = Arc::new(AtomicBool::new(false));
    state.set_view_cancellation(&cancellation);
    widgets.check.label.set_text("Checking…");
    widgets.check.button.set_sensitive(false);
    widgets.check.card.detail.set_text("");
    widgets.check.card.detail.set_tooltip_text(None);
    set_repository_badge(&widgets.check.card, "Checking", None);
    widgets.check.card.badge_container.start();
    let (sender, receiver) = mpsc::channel();
    let worker_cancellation = Arc::clone(&cancellation);
    thread::spawn(move || {
        let result = repository::check(&flake_dir, &worker_cancellation);
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
                            Ok(status) => render_repository_status(&widgets, &status),
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
    widgets.check.card.detail.set_tooltip_text(Some(&format!(
        "{} incoming · {} local",
        status.incoming, status.local
    )));
    let counts = format!("↓{} ↑{}", status.incoming, status.local);
    if status.incoming > 0 {
        set_repository_badge(&widgets.check.card, &counts, Some("attention"));
        widgets.update.label.set_text("Pull");
        widgets.update.button.remove_css_class("error");
        widgets.update.button.set_sensitive(true);
        widgets
            .update
            .button
            .set_tooltip_text(Some("Fast-forward or rebase incoming changes"));
        widgets.update.reveal.set_reveal_child(true);
    } else {
        let badge_state = (status.local == 0).then_some("success");
        set_repository_badge(&widgets.check.card, &counts, badge_state);
        widgets.update.reveal.set_reveal_child(false);
    }
}

fn render_repository_check_error(widgets: &RepositoryWidgets, error: &str) {
    widgets.check.label.set_text("Retry");
    widgets.check.button.set_sensitive(true);
    widgets.check.card.detail.set_text(&single_line(error));
    widgets.check.card.detail.set_tooltip_text(Some(error));
    set_repository_badge(&widgets.check.card, "Check failed", Some("error"));
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
    widgets.check.card.detail.set_tooltip_text(Some(error));
    set_repository_badge(&widgets.check.card, "Update failed", Some("error"));
    widgets.update.reveal.set_reveal_child(true);
}

fn set_repository_badge(card: &HomeCardWidgets, text: &str, state: Option<&str>) {
    for class in ["success", "attention", "error"] {
        card.badge.remove_css_class(class);
    }
    if let Some(state) = state {
        card.badge.add_css_class(state);
    }
    card.badge.set_text(text);
    card.badge_container.root.set_visible(true);
}

fn single_line(message: &str) -> String {
    message.lines().next().unwrap_or(message).trim().to_owned()
}
