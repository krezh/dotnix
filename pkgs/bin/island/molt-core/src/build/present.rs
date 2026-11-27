//! The build screen as it is shown: every word on it, for any front end to lay out.

use std::time::{Duration, Instant};

use crate::build::evaluation::InputState;
use crate::build::flow::{AfterBuild, Build, Stage};
use crate::build::rows::{GroupSpec, INPUT_GROUPS, RowContent, WORK_GROUPS, activity_name};
use crate::build::work::{download_bytes, remaining_estimate};
use crate::build::{BuildPhase, Target};
use crate::units::{Hue, Units, duration_text};

/// Lines of a build log shown at once; longer logs show their end, which is where builds fail.
pub const LOG_LINES: usize = 5_000;
/// Warnings shown before Show all.
const WARNING_PREVIEW: usize = 3;
pub const STEP_NAMES: [&str; 3] = ["Evaluate", "Build", "Compare"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepState {
    Done,
    Now,
    Failed,
    Waiting,
}

pub struct Group {
    pub name: &'static str,
    pub hue: Hue,
    /// `Fetched · 1`.
    pub heading: String,
    pub rows: Vec<RowContent>,
}

pub struct ListView {
    /// `Flake inputs · 40`.
    pub title: String,
    /// `All` and how many rows, then each group shown and how many it has.
    pub tabs: Vec<(&'static str, usize)>,
    pub tab: usize,
    pub query: String,
    pub placeholder: &'static str,
    pub groups: Vec<Group>,
    /// In place of the rows when there are none.
    pub empty: Option<&'static str>,
    /// How many more match than are listed, said.
    pub more: Option<String>,
}

pub struct LogView {
    pub back: &'static str,
    /// `Build log · molt-0.1.0`.
    pub title: String,
    pub derivation: String,
    /// While nix fetches it, or why there is none.
    pub status: Option<(String, bool)>,
    pub note: Option<String>,
    pub text: Option<String>,
    pub copy: &'static str,
}

pub struct EvaluationCard {
    pub title: &'static str,
    pub elapsed: String,
    pub estimate: String,
    /// How far along the estimate; `None` when it cannot say.
    pub fraction: Option<f32>,
    pub note: &'static str,
    pub inputs_label: &'static str,
    pub inputs: String,
}

pub struct ProgressCard {
    pub title: &'static str,
    pub count: String,
    pub done: f32,
    pub pending: f32,
    pub elapsed: String,
    pub remaining: String,
    pub note: String,
}

pub struct DownloadsCard {
    pub title: &'static str,
    pub done: String,
    pub total: String,
    pub rate: String,
    /// Bytes a second, oldest first.
    pub history: Vec<f64>,
}

pub struct ErrorCard {
    pub title: &'static str,
    pub derivation: Option<(String, String)>,
    pub message: String,
    pub copy: &'static str,
    pub view_log: Option<&'static str>,
}

pub struct WarningsCard {
    /// `Warnings · 3`.
    pub title: String,
    pub shown: Vec<String>,
    pub empty: Option<&'static str>,
    pub toggle: Option<&'static str>,
    pub more: Option<String>,
}

pub enum Side {
    Evaluation(EvaluationCard),
    Building(ProgressCard, DownloadsCard),
    Failed(ErrorCard),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionId {
    Minimize,
    Cancel,
    Retry,
}

pub struct Action {
    pub id: ActionId,
    pub label: &'static str,
    pub primary: bool,
    pub tooltip: Option<String>,
}

/// What a closed panel shows of the build: a ring, its title and a note.
pub struct Mini {
    pub step: &'static str,
    pub title: String,
    /// A few words beside the title where there is little room, such as `13 / 33`; empty when
    /// the time alone says enough.
    pub brief: String,
    pub note: String,
    pub elapsed: Duration,
    pub progress: Option<f32>,
    pub failed: bool,
    pub ready: bool,
}

pub struct BuildView {
    pub title: String,
    pub failed: bool,
    pub subtitle: String,
    pub steps: Vec<(&'static str, StepState)>,
    pub list: ListView,
    pub log: Option<LogView>,
    pub side: Side,
    pub warnings: WarningsCard,
    pub hint: Option<&'static str>,
    pub actions: Vec<Action>,
}

pub fn phase_title(phase: BuildPhase, target: Target) -> String {
    let verb = match phase {
        BuildPhase::Evaluate => "Evaluating",
        BuildPhase::Build => "Building",
        BuildPhase::Compare => "Comparing",
    };
    format!("{verb} {}", target.name())
}

pub const fn failed_title(phase: BuildPhase) -> &'static str {
    match phase {
        BuildPhase::Evaluate => "Evaluation failed",
        BuildPhase::Build => "Build failed",
        BuildPhase::Compare => "Comparison failed",
    }
}

/// The error without the `nix build failed:` line Molt puts above nix's own message.
pub fn error_summary(error: &str) -> &str {
    match error.split_once('\n') {
        Some((first, rest)) if first.ends_with(" failed:") && !rest.trim().is_empty() => {
            rest.trim()
        }
        _ => error.trim(),
    }
}

/// The last `max_lines` lines of `log`, and how many lines came before them.
pub fn log_tail(log: &str, max_lines: usize) -> (&str, usize) {
    let log = log.trim_end();
    let total = log.lines().count();
    if total <= max_lines {
        return (log, 0);
    }
    let skipped = total - max_lines;
    let start = log
        .match_indices('\n')
        .nth(skipped - 1)
        .map_or(0, |(index, _)| index + 1);
    (&log[start..], skipped)
}

/// `~/dotnix` for a folder under home.
fn home_relative(path: &std::path::Path) -> String {
    match std::env::var_os("HOME").map(std::path::PathBuf::from) {
        Some(home) if !home.as_os_str().is_empty() => match path.strip_prefix(&home) {
            Ok(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
            Ok(rest) => format!("~/{}", rest.display()),
            Err(_) => path.display().to_string(),
        },
        _ => path.display().to_string(),
    }
}

fn steps(build: &Build) -> Vec<(&'static str, StepState)> {
    let at = build.phase.index();
    let failed = matches!(build.stage, Stage::Failed { .. });
    let ready = matches!(build.stage, Stage::Ready(_));
    STEP_NAMES
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let state = if ready || index < at {
                StepState::Done
            } else if index == at && failed {
                StepState::Failed
            } else if index == at {
                StepState::Now
            } else {
                StepState::Waiting
            };
            (*name, state)
        })
        .collect()
}

fn list(build: &Build, limit: usize, now: Instant, units: &dyn Units) -> ListView {
    let inputs = build.lists_inputs();
    let (title, specs, empty): (&str, &[GroupSpec], &str) = if inputs {
        ("Flake inputs", INPUT_GROUPS, "No flake inputs to fetch")
    } else {
        ("Activity", WORK_GROUPS, "Waiting for nix to plan the build")
    };
    let rows = build.rows(now, units);
    let mut counts = vec![0_usize; specs.len()];
    for (_, row) in &rows {
        if let Some(count) = counts.get_mut(row.group) {
            *count += 1;
        }
    }
    let shown_groups: Vec<usize> = (0..specs.len())
        .filter(|group| !specs[*group].optional || counts[*group] > 0)
        .collect();
    let mut tabs = vec![("All", rows.len())];
    tabs.extend(
        shown_groups
            .iter()
            .map(|group| (specs[*group].name, counts[*group])),
    );
    let tab = build.tab.min(tabs.len() - 1);
    let only = (tab > 0).then(|| shown_groups[tab - 1]);
    let query = build.query.trim().to_lowercase();
    let matching: Vec<&RowContent> = rows
        .iter()
        .map(|(_, row)| row)
        .filter(|row| only.is_none_or(|group| row.group == group))
        .filter(|row| query.is_empty() || row.search_text().contains(&query))
        .collect();
    let more = matching.len().saturating_sub(limit);
    let mut groups: Vec<Group> = Vec::new();
    for row in matching.iter().take(limit) {
        let Some(spec) = specs.get(row.group) else {
            continue;
        };
        if groups.last().is_none_or(|group| group.name != spec.name) {
            let count = matching
                .iter()
                .filter(|other| other.group == row.group)
                .count();
            groups.push(Group {
                name: spec.name,
                hue: spec.hue,
                heading: format!("{} · {count}", spec.name),
                rows: Vec::new(),
            });
        }
        if let Some(group) = groups.last_mut() {
            group.rows.push((*row).clone());
        }
    }
    ListView {
        title: format!("{title} · {}", rows.len()),
        tabs,
        tab,
        query: build.query.clone(),
        placeholder: "Filter",
        empty: groups.is_empty().then_some(if rows.is_empty() {
            empty
        } else {
            "Nothing matches the filter"
        }),
        groups,
        more: (more > 0).then(|| format!("{more} more not listed · filter to find them")),
    }
}

fn log(build: &Build, lines: usize) -> Option<LogView> {
    let log = build.log.as_ref()?;
    let name = activity_name(&log.derivation);
    let (status, note, text) = match &log.text {
        None => (
            Some(("Fetching the build log from nix…".to_owned(), false)),
            None,
            None,
        ),
        Some(Ok(text)) if text.trim().is_empty() => (
            Some(("Nix kept no build log for this derivation".to_owned(), true)),
            None,
            None,
        ),
        Some(Ok(text)) => {
            let (shown, skipped) = log_tail(text, lines);
            let note = (skipped > 0).then(|| {
                format!("Showing the last {lines} lines; {skipped} earlier lines are in Copy log")
            });
            (None, note, Some(shown.to_owned()))
        }
        Some(Err(error)) => (Some((error_summary(error).to_owned(), true)), None, None),
    };
    Some(LogView {
        back: "Activity",
        title: format!("Build log · {name}"),
        derivation: log.derivation.clone(),
        status,
        note,
        text,
        copy: "Copy log",
    })
}

fn evaluation(build: &Build, now: Instant, units: &dyn Units) -> EvaluationCard {
    let elapsed = now - build.started;
    let (estimate, fraction, note) = match build.evaluation_estimate {
        Some(estimate) if elapsed < estimate => (
            format!("of about {}", units.elapsed(estimate)),
            Some((elapsed.as_secs_f64() / estimate.as_secs_f64()) as f32),
            "Estimate from the last evaluation",
        ),
        Some(estimate) => (
            format!("of about {}", units.elapsed(estimate)),
            None,
            "Taking longer than the last evaluation",
        ),
        None => (
            String::new(),
            None,
            "No earlier evaluation to estimate from",
        ),
    };
    let rows = build.input_rows();
    let cached = rows
        .iter()
        .filter(|row| row.state == InputState::Cached)
        .count();
    EvaluationCard {
        title: "Evaluation",
        elapsed: units.elapsed(elapsed),
        estimate,
        fraction,
        note,
        inputs_label: "Inputs",
        inputs: format!("{} fetched · {cached} cached", rows.len() - cached),
    }
}

fn progress(build: &Build, now: Instant, units: &dyn Units) -> (ProgressCard, DownloadsCard) {
    let summary = build.plan.summary();
    let finished = summary.done + summary.failed;
    let total = summary.total.max(1) as f32;
    let elapsed = build.evaluation_done.map_or(Duration::ZERO, |started| {
        now.saturating_duration_since(started)
    });
    let status = build.compare_status;
    let estimate = status
        .is_none()
        .then(|| remaining_estimate(&summary))
        .flatten();
    let remaining = match estimate {
        Some(left) if left < Duration::from_secs(60) => "under a minute left".to_owned(),
        Some(left) => format!("about {} left", duration_text(left.as_secs())),
        None => String::new(),
    };
    let note = match (status, estimate) {
        (Some(status), _) => status.to_owned(),
        (None, Some(_)) => "Estimate from finished build times".to_owned(),
        (None, None) => String::new(),
    };
    let (done, expected) = download_bytes(&build.progress);
    let rate = build.throughput.rate();
    (
        ProgressCard {
            title: "Progress",
            count: format!("{finished} / {}", summary.total),
            done: finished as f32 / total,
            pending: summary.in_flight as f32 / total,
            elapsed: format!("{} elapsed", units.elapsed(elapsed)),
            remaining,
            note,
        },
        DownloadsCard {
            title: "Downloads",
            done: units.amount(done),
            total: if expected > 0 {
                format!("of {}", units.amount(expected))
            } else {
                "nothing to download".to_owned()
            },
            rate: if expected > done {
                units.rate(rate)
            } else {
                String::new()
            },
            history: build.throughput.history.iter().copied().collect(),
        },
    )
}

fn warnings(build: &Build) -> WarningsCard {
    let count = build.warnings.len();
    let open = build.warnings_open;
    let hidden = count.saturating_sub(WARNING_PREVIEW);
    WarningsCard {
        title: format!("Warnings · {count}"),
        shown: build
            .warnings
            .iter()
            .take(if open { count } else { WARNING_PREVIEW })
            .cloned()
            .collect(),
        empty: (count == 0).then_some("Warnings appear here as nix reports them"),
        toggle: (count > WARNING_PREVIEW).then_some(if open { "Show less" } else { "Show all" }),
        more: (hidden > 0 && !open).then(|| format!("+ {hidden} more")),
    }
}

/// The build screen, listing at most `limit` rows and the last `log_lines` of a build log.
pub fn view(
    build: &Build,
    limit: usize,
    log_lines: usize,
    now: Instant,
    units: &dyn Units,
) -> BuildView {
    let (title, failed) = match &build.stage {
        Stage::Failed { .. } => (failed_title(build.phase).to_owned(), true),
        _ => (phase_title(build.phase, build.target), false),
    };
    let side = match &build.stage {
        Stage::Failed {
            error, derivations, ..
        } => {
            let derivation = derivations
                .first()
                .map(|path| (activity_name(path), path.clone()));
            Side::Failed(ErrorCard {
                title: "Error",
                view_log: derivation.is_some().then_some("View log"),
                derivation,
                message: error_summary(error).to_owned(),
                copy: "Copy error",
            })
        }
        _ if build.evaluation_done.is_none() => Side::Evaluation(evaluation(build, now, units)),
        _ => {
            let (progress, downloads) = progress(build, now, units);
            Side::Building(progress, downloads)
        }
    };
    let (hint, actions) = if failed {
        (
            None,
            vec![Action {
                id: ActionId::Retry,
                label: "Retry",
                primary: true,
                tooltip: Some(format!("Build {} again", build.target.name())),
            }],
        )
    } else {
        (
            Some(match build.after {
                AfterBuild::Review => "Shows the report when the build finishes",
                AfterBuild::Switch => "Switches as soon as the build finishes",
            }),
            vec![
                Action {
                    id: ActionId::Minimize,
                    label: "Minimize",
                    primary: false,
                    tooltip: None,
                },
                Action {
                    id: ActionId::Cancel,
                    label: "Cancel",
                    primary: false,
                    tooltip: None,
                },
            ],
        )
    };
    BuildView {
        title,
        failed,
        subtitle: format!("#{} · {}", build.flake, home_relative(&build.flake_dir)),
        steps: steps(build),
        list: list(build, limit, now, units),
        log: log(build, log_lines),
        side,
        warnings: warnings(build),
        hint,
        actions,
    }
}

/// `12 package changes · +212 MiB`: what a finished build changes, for the closed panel.
fn report_note(report: &crate::report::Report, units: &dyn Units) -> String {
    let changes = match report.changes.len() {
        0 => "No package changes".to_owned(),
        1 => "1 package change".to_owned(),
        count => format!("{count} package changes"),
    };
    match report.sizes {
        Some((old, new)) => {
            let delta = new - old;
            let sign = if delta >= 0 { "+" } else { "-" };
            format!("{changes} · {sign}{}", units.size(delta.unsigned_abs()))
        }
        None => changes,
    }
}

/// `12 changes · +212 MiB`: the same, for where there is little room.
fn report_brief(report: &crate::report::Report, units: &dyn Units) -> String {
    let changes = match report.changes.len() {
        1 => "1 change".to_owned(),
        count => format!("{count} changes"),
    };
    match report.sizes {
        Some((old, new)) => {
            let delta = new - old;
            let sign = if delta >= 0 { "+" } else { "-" };
            format!("{changes} · {sign}{}", units.size(delta.unsigned_abs()))
        }
        None => changes,
    }
}

/// What the closed panel shows of the build.
pub fn mini(build: &Build, now: Instant, units: &dyn Units) -> Mini {
    let elapsed = build.took.unwrap_or_else(|| now - build.started);
    match &build.stage {
        Stage::Ready(report) => Mini {
            step: "Ready",
            title: crate::report::ready_title(build.target).to_owned(),
            brief: report_brief(report, units),
            note: report_note(report, units),
            elapsed,
            progress: Some(1.0),
            failed: false,
            ready: true,
        },
        Stage::Failed {
            error, derivations, ..
        } => Mini {
            step: STEP_NAMES[build.phase.index()],
            title: failed_title(build.phase).to_owned(),
            brief: derivations
                .first()
                .map(|path| activity_name(path))
                .unwrap_or_default(),
            note: error_summary(error)
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned(),
            elapsed,
            progress: None,
            failed: true,
            ready: false,
        },
        Stage::Running => {
            let (note, progress) = match build.phase {
                BuildPhase::Evaluate => match build.evaluation_estimate {
                    Some(estimate) => (
                        format!("usually {}", units.elapsed(estimate)),
                        Some((elapsed.as_secs_f64() / estimate.as_secs_f64()).min(0.95) as f32),
                    ),
                    None => ("Fetching inputs and evaluating".to_owned(), None),
                },
                BuildPhase::Build => {
                    let summary = build.plan.summary();
                    let finished = summary.done + summary.failed;
                    let mut note = format!("{finished} / {} built", summary.total);
                    let (downloaded, _) = download_bytes(&build.progress);
                    if downloaded > 0 {
                        note.push_str(" · ↓ ");
                        note.push_str(&units.amount(downloaded));
                    }
                    let share = (summary.total > 0).then(|| finished as f32 / summary.total as f32);
                    (note, share)
                }
                BuildPhase::Compare => (
                    build
                        .compare_status
                        .unwrap_or("Comparing closures")
                        .to_owned(),
                    None,
                ),
            };
            let brief = match build.phase {
                BuildPhase::Build => {
                    let summary = build.plan.summary();
                    format!("{} / {}", summary.done + summary.failed, summary.total)
                }
                BuildPhase::Evaluate | BuildPhase::Compare => String::new(),
            };
            Mini {
                step: STEP_NAMES[build.phase.index()],
                title: phase_title(build.phase, build.target),
                brief,
                note,
                elapsed,
                progress,
                failed: false,
                ready: false,
            }
        }
    }
}

#[cfg(test)]
#[path = "present_test.rs"]
mod tests;
