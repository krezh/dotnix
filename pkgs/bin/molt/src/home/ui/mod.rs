mod repository;

use std::cell::Cell;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use gtk::prelude::*;

use crate::build::{Target, active_profile};
use crate::cleanup::CleanupEstimate;
use crate::config::{Config, load_config};
use crate::inputs::{self, FlakeInput};
use crate::app::state::UiState;
use crate::ui::theme;
use crate::home::system::{self, SystemStatus};
use crate::build::ui::{AfterBuild, start_build};
use crate::cleanup::ui::show_cleanup;
use crate::ui::{
    MorphLabel, action_close_button, back_button, duration_text, fit_window, format_size,
    format_timestamp, label, replace_screen, seconds_since, short_age, title,
};
use crate::inputs::ui::show_inputs;
use crate::report::ui::show_report;
use crate::watchlist::db::{open_db, summary};
use crate::watchlist::ui::show_watchlist;
use crate::ui::EnergyOverlay;
use repository::{
    RepositoryCheckWidgets, RepositoryWidgets, connect_repository_actions,
    repository_update_action, start_repository_check,
};

#[derive(Clone)]
struct HomeCardWidgets {
    root: gtk::Box,
    detail: MorphLabel,
    badge: MorphLabel,
    badge_container: EnergyOverlay,
    actions: gtk::Box,
}

