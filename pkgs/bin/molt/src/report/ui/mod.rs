//! The report screen, and the changelog and file-diff screens it opens.

mod changelog;
pub(crate) mod config_diff;
mod lists;
pub(crate) mod release_notes;
mod side;

use std::cell::OnceCell;
use std::rc::Rc;
use std::time::Duration;

use gtk::prelude::*;
use molt::protocol::ActivationMode;

use crate::activation::ui::{
    SwitchView, connect_switch, current_hostname, requires_host_switch_confirmation, switch_action,
    switch_animation,
};
use crate::app::state::UiState;
use crate::build::Target;
use crate::build::ui::timeline::{BuildTimeline, phase_header};
use crate::home::ui::back_to_home_button;
use crate::report::{ChangeStatus, Report};
use crate::ui::{
    Slide, action_close_button, fit_screen, key_hints, label, minimize_button, phase_grid,
    replace_screen, slide_screen,
};
use config_diff::show_config_diff;
use lists::{Entry, change_list, store_path_footer};
use release_notes::show_changelog;
use side::side_column;

/// The change list's tab, filter text, and scroll offset, kept across a visit to a changelog or
/// a file diff.
#[derive(Clone, Default)]
pub(crate) struct ReportPosition {
    tab: Option<ChangeStatus>,
    filter: String,
    scroll: f64,
}

#[derive(Clone)]
struct ReportNavigation {
    window: gtk::ApplicationWindow,
    root: gtk::Box,
    state: Rc<UiState>,
    report: Report,
}

pub(crate) fn show_report(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    report: Report,
    restore: Option<ReportPosition>,
) {
    state.clear_actions();
    fit_screen(window, &state);
    // A saved position means the report is being returned to from one of its detail screens.
    if restore.is_some() {
        slide_screen(root, Slide::Back);
    } else {
        replace_screen(root);
    }
    root.remove_css_class("home-root");

    let timeline = BuildTimeline::new(&state.appearance.sans_font);
    timeline.set_complete();
    root.append(
        &phase_header(
            ready_title(report.target),
            &report_subtitle(&report),
            &timeline,
        )
        .root,
    );

    let navigation = ReportNavigation {
        window: window.clone(),
        root: root.clone(),
        state: Rc::clone(&state),
        report: report.clone(),
    };
    let entrance = restore.is_none();
    let packages = changes_panel(&navigation, restore);

    let activity = switch_animation(
        report.target.name(),
        &report.flake,
        &state.appearance.sans_font,
    );
    activity.root.set_valign(gtk::Align::Start);
    let surface = gtk::Stack::new();
    surface.set_transition_type(gtk::StackTransitionType::Crossfade);
    surface.set_transition_duration(220);
    surface.set_hhomogeneous(true);
    surface.set_vhomogeneous(false);
    let side = side_column(&report, entrance);
    surface.add_child(&side);
    surface.add_child(&activity.root);
    surface.set_visible_child(&side);
    root.append(&phase_grid(&packages, &surface));

    let requires_confirmation = requires_host_switch_confirmation(
        report.target,
        &report.flake,
        current_hostname().as_deref(),
    );
    let switch = switch_action(ActivationMode::Switch, requires_confirmation);
    let minimize = minimize_button(&state);
    minimize.set_visible(false);
    let boot = (report.target == Target::NixOs)
        .then(|| switch_action(ActivationMode::Boot, requires_confirmation));
    let mut confirmations = vec![connect_switch(
        &switch,
        &SwitchView {
            activity: activity.clone(),
            surface: surface.clone(),
            siblings: boot.iter().map(|boot| boot.button.clone()).collect(),
            minimize: minimize.clone(),
        },
        report.clone(),
        Rc::clone(&state),
        requires_confirmation,
    )];
    if let Some(boot) = &boot {
        confirmations.push(connect_switch(
            boot,
            &SwitchView {
                activity,
                surface,
                siblings: vec![switch.button.clone()],
                minimize: minimize.clone(),
            },
            report.clone(),
            Rc::clone(&state),
            requires_confirmation,
        ));
    }
    state.switch_confirmations.replace(confirmations);

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.add_css_class("report-footer");
    footer.add_css_class("report-actions");
    footer.append(&back_to_home_button(window, root, &state));
    footer.append(&action_close_button(
        window,
        Rc::clone(&state),
        "report-close-button",
        "report-close-label",
    ));
    if state.appearance.keybinds {
        let config_changes = report
            .config
            .as_ref()
            .map_or(0, |config| config.changes.len());
        let mut hints = if report.changes.is_empty() && config_changes == 0 {
            Vec::new()
        } else {
            vec![("↑", "Scroll Up"), ("↓", "Scroll Down")]
        };
        hints.extend([("S", "Switch now"), ("←", "Back"), ("Esc", "Close")]);
        let hints = key_hints(&hints);
        hints.set_hexpand(true);
        hints.set_halign(gtk::Align::Center);
        hints.set_valign(gtk::Align::Center);
        footer.append(&hints);
    } else {
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        footer.append(&spacer);
    }
    footer.append(&minimize);
    if let Some(boot) = &boot {
        footer.append(&boot.energy.root);
    }
    footer.append(&switch.energy.root);
    root.append(&footer);
}

