//! What the Clean up screen knows and how it changes: every cleaner's scan, the categories picked
//! and the one open, the retention to clean with, and the cleanup itself, armed before it runs.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use crate::cleanup::{
    self, CleanupEstimate, CleanupEvent, CleanupGroup, CleanupKind, ScanEvent, Volume,
};
use crate::config::CleanupPolicy;

/// How long retention must stay unchanged before the Nix plan is scanned again for it.
const SETTLE: Duration = Duration::from_millis(400);
pub const MAX_GENERATIONS: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Review,
    Cleaning,
    Complete,
}

/// What a finished cleanup freed, and how many categories failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Freed {
    pub bytes: u64,
    pub elapsed: Duration,
    pub failures: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cleanup {
    /// In the order the scans finished.
    pub groups: Vec<CleanupGroup>,
    /// The filesystems cleanup frees space on, once read.
    pub volumes: Option<Vec<Volume>>,
    pub selected: Vec<CleanupKind>,
    pub open: CleanupKind,
    pub query: String,
    /// What is entered for each retention setting.
    pub generations: String,
    pub journals: String,
    /// The policy the screen opened with, and later the one the cleanup ran with.
    pub policy: CleanupPolicy,
    pub scanning: bool,
    pub rescanning: bool,
    /// Generation retention the shown Nix group was scanned with.
    pub nix_generations: Option<usize>,
    /// Generation retention of the latest Nix scan started.
    pub nix_target: usize,
    /// Identifies the latest scan and cleanup; results of superseded ones are dropped.
    pub serial: u64,
    /// Identifies the latest retention change, to settle before rescanning.
    pub settle: u64,
    pub armed: bool,
    pub phase: Phase,
    /// Categories cleaning now, in the order they started.
    pub running: Vec<CleanupKind>,
    pub outcomes: Vec<(CleanupKind, Result<u64, String>)>,
    pub freed: Option<Freed>,
    /// A message that stands until the plan next changes.
    pub notice: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Open(CleanupKind),
    Toggle(CleanupKind),
    Query(String),
    Generations(String),
    /// Keeps this many more generations, or fewer.
    StepGenerations(isize),
    Journals(String),
    Settled(u64),
    /// A category's scan, with the serial it ran under and the generations it kept.
    Group(u64, CleanupGroup, usize),
    /// Every category of the scan is in, and the volumes they free space on.
    Scanned(u64, Vec<Volume>),
    /// The main button: arm, confirm, cancel, or scan again.
    Act,
    Started(u64, CleanupKind),
    Finished(u64, CleanupKind, Result<u64, String>),
    Cleaned {
        serial: u64,
        before: Vec<Volume>,
        after: Vec<Volume>,
        elapsed: Duration,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Scan {
        serial: u64,
        kinds: Vec<CleanupKind>,
        policy: CleanupPolicy,
    },
    Settle(u64),
    Clean {
        serial: u64,
        groups: Vec<CleanupGroup>,
        policy: CleanupPolicy,
    },
    /// Stops the cleanup under way; the screen is left for Home.
    Cancel,
    /// Remembers what a full scan found reclaimable for Home, or forgets it after cleaning.
    Estimate(Option<u64>),
}

/// Free space across `volumes`.
pub fn available(volumes: &[Volume]) -> u64 {
    volumes.iter().fold(0_u64, |total, volume| {
        total.saturating_add(volume.usage.available)
    })
}

/// A category's name as the cleanup screen shows it.
pub const fn category_title(kind: CleanupKind) -> &'static str {
    match kind {
        CleanupKind::Nix => "Nix store",
        _ => kind.title(),
    }
}

/// Whether the scan found something `group`'s cleaner can remove.
pub fn selectable(group: &CleanupGroup) -> bool {
    group.available && !(group.items.is_empty() && group.reclaimable == 0)
}

impl Cleanup {
    /// The screen as it opens, scanning every cleaner with `policy`.
    pub fn open(policy: CleanupPolicy) -> (Self, Effect) {
        let screen = Self {
            groups: Vec::new(),
            volumes: None,
            selected: Vec::new(),
            open: CleanupKind::Nix,
            query: String::new(),
            generations: policy.keep_generations.to_string(),
            journals: policy.journal_max_age.clone(),
            nix_target: policy.keep_generations,
            policy: policy.clone(),
            scanning: true,
            rescanning: false,
            nix_generations: None,
            serial: 0,
            settle: 0,
            armed: false,
            phase: Phase::Review,
            running: Vec::new(),
            outcomes: Vec::new(),
            freed: None,
            notice: None,
        };
        let effect = Effect::Scan {
            serial: 0,
            kinds: CleanupKind::ALL.to_vec(),
            policy,
        };
        (screen, effect)
    }

    pub fn group(&self, kind: CleanupKind) -> Option<&CleanupGroup> {
        self.groups.iter().find(|group| group.kind == kind)
    }

    pub fn is_selected(&self, kind: CleanupKind) -> bool {
        self.selected.contains(&kind)
    }