pub(crate) fn back_to_home_button(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: &Rc<UiState>,
) -> gtk::Button {
    let tooltip = if state.appearance.keybinds {
        "Back to Molt home (←)"
    } else {
        "Back to Molt home"
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
    replace_screen(root);
    root.add_css_class("home-root");
    let Ok(config) = config else {
        let error = gtk::Label::new(config.err().as_deref());
        error.add_css_class("error");
        error.set_wrap(true);
        root.append(&error);
        return;
    };

    let preferred_width = 900 + if state.appearance.keybinds { 60 } else { 0 };
    fit_window(window, &state, (preferred_width, 520), (720, 460));

    let home = gtk::Box::new(gtk::Orientation::Vertical, 14);
    home.add_css_class("home");
    home.set_vexpand(true);
    let system = system::system_status();
    home.append(&home_header(&config, &system));
    let flake_inputs = inputs::load(&config.flake_dir)
        .ok()
        .map(|(_, inputs)| inputs);
    home.append(&home_status_strip(&system, flake_inputs.as_deref()));

    let mut cards = Vec::new();
    let mut targets = vec![(Target::NixOs, "drive-harddisk-symbolic", 'N', theme::ACCENT)];
    if config.home_flake.is_some() {
        targets.insert(
            0,
            (Target::HomeManager, "user-home-symbolic", 'M', theme::HOME),
        );
    }
    let mut first_button = None;
    for (target, icon_name, key, tone) in targets {
        let detail = target.flake(&config).ok().map_or_else(
            || "User environment".to_owned(),
            |flake| format!("#{flake} · build, review, switch"),
        );
        let card = home_card(target.name(), &detail, icon_name, tone);
        if let Some(report) = retained_report(&state, target) {
            set_card_status(&card, "Reviewed build ready", Some("success"));
            let (review, _) = home_card_action("Review", None, &state);
            review.set_tooltip_text(Some("Reopen the latest reviewed build"));
            let review_window = window.clone();
            let review_root = root.clone();
            let review_state = Rc::clone(&state);
            review.connect_clicked(move |_| {
                show_report(
                    &review_window,
                    &review_root,
                    Rc::clone(&review_state),
                    report.clone(),
                    None,
                );
            });
            card.actions.append(&review);
        }
        for (text, shortcut, after) in [
            ("Build & switch", None, AfterBuild::Switch),
            ("Build", Some(key), AfterBuild::Review),
        ] {
            let (button, _) = home_card_action(text, shortcut, &state);
            match after {
                AfterBuild::Review => {
                    button.remove_css_class("surface-button");
                    button.add_css_class("accent-button");
                    first_button.get_or_insert_with(|| button.clone());
                }
                AfterBuild::Switch => button.set_tooltip_text(Some(&format!(
                    "Build {} and switch to it without stopping to review",
                    target.name()
                ))),
            }
            let build_window = window.clone();
            let build_root = root.clone();
            let callback_state = Rc::clone(&state);
            let config = config.clone();
            button.connect_clicked(move |_| {
                start_build(
                    &build_window,
                    &build_root,
                    Rc::clone(&callback_state),
                    config.clone(),
                    target,
                    after,
                );
            });
            card.actions.append(&button);
        }
        cards.push(card.root);
    }

    let repository_card = home_card(
        "Repository",
        "",
        "folder-remote-symbolic",
        theme::PINK,
    );
    set_card_status(&repository_card, "Checking", None);
    let (repository_check_button, repository_check_label) =
        home_card_action("Check", Some('R'), &state);
    repository_card.actions.append(&repository_check_button);
    let repository_update = repository_update_action(&state);
    repository_card.actions.append(&repository_update.reveal);
    cards.push(repository_card.root.clone());
    let repository_check = RepositoryCheckWidgets {
        button: repository_check_button,
        label: repository_check_label,
        card: repository_card,
    };

    let input_count = flake_inputs.as_ref().map_or_else(
        || "Manage flake.lock".to_owned(),
        |inputs| format!("{} direct inputs", inputs.len()),
    );
    let inputs_card = home_card(
        "Inputs",
        &input_count,
        "view-list-symbolic",
        theme::VERSION,
    );
    if let Some(summary) = flake_inputs.as_deref().and_then(input_age_summary) {
        set_card_status(&inputs_card, &summary, Some("muted"));
    }
    let (inputs_button, _) = home_card_action("Manage", Some('I'), &state);
    let inputs_window = window.clone();
    let inputs_root = root.clone();
    let inputs_state = Rc::clone(&state);
    let inputs_config = config.clone();
    inputs_button.connect_clicked(move |_| {
        show_inputs(
            &inputs_window,
            &inputs_root,
            Rc::clone(&inputs_state),
            inputs_config.clone(),
        );
    });
    inputs_card.actions.append(&inputs_button);
    cards.push(inputs_card.root);

    let cleanup_card = home_card(
        "Cleanup",
        "Generations, journals, caches, trash",
        "user-trash-symbolic",
        theme::PEACH,
    );
    cleanup_card.detail.root.set_tooltip_text(Some(
        "Generations, journals, containers, build artifacts, caches, game leftovers, and trash",
    ));
    set_card_status(
        &cleanup_card,
        &CleanupEstimate::load().map_or_else(
            || "Not scanned yet".to_owned(),
            |estimate| {
                format!(
                    "{} reclaimable · scanned {} ago",
                    format_size(estimate.bytes),
                    duration_text(seconds_since(SystemTime::now(), estimate.scanned_at))
                )
            },
        ),
        Some("muted"),
    );
    let (cleanup_button, _) = home_card_action("Clean up", Some('C'), &state);
    let cleanup_window = window.clone();
    let cleanup_root = root.clone();
    let cleanup_state = Rc::clone(&state);
    let cleanup_policy = config.cleanup.clone();
    cleanup_button.connect_clicked(move |_| {
        show_cleanup(
            &cleanup_window,
            &cleanup_root,
            Rc::clone(&cleanup_state),
            cleanup_policy.clone(),
        );
    });
    cleanup_card.actions.append(&cleanup_button);
    cards.push(cleanup_card.root);

    let watchlist_summary = open_db()
        .ok()
        .flatten()
        .and_then(|conn| summary(&conn).ok())
        .unwrap_or_default();
    let watchlist_detail = if watchlist_summary.total == 0 {
        "Track PRs across branches".to_owned()
    } else if watchlist_summary.ready_in_unstable > 0 {
        format!(
            "{} tracked · {} ready in nixos-unstable",
            watchlist_summary.total, watchlist_summary.ready_in_unstable
        )
    } else {
        format!("{} tracked · waiting on branches", watchlist_summary.total)
    };
    let watchlist_card = home_card(
        "Watchlist",
        &watchlist_detail,
        "eye-symbolic",
        theme::LAVENDER,
    );
    if watchlist_summary.ready_in_unstable > 0 {
        set_card_status(
            &watchlist_card,
            &format!("{} ready", watchlist_summary.ready_in_unstable),
            Some("success"),
        );
    } else if watchlist_summary.total > 0 {
        set_card_status(&watchlist_card, "Waiting", Some("muted"));
    }
    let (watchlist_button, _) = home_card_action("Track", Some('W'), &state);
    let watchlist_window = window.clone();
    let watchlist_root = root.clone();
    let watchlist_state = Rc::clone(&state);
    let watchlist_config = config.clone();
    watchlist_button.connect_clicked(move |_| {
        show_watchlist(
            &watchlist_window,
            &watchlist_root,
            Rc::clone(&watchlist_state),
            watchlist_config.clone(),
        );
    });
    watchlist_card.actions.append(&watchlist_button);
    cards.push(watchlist_card.root);

    let dashboard = gtk::Grid::new();
    dashboard.set_column_spacing(10);
    dashboard.set_row_spacing(10);
    dashboard.set_column_homogeneous(true);
    dashboard.set_vexpand(false);
    let count = cards.len();
    for (index, card) in cards.iter().enumerate() {
        // An odd card out spans the full row instead of leaving a hole beside it.
        let width = if index + 1 == count && count % 2 == 1 { 2 } else { 1 };
        dashboard.attach(card, (index % 2) as i32, (index / 2) as i32, width, 1);
    }

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
    // Click handlers hold the card weakly; the home page owns it and frees it on dispose.
    home.connect_destroy(move |_| {
        let _owned = &widgets;
    });
}

fn home_header(config: &Config, system: &SystemStatus) -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    header.set_valign(gtk::Align::Start);

    let heading = gtk::Box::new(gtk::Orientation::Vertical, 0);
    heading.set_hexpand(true);
    heading.append(&title("Molt"));
    let host = Target::NixOs.flake(config).ok();
    let version = system.version.as_deref().map(|version| format!("NixOS {version}"));
    let subtitle = match (host, version) {
        (Some(host), Some(version)) => format!("{host} · {version}"),
        (Some(host), None) => host.to_owned(),
        (None, Some(version)) => version,
        (None, None) => "System configuration and maintenance".to_owned(),
    };
    heading.append(&label(&subtitle, &["home-subtitle"], 0.0));
    header.append(&heading);
    header
}

