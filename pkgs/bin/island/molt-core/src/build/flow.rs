//! What the build screen knows and how it changes: evaluation with its flake inputs, building with
//! the work nix plans, comparing, and then the report, or the failure with its build log.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::build::evaluation::{
    InputRow, fail_open_fetches, input_rows, last_evaluation, remember_evaluation,
};
use crate::build::rows::RowContent;
use crate::build::work::{Throughput, WorkPlan, download_bytes};
use crate::build::{BuildPhase, BuildUpdate, Target, build_report, derivation_log};
use crate::command::CANCELLED;
use crate::config::Config;
use crate::inputs::{self, FlakeInput};
use crate::nix::{FlakeFetch, NixBuildProgress, failed_derivations};
use crate::report::Report;
use crate::units::Units;

const TICK: Duration = Duration::from_secs(1);

/// What a successful build leads to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterBuild {
    /// Show the report and wait for the user to switch.
    Review,
    /// Show the report and switch right away, as if Switch now had been clicked.
    Switch,
}

#[derive(Clone)]
pub enum Stage {
    Running,
    Failed {
        error: String,
        /// Failed derivations, the one nix named first.
        derivations: Vec<String>,
        /// The flake inputs as they stood, open fetches marked failed, when evaluation failed.
        inputs: Option<Vec<InputRow>>,
    },
    Ready(Box<Report>),
}

/// A failed derivation's build log, shown in place of the list.
#[derive(Clone, PartialEq, Eq)]
pub struct Log {
    pub derivation: String,
    /// `None` while nix fetches it.
    pub text: Option<Result<String, String>>,
}

#[derive(Clone)]
pub struct Build {
    pub target: Target,
    pub after: AfterBuild,
    pub flake: String,
    pub flake_dir: PathBuf,
    /// Tells this run's events from an earlier one's.
    pub generation: u64,
    pub phase: BuildPhase,
    pub started: Instant,
    pub evaluation_estimate: Option<Duration>,
    pub evaluation_done: Option<Instant>,
    pub compare_status: Option<&'static str>,
    pub flake_inputs: Vec<FlakeInput>,
    pub fetches: Vec<FlakeFetch>,
    pub progress: NixBuildProgress,
    pub plan: WorkPlan,
    pub throughput: Throughput,
    /// Warnings without their `warning:` prefix, each once, in the order nix gave them.
    pub warnings: Vec<String>,
    pub stage: Stage,
    /// How long it ran, once it ended.
    pub took: Option<Duration>,
    /// The list's filter tab, 0 for all, and what is typed into its filter.
    pub tab: usize,
    pub query: String,
    pub log: Option<Log>,
    pub warnings_open: bool,
    /// The order rows entered their group in, by key.
    entered: HashMap<String, (usize, u64)>,
    seq: u64,
    /// Bumped on every change, so a front end can tell two states apart cheaply.
    version: u64,
}

impl PartialEq for Build {
    fn eq(&self, other: &Self) -> bool {
        self.generation == other.generation && self.version == other.version
    }
}

impl std::fmt::Debug for Build {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Build")
            .field("target", &self.target)
            .field("generation", &self.generation)
            .field("phase", &self.phase)
            .field("running", &self.running())
            .finish_non_exhaustive()
    }
}

pub enum Msg {
    Update(u64, BuildUpdate),
    Finished(u64, Box<Result<Report, String>>),
    /// A second passed; times shown move on.
    Tick(u64),
    Cancel,
    Tab(usize),
    Query(String),
    ShowLog,
    HideLog,
    Log(String, Result<String, String>),
    ToggleWarnings,
}

pub enum Effect {
    Run {
        generation: u64,
        target: Target,
        config: Box<Config>,
    },
    Cancel,
    Log(String),
}

/// A nix warning without its `warning:` prefix, which the card's heading already says.
pub fn warning_text(warning: &str) -> &str {
    warning
        .strip_prefix("warning:")
        .map_or(warning, str::trim_start)
}

