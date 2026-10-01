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
use crate::repository::{self, RepositoryStatus};
use crate::state::{Operation, UiState};
use crate::ui::build_screen::start_build;
use crate::ui::common::{action_close_button, back_button, clear, fit_window, label, title};
use crate::ui::timeline::EnergyIndicator;

#[derive(Clone)]
struct HomeActionWidgets {
    button: gtk::Button,
    title: gtk::Label,
    detail: gtk::Label,
    badge: gtk::Label,
    badge_container: gtk::Overlay,
    chevron: gtk::Image,
    energy: EnergyIndicator,
}

#[derive(Clone)]
struct RepositoryUpdateWidgets {
    button: gtk::Button,
    label: gtk::Label,
    energy: EnergyIndicator,
}

#[derive(Clone)]
struct RepositoryWidgets {
    check: HomeActionWidgets,
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

    let preferred_height = if config.home_flake.is_some() {
        416
    } else {
        350
    };
    fit_window(window, (560, preferred_height), (420, 330));

    let home = gtk::Box::new(gtk::Orientation::Vertical, 14);
    home.add_css_class("home");
    home.append(&home_header());

    let configuration = gtk::Box::new(gtk::Orientation::Vertical, 8);
    configuration.add_css_class("home-section");
    configuration.append(&home_section_label("Configuration"));
    let configuration_group = home_group();
    let mut targets = vec![(Target::NixOs, "drive-harddisk-symbolic", 'N')];
    if config.home_flake.is_some() {
        targets.insert(0, (Target::HomeManager, "user-home-symbolic", 'M'));
    }
    let target_count = targets.len();
    let mut first_button = None;
    for (index, (target, icon_name, key)) in targets.into_iter().enumerate() {
        let detail = format!("#{}", target.flake(&config).unwrap_or("User environment"));
        let action = home_action(target.name(), &detail, icon_name, key, true, &state);
        first_button.get_or_insert_with(|| action.button.clone());
        let window = window.clone();
        let root = root.clone();
        let callback_state = Rc::clone(&state);
        let config = config.clone();
        action.button.connect_clicked(move |_| {
            start_build(
                &window,
                &root,
                Rc::clone(&callback_state),
                config.clone(),
                target,
            );
        });
        configuration_group.append(&action.button);
        if index + 1 < target_count {
            configuration_group.append(&home_separator());
        }
    }
    configuration.append(&configuration_group);
    home.append(&configuration);

    let maintenance = gtk::Box::new(gtk::Orientation::Vertical, 8);
    maintenance.add_css_class("home-section");
    maintenance.append(&home_section_label("Maintenance"));
    let maintenance_group = home_group();
    let repository_check = home_action(
        "Checking repository…",
        &config.flake_dir.display().to_string(),
        "folder-remote-symbolic",
        'R',
        false,
        &state,
    );
    repository_check.chevron.set_visible(false);
    set_repository_badge(&repository_check, "Checking", None);
    let repository_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    repository_row.add_css_class("repository-row");
    repository_check.button.set_hexpand(true);
    repository_row.append(&repository_check.button);
    let repository_update = repository_update_action(&state);
    repository_row.append(&repository_update.button);
    maintenance_group.append(&repository_row);
    maintenance.append(&maintenance_group);
    home.append(&maintenance);
    home.append(&home_footer(window, &state));
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
    heading.append(&label("Choose an action", &["home-subtitle"], 0.0));
    header.append(&heading);
    header
}

fn home_footer(window: &gtk::ApplicationWindow, state: &Rc<UiState>) -> gtk::Box {
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    footer.add_css_class("home-footer");
    footer.append(&action_close_button(
        window,
        Rc::clone(state),
        "report-close-button",
        "report-close-label",
    ));
    footer
}

fn home_section_label(text: &str) -> gtk::Label {
    label(text, &["home-section-label"], 0.0)
}

fn home_group() -> gtk::Box {
    let group = gtk::Box::new(gtk::Orientation::Vertical, 0);
    group.add_css_class("home-group");
    group
}

fn home_separator() -> gtk::Separator {
    let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
    separator.add_css_class("home-separator");
    separator
}