    /// Each selected category and the bytes it would free, in display order.
    pub fn plan(&self) -> Vec<(CleanupKind, u64)> {
        CleanupKind::ALL
            .into_iter()
            .filter(|kind| self.is_selected(*kind))
            .filter_map(|kind| self.group(kind).map(|group| (kind, group.reclaimable)))
            .collect()
    }

    fn selected_groups(&self) -> Vec<CleanupGroup> {
        CleanupKind::ALL
            .into_iter()
            .filter(|kind| self.is_selected(*kind))
            .filter_map(|kind| self.group(kind).cloned())
            .collect()
    }

    /// The generation retention entered, when it is a whole number.
    pub fn keep_generations(&self) -> Option<usize> {
        self.generations.trim().parse().ok()
    }

    /// The policy entered for what is selected.
    pub fn entered_policy(&self) -> Result<CleanupPolicy, String> {
        let policy = CleanupPolicy {
            keep_generations: if self.is_selected(CleanupKind::Nix) {
                self.keep_generations().ok_or_else(|| {
                    "Nix generations to keep must be a whole number from 0 to 100".to_owned()
                })?
            } else {
                0
            },
            journal_max_age: if self.is_selected(CleanupKind::Journals) {
                self.journals.trim().to_owned()
            } else {
                "0".to_owned()
            },
        };
        policy.validate()?;
        Ok(policy)
    }

    /// Whether the shown Nix plan was scanned with the retention entered, or Nix is left out.
    pub fn plan_is_current(&self) -> bool {
        !self.is_selected(CleanupKind::Nix) || self.keep_generations() == self.nix_generations
    }

    pub fn busy(&self) -> bool {
        self.scanning || self.rescanning
    }

    /// Whether the main button can be pressed.
    pub fn can_act(&self) -> bool {
        match self.phase {
            Phase::Cleaning | Phase::Complete => true,
            Phase::Review => {
                !self.busy()
                    && !self.plan().is_empty()
                    && self.entered_policy().is_ok()
                    && self.plan_is_current()
            }
        }
    }

    fn changed(&mut self) {
        self.armed = false;
        self.notice = None;
    }

    fn set_group(&mut self, group: CleanupGroup) {
        let kind = group.kind;
        let can = selectable(&group);
        let pick = if self.scanning {
            can && kind.selected_by_default()
        } else {
            can && self.is_selected(kind)
        };
        self.selected.retain(|other| *other != kind);
        if pick {
            self.selected.push(kind);
        }
        self.groups.retain(|other| other.kind != kind);
        self.groups.push(group);
        self.changed();
    }

    /// Scans the Nix category again when the valid retention entered differs from the latest Nix
    /// scan's, so the reviewed plan always matches the policy cleanup runs with. Waits for the
    /// first scan, which asks again when it completes.
    fn rescan_if_stale(&mut self) -> Option<Effect> {
        if self.scanning || self.phase != Phase::Review {
            return None;
        }
        let keep_generations = self.keep_generations()?;
        let policy = CleanupPolicy {
            keep_generations,
            ..self.policy.clone()
        };
        if policy.validate().is_err() || self.nix_target == keep_generations {
            return None;
        }
        self.nix_target = keep_generations;
        self.serial = self.serial.wrapping_add(1);
        self.rescanning = true;
        self.changed();
        Some(Effect::Scan {
            serial: self.serial,
            kinds: vec![CleanupKind::Nix],
            policy,
        })
    }

    fn retention_changed(&mut self) -> Vec<Effect> {
        self.changed();
        self.settle = self.settle.wrapping_add(1);
        vec![Effect::Settle(self.settle)]
    }

    pub fn update(&mut self, msg: Msg) -> Vec<Effect> {
        match msg {
            Msg::Open(kind) => {
                if self.open != kind {
                    self.open = kind;
                    self.query.clear();
                }
                Vec::new()
            }
            Msg::Toggle(kind) if self.phase == Phase::Review => {
                if self.group(kind).is_some_and(selectable) {
                    if self.is_selected(kind) {
                        self.selected.retain(|other| *other != kind);
                    } else {
                        self.selected.push(kind);
                    }
                    self.changed();
                }
                Vec::new()
            }
            Msg::Query(query) => {
                self.query = query;
                Vec::new()
            }
            Msg::Generations(text) if self.phase == Phase::Review => {
                self.generations = text;
                self.retention_changed()
            }
            Msg::StepGenerations(by) if self.phase == Phase::Review => {
                let now = self
                    .keep_generations()
                    .unwrap_or(self.policy.keep_generations);
                let next = now.saturating_add_signed(by).min(MAX_GENERATIONS);
                if self.keep_generations() == Some(next) {
                    return Vec::new();
                }
                self.generations = next.to_string();
                self.retention_changed()
            }
            Msg::Journals(text) if self.phase == Phase::Review => {
                self.journals = text;
                self.retention_changed()
            }
            Msg::Settled(settle) if settle == self.settle => {
                self.rescan_if_stale().into_iter().collect()
            }
            Msg::Group(serial, group, keep_generations) if serial == self.serial => {
                if group.kind == CleanupKind::Nix {
                    self.nix_generations = Some(keep_generations);
                }
                self.set_group(group);
                Vec::new()
            }
            Msg::Scanned(serial, volumes) if serial == self.serial => {
                let mut effects = Vec::new();
                if self.scanning {
                    self.scanning = false;
                    self.volumes = Some(volumes);
                    let reclaimable = self
                        .groups
                        .iter()
                        .filter(|group| group.available)
                        .fold(0_u64, |total, group| total.saturating_add(group.reclaimable));
                    effects.push(Effect::Estimate(Some(reclaimable)));
                }
                self.rescanning = false;
                effects.extend(self.rescan_if_stale());
                effects
            }
            Msg::Act => self.act(),
            Msg::Started(serial, kind) if serial == self.serial => {
                self.running.push(kind);
                Vec::new()
            }
            Msg::Finished(serial, kind, result) if serial == self.serial => {
                self.running.retain(|other| *other != kind);
                self.outcomes.retain(|(other, _)| *other != kind);
                self.outcomes.push((kind, result));
                Vec::new()
            }
            Msg::Cleaned {
                serial,
                before,
                after,
                elapsed,
            } if serial == self.serial && self.phase == Phase::Cleaning => {
                self.phase = Phase::Complete;
                self.running.clear();
                self.freed = Some(Freed {
                    bytes: available(&after).saturating_sub(available(&before)),
                    elapsed,
                    failures: self
                        .outcomes
                        .iter()
                        .filter(|(_, result)| result.is_err())
                        .count(),
                });
                self.volumes = Some(after);
                vec![Effect::Estimate(None)]
            }
            _ => Vec::new(),
        }
    }