impl Build {
    /// A build of `target` with `config`, to be run with the returned effect.
    pub fn start(
        config: Config,
        target: Target,
        after: AfterBuild,
        generation: u64,
    ) -> (Self, Effect) {
        let flake = target.flake(&config).unwrap_or("<unset>").to_owned();
        let flake_inputs = inputs::load(&config.flake_dir)
            .map(|(_, inputs)| inputs)
            .unwrap_or_default();
        let build = Self {
            target,
            after,
            flake,
            flake_dir: config.flake_dir.clone(),
            generation,
            phase: BuildPhase::Evaluate,
            started: Instant::now(),
            evaluation_estimate: last_evaluation(target),
            evaluation_done: None,
            compare_status: None,
            flake_inputs,
            fetches: Vec::new(),
            progress: NixBuildProgress::default(),
            plan: WorkPlan::default(),
            throughput: Throughput::default(),
            warnings: Vec::new(),
            stage: Stage::Running,
            took: None,
            tab: 0,
            query: String::new(),
            log: None,
            warnings_open: false,
            entered: HashMap::new(),
            seq: 0,
            version: 0,
        };
        let effect = Effect::Run {
            generation,
            target,
            config: Box::new(config),
        };
        (build, effect)
    }

    pub const fn running(&self) -> bool {
        matches!(self.stage, Stage::Running)
    }

    /// Flake-input rows as they stand.
    pub fn input_rows(&self) -> Vec<InputRow> {
        match &self.stage {
            Stage::Failed {
                inputs: Some(rows), ..
            } => rows.clone(),
            _ => input_rows(&self.flake_inputs, &self.fetches, self.evaluation_done),
        }
    }

    /// Whether the list shows flake inputs rather than build work.
    pub fn lists_inputs(&self) -> bool {
        match &self.stage {
            Stage::Failed { inputs, .. } => inputs.is_some(),
            _ => self.evaluation_done.is_none(),
        }
    }

    /// The list's rows with their content, in each group's order.
    pub fn rows(&self, now: Instant, units: &dyn Units) -> Vec<(String, RowContent)> {
        let mut rows: Vec<(String, RowContent)> = if self.lists_inputs() {
            self.input_rows()
                .into_iter()
                .map(|row| {
                    let content = row.content(now, units);
                    (row.key, content)
                })
                .collect()
        } else {
            self.plan
                .paths()
                .filter_map(|path| {
                    self.plan
                        .content(path, now, units)
                        .map(|content| (path.to_owned(), content))
                })
                .collect()
        };
        let specs = if self.lists_inputs() {
            crate::build::rows::INPUT_GROUPS
        } else {
            crate::build::rows::WORK_GROUPS
        };
        let order = |key: &str, group: usize| {
            let entered = self
                .entered
                .get(key)
                .filter(|(at, _)| *at == group)
                .map_or(u64::MAX, |(_, seq)| *seq) as i128;
            let newest_first = specs.get(group).is_some_and(|spec| spec.newest_first);
            (group, if newest_first { -entered } else { entered })
        };
        rows.sort_by_cached_key(|(key, content)| {
            (order(key, content.group), content.title.clone())
        });
        rows
    }

    fn note_groups(&mut self, now: Instant, units: &dyn Units) {
        for (key, content) in self.rows(now, units) {
            let entry = self.entered.entry(key).or_insert((usize::MAX, 0));
            if entry.0 != content.group {
                self.seq += 1;
                *entry = (content.group, self.seq);
            }
        }
    }

    fn push_warnings(&mut self, warnings: &[String]) {
        for warning in warnings {
            let text = warning_text(warning);
            if !self.warnings.iter().any(|known| known == text) {
                self.warnings.push(text.to_owned());
            }
        }
    }

