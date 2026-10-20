//! The build screen: evaluation, building, and comparing, until the report replaces it.

pub(crate) mod activity;
pub(crate) mod failure;
pub(crate) mod progress;
pub(crate) mod timeline;

use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use gtk::glib;
use gtk::prelude::*;

use molt::command;

use crate::app::mini::{MiniEnd, MiniStatus};
use crate::app::state::{Operation, UiState};
use crate::build::{BuildEvent, BuildPhase, BuildUpdate, Target, build_report};
use crate::config::{Config, load_config};
use crate::home::ui::{back_to_home_button, show_home};
use crate::inputs::{self, FlakeInput};
use crate::nix::{FlakeFetch, NixBuildProgress, failed_derivations};
use crate::report::Report;
use crate::report::ui::{ready_title, show_report};
use crate::ui::{
    action_close_button, compact_size, fit_screen, glide, key_hints, label, minimize_button,
    phase_grid, replace_screen, signed_size,
};
use activity::GroupedList;
use failure::{ErrorCard, LogView, error_summary};
use progress::{
    DownloadsCard, EvaluationCard, INPUT_GROUPS, ProgressCard, WORK_GROUPS, WarningsCard, WorkPlan,
    WorkSummary, clock, download_bytes, fail_open_fetches, input_rows, last_evaluation,
    remember_evaluation,
};
use timeline::{BuildTimeline, PhaseHeader, phase_header, step_name};

fn phase_title(phase: BuildPhase, target: Target) -> String {
    let verb = match phase {
        BuildPhase::Evaluate => "Evaluating",
        BuildPhase::Build => "Building",
        BuildPhase::Compare => "Comparing",
    };
    format!("{verb} {}", target.name())
}

const fn failed_title(phase: BuildPhase) -> &'static str {
    match phase {
        BuildPhase::Evaluate => "Evaluation failed",
        BuildPhase::Build => "Build failed",
        BuildPhase::Compare => "Comparison failed",
    }
}

/// A stack that crossfades between its children, easing its height to the one shown.
fn crossfade() -> gtk::Stack {
    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_transition_duration(220);
    stack.set_vhomogeneous(false);
    stack.set_interpolate_size(true);
    stack
}

/// What a successful build leads to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AfterBuild {
    /// Show the report and wait for the user to switch.
    Review,
    /// Show the report and switch right away, as if Switch now had been clicked.
    Switch,
}

/// The build screen's widgets, kept so a failure can turn the same layout into its report.
struct BuildScreen {
    header: PhaseHeader,
    timeline: BuildTimeline,
    lists: gtk::Stack,
    input_list: Rc<GroupedList>,
    work_list: Rc<GroupedList>,
    /// The side column, crossfading to the error once the build fails.
    side: gtk::Stack,
    /// The evaluation card, crossfading to the build's cards once building starts.
    summary: gtk::Stack,
    build_cards: gtk::Box,
    evaluation: EvaluationCard,
    progress_card: ProgressCard,
    downloads: DownloadsCard,
    warnings: Rc<WarningsCard>,
    /// The footer, crossfading to the failure's actions.
    footer: gtk::Stack,
}

/// What the build had seen when it failed.
struct FailedBuild<'a> {
    after: AfterBuild,
    target: Target,
    phase: BuildPhase,
    error: &'a str,
    flake_inputs: &'a [FlakeInput],
    fetches: &'a [FlakeFetch],
    progress: &'a mut NixBuildProgress,
    plan: &'a mut WorkPlan,
}

