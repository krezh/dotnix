use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use gtk::glib;
use gtk::prelude::*;

use crate::build::{BuildEvent, BuildPhase, BuildUpdate, Target, build_report};
use crate::config::{Config, load_config};
use crate::nix::NixBuildProgress;
use crate::state::{Operation, UiState};
use crate::ui::build_progress::{
    BuildPlan, append_evaluation_warnings, build_summary, render_build_progress,
    render_flake_fetches,
};
use crate::ui::common::{clear, fit_window, label, target_subtitle, wide_header};
use crate::ui::error::show_error;
use crate::ui::home::show_home;
use crate::ui::report::show_report;
use crate::ui::timeline::BuildTimeline;
use swix::command;

pub(crate) fn start_build(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    config: Config,
    target: Target,
) {
    let Some((generation, cancellation)) = state.begin(Operation::Building) else {
        return;
    };
    state.clear_actions();
    fit_window(window, (1000, 620), (760, 500));
    clear(root);
    root.remove_css_class("home-root");
    let flake = target.flake(&config).unwrap_or("<unset>");
    let timeline = BuildTimeline::new(&state.appearance.sans_font);
    timeline.start_pulse();
    let action_slot = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    action_slot.set_halign(gtk::Align::End);
    let elapsed = label("0s", &["build-elapsed"], 1.0);
    elapsed.set_valign(gtk::Align::Center);
    action_slot.append(&elapsed);
    let cancel = gtk::Button::with_label("Cancel");
    cancel.add_css_class("report-close-button");
    let cancel_window = window.clone();
    let cancel_root = root.clone();
    let cancel_state = Rc::clone(&state);
    cancel.connect_clicked(move |_| {
        cancel_state.cancel();
        show_home(
            &cancel_window,
            &cancel_root,
            Rc::clone(&cancel_state),
            load_config(),
        );
    });
    let header = wide_header(&timeline.root, &action_slot);
    header.append(&target_subtitle(target, flake));
    root.append(&header);
    let visualization = gtk::Box::new(gtk::Orientation::Vertical, 12);
    visualization.add_css_class("build-visualization");
    visualization.set_vexpand(true);
    let build_heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let phase_spinner = gtk::Spinner::new();
    phase_spinner.set_size_request(18, 18);
    phase_spinner.start();
    build_heading.append(&phase_spinner);
    let status = gtk::Label::new(Some(target.progress_message()));
    status.add_css_class("loading-text");
    status.set_xalign(0.0);
    status.set_hexpand(true);
    build_heading.append(&status);
    visualization.append(&build_heading);
    let overview = build_summary(&state.appearance.sans_font);
    let dashboard = gtk::Box::new(gtk::Orientation::Horizontal, 18);
    dashboard.add_css_class("build-dashboard");
    dashboard.set_vexpand(true);

    let context_panel = gtk::Box::new(gtk::Orientation::Vertical, 6);
    context_panel.add_css_class("build-dashboard-panel");
    context_panel.add_css_class("build-context-panel");
    context_panel.add_css_class("evaluation-context-panel");
    context_panel.set_hexpand(true);
    let warning_heading = label("Warnings · 0", &["graph-heading"], 0.0);
    context_panel.append(&warning_heading);
    let warning_list = gtk::Box::new(gtk::Orientation::Vertical, 6);
    warning_list.add_css_class("evaluation-warnings");
    let warning_scroll = gtk::ScrolledWindow::new();
    warning_scroll.add_css_class("build-scroll");
    warning_scroll.set_vexpand(true);
    warning_scroll.set_propagate_natural_height(false);
    warning_scroll.set_min_content_height(72);
    warning_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    warning_scroll.set_child(Some(&warning_list));
    state.set_scroll_adjustment(warning_scroll.vadjustment());
    context_panel.append(&warning_scroll);
    let fetch_heading = label("Flake inputs · 0", &["graph-heading"], 0.0);
    context_panel.append(&fetch_heading);
    let fetch_list = gtk::Box::new(gtk::Orientation::Vertical, 3);
    fetch_list.add_css_class("flake-fetches");
    context_panel.append(&fetch_list);
    overview.root.set_hexpand(true);
    overview.root.set_vexpand(true);
    overview.root.set_visible(false);
    dashboard.prepend(&overview.root);
    dashboard.append(&context_panel);
    visualization.append(&dashboard);
    let latest_progress = Rc::new(RefCell::new(NixBuildProgress::default()));
    root.append(&visualization);
    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    footer.add_css_class("report-footer");
    let footer_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    footer_spacer.set_hexpand(true);
    footer.append(&footer_spacer);
    footer.append(&cancel);
    root.append(&footer);

    let (sender, receiver) = mpsc::channel();
    let build_started = Instant::now();
    thread::spawn(move || {
        let result = build_report(&config, target, generation, &cancellation, |update| {
            let _ = sender.send(BuildEvent::Update(update));
        });
        let _ = sender.send(BuildEvent::Finished(result));
    });
    let window = window.downgrade();
    let root = root.downgrade();
    let mut build_plan = BuildPlan::default();
    let mut current_phase = BuildPhase::Evaluate;
    let mut rendered_warning_count = 0;
    let mut flake_fetches = Vec::new();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        elapsed.set_text(&format!("{}s", build_started.elapsed().as_secs()));
        let mut phase = None;
        let mut nix_update = None;
        let mut finished = None;
        for event in receiver.try_iter() {
            match event {
                BuildEvent::Update(BuildUpdate::Phase(build_phase, update)) => {
                    phase = Some((build_phase, update));
                }
                BuildEvent::Update(BuildUpdate::Nix(update_phase, update)) => {
                    if state.generation.get() == generation
                        && update_phase == BuildPhase::Evaluate
                        && update.warnings.len() > rendered_warning_count
                    {
                        append_evaluation_warnings(
                            &warning_list,
                            &update.warnings[rendered_warning_count..],
                        );
                        rendered_warning_count = update.warnings.len();
                        warning_heading.set_text(&format!("Warnings · {rendered_warning_count}"));
                    }
                    nix_update = Some(update);
                }
                BuildEvent::Finished(result) => finished = Some(result),
            }
        }
        let render_progress = nix_update.is_some();
        if state.generation.get() == generation {
            if let Some((phase, message)) = phase {
                current_phase = phase;
                timeline.set_phase(phase);
                status.set_text(message);
                phase_spinner.set_visible(phase == BuildPhase::Evaluate);
                let evaluating = phase == BuildPhase::Evaluate;
                overview.root.set_visible(!evaluating);
                context_panel.set_hexpand(evaluating);
                context_panel.set_width_request(if evaluating { -1 } else { 310 });
                if evaluating {
                    context_panel.add_css_class("evaluation-context-panel");
                } else {
                    context_panel.remove_css_class("evaluation-context-panel");
                }
                if !flake_fetches.is_empty() {
                    render_flake_fetches(
                        &fetch_list,
                        &flake_fetches,
                        phase == BuildPhase::Evaluate,
                    );
                }
            }
            if let Some(progress) = nix_update {
                if !progress.flake_fetches.is_empty() {
                    flake_fetches.clone_from(&progress.flake_fetches);
                    fetch_heading.set_text(&format!("Flake inputs · {}", flake_fetches.len()));
                    render_flake_fetches(
                        &fetch_list,
                        &flake_fetches,
                        current_phase == BuildPhase::Evaluate,
                    );
                }
                latest_progress.replace(*progress);
            }
            if render_progress {
                render_build_progress(&overview, &latest_progress.borrow(), &mut build_plan);
            }
        }
        if let Some(result) = finished {
            if state.finish(generation)
                && let (Some(window), Some(root)) = (window.upgrade(), root.upgrade())
            {
                if state.close_pending.replace(false) {
                    window.close();
                } else {
                    match result {
                        Ok(report) => show_report(&window, &root, Rc::clone(&state), report, None),
                        Err(error) if error == command::CANCELLED => {
                            show_home(&window, &root, Rc::clone(&state), load_config())
                        }
                        Err(error) => show_error(&window, &root, Rc::clone(&state), &error),
                    }
                }
            }
            return glib::ControlFlow::Break;
        }
        glib::ControlFlow::Continue
    });
}