    /// Hands `msg` to the build; `units` only orders rows, so any front end's will do.
    pub fn update(&mut self, msg: Msg, units: &dyn Units) -> Option<Effect> {
        let now = Instant::now();
        self.version += 1;
        match msg {
            Msg::Update(generation, update) if generation == self.generation && self.running() => {
                match update {
                    BuildUpdate::Phase(next, message) => {
                        if next != self.phase {
                            if self.phase == BuildPhase::Evaluate {
                                self.evaluation_done = Some(now);
                                remember_evaluation(self.target, now - self.started);
                                self.tab = 0;
                                self.query.clear();
                            }
                            self.phase = next;
                        }
                        if next == BuildPhase::Compare {
                            self.compare_status = Some(message);
                        }
                    }
                    BuildUpdate::Nix(BuildPhase::Evaluate, update) => {
                        self.push_warnings(&update.warnings);
                        self.fetches.clone_from(&update.flake_fetches);
                    }
                    BuildUpdate::Nix(_, update) => {
                        self.push_warnings(&update.warnings);
                        self.plan.update(&update, now);
                        self.progress = *update;
                        self.throughput
                            .sample(now, download_bytes(&self.progress).0);
                    }
                }
                self.note_groups(now, units);
                None
            }
            Msg::Tick(generation) if generation == self.generation && self.running() => {
                if self.evaluation_done.is_some() {
                    self.throughput
                        .sample(now, download_bytes(&self.progress).0);
                }
                None
            }
            Msg::Finished(generation, result)
                if generation == self.generation && self.running() =>
            {
                self.took = Some(now - self.started);
                match *result {
                    Ok(report) => self.stage = Stage::Ready(Box::new(report)),
                    Err(error) if error == CANCELLED => {}
                    Err(error) => self.fail(error, now),
                }
                self.note_groups(now, units);
                None
            }
            Msg::Cancel if self.running() => Some(Effect::Cancel),
            Msg::Tab(tab) => {
                self.tab = tab;
                None
            }
            Msg::Query(query) => {
                self.query = query;
                None
            }
            Msg::ShowLog => {
                let Stage::Failed { derivations, .. } = &self.stage else {
                    return None;
                };
                let derivation = derivations.first()?.clone();
                if self
                    .log
                    .as_ref()
                    .is_some_and(|log| log.derivation == derivation && log.text.is_some())
                {
                    return None;
                }
                self.log = Some(Log {
                    derivation: derivation.clone(),
                    text: None,
                });
                Some(Effect::Log(derivation))
            }
            Msg::HideLog => {
                self.log = None;
                None
            }
            Msg::Log(derivation, text) => {
                if let Some(log) = &mut self.log
                    && log.derivation == derivation
                {
                    log.text = Some(text);
                }
                None
            }
            Msg::ToggleWarnings => {
                self.warnings_open = !self.warnings_open;
                None
            }
            _ => None,
        }
    }

    /// Turns the build into its failure: failed rows sorted to the top, nix's error kept.
    fn fail(&mut self, error: String, now: Instant) {
        let mut derivations = failed_derivations(&error);
        let inputs = if self.phase == BuildPhase::Evaluate {
            let mut rows = input_rows(&self.flake_inputs, &self.fetches, None);
            fail_open_fetches(&mut rows);
            Some(rows)
        } else {
            // The last snapshot still lists the work in flight, which `interrupt` sends back to
            // Queued.
            self.progress.failed.extend(derivations.iter().cloned());
            self.plan.update(&self.progress, now);
            self.plan.interrupt();
            let mut known = self
                .plan
                .failed()
                .filter(|path| {
                    path.ends_with(".drv") && !derivations.iter().any(|named| named == path)
                })
                .map(str::to_owned)
                .collect::<Vec<_>>();
            known.sort();
            derivations.extend(known);
            None
        };
        self.tab = 0;
        self.query.clear();
        self.stage = Stage::Failed {
            error,
            derivations,
            inputs,
        };
    }
}

/// Runs an [`Effect`] to the end, telling `emit` what happens on the way. A build stops once
/// `cancellation` is set, which [`Effect::Cancel`] does.
pub fn run(effect: Effect, cancellation: &Arc<AtomicBool>, emit: &(dyn Fn(Msg) + Sync)) {
    match effect {
        Effect::Run {
            generation,
            target,
            config,
        } => {
            let done = AtomicBool::new(false);
            thread::scope(|scope| {
                scope.spawn(|| {
                    while !done.load(Ordering::Relaxed) {
                        thread::sleep(TICK);
                        emit(Msg::Tick(generation));
                    }
                });
                let result = build_report(&config, target, generation, cancellation, |update| {
                    emit(Msg::Update(generation, update));
                });
                done.store(true, Ordering::Relaxed);
                emit(Msg::Finished(generation, Box::new(result)));
            });
        }
        Effect::Cancel => cancellation.store(true, Ordering::Relaxed),
        Effect::Log(derivation) => {
            let text = derivation_log(&derivation, cancellation);
            emit(Msg::Log(derivation, text));
        }
    }
}