    fn act(&mut self) -> Vec<Effect> {
        match self.phase {
            Phase::Cleaning => vec![Effect::Cancel],
            Phase::Complete => {
                let (screen, effect) = Self::open(self.policy.clone());
                let serial = self.serial.wrapping_add(1);
                *self = Self { serial, ..screen };
                match effect {
                    Effect::Scan { kinds, policy, .. } => vec![Effect::Scan {
                        serial,
                        kinds,
                        policy,
                    }],
                    other => vec![other],
                }
            }
            Phase::Review if !self.can_act() => Vec::new(),
            Phase::Review if !self.armed => {
                self.armed = true;
                Vec::new()
            }
            Phase::Review => self.start(),
        }
    }

    fn start(&mut self) -> Vec<Effect> {
        let policy = match self.entered_policy() {
            Ok(policy) => policy,
            Err(error) => {
                self.armed = false;
                self.notice = Some(error);
                return Vec::new();
            }
        };
        if self.rescanning || !self.plan_is_current() {
            let effects = self.rescan_if_stale().into_iter().collect();
            self.notice = Some(
                "Retention changed since the scan; review the updated Nix plan first".to_owned(),
            );
            return effects;
        }
        self.armed = false;
        self.notice = None;
        self.phase = Phase::Cleaning;
        self.serial = self.serial.wrapping_add(1);
        self.running.clear();
        self.outcomes.clear();
        self.policy = policy.clone();
        vec![Effect::Clean {
            serial: self.serial,
            groups: self.selected_groups(),
            policy,
        }]
    }
}

/// Runs an [`Effect`] to the end, telling `emit` what happens on the way. Scans and cleanups stop
/// once `cancellation` is set, which [`Effect::Cancel`] does.
pub fn run(effect: Effect, cancellation: &Arc<AtomicBool>, emit: &(dyn Fn(Msg) + Sync)) {
    match effect {
        Effect::Scan {
            serial,
            kinds,
            policy,
        } => {
            let keep_generations = policy.keep_generations;
            let (sender, receiver) = mpsc::channel();
            let flag = Arc::clone(cancellation);
            let scanner = thread::spawn(move || cleanup::scan(&kinds, policy, flag, sender));
            for event in receiver {
                emit(match event {
                    ScanEvent::Group(group) => Msg::Group(serial, group, keep_generations),
                    ScanEvent::Complete(volumes) => Msg::Scanned(serial, volumes),
                });
            }
            let _ = scanner.join();
        }
        Effect::Settle(settle) => {
            thread::sleep(SETTLE);
            emit(Msg::Settled(settle));
        }
        Effect::Clean {
            serial,
            groups,
            policy,
        } => {
            let (sender, receiver) = mpsc::channel();
            let flag = Arc::clone(cancellation);
            let cleaner = thread::spawn(move || cleanup::clean(groups, policy, flag, sender));
            for event in receiver {
                emit(match event {
                    CleanupEvent::Started(kind) => Msg::Started(serial, kind),
                    CleanupEvent::Finished(kind, result) => Msg::Finished(serial, kind, result),
                    CleanupEvent::Complete {
                        before,
                        after,
                        elapsed,
                    } => Msg::Cleaned {
                        serial,
                        before,
                        after,
                        elapsed,
                    },
                });
            }
            let _ = cleaner.join();
        }
        Effect::Cancel => cancellation.store(true, Ordering::Relaxed),
        Effect::Estimate(Some(bytes)) => CleanupEstimate::save(bytes),
        Effect::Estimate(None) => CleanupEstimate::clear(),
    }
}

#[cfg(test)]
#[path = "flow_test.rs"]
mod tests;
