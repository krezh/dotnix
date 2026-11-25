//! The Inputs screen as it is shown: every word on it, for any front end to lay out.

use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::inputs::flow::{History, Inputs, STALE_AFTER, short_revision};
use crate::inputs::{FlakeInput, LockProgress, RevisionRecord};
use crate::units::Units;

pub struct InputsView {
    pub title: &'static str,
    pub subtitle: String,
    /// When flake.lock cannot be read: then there is nothing else.
    pub error: Option<String>,
    pub sort: [&'static str; 2],
    pub oldest_first: bool,
    pub filter_placeholder: &'static str,
    pub query: String,
    /// The list and every action wait while a lock change runs.
    pub busy: bool,
    pub rows: Vec<Row>,
    pub detail: Detail,
    pub history: HistoryView,
    pub status: Option<StatusLine>,
    pub transaction: Vec<Action>,
    pub update_all: Action,
}

pub struct Row {
    pub name: String,
    pub source: String,
    /// `4d`, and whether it is old enough to flag.
    pub age: Option<(String, bool)>,
    pub age_tip: Option<String>,
    pub selected: bool,
}

pub struct Detail {
    pub name: String,
    pub kind: Option<String>,
    pub follows: Option<String>,
    pub source: String,
    pub revision: String,
    pub revision_tip: String,
    /// `· 4 days old`.
    pub age: String,
    pub age_tip: Option<String>,
    /// `· up to date`, and whether it says there is something newer.
    pub upstream: (String, bool),
    pub pin: Action,
    pub update: Action,
    /// The field under the name while pinning, with what is typed and whether Apply can run.
    pub pin_field: Option<PinField>,
    pub picked: bool,
}

pub struct PinField {
    pub text: String,
    pub placeholder: &'static str,
    pub apply: Action,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionId {
    TogglePin,
    ApplyPin,
    UpdateSelected,
    UpdateAll,
    Undo,
    BuildHomeManager,
    BuildNixOs,
}

pub struct Action {
    pub id: ActionId,
    pub label: &'static str,
    pub primary: bool,
    pub enabled: bool,
    pub busy: bool,
    pub tooltip: Option<&'static str>,
}

impl Action {
    fn new(id: ActionId, label: &'static str, enabled: bool) -> Self {
        Self {
            id,
            label,
            primary: false,
            enabled,
            busy: false,
            tooltip: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Newer,
    Current,
    Older,
    /// The locked commit is not in the fetched history, so its order to it is unknown.
    Unplaced,
}

pub struct HistoryRow {
    /// Its place in the history, for locking to it.
    pub index: usize,
    pub revision: String,
    pub summary: String,
    pub date: String,
    pub date_tip: String,
    pub place: Place,
    pub action: &'static str,
    pub tooltip: Option<&'static str>,
}

pub struct HistoryView {
    pub loading: bool,
    /// `0 newer · 47 older`, and whether it warns.
    pub status: (String, bool),
    /// In place of the rows.
    pub message: Option<String>,
    /// Over the rows.
    pub notice: Option<String>,
    pub rows: Vec<HistoryRow>,
}

pub struct StatusLine {
    pub text: String,
    pub error: Option<String>,
    pub busy: bool,
}

fn since(now: SystemTime, unix: i64) -> u64 {
    let then = UNIX_EPOCH + std::time::Duration::from_secs(unix.max(0) as u64);
    now.duration_since(then).map_or(0, |elapsed| elapsed.as_secs())
}

fn lock_age(input: &FlakeInput, now: SystemTime) -> Option<u64> {
    input.last_modified.map(|timestamp| since(now, timestamp))
}

/// `Setting nixpkgs to 494ce7f… · 31 MB downloaded`, then `… · adding to the Nix store` once the
/// transfer ends and Nix unpacks it silently.
fn progress_text(message: &str, progress: LockProgress, units: &dyn Units) -> Option<String> {
    if progress.downloaded == 0 {
        return None;
    }
    let size = units.size(progress.downloaded);
    Some(if progress.downloading {
        format!("{message} · {size} downloaded")
    } else {
        format!("{message} · {size} downloaded · adding to the Nix store")
    })
}

pub fn view(screen: &Inputs, now: SystemTime, units: &dyn Units) -> InputsView {
    let busy = screen.busy.is_some();
    let stale = screen
        .inputs
        .iter()
        .filter(|input| lock_age(input, now).is_some_and(|age| age >= STALE_AFTER))
        .count();
    let subtitle = if stale == 0 {
        format!("{} direct inputs", screen.inputs.len())
    } else {
        format!(
            "{} direct inputs · {stale} locked over {} days ago",
            screen.inputs.len(),
            STALE_AFTER / 86_400
        )
    };
    let rows = screen
        .shown()
        .into_iter()
        .map(|input| {
            let age = lock_age(input, now);
            Row {
                name: input.name.clone(),
                source: input.source.clone(),
                age: age.map(|age| (units.age(age), age >= STALE_AFTER)),
                age_tip: age.map(|age| format!("Locked {} ago", units.old(age))),
                selected: screen.selected.as_ref() == Some(&input.name),
            }
        })
        .collect();
    let current = screen.current();
    let history = history(screen, current, now, units);
    let detail = detail(screen, current, now, units);
    let status = match &screen.busy {
        Some(running) => Some(StatusLine {
            text: running
                .progress
                .and_then(|progress| progress_text(&running.message, progress, units))
                .unwrap_or_else(|| running.message.clone()),
            error: None,
            busy: true,
        }),
        None => screen.status.as_ref().map(|status| StatusLine {
            text: status.text.clone(),
            error: status.error.clone(),
            busy: false,
        }),
    };
    let mut transaction = Vec::new();
    if screen.undo.is_some() {
        transaction.push(Action::new(ActionId::Undo, "Undo update", !busy));
        if screen.home_manager {
            transaction.push(Action::new(
                ActionId::BuildHomeManager,
                "Build Home Manager",
                !busy,
            ));
        }
        transaction.push(Action {
            primary: true,
            ..Action::new(ActionId::BuildNixOs, "Build NixOS", !busy)
        });
    }
    InputsView {
        title: "Inputs",
        subtitle,
        error: screen.load_error.clone(),
        sort: ["Name", "Oldest first"],
        oldest_first: screen.oldest_first,
        filter_placeholder: "Filter inputs",
        query: screen.query.clone(),
        busy,
        rows,
        detail,
        history,
        status,
        transaction,
        update_all: Action::new(
            ActionId::UpdateAll,
            "Update all",
            !busy && !screen.inputs.is_empty(),
        ),
    }
}

fn detail(
    screen: &Inputs,
    current: Option<&FlakeInput>,
    now: SystemTime,
    units: &dyn Units,
) -> Detail {
    let busy = screen.busy.is_some();
    let pinnable = current.is_some_and(|input| input.flake_ref.is_some());
    let pin = Action {
        tooltip: Some("Lock this input to a specific commit, tag, or branch"),
        ..Action::new(
            ActionId::TogglePin,
            if screen.pin.is_some() { "Cancel" } else { "Pin…" },
            pinnable,
        )
    };
    let update = Action {
        primary: true,
        busy,
        ..Action::new(
            ActionId::UpdateSelected,
            "Update to latest",
            !busy && current.is_some(),
        )
    };
    let Some(input) = current else {
        return Detail {
            name: "No input selected".to_owned(),
            kind: None,
            follows: None,
            source: String::new(),
            revision: String::new(),
            revision_tip: String::new(),
            age: String::new(),
            age_tip: None,
            upstream: (String::new(), false),
            pin,
            update,
            pin_field: None,
            picked: false,
        };
    };
    let upstream = match &screen.history {
        History::Ready(records) => match position(records, &input.revision) {
            Some((_, newer, _)) if newer > 0 => (
                format!(
                    "· {newer} newer commit{} upstream",
                    if newer == 1 { "" } else { "s" }
                ),
                true,
            ),
            Some(_) => ("· up to date".to_owned(), false),
            None => ("· not in recent history".to_owned(), false),
        },
        _ => (String::new(), false),
    };
    Detail {
        name: input.name.clone(),
        kind: Some(input.source_kind.clone()).filter(|kind| !kind.is_empty()),
        follows: input
            .follows
            .as_ref()
            .map(|follows| format!("follows {follows}")),
        source: input.source.clone(),
        revision: short_revision(&input.revision),
        revision_tip: input.revision.clone(),
        age: lock_age(input, now).map_or_else(
            || "· date unavailable".to_owned(),
            |age| format!("· {} old", units.old(age)),
        ),
        age_tip: input.last_modified.map(|unix| units.date(unix)),
        upstream,
        pin,
        update,
        pin_field: screen.pin.as_ref().map(|text| PinField {
            text: text.clone(),
            placeholder: "Commit, tag, or branch",
            apply: Action::new(ActionId::ApplyPin, "Apply", screen.can_apply_pin()),
        }),
        picked: true,
    }
}

fn position(records: &[RevisionRecord], current: &str) -> Option<(usize, usize, usize)> {
    let index = records
        .iter()
        .position(|record| record.revision == current)?;
    Some((index, index, records.len().saturating_sub(index + 1)))
}

fn history(
    screen: &Inputs,
    current: Option<&FlakeInput>,
    now: SystemTime,
    units: &dyn Units,
) -> HistoryView {
    let empty = |loading: bool, status: &str, message: &str| HistoryView {
        loading,
        status: (status.to_owned(), false),
        message: Some(message.to_owned()),
        notice: None,
        rows: Vec::new(),
    };
    let Some(input) = current else {
        return empty(false, "", "Select an input");
    };
    let records = match &screen.history {
        History::None => return empty(false, "", "Select an input"),
        History::Loading => return empty(true, "Loading…", "Fetching remote commits…"),
        History::Failed => return empty(false, "Unavailable", "Remote revisions unavailable"),
        History::Ready(records) => records,
    };
    let placed = position(records, &input.revision);
    let rows = records
        .iter()
        .enumerate()
        .map(|(index, record)| {
            let place = match placed {
                Some((current, _, _)) => match index.cmp(&current) {
                    Ordering::Less => Place::Newer,
                    Ordering::Equal => Place::Current,
                    Ordering::Greater => Place::Older,
                },
                None => Place::Unplaced,
            };
            HistoryRow {
                index,
                revision: short_revision(&record.revision),
                summary: record.summary.clone(),
                date: units
                    .parse_moment(&record.date)
                    .map(|unix| format!("{} ago", units.old(since(now, unix))))
                    .unwrap_or_else(|| history_date(&record.date, units)),
                date_tip: history_date(&record.date, units),
                place,
                action: match place {
                    Place::Newer => "Update here",
                    Place::Current => "Locked",
                    Place::Older => "Roll back",
                    Place::Unplaced => "Lock here",
                },
                tooltip: match place {
                    Place::Newer => Some("Move this input forward to this commit"),
                    Place::Older => Some("Roll this input back to this commit"),
                    Place::Unplaced => Some("Lock this input to this commit"),
                    Place::Current => None,
                },
            }
        })
        .collect();
    let (status, notice) = match placed {
        Some((_, newer, older)) => ((format!("{newer} newer · {older} older"), false), None),
        None => (
            ("Locked commit not in recent history".to_owned(), true),
            (!records.is_empty()).then(|| {
                format!(
                    "The locked commit is not among the latest {} remote commits",
                    records.len()
                )
            }),
        ),
    };
    HistoryView {
        loading: false,
        status,
        message: records
            .is_empty()
            .then(|| "No other remote revisions found".to_owned()),
        notice,
        rows,
    }
}

fn history_date(value: &str, units: &dyn Units) -> String {
    if let Ok(timestamp) = value.parse::<i64>() {
        return units.date(timestamp);
    }
    value.get(..10).unwrap_or(value).to_owned()
}
