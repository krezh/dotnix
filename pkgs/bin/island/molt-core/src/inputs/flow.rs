//! What the Inputs screen knows and how it changes: the inputs, which is picked and its remote
//! history, the filter and sort, pinning, and lock updates with their undo.

use std::cmp::Ordering;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use crate::inputs::{self, FlakeInput, LockProgress, LockUpdate, RevisionRecord};

/// Locks older than this are flagged in the list as worth updating.
pub const STALE_AFTER: u64 = 60 * 86_400;

/// Inputs listed first, in this order, regardless of the sort mode.
const PINNED: [&str; 2] = ["nixpkgs", "home-manager"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum History {
    /// Nothing is picked.
    None,
    Loading,
    Ready(Vec<RevisionRecord>),
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Busy {
    pub message: String,
    pub progress: Option<LockProgress>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub text: String,
    /// The whole error, when the status is its first line.
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inputs {
    pub directory: PathBuf,
    /// Home Manager is built as well as NixOS.
    pub home_manager: bool,
    pub inputs: Vec<FlakeInput>,
    pub load_error: Option<String>,
    pub query: String,
    pub oldest_first: bool,
    pub selected: Option<String>,
    pub history: History,
    /// Counts up with each history fetch, so a late answer for another input is dropped.
    pub history_generation: u64,
    /// The pin field, open with what is typed in it.
    pub pin: Option<String>,
    pub busy: Option<Busy>,
    pub status: Option<Status>,
    /// What undoing every change Molt made to flake.lock restores.
    pub undo: Option<LockUpdate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Done {
    Updated(LockUpdate),
    Restored,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    Select(String),
    /// Moves the pick by this many shown inputs.
    Step(isize),
    Query(String),
    OldestFirst(bool),
    TogglePin,
    PinText(String),
    ApplyPin,
    UpdateSelected,
    UpdateAll,
    /// Locks the picked input to its history's commit at this place.
    LockAt(usize),
    Undo,
    History(u64, Result<Vec<RevisionRecord>, String>),
    Progress(LockProgress),
    /// A lock change finished, with flake.lock read again after it.
    Finished(
        Result<Done, String>,
        Result<Vec<FlakeInput>, String>,
    ),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    History { generation: u64, input: String },
    Update(Option<String>),
    Revision { input: String, flake_ref: String },
    Restore(LockUpdate),
}

/// What a finished lock change asks of the rest of Molt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Reviewed builds no longer match flake.lock.
    pub forget_reports: bool,
}

pub fn compare_inputs(left: &FlakeInput, right: &FlakeInput, oldest_first: bool) -> Ordering {
    let pin_rank = |input: &FlakeInput| {
        PINNED
            .iter()
            .position(|name| *name == input.name)
            .unwrap_or(PINNED.len())
    };
    let by_pin = pin_rank(left).cmp(&pin_rank(right));
    if by_pin.is_ne() {
        return by_pin;
    }
    let by_name = || left.name.cmp(&right.name);
    if !oldest_first {
        return by_name();
    }
    match (left.last_modified, right.last_modified) {
        (Some(left_time), Some(right_time)) => left_time.cmp(&right_time).then_with(by_name),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => by_name(),
    }
}

pub fn input_matches(input: &FlakeInput, query: &str) -> bool {
    let query = query.to_lowercase();
    query.is_empty()
        || input.name.to_lowercase().contains(&query)
        || input.source.to_lowercase().contains(&query)
        || input.revision.to_lowercase().contains(&query)
}

impl Inputs {
    /// Reads flake.lock and picks the first input, asking for its history.
    pub fn open(
        directory: PathBuf,
        home_manager: bool,
        undo: Option<LockUpdate>,
    ) -> (Self, Option<Effect>) {
        let mut screen = Self {
            directory,
            home_manager,
            inputs: Vec::new(),
            load_error: None,
            query: String::new(),
            oldest_first: false,
            selected: None,
            history: History::None,
            history_generation: 0,
            pin: None,
            busy: None,
            status: None,
            undo,
        };
        match inputs::load(&screen.directory) {
            Ok((_, loaded)) => {
                let effect = screen.set_inputs(loaded);
                screen.show_transaction();
                (screen, effect)
            }
            Err(error) => {
                screen.load_error = Some(error);
                (screen, None)
            }
        }
    }

    /// Shown in the order the sort asks for, filtered by what is typed.
    pub fn shown(&self) -> Vec<&FlakeInput> {
        let mut shown: Vec<&FlakeInput> = self
            .inputs
            .iter()
            .filter(|input| input_matches(input, &self.query))
            .collect();
        shown.sort_by(|left, right| compare_inputs(left, right, self.oldest_first));
        shown
    }

    pub fn current(&self) -> Option<&FlakeInput> {
        let selected = self.selected.as_ref()?;
        self.inputs.iter().find(|input| &input.name == selected)
    }

    fn first(&self) -> Option<String> {
        self.inputs
            .iter()
            .min_by(|left, right| compare_inputs(left, right, self.oldest_first))
            .map(|input| input.name.clone())
    }

    /// Takes freshly read inputs, keeping the pick where it still exists.
    fn set_inputs(&mut self, loaded: Vec<FlakeInput>) -> Option<Effect> {
        self.inputs = loaded;
        let keep = self
            .selected
            .clone()
            .filter(|name| self.inputs.iter().any(|input| &input.name == name));
        match keep.or_else(|| self.first()) {
            Some(name) => {
                self.selected = Some(name);
                self.history()
            }
            None => {
                self.clear();
                None
            }
        }
    }

    fn clear(&mut self) {
        self.selected = None;
        self.pin = None;
        self.history = History::None;
    }

    fn history(&mut self) -> Option<Effect> {
        let input = self.selected.clone()?;
        self.history_generation = self.history_generation.wrapping_add(1);
        self.history = History::Loading;
        if self.busy.is_none() {
            self.status = None;
        }
        Some(Effect::History {
            generation: self.history_generation,
            input,
        })
    }

    fn select(&mut self, name: String) -> Option<Effect> {
        if !self.inputs.iter().any(|input| input.name == name) {
            return None;
        }
        if self.selected.as_ref() != Some(&name) {
            self.pin = None;
        }
        self.selected = Some(name);
        self.history()
    }

    fn show_transaction(&mut self) {
        if self.undo.is_some() {
            self.status = Some(Status {
                text: "flake.lock updated · build the configuration or undo every Molt change"
                    .to_owned(),
                error: None,
            });
        }
    }

    fn fail(&mut self, error: &str) {
        self.status = Some(Status {
            text: error.lines().next().unwrap_or(error).to_owned(),
            error: Some(error.to_owned()),
        });
    }

    fn start(&mut self, message: String, effect: Effect) -> Option<Effect> {
        if self.busy.is_some() {
            return None;
        }
        self.status = None;
        self.busy = Some(Busy {
            message,
            progress: None,
        });
        Some(effect)
    }

    pub fn update(&mut self, msg: Msg) -> (Option<Effect>, Outcome) {
        let mut outcome = Outcome::default();
        let effect = match msg {
            Msg::Select(name) if self.busy.is_none() => self.select(name),
            Msg::Step(by) if self.busy.is_none() => {
                let shown = self.shown();
                let at = shown
                    .iter()
                    .position(|input| Some(&input.name) == self.selected.as_ref());
                let next = match at {
                    Some(at) => at.saturating_add_signed(by).min(shown.len().saturating_sub(1)),
                    None => 0,
                };
                match shown.get(next).map(|input| input.name.clone()) {
                    Some(name) => self.select(name),
                    None => None,
                }
            }
            Msg::Query(query) if self.busy.is_none() => {
                self.query = query;
                let current = self
                    .current()
                    .is_some_and(|input| input_matches(input, &self.query));
                if current {
                    None
                } else {
                    match self
                        .inputs
                        .iter()
                        .find(|input| input_matches(input, &self.query))
                        .map(|input| input.name.clone())
                    {
                        Some(name) => self.select(name),
                        None => {
                            self.clear();
                            None
                        }
                    }
                }
            }
            Msg::OldestFirst(oldest_first) => {
                self.oldest_first = oldest_first;
                None
            }
            Msg::TogglePin if self.current().is_some_and(|input| input.flake_ref.is_some()) => {
                self.pin = match self.pin {
                    Some(_) => None,
                    None => Some(String::new()),
                };
                None
            }
            Msg::PinText(text) => {
                if self.pin.is_some() {
                    self.pin = Some(text);
                }
                None
            }
            Msg::ApplyPin if self.can_apply_pin() => {
                let Some(input) = self.current().cloned() else {
                    return (None, outcome);
                };
                let revision = self.pin.clone().unwrap_or_default();
                match inputs::revision_ref(&input, &revision) {
                    Ok(flake_ref) => self.start(
                        format!("Resolving {} at {}…", input.name, revision.trim()),
                        Effect::Revision {
                            input: input.name,
                            flake_ref,
                        },
                    ),
                    Err(error) => {
                        self.fail(&error);
                        None
                    }
                }
            }
            Msg::UpdateSelected => match self.selected.clone() {
                Some(name) => self.start(format!("Updating {name}…"), Effect::Update(Some(name))),
                None => None,
            },
            Msg::UpdateAll if !self.inputs.is_empty() => {
                self.start("Updating every input…".to_owned(), Effect::Update(None))
            }
            Msg::LockAt(index) => {
                let input = self.current().map(|input| input.name.clone());
                let record = match &self.history {
                    History::Ready(records) => records.get(index).cloned(),
                    _ => None,
                };
                match (input, record) {
                    (Some(input), Some(record)) => self.start(
                        format!(
                            "Setting {input} to {}…",
                            short_revision(&record.revision)
                        ),
                        Effect::Revision {
                            input,
                            flake_ref: record.flake_ref,
                        },
                    ),
                    _ => None,
                }
            }
            Msg::Undo => match self.undo.clone() {
                Some(update) => self.start(
                    "Restoring flake.lock to before Molt's changes…".to_owned(),
                    Effect::Restore(update),
                ),
                None => None,
            },
            Msg::History(generation, result) if generation == self.history_generation => {
                match result {
                    Ok(records) => self.history = History::Ready(records),
                    Err(error) => {
                        self.history = History::Failed;
                        self.fail(&error);
                    }
                }
                None
            }
            Msg::Progress(progress) => {
                if let Some(busy) = &mut self.busy {
                    busy.progress = Some(progress);
                }
                None
            }
            Msg::Finished(result, reloaded) if self.busy.is_some() => {
                self.busy = None;
                match result {
                    Ok(Done::Updated(update)) => {
                        let changed = update.changed();
                        let pending = match self.undo.take() {
                            Some(previous) => previous.followed_by(update),
                            None => update,
                        };
                        let undoable = pending.changed();
                        self.undo = undoable.then_some(pending);
                        outcome.forget_reports = true;
                        match reloaded {
                            Ok(loaded) => {
                                self.pin = self.pin.as_ref().map(|_| String::new());
                                let effect = self.set_inputs(loaded);
                                self.show_transaction();
                                if !changed {
                                    self.status = Some(Status {
                                        text: "flake.lock is already current".to_owned(),
                                        error: None,
                                    });
                                } else if !undoable {
                                    self.status = Some(Status {
                                        text: "flake.lock is back to how Molt found it".to_owned(),
                                        error: None,
                                    });
                                }
                                effect
                            }
                            Err(error) => {
                                self.fail(&error);
                                None
                            }
                        }
                    }
                    Ok(Done::Restored) => {
                        self.undo = None;
                        outcome.forget_reports = true;
                        match reloaded {
                            Ok(loaded) => {
                                let effect = self.set_inputs(loaded);
                                self.status = Some(Status {
                                    text: "flake.lock restored to before Molt's changes"
                                        .to_owned(),
                                    error: None,
                                });
                                effect
                            }
                            Err(error) => {
                                self.fail(&error);
                                None
                            }
                        }
                    }
                    Err(error) => {
                        self.fail(&error);
                        None
                    }
                }
            }
            _ => None,
        };
        (effect, outcome)
    }

    pub fn can_apply_pin(&self) -> bool {
        self.busy.is_none()
            && self.current().is_some_and(|input| input.flake_ref.is_some())
            && self.pin.as_ref().is_some_and(|text| !text.trim().is_empty())
    }
}

pub fn short_revision(revision: &str) -> String {
    if revision.len() > 12
        && revision
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        revision[..12].to_owned()
    } else {
        revision.to_owned()
    }
}

/// Runs an [`Effect`] to the end; a lock change reports what Nix has fetched through `progress`.
pub fn run(
    effect: Effect,
    directory: &std::path::Path,
    cancellation: &AtomicBool,
    progress: impl Fn(LockProgress),
) -> Msg {
    let finished = |result: Result<Done, String>| {
        let reloaded = inputs::load(directory).map(|(_, loaded)| loaded);
        Msg::Finished(result, reloaded)
    };
    match effect {
        Effect::History { generation, input } => {
            Msg::History(generation, inputs::history(directory, &input, cancellation))
        }
        Effect::Update(input) => finished(
            inputs::update(directory, input.as_deref(), cancellation, progress).map(Done::Updated),
        ),
        Effect::Revision { input, flake_ref } => finished(
            inputs::update_at_revision(directory, &input, &flake_ref, cancellation, progress)
                .map(Done::Updated),
        ),
        Effect::Restore(update) => finished(inputs::restore(&update).map(|()| Done::Restored)),
    }
}

#[cfg(test)]
#[path = "flow_test.rs"]
mod tests;