fn home_action(
    title_text: &str,
    detail_text: &str,
    icon_name: &str,
    key: char,
    branch_detail: bool,
    state: &Rc<UiState>,
) -> HomeActionWidgets {
    let button = gtk::Button::new();
    button.add_css_class("home-action");

    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_valign(gtk::Align::Center);
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.add_css_class("home-action-icon");
    icon.set_pixel_size(20);
    icon.set_size_request(32, 32);
    icon.set_valign(gtk::Align::Center);
    row.append(&icon);

    let copy = gtk::Box::new(gtk::Orientation::Vertical, 2);
    copy.set_hexpand(true);
    let title = label(title_text, &["home-action-title"], 0.0);
    copy.append(&title);
    let detail_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    if branch_detail {
        detail_row.append(&label(
            "╰── ",
            &["metric-branch", "home-action-branch"],
            0.0,
        ));
    }
    let detail = label(detail_text, &["home-action-detail"], 0.0);
    detail.set_hexpand(true);
    detail_row.append(&detail);
    copy.append(&detail_row);
    row.append(&copy);

    let badge = label("", &["home-action-badge"], 0.5);
    badge.set_valign(gtk::Align::Center);
    let badge_container = gtk::Overlay::new();
    badge_container.set_visible(false);
    badge_container.set_valign(gtk::Align::Center);
    badge_container.set_child(Some(&badge));
    let energy = EnergyIndicator::new();
    energy.root.set_content_width(0);
    energy.root.set_content_height(0);
    energy.root.set_halign(gtk::Align::Fill);
    energy.root.set_valign(gtk::Align::Fill);
    energy.root.set_hexpand(true);
    energy.root.set_vexpand(true);
    badge_container.add_overlay(&energy.root);
    row.append(&badge_container);
    if state.appearance.keybinds {
        row.append(&label(
            &key.to_string(),
            &["keycap", "home-action-key"],
            0.5,
        ));
    }
    let chevron = gtk::Image::from_icon_name("go-next-symbolic");
    chevron.add_css_class("home-action-chevron");
    chevron.set_pixel_size(14);
    row.append(&chevron);

    button.set_child(Some(&row));
    state.home_buttons.borrow_mut().push(button.clone());
    state.register_home_action(key, &button);
    HomeActionWidgets {
        button,
        title,
        detail,
        badge,
        badge_container,
        chevron,
        energy,
    }
}

fn repository_update_action(state: &Rc<UiState>) -> RepositoryUpdateWidgets {
    let button = gtk::Button::new();
    button.add_css_class("repository-update-button");
    button.set_tooltip_text(Some(if state.appearance.keybinds {
        "Fast-forward or rebase incoming changes (P)"
    } else {
        "Fast-forward or rebase incoming changes"
    }));
    button.set_valign(gtk::Align::Center);
    button.set_visible(false);

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let update_label = label("Pull", &["repository-update-label"], 0.5);
    content.append(&update_label);
    if state.appearance.keybinds {
        content.append(&label("P", &["keycap", "keycap-subtle"], 0.5));
    }
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&content));
    let energy = EnergyIndicator::new();
    energy.root.set_content_width(0);
    energy.root.set_content_height(0);
    energy.root.set_halign(gtk::Align::Fill);
    energy.root.set_valign(gtk::Align::Fill);
    energy.root.set_hexpand(true);
    energy.root.set_vexpand(true);
    overlay.add_overlay(&energy.root);
    button.set_child(Some(&overlay));
    state.register_home_action('P', &button);

    RepositoryUpdateWidgets {
        button,
        label: update_label,
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
    widgets.check.title.set_text("Checking repository…");
    widgets
        .check
        .detail
        .set_text(&flake_dir.display().to_string());
    set_repository_badge(&widgets.check, "Checking", None);
    widgets.check.energy.start();
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
                widgets.check.energy.stop();
                render_repository_status(&widgets, &status);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                widgets.busy.set(false);
                widgets.check.energy.stop();
                render_repository_check_error(&widgets, &error);
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                widgets.busy.set(false);
                widgets.check.energy.stop();
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
    widgets.check.title.set_text("Check repository");
    widgets
        .check
        .detail
        .set_text(&format!("{} · {}", status.kind.name(), status.detail()));
    if status.incoming > 0 {
        set_repository_badge(
            &widgets.check,
            &format!("{} incoming", status.incoming),
            Some("attention"),
        );
        widgets
            .update
            .label
            .set_text(&format!("Pull {}", status.incoming));
        widgets.update.button.remove_css_class("error");
        widgets
            .update
            .button
            .set_tooltip_text(Some("Fast-forward or rebase incoming changes"));
        widgets.update.button.set_visible(true);
    } else {
        set_repository_badge(&widgets.check, "Synced", Some("success"));
        widgets.update.button.set_visible(false);
    }
}

fn render_repository_check_error(widgets: &RepositoryWidgets, error: &str) {
    widgets.check.title.set_text("Retry repository check");
    widgets.check.detail.set_text(&single_line(error));
    set_repository_badge(&widgets.check, "Check failed", Some("error"));
    widgets.update.button.set_visible(false);
}

fn render_repository_update_error(widgets: &RepositoryWidgets, error: &str) {
    widgets.update.label.set_text("Retry");
    widgets.update.button.add_css_class("error");
    widgets
        .update
        .button
        .set_tooltip_text(Some(&single_line(error)));
    widgets.update.button.set_visible(true);
}

fn set_repository_badge(action: &HomeActionWidgets, text: &str, state: Option<&str>) {
    for class in ["success", "attention", "error"] {
        action.badge.remove_css_class(class);
    }
    if let Some(state) = state {
        action.badge.add_css_class(state);
    }
    action.badge.set_text(text);
    action.badge_container.set_visible(true);
}

fn single_line(message: &str) -> String {
    message.lines().next().unwrap_or(message).trim().to_owned()
}