/// The report's title: what the built configuration is ready for.
pub(crate) const fn ready_title(target: Target) -> &'static str {
    match target {
        Target::NixOs => "Ready to switch",
        Target::HomeManager => "Ready to activate",
    }
}

fn report_subtitle(report: &Report) -> String {
    let subtitle = format!("{} #{}", report.target.name(), report.flake);
    match report.built_in {
        Some(duration) => format!("{subtitle} · built in {}", clock(duration)),
        None => subtitle,
    }
}

/// `1:52`, `1:04:09`: a stopwatch reading.
fn clock(duration: Duration) -> String {
    let seconds = duration.as_secs();
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

fn eyebrow(text: &str) -> gtk::Label {
    let eyebrow = label(text, &["panel-eyebrow"], 0.0);
    eyebrow.set_ellipsize(gtk::pango::EllipsizeMode::None);
    eyebrow.set_valign(gtk::Align::Center);
    eyebrow
}

fn changes_panel(navigation: &ReportNavigation, restore: Option<ReportPosition>) -> gtk::Box {
    let report = &navigation.report;
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    panel.add_css_class("phase-panel");
    // Filled once the list exists; a row opening a changelog or diff reads it to come back here.
    let snapshot: Rc<OnceCell<Rc<dyn Fn() -> ReportPosition>>> = Rc::default();

    let (files, footer) = match &report.config {
        Ok(config) => (
            config.changes.as_slice(),
            (!config.store_path_only.is_empty())
                .then(|| store_path_footer(&config.store_path_only)),
        ),
        Err(error) => {
            let note = label(
                &format!("Configuration comparison unavailable: {error}"),
                &["report-empty"],
                0.0,
            );
            note.set_wrap(true);
            (&[][..], Some(note.upcast()))
        }
    };
    let entries = report
        .changes
        .iter()
        .cloned()
        .map(Entry::Package)
        .chain(files.iter().cloned().map(Entry::File))
        .collect();
    let open = {
        let target = navigation.clone();
        let snapshot = Rc::clone(&snapshot);
        Rc::new(move |entry: &Entry| {
            let position = snapshot
                .get()
                .map(|snapshot| snapshot())
                .unwrap_or_default();
            let (window, root, state) = (&target.window, &target.root, Rc::clone(&target.state));
            match entry {
                Entry::Package(change) => {
                    show_changelog(window, root, change, state, target.report.clone(), position);
                }
                Entry::File(file) => {
                    show_config_diff(window, root, file, state, target.report.clone(), position);
                }
            }
        })
    };
    let list = change_list(
        entries,
        "No package or configuration changes",
        footer,
        open,
        &navigation.state,
        restore.as_ref(),
    );

    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    bar.add_css_class("report-packages-bar");
    bar.append(&list.controls);
    panel.append(&bar);
    panel.append(&list.body);
    navigation.state.scroll.replace(list.adjustment);
    let _ = snapshot.set(list.snapshot);
    panel
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_numbers() {
        assert_eq!(clock(Duration::from_secs(112)), "1:52");
        assert_eq!(clock(Duration::from_secs(3849)), "1:04:09");
    }
}