pub(crate) fn start_build(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: Rc<UiState>,
    config: Config,
    target: Target,
    after: AfterBuild,
) {
    let Some((generation, cancellation)) = state.begin(Operation::Building) else {
        return;
    };
    state.clear_actions();
    fit_screen(window, &state);
    replace_screen(root);
    root.remove_css_class("home-root");
    let flake = target.flake(&config).unwrap_or("<unset>").to_owned();
    let timeline = BuildTimeline::new(&state.appearance.sans_font);
    timeline.start_pulse();
    let header = phase_header(
        &phase_title(BuildPhase::Evaluate, target),
        &format!("#{flake} · {}", config.flake_dir.display()),
        &timeline,
    );
    root.append(&header.root);

    let flake_inputs = inputs::load(&config.flake_dir)
        .map(|(_, inputs)| inputs)
        .unwrap_or_default();
    let input_list = GroupedList::new(
        "FLAKE INPUTS",
        INPUT_GROUPS,
        "No flake inputs to fetch",
        true,
    );
    let work_list = GroupedList::new(
        "ACTIVITY",
        WORK_GROUPS,
        "Waiting for nix to plan the build",
        false,
    );
    let lists = gtk::Stack::new();
    lists.set_transition_type(gtk::StackTransitionType::Crossfade);
    lists.set_transition_duration(220);
    lists.add_child(&input_list.root);
    lists.add_child(&work_list.root);
    lists.set_visible_child(&input_list.root);
    state.set_scroll_adjustment(input_list.scroll.vadjustment());

    let evaluation = EvaluationCard::new();
    let progress_card = ProgressCard::new();
    let downloads = DownloadsCard::new();
    let build_cards = gtk::Box::new(gtk::Orientation::Vertical, 14);
    build_cards.append(&progress_card.root);
    build_cards.append(&downloads.root);
    let summary = crossfade();
    summary.add_child(&evaluation.root);
    summary.add_child(&build_cards);
    let warnings = WarningsCard::new();
    let running_side = gtk::Box::new(gtk::Orientation::Vertical, 14);
    running_side.append(&summary);
    running_side.append(&warnings.root);
    let side = crossfade();
    side.add_child(&running_side);
    root.append(&phase_grid(&lists, &side));

    let running_footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let hint = label(
        match after {
            AfterBuild::Review => "Shows the report when the build finishes",
            AfterBuild::Switch => "Switches as soon as the build finishes",
        },
        &["phase-footer-hint"],
        0.0,
    );
    hint.set_hexpand(true);
    hint.set_valign(gtk::Align::Center);
    running_footer.append(&hint);
    running_footer.append(&minimize_button(&state));
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
    running_footer.append(&cancel);
    let footer = crossfade();
    footer.add_css_class("report-footer");
    footer.add_css_class("build-footer");
    footer.add_child(&running_footer);
    root.append(&footer);

    let screen = BuildScreen {
        header,
        timeline,
        lists,
        input_list,
        work_list,
        side,
        summary,
        build_cards,
        evaluation,
        progress_card,
        downloads,
        warnings,
        footer,
    };

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = build_report(&config, target, generation, &cancellation, |update| {
            let _ = sender.send(BuildEvent::Update(update));
        });
        let _ = sender.send(BuildEvent::Finished(Box::new(result)));
    });

    let started = Instant::now();
    let evaluation_estimate = last_evaluation(target);
    let mut phase = BuildPhase::Evaluate;
    let mut evaluation_done = None;
    let mut build_started = None;
    let mut compare_status = None;
    let mut fetches = Vec::new();
    let mut progress = NixBuildProgress::default();
    let mut plan = WorkPlan::default();
    let mut summary = WorkSummary::default();
    // Building rows, whose running time is the only thing that changes without new events.
    let mut running = Vec::new();
    let mut shown_second = None;
    // Snapshot warnings are cumulative per nix command; how many each has already shown.
    let mut evaluation_warnings = 0;
    let mut build_warnings = 0;
    let window = window.downgrade();
    let root = root.downgrade();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        let now = Instant::now();
        let current = state.generation.get() == generation;
        let mut finished = None;
        let mut consumed = false;
        let mut inputs_changed = false;
        // Evaluation snapshots are cumulative, so only the newest matters each tick; build
        // snapshots carry only the paths that changed, so each one is folded in.
        let mut evaluation_update = None;
        let mut build_updates = Vec::new();
        for event in receiver.try_iter() {
            consumed = true;
            match event {
                BuildEvent::Finished(result) => finished = Some(result),
                _ if !current => {}
                BuildEvent::Update(BuildUpdate::Phase(next, message)) => {
                    if next != phase {
                        if phase == BuildPhase::Evaluate {
                            evaluation_done = Some(now);
                            build_started = Some(now);
                            remember_evaluation(target, now - started);
                            inputs_changed = true;
                            screen.lists.set_visible_child(&screen.work_list.root);
                            state.set_scroll_adjustment(screen.work_list.scroll.vadjustment());
                            screen.summary.set_visible_child(&screen.build_cards);
                        }
                        phase = next;
                        screen.timeline.set_phase(next);
                        screen.header.title.set_text(&phase_title(next, target));
                    }
                    if next == BuildPhase::Compare {
                        compare_status = Some(message);
                    }
                }
                BuildEvent::Update(BuildUpdate::Nix(BuildPhase::Evaluate, update)) => {
                    evaluation_update = Some(update);
                }
                BuildEvent::Update(BuildUpdate::Nix(_, update)) => build_updates.push(update),
            }
        }
        if let Some(update) = evaluation_update {
            screen
                .warnings
                .extend(&update.warnings[evaluation_warnings..]);
            evaluation_warnings = update.warnings.len();
            fetches.clone_from(&update.flake_fetches);
            inputs_changed = true;
        }
        let mut work_changed = Vec::new();
        for update in build_updates {
            screen.warnings.extend(&update.warnings[build_warnings..]);
            build_warnings = update.warnings.len();
            work_changed.extend(plan.update(&update, now));
            progress = *update;
        }

        if current {
            if phase == BuildPhase::Evaluate || inputs_changed {
                let rows = input_rows(&flake_inputs, &fetches, evaluation_done);
                for row in &rows {
                    screen.input_list.upsert(&row.key, row.content(now));
                }
                screen.input_list.refresh();
                if phase == BuildPhase::Evaluate {
                    screen
                        .evaluation
                        .update(now - started, evaluation_estimate, &rows);
                }
            }
            if let Some(build_started) = build_started {
                let elapsed = now - build_started;
                if !work_changed.is_empty() {
                    for path in &work_changed {
                        if let Some(content) = plan.content(path, now) {
                            screen.work_list.upsert(path, content);
                        }
                    }
                    screen.work_list.refresh();
                    running = plan.running().map(str::to_owned).collect();
                    summary = plan.summary();
                }
                // Time labels show whole seconds, so idle ticks only redraw when one passes.
                let second_passed = shown_second != Some(elapsed.as_secs());
                if second_passed {
                    shown_second = Some(elapsed.as_secs());
                    for path in &running {
                        if let Some(content) = plan.content(path, now) {
                            screen.work_list.upsert(path, content);
                        }
                    }
                }
                if consumed || second_passed {
                    screen
                        .progress_card
                        .update(&summary, elapsed, compare_status);
                    screen.downloads.update(now, download_bytes(&progress));
                }
            }
            if let Some(mini) = state.mini.get() {
                let elapsed = now - started;
                let (note, progress) = match phase {
                    BuildPhase::Evaluate => match evaluation_estimate {
                        Some(estimate) => (
                            format!("usually {}", clock(estimate)),
                            Some((elapsed.as_secs_f64() / estimate.as_secs_f64()).min(0.95)),
                        ),
                        None => ("Fetching inputs and evaluating".to_owned(), None),
                    },
                    BuildPhase::Build => {
                        let finished = summary.done + summary.failed;
                        let mut note = format!("{finished} / {} built", summary.total);
                        let (downloaded, _) = download_bytes(&progress);
                        if downloaded > 0 {
                            note.push_str(" · ↓ ");
                            note.push_str(&compact_size(downloaded as f64));
                        }
                        let share =
                            (summary.total > 0).then(|| finished as f64 / summary.total as f64);
                        (note, share)
                    }
                    BuildPhase::Compare => (
                        compare_status.unwrap_or("Comparing closures").to_owned(),
                        None,
                    ),
                };
                mini.running(&MiniStatus {
                    step: step_name(phase),
                    title: &phase_title(phase, target),
                    note: &note,
                    elapsed,
                    progress,
                });
            }
        }

        let Some(result) = finished else {
            return glib::ControlFlow::Continue;
        };
        if state.finish(generation)
            && let (Some(window), Some(root)) = (window.upgrade(), root.upgrade())
        {
            if state.close_pending.replace(false) {
                window.close();
            } else {
                let took = started.elapsed();
                match *result {
                    Ok(report) => {
                        let note = report_note(&report);
                        state.reports.borrow_mut().insert(target, report.clone());
                        show_report(&window, &root, Rc::clone(&state), report, None);
                        if after == AfterBuild::Switch {
                            let switch = state.switch_confirmations.borrow().first().cloned();
                            if let Some(switch) = switch {
                                switch.activate();
                            }
                        }
                        // A switch started right away carries the bar on from here.
                        if state.operation.get() != Operation::Switching
                            && let Some(mini) = state.mini.get()
                        {
                            let status = MiniStatus {
                                step: "Ready",
                                title: ready_title(target),
                                note: &note,
                                elapsed: took,
                                progress: Some(1.0),
                            };
                            mini.finish(MiniEnd::Done, &status, "Review");
                        }
                    }
                    Err(error) if error == command::CANCELLED => {
                        show_home(&window, &root, Rc::clone(&state), load_config());
                    }
                    Err(error) => {
                        if let Some(mini) = state.mini.get() {
                            let summary = error_summary(&error);
                            let status = MiniStatus {
                                step: step_name(phase),
                                title: failed_title(phase),
                                note: summary.lines().next().unwrap_or_default(),
                                elapsed: took,
                                progress: None,
                            };
                            mini.finish(MiniEnd::Failed, &status, "Details");
                        }
                        show_failure(
                            &window,
                            &root,
                            &state,
                            &screen,
                            FailedBuild {
                                target,
                                after,
                                phase,
                                error: &error,
                                flake_inputs: &flake_inputs,
                                fetches: &fetches,
                                progress: &mut progress,
                                plan: &mut plan,
                            },
                        );
                    }
                }
            }
        }
        glib::ControlFlow::Break
    });
}