fn home_status_strip(system: &SystemStatus, inputs: Option<&[FlakeInput]>) -> gtk::Box {
    let strip = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    strip.add_css_class("home-status");
    strip.add_css_class("phase-panel");
    strip.set_homogeneous(true);
    let now = SystemTime::now();

    let generation = system
        .generation
        .map_or_else(|| "—".to_owned(), |generation| generation.to_string());
    let generation_item = status_item("GENERATION", &generation, "running");
    if let Some(staged) = system.staged {
        let chip = label(
            &format!("{staged} on reboot"),
            &["chip", "tinted", theme::tone(theme::YELLOW)],
            0.5,
        );
        chip.set_valign(gtk::Align::Center);
        chip.set_tooltip_text(Some(&format!(
            "Generation {staged} is applied and becomes active after the next reboot"
        )));
        if let Some(row) = generation_item.last_child().and_downcast::<gtk::Box>() {
            row.append(&chip);
        }
    }
    strip.append(&generation_item);

    let (activated, activated_at) = system.activated.map_or_else(
        || ("Unknown".to_owned(), String::new()),
        |activated| {
            let elapsed = now.duration_since(activated).unwrap_or_default().as_secs();
            let timestamp = activated
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.as_secs() as i64);
            (
                format!("{} ago", duration_text(elapsed)),
                format_timestamp(timestamp, "%a %H:%M").unwrap_or_default(),
            )
        },
    );
    strip.append(&status_item("ACTIVATED", &activated, &activated_at));

    let nixpkgs = inputs
        .into_iter()
        .flatten()
        .find(|input| input.name == "nixpkgs");
    let (nixpkgs_age, nixpkgs_revision) = nixpkgs.map_or_else(
        || ("—".to_owned(), String::new()),
        |input| {
            let age = input.last_modified.map_or_else(
                || "Unknown".to_owned(),
                |modified| format!("{} old", duration_text(seconds_since(now, modified))),
            );
            (age, input.revision.chars().take(7).collect())
        },
    );
    strip.append(&status_item("NIXPKGS", &nixpkgs_age, &nixpkgs_revision));
    strip
}

fn status_item(caption: &str, value: &str, detail: &str) -> gtk::Box {
    let item = gtk::Box::new(gtk::Orientation::Vertical, 2);
    item.add_css_class("home-status-item");
    item.append(&label(caption, &["panel-eyebrow"], 0.0));
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.append(&label(value, &["home-status-value"], 0.0));
    if !detail.is_empty() {
        let detail = label(detail, &["home-status-detail"], 0.0);
        detail.set_valign(gtk::Align::Baseline);
        detail.set_ellipsize(gtk::pango::EllipsizeMode::End);
        row.append(&detail);
    }
    item.append(&row);
    item
}

