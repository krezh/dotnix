//! The Clean up screen as it is shown: every word on it, for any front end to lay out.

use std::cmp::Reverse;

use crate::cleanup::flow::{Cleanup, Phase, category_title};
use crate::cleanup::{CleanupKind, DiskUsage, Volume};
use crate::units::{Hue, Units};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Error,
    Success,
}

pub struct Status {
    pub text: String,
    pub tone: Tone,
    pub busy: bool,
}

pub struct VolumeRow {
    pub mount: String,
    /// `121.9 GB of 195.8 GB used`.
    pub used: String,
    /// `→ 65.2 GB`, once something on it is selected.
    pub after: String,
    /// Each selected category's share of the volume, in order.
    pub breakdown: Vec<String>,
    /// Shares of the volume's capacity: what stays used, then what each selected category frees.
    pub parts: Vec<(f32, Option<Hue>)>,
}

pub struct ItemRow {
    pub label: String,
    pub detail: String,
    /// Against the category's largest item.
    pub fraction: Option<f32>,
    pub size: String,
    /// Sized only when cleaned, such as a Nix generation.
    pub pending: bool,
}

pub struct Items {
    pub kind: CleanupKind,
    pub title: &'static str,
    pub summary: String,
    pub note: Option<String>,
    pub filter_placeholder: &'static str,
    pub query: String,
    pub columns: [&'static str; 4],
    pub rows: Vec<ItemRow>,
    /// How many more match than are listed, said.
    pub more: Option<String>,
    /// In place of the rows when there are none.
    pub empty: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    Ticked(bool),
    /// Nothing to remove, or the cleaner is unavailable.
    Unavailable,
    Cleaning,
    Cleaned,
    Failed(String),
}

pub struct PlanRow {
    pub kind: CleanupKind,
    pub title: &'static str,
    pub description: String,
    pub size: String,
    /// Why the cleaner is unavailable.
    pub size_tip: Option<String>,
    pub open: bool,
    pub pick: Pick,
    /// Locked while cleaning or once cleaned.
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    Generations,
    Journals,
}

pub struct Retention {
    pub setting: Setting,
    pub title: &'static str,
    pub hint: &'static str,
    pub warning: &'static str,
    pub tooltip: &'static str,
    pub value: String,
    pub placeholder: &'static str,
    /// Whether it steps by one, as a count does.
    pub stepped: bool,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionLook {
    Primary,
    Danger,
    Plain,
}

pub struct Action {
    pub label: String,
    pub look: ActionLook,
    pub enabled: bool,
    pub busy: bool,
}

pub struct CleanupView {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub status: Status,
    /// `648.6 GB available`.
    pub available: String,
    /// The volume rows, or why there are none yet.
    pub volumes: Result<Vec<VolumeRow>, &'static str>,
    pub reclaim: String,
    pub reclaim_detail: String,
    pub items: Items,
    pub plan_title: &'static str,
    pub plan_summary: String,
    pub plan: Vec<PlanRow>,
    pub retention: Vec<Retention>,
    pub action: Action,
    pub back_enabled: bool,
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

fn total(plan: &[(CleanupKind, u64)]) -> u64 {
    plan.iter()
        .fold(0_u64, |total, (_, bytes)| total.saturating_add(*bytes))
}

/// `Cleaning Nix store` / `Cleaning Nix store and Trash` / `Cleaning Nix store and 3 more`.
fn running_status(running: &[CleanupKind]) -> String {
    match running {
        [] => "Cleanup in progress".to_owned(),
        [only] => format!("Cleaning {}", category_title(*only)),
        [first, second] => format!(
            "Cleaning {} and {}",
            category_title(*first),
            category_title(*second)
        ),
        [first, rest @ ..] => format!(
            "Cleaning {} and {} more",
            category_title(*first),
            rest.len()
        ),
    }
}

fn confirmation(screen: &Cleanup) -> String {
    let Ok(policy) = screen.entered_policy() else {
        return "Enter a valid cleanup retention policy".to_owned();
    };
    let mut policies = Vec::new();
    if screen.is_selected(CleanupKind::Nix) {
        policies.push(if policy.keep_generations == 0 {
            "keep no old Nix profile generations".to_owned()
        } else {
            format!(
                "keep the latest {} generation(s) per Nix profile",
                policy.keep_generations
            )
        });
    }
    if screen.is_selected(CleanupKind::Journals) {
        policies.push(if policy.journal_max_age == "0" {
            "keep no archived journal history".to_owned()
        } else {
            format!(
                "keep journal entries from the last {}",
                policy.journal_max_age
            )
        });
    }
    if screen.is_selected(CleanupKind::Docker) {
        policies.push("prune all unused Docker data".to_owned());
    }
    if screen.is_selected(CleanupKind::GamePrefixes) {
        policies.push(
            "trim Proton prefixes of uninstalled games, keeping their users and ProgramData folders"
                .to_owned(),
        );
    }
    if policies.is_empty() {
        "Confirm deletion of every item shown in the selected categories".to_owned()
    } else {
        format!("Confirm cleanup: {}", policies.join("; "))
    }
}

fn status(screen: &Cleanup, units: &dyn Units) -> Status {
    let plain = |text: String, busy| Status {
        text,
        tone: Tone::Plain,
        busy,
    };
    match screen.phase {
        Phase::Complete => {
            let freed = screen.freed.as_ref();
            let bytes = units.size(freed.map_or(0, |freed| freed.bytes));
            let seconds = freed.map_or(0.0, |freed| freed.elapsed.as_secs_f64());
            let failures = freed.map_or(0, |freed| freed.failures);
            if failures == 0 {
                Status {
                    text: format!("Freed {bytes} in {seconds:.1}s · cleanup complete"),
                    tone: Tone::Success,
                    busy: false,
                }
            } else {
                Status {
                    text: format!(
                        "Freed {bytes} in {seconds:.1}s · {failures} categor{} failed",
                        if failures == 1 { "y" } else { "ies" }
                    ),
                    tone: Tone::Error,
                    busy: false,
                }
            }
        }
        Phase::Cleaning => plain(running_status(&screen.running), true),
        Phase::Review if screen.scanning => {
            plain("Inspecting every available cleaner".to_owned(), true)
        }
        Phase::Review if screen.armed => Status {
            text: confirmation(screen),
            tone: Tone::Error,
            busy: false,
        },
        Phase::Review => match (&screen.notice, screen.entered_policy()) {
            (Some(notice), _) => plain(notice.clone(), screen.rescanning),
            (None, Err(error)) => Status {
                text: error,
                tone: Tone::Error,
                busy: false,
            },
            (None, Ok(_)) if !screen.plan_is_current() || screen.rescanning => plain(
                "Updating the Nix plan for the new retention".to_owned(),
                true,
            ),
            (None, Ok(_)) => plain("Review every category before cleaning".to_owned(), false),
        },
    }
}

fn used_text(usage: DiskUsage, units: &dyn Units) -> String {
    format!(
        "{} of {} used",
        units.size(usage.used),
        units.size(usage.total)
    )
}

fn volume_row(
    screen: &Cleanup,
    volume: &Volume,
    plan: &[(CleanupKind, u64)],
    units: &dyn Units,
) -> VolumeRow {
    // Categories spread over several filesystems count only their share on this one.
    let parts: Vec<(CleanupKind, u64)> = plan
        .iter()
        .map(|(kind, bytes)| {
            let share = screen
                .group(*kind)
                .filter(|group| !group.volumes.is_empty())
                .map(|group| {
                    group
                        .volumes
                        .iter()
                        .find(|(device, _)| *device == volume.device)
                        .map_or(0, |(_, bytes)| *bytes)
                });
            (*kind, share.unwrap_or(*bytes))
        })
        .filter(|(kind, bytes)| *bytes > 0 && volume.kinds.contains(kind))
        .collect();
    let usage = volume.usage;
    let selected = total(&parts);
    let reclaim = selected.min(usage.used);
    let after = if reclaim > 0 {
        format!("→ {}", units.size(usage.used - reclaim))
    } else {
        String::new()
    };
    let breakdown = parts
        .iter()
        .map(|(kind, bytes)| format!("{} · {}", category_title(*kind), units.size(*bytes)))
        .collect();
    let mut shares = Vec::new();
    if usage.total > 0 {
        // Estimates can exceed the used space; every category then shrinks by the same factor.
        let scale = if selected == 0 {
            0.0
        } else {
            reclaim as f64 / selected as f64
        };
        let capacity = usage.total as f64;
        shares.push((((usage.used - reclaim) as f64 / capacity) as f32, None));
        for (kind, bytes) in &parts {
            shares.push(((*bytes as f64 * scale / capacity) as f32, Some(kind.hue())));
        }
    }
    VolumeRow {
        mount: volume.mount.display().to_string(),
        used: used_text(usage, units),
        after,
        breakdown,
        parts: shares,
    }
}

fn items(screen: &Cleanup, limit: usize, units: &dyn Units) -> Items {
    let kind = screen.open;
    let base = Items {
        kind,
        title: category_title(kind),
        summary: String::new(),
        note: None,
        filter_placeholder: "Filter items",
        query: screen.query.clone(),
        columns: ["Item", "Detail", "", "Size"],
        rows: Vec::new(),
        more: None,
        empty: None,
    };
    let Some(group) = screen.group(kind) else {
        return Items {
            summary: "Scanning…".to_owned(),
            empty: Some("Scanning this cleanup job…"),
            ..base
        };
    };
    let summary = if group.available {
        format!(
            "{} · {} estimated",
            plural(group.items.len(), "item", "items"),
            units.size(group.reclaimable)
        )
    } else {
        "Cleanup job unavailable".to_owned()
    };
    let largest = group
        .items
        .iter()
        .filter_map(|item| item.bytes)
        .max()
        .unwrap_or(0);
    let query = screen.query.trim().to_lowercase();
    let mut matching: Vec<_> = group
        .items
        .iter()
        .filter(|item| {
            query.is_empty()
                || item.label.to_lowercase().contains(&query)
                || item.detail.to_lowercase().contains(&query)
        })
        .collect();
    // Items sized only at cleanup time (Nix generations) lead, then the largest first.
    matching.sort_by_key(|item| Reverse(item.bytes.unwrap_or(u64::MAX)));
    let more = matching.len().saturating_sub(limit);
    let rows = matching
        .into_iter()
        .take(limit)
        .map(|item| ItemRow {
            label: item.label.clone(),
            detail: item.detail.clone(),
            fraction: item
                .bytes
                .filter(|_| largest > 0)
                .map(|bytes| (bytes as f64 / largest as f64).clamp(0.0, 1.0) as f32)
                .filter(|fraction| *fraction > 0.0),
            size: item
                .bytes
                .map_or_else(|| "On cleanup".to_owned(), |bytes| units.size(bytes)),
            pending: item.bytes.is_none(),
        })
        .collect::<Vec<_>>();
    let empty = if !rows.is_empty() {
        None
    } else if !group.items.is_empty() {
        Some("No items match this filter")
    } else if group.available {
        Some("Nothing is currently eligible for removal")
    } else {
        Some("The scan did not return a removal plan")
    };
    Items {
        summary,
        note: group.note.clone(),
        rows,
        more: (more > 0).then(|| format!("{more} more not listed · filter to find them")),
        empty,
        ..base
    }
}

fn plan_row(screen: &Cleanup, kind: CleanupKind, units: &dyn Units) -> PlanRow {
    let outcome = screen
        .outcomes
        .iter()
        .find(|(other, _)| *other == kind)
        .map(|(_, result)| result);
    let group = screen.group(kind);
    let (size, size_tip) = if screen.running.contains(&kind) {
        ("Cleaning…".to_owned(), None)
    } else if let Some(result) = outcome {
        match result {
            Ok(bytes) => (format!("{} cleaned", units.size(*bytes)), None),
            Err(error) => ("Failed".to_owned(), Some(error.clone())),
        }
    } else {
        match group {
            None => ("Scanning…".to_owned(), None),
            Some(group) if group.available => (units.size(group.reclaimable), None),
            Some(group) => (
                "Unavailable".to_owned(),
                Some(
                    group
                        .note
                        .clone()
                        .unwrap_or_else(|| "Cleaner unavailable".to_owned()),
                ),
            ),
        }
    };
    let pick = if screen.running.contains(&kind) {
        Pick::Cleaning
    } else {
        match outcome {
            Some(Ok(_)) => Pick::Cleaned,
            Some(Err(error)) => Pick::Failed(error.clone()),
            None if group.is_some_and(super::flow::selectable) => {
                Pick::Ticked(screen.is_selected(kind))
            }
            None => Pick::Unavailable,
        }
    };
    PlanRow {
        kind,
        title: category_title(kind),
        description: kind.description().replace('\n', " "),
        size,
        size_tip,
        open: screen.open == kind,
        pick,
        enabled: screen.phase == Phase::Review,
    }
}

fn action(screen: &Cleanup, units: &dyn Units, bytes: u64, count: usize) -> Action {
    let enabled = screen.can_act();
    match screen.phase {
        Phase::Cleaning => Action {
            label: "Cancel".to_owned(),
            look: ActionLook::Plain,
            enabled,
            busy: true,
        },
        Phase::Complete => Action {
            label: "Scan again".to_owned(),
            look: ActionLook::Plain,
            enabled,
            busy: false,
        },
        Phase::Review if screen.armed => Action {
            label: "Confirm cleanup".to_owned(),
            look: ActionLook::Danger,
            enabled,
            busy: false,
        },
        Phase::Review => {
            let busy = screen.busy();
            let label = if busy || (screen.entered_policy().is_ok() && !screen.plan_is_current()) {
                "Scanning…".to_owned()
            } else if count == 0 {
                "Nothing selected".to_owned()
            } else {
                format!("Clean {}", units.size(bytes))
            };
            Action {
                label,
                look: ActionLook::Primary,
                enabled,
                busy,
            }
        }
    }
}

/// The screen, listing at most `limit` of the open category's items.
pub fn view(screen: &Cleanup, limit: usize, units: &dyn Units) -> CleanupView {
    let plan = screen.plan();
    let bytes = total(&plan);
    let count = plan.len();
    let (available, volumes) = match &screen.volumes {
        None => ("Inspecting storage…".to_owned(), Err("Reading filesystem capacity")),
        Some(volumes) if volumes.is_empty() => (
            "Storage unavailable".to_owned(),
            Err("Could not read filesystem usage"),
        ),
        Some(volumes) => (
            format!(
                "{} available",
                units.size(super::flow::available(volumes))
            ),
            Ok(volumes
                .iter()
                .map(|volume| volume_row(screen, volume, &plan, units))
                .collect()),
        ),
    };
    let editable = screen.phase == Phase::Review;
    let mut retention = Vec::new();
    if screen.is_selected(CleanupKind::Nix) {
        retention.push(Retention {
            setting: Setting::Generations,
            title: "Keep Nix generations",
            hint: "0 keeps only the current one · fewer rollbacks",
            warning: "Removing old generations reduces rollback history. Keeping 0 removes non-current generations; the current generation remains.",
            tooltip: "Old generations to keep per Nix profile: 0–100",
            value: screen.generations.clone(),
            placeholder: "0",
            stepped: true,
            enabled: editable,
        });
    }
    if screen.is_selected(CleanupKind::Journals) {
        retention.push(Retention {
            setting: Setting::Journals,
            title: "Keep journal history",
            hint: "0 removes all archived history · or 7d, 12h, 30min",
            warning: "Keeping 0 removes archived journal history. Active journals are rotated before archived entries are removed.",
            tooltip: "Journal history to keep: 0, 7d, 12h, or 30min",
            value: screen.journals.clone(),
            placeholder: "0",
            stepped: false,
            enabled: editable,
        });
    }
    CleanupView {
        title: "Clean up",
        subtitle: "System cleanup",
        status: status(screen, units),
        available,
        volumes,
        reclaim: units.size(bytes),
        reclaim_detail: format!(
            "to reclaim · {}",
            plural(count, "category", "categories")
        ),
        items: items(screen, limit, units),
        plan_title: "Removal plan",
        plan_summary: match count {
            0 => "Nothing selected".to_owned(),
            1 => format!("{} in 1 category", units.size(bytes)),
            _ => format!("{} across {count} categories", units.size(bytes)),
        },
        plan: CleanupKind::ALL
            .into_iter()
            .map(|kind| plan_row(screen, kind, units))
            .collect(),
        retention,
        action: action(screen, units, bytes, count),
        back_enabled: screen.phase != Phase::Cleaning,
    }
}