/// Turns the build screen into the failure report in place: failed rows sorted to the top, the
/// error in the side column, its build log a click away, and a way to retry.
fn show_failure(
    window: &gtk::ApplicationWindow,
    root: &gtk::Box,
    state: &Rc<UiState>,
    screen: &BuildScreen,
    failed: FailedBuild,
) {
    let now = Instant::now();
    screen.header.title.set_text(failed_title(failed.phase));
    screen.timeline.set_failed();

    let mut derivations = failed_derivations(failed.error);
    let list = if failed.phase == BuildPhase::Evaluate {
        let mut rows = input_rows(failed.flake_inputs, failed.fetches, None);
        fail_open_fetches(&mut rows);
        for row in &rows {
            screen.input_list.upsert(&row.key, row.content(now));
        }
        screen.input_list.refresh();
        Rc::clone(&screen.input_list)
    } else {
        // The last snapshot still lists the work in flight, which `interrupt` sends back to Queued.
        failed.progress.failed.extend(derivations.iter().cloned());
        let mut changed = failed.plan.update(failed.progress, now);
        changed.extend(failed.plan.interrupt());
        let mut known = failed
            .plan
            .failed()
            .filter(|path| path.ends_with(".drv") && !derivations.iter().any(|named| named == path))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        known.sort();
        derivations.extend(known);
        for path in &changed {
            if let Some(content) = failed.plan.content(path, now) {
                screen.work_list.upsert(path, content);
            }
        }
        screen.work_list.refresh();
        Rc::clone(&screen.work_list)
    };
    glide(&list.scroll, &list.scroll.vadjustment(), 0.0);
    state.set_scroll_adjustment(list.scroll.vadjustment());

    let derivation = derivations.first().map(String::as_str);
    let error_card = ErrorCard::new(failed.error, derivation);
    let failed_side = gtk::Box::new(gtk::Orientation::Vertical, 14);
    failed_side.append(&error_card.root);
    let warnings = screen.warnings.compact_copy();
    failed_side.append(&warnings.root);
    // The column keeps the card alive; the card's own toggle only holds it weakly.
    failed_side.connect_destroy(move |_| {
        let _ = &warnings;
    });
    screen.side.add_child(&failed_side);
    screen.side.set_visible_child(&failed_side);

    if let Some(derivation) = derivation {
        let log = LogView::new(derivation);
        screen.lists.add_child(&log.root);
        let lists = screen.lists.clone();
        let log_root = log.root.clone();
        let log_scroll = log.scroll.clone();
        let log_view = Rc::clone(&log);
        let log_state = Rc::clone(state);
        error_card.view_log.connect_clicked(move |_| {
            lists.set_visible_child(&log_root);
            log_state.set_scroll_adjustment(log_scroll.vadjustment());
            log_view.load(&log_state);
        });
        let lists = screen.lists.clone();
        let list_state = Rc::clone(state);
        log.back.connect_clicked(move |_| {
            lists.set_visible_child(&list.root);
            list_state.set_scroll_adjustment(list.scroll.vadjustment());
        });
    }

    let failed_footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let back = back_to_home_button(window, root, state);
    failed_footer.append(&back);
    failed_footer.append(&action_close_button(
        window,
        Rc::clone(state),
        "report-close-button",
        "report-close-label",
    ));
    if state.appearance.keybinds {
        let hints = key_hints(&[("←", "Back"), ("Esc", "Close")]);
        hints.set_hexpand(true);
        hints.set_halign(gtk::Align::Center);
        hints.set_valign(gtk::Align::Center);
        failed_footer.append(&hints);
    } else {
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        failed_footer.append(&spacer);
    }
    let retry = gtk::Button::with_label("Retry");
    retry.add_css_class("build-retry-button");
    retry.add_css_class("accent-button");
    retry.set_tooltip_text(Some(&format!("Build {} again", failed.target.name())));
    let retry_window = window.clone();
    let retry_root = root.clone();
    let retry_state = Rc::clone(state);
    let target = failed.target;
    let after = failed.after;
    retry.connect_clicked(move |_| match load_config() {
        Ok(config) => start_build(
            &retry_window,
            &retry_root,
            Rc::clone(&retry_state),
            config,
            target,
            after,
        ),
        Err(error) => show_home(
            &retry_window,
            &retry_root,
            Rc::clone(&retry_state),
            Err(error),
        ),
    });
    failed_footer.append(&retry);
    screen.footer.add_child(&failed_footer);
    screen.footer.set_visible_child(&failed_footer);
    retry.grab_focus();
}

/// `12 package changes · +212 MiB`: what a finished build changes, for the minimized bar.
fn report_note(report: &Report) -> String {
    let changes = match report.changes.len() {
        0 => "No package changes".to_owned(),
        1 => "1 package change".to_owned(),
        count => format!("{count} package changes"),
    };
    match report.sizes {
        Some((old, new)) => format!("{changes} · {}", signed_size(new - old)),
        None => changes,
    }
}