/// `nixpkgs 12d · oldest 41d`: how stale the lock is, from the inputs' locked timestamps.
fn input_age_summary(inputs: &[FlakeInput]) -> Option<String> {
    let now = SystemTime::now();
    let nixpkgs = inputs
        .iter()
        .find(|input| input.name == "nixpkgs")
        .and_then(|input| input.last_modified);
    let oldest = inputs.iter().filter_map(|input| input.last_modified).min();
    match (nixpkgs, oldest) {
        (Some(nixpkgs), Some(oldest)) => Some(format!(
            "nixpkgs {} · oldest {}",
            short_age(seconds_since(now, nixpkgs)),
            short_age(seconds_since(now, oldest))
        )),
        (None, Some(oldest)) => Some(format!(
            "oldest locked {}",
            short_age(seconds_since(now, oldest))
        )),
        _ => None,
    }
}

fn retained_report(state: &UiState, target: Target) -> Option<crate::report::Report> {
    let report = state.reports.borrow().get(&target).cloned()?;
    if active_profile(target).ok().as_ref() == Some(&report.baseline) {
        Some(report)
    } else {
        state.reports.borrow_mut().remove(&target);
        None
    }
}

/// `tone` tints the icon tile so each card is recognisable by colour.
fn home_card(
    title_text: &str,
    detail_text: &str,
    icon_name: &str,
    tone: theme::Color,
) -> HomeCardWidgets {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.add_css_class("home-card");
    root.set_hexpand(true);
    root.set_vexpand(false);

    let card_header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.add_css_class("home-card-icon");
    icon.add_css_class("tinted");
    icon.add_css_class(theme::tone(tone));
    icon.set_pixel_size(16);
    icon.set_size_request(34, 34);
    icon.set_valign(gtk::Align::Start);
    card_header.append(&icon);
    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_hexpand(true);
    text.append(&label(title_text, &["home-card-title"], 0.0));
    let detail = MorphLabel::new(detail_text, &["home-card-detail"], 0.0);
    text.append(&detail.root);
    card_header.append(&text);
    root.append(&card_header);

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.add_css_class("home-card-footer");
    footer.set_vexpand(true);
    footer.set_valign(gtk::Align::End);
    let badge = MorphLabel::new("", &["home-card-badge", "chip"], 0.5);
    badge.root.set_valign(gtk::Align::Center);
    let badge_container = EnergyOverlay::new(&badge.root, 6.0);
    badge_container.root.set_visible(false);
    badge_container.root.set_halign(gtk::Align::Start);
    badge_container.root.set_valign(gtk::Align::Center);
    footer.append(&badge_container.root);
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_hexpand(true);
    actions.set_halign(gtk::Align::End);
    actions.set_valign(gtk::Align::Center);
    footer.append(&actions);
    root.append(&footer);

    HomeCardWidgets {
        root,
        detail,
        badge,
        badge_container,
        actions,
    }
}

fn home_card_action(
    text: &str,
    key: Option<char>,
    state: &Rc<UiState>,
) -> (gtk::Button, MorphLabel) {
    let button = gtk::Button::new();
    button.add_css_class("home-card-action");
    button.add_css_class("surface-button");
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    content.set_halign(gtk::Align::Center);
    content.set_valign(gtk::Align::Center);
    let action_label = MorphLabel::new(text, &["home-card-action-label"], 0.5);
    content.append(&action_label.root);
    if let Some(key) = key {
        if state.appearance.keybinds {
            content.append(&label(&key.to_string(), &["keycap", "keycap-subtle"], 0.5));
        }
        state.register_home_action(key, &button);
    }
    button.set_child(Some(&content));
    state.home_buttons.borrow_mut().push(button.clone());
    (button, action_label)
}

/// Shows the card's status pill: tinted green, yellow, or red for `success`, `attention`, and
/// `error`; plain secondary text for `muted`; neutral for `None`.
fn set_card_status(card: &HomeCardWidgets, text: &str, state: Option<&str>) {
    let tones = [theme::GREEN, theme::YELLOW, theme::RED].map(theme::tone);
    for class in tones.into_iter().chain(["tinted", "muted"]) {
        card.badge.remove_css_class(class);
    }
    let tone = match state {
        Some("success") => Some(theme::GREEN),
        Some("attention") => Some(theme::YELLOW),
        Some("error") => Some(theme::RED),
        Some(other) => {
            card.badge.add_css_class(other);
            None
        }
        None => None,
    };
    if let Some(tone) = tone {
        card.badge.add_css_class("tinted");
        card.badge.add_css_class(theme::tone(tone));
    }
    card.badge.set_text(text);
    card.badge_container.root.set_visible(true);
}

fn single_line(message: &str) -> String {
    message.lines().next().unwrap_or(message).trim().to_owned()
}
