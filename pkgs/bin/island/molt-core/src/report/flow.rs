//! What the review screen knows and how it changes: the report with its list's tab and filter,
//! and switching to the reviewed configuration or applying it on reboot.

use std::fs;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use crate::activation::activate;
use crate::build::Target;
use crate::home::system::system_status;
use crate::protocol::ActivationMode;
use crate::report::{ChangeStatus, Report};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The generation it made, when it could be read.
    Done(Option<u64>),
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Activation {
    pub mode: ActivationMode,
    pub started: Instant,
    /// How it ended, and how long it took.
    pub outcome: Option<(Outcome, Duration)>,
}

#[derive(Clone)]
pub struct Review {
    pub report: Box<Report>,
    /// The list's tab, `None` for all.
    pub tab: Option<ChangeStatus>,
    pub query: String,
    pub files_open: bool,
    pub output_open: bool,
    /// The configuration targets another host, so activating takes a second press.
    pub cross_host: bool,
    /// The mode pressed once, waiting for the second press.
    pub confirming: Option<ActivationMode>,
    pub activation: Option<Activation>,
    /// Modes that succeeded, whose buttons then stay done.
    pub done: Vec<ActivationMode>,
    version: u64,
}

impl PartialEq for Review {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version && self.report.output == other.report.output
    }
}

impl std::fmt::Debug for Review {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Review")
            .field("output", &self.report.output)
            .field("activation", &self.activation)
            .finish_non_exhaustive()
    }
}

pub enum Msg {
    Tab(Option<ChangeStatus>),
    Query(String),
    ToggleFiles,
    ToggleOutput,
    /// A press of Switch now or Apply on reboot.
    Activate(ActivationMode),
    Activated(ActivationMode, Result<(), String>),
    /// Time moves on while it activates.
    Tick,
}

pub enum Effect {
    Activate(ActivationMode, Box<Report>),
}

pub fn current_hostname() -> Option<String> {
    let hostname = fs::read_to_string("/proc/sys/kernel/hostname").ok()?;
    let hostname = hostname.trim();
    (!hostname.is_empty()).then(|| hostname.to_owned())
}

pub fn requires_host_switch_confirmation(
    target: Target,
    flake: &str,
    current_hostname: Option<&str>,
) -> bool {
    target == Target::NixOs
        && current_hostname.is_none_or(|hostname| {
            !hostname
                .split('.')
                .next()
                .is_some_and(|hostname| hostname.eq_ignore_ascii_case(flake))
        })
}

impl Review {
    pub fn open(report: Report) -> Self {
        let cross_host = requires_host_switch_confirmation(
            report.target,
            &report.flake,
            current_hostname().as_deref(),
        );
        Self {
            report: Box::new(report),
            tab: None,
            query: String::new(),
            files_open: false,
            output_open: false,
            cross_host,
            confirming: None,
            activation: None,
            done: Vec::new(),
            version: 0,
        }
    }

    pub fn activating(&self) -> bool {
        self.activation
            .as_ref()
            .is_some_and(|activation| activation.outcome.is_none())
    }

    /// Whether a mode's button can be pressed: not while either runs, and not to apply on reboot
    /// what a switch already made the boot default.
    pub fn can_activate(&self, mode: ActivationMode) -> bool {
        !self.activating()
            && !self.done.contains(&mode)
            && !(mode == ActivationMode::Boot && self.done.contains(&ActivationMode::Switch))
    }

    pub fn update(&mut self, msg: Msg) -> Option<Effect> {
        self.version += 1;
        match msg {
            Msg::Tab(tab) => self.tab = tab,
            Msg::Query(query) => self.query = query,
            Msg::ToggleFiles => self.files_open = !self.files_open,
            Msg::ToggleOutput => self.output_open = !self.output_open,
            Msg::Activate(mode) if self.can_activate(mode) => {
                if self.cross_host && self.confirming != Some(mode) {
                    self.confirming = Some(mode);
                    return None;
                }
                self.confirming = None;
                self.activation = Some(Activation {
                    mode,
                    started: Instant::now(),
                    outcome: None,
                });
                return Some(Effect::Activate(mode, self.report.clone()));
            }
            Msg::Activated(mode, result) => {
                if let Some(activation) = &mut self.activation
                    && activation.mode == mode
                    && activation.outcome.is_none()
                {
                    let took = activation.started.elapsed();
                    let outcome = match result {
                        Ok(()) => {
                            self.done.push(mode);
                            let status = system_status();
                            Outcome::Done(match mode {
                                ActivationMode::Switch => status.generation,
                                ActivationMode::Boot => status.staged.or(status.generation),
                            })
                        }
                        Err(error) => Outcome::Failed(error),
                    };
                    activation.outcome = Some((outcome, took));
                }
            }
            Msg::Tick | Msg::Activate(_) => {}
        }
        None
    }
}

/// Runs an [`Effect`] to the end.
pub fn run(effect: Effect, cancellation: &AtomicBool) -> Msg {
    match effect {
        Effect::Activate(mode, report) => Msg::Activated(mode, activate(&report, mode, cancellation)),
    }
}
