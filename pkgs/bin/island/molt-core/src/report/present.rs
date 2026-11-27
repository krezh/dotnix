//! The review screen as it is shown: every word on it, for any front end to lay out.

use std::time::Instant;

use crate::build::Target;
use crate::protocol::ActivationMode;
use crate::report::config_diff::{ConfigChange, ConfigContent};
use crate::report::flow::{Outcome, Review};
use crate::report::{Change, ChangeStatus, ready_title, service_impact};
use crate::units::{Hue, Units};

/// Section and tab order.
pub const SECTION_ORDER: [ChangeStatus; 5] = [
    ChangeStatus::Upgraded,
    ChangeStatus::Downgraded,
    ChangeStatus::Added,
    ChangeStatus::Removed,
    ChangeStatus::Changed,
];

/// How a part of a line is coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    Plain,
    /// What it was: a version before the change.
    Old,
    Muted,
    /// What it is now: a version after the change.
    New,
    /// A closure or package that grew, which costs space.
    Grew,
    Shrank,
    /// Lines a file gained or lost.
    Added,
    Removed,
}

/// What a row opens: a package's release notes or a file's diff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opens {
    Changelog(usize),
    Diff(usize),
}

pub struct ChangeRow {
    pub name: String,
    pub version: Vec<(String, Ink)>,
    /// The same, unstyled, for a tooltip.
    pub version_plain: String,
    pub change: Vec<(String, Ink)>,
    /// A file's path, cut short in its middle rather than its end.
    pub path: bool,
    pub opens: Option<(Opens, &'static str, String)>,
}

pub struct ChangeGroup {
    pub status: ChangeStatus,
    pub hue: Hue,
    /// `Upgraded · 4`.
    pub heading: String,
    pub rows: Vec<ChangeRow>,
}

pub struct ServiceGroupView {
    /// `Restart · 2`.
    pub heading: String,
    pub hue: Hue,
    /// Each unit's name as shown, and its full name for a tooltip.
    pub units: Vec<(String, String)>,
}

pub struct Services {
    pub title: &'static str,
    pub notice: Option<String>,
    pub empty: Option<&'static str>,
    pub groups: Vec<ServiceGroupView>,
    /// The toggle for the raw dry-activate output, and the output when shown.
    pub output: Option<(&'static str, Option<String>)>,
}

pub struct Closure {
    pub title: &'static str,
    /// `+4.0 MB`, and whether it grew.
    pub delta: Option<(String, Ink)>,
    /// `32.0 GB → 32.0 GB`.
    pub range: Option<String>,
    pub rows: Vec<(&'static str, String, bool)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapsuleState {
    Running,
    Done,
    Failed,
}

pub struct ActivationCard {
    pub title: String,
    /// `1.2s`, `✓ in 4.0s`, `Failed`.
    pub elapsed: String,
    pub capsule: &'static str,
    pub state: CapsuleState,
    pub note: Option<String>,
    /// `Generation 5076`, once known.
    pub chip: Option<String>,
    pub error: Option<String>,
    pub copy: Option<&'static str>,
}

pub struct ActivateButton {
    pub mode: ActivationMode,
    pub label: &'static str,
    pub tooltip: String,
    pub primary: bool,
    /// Pressed once on a configuration for another host: a second press activates.
    pub confirming: bool,
    pub enabled: bool,
    pub busy: bool,
    pub done: bool,
    pub failed: bool,
}

pub struct ReviewView {
    pub title: &'static str,
    pub subtitle: String,
    /// Each tab's status, `None` for all, its name and how many it holds.
    pub tabs: Vec<(Option<ChangeStatus>, &'static str, usize)>,
    pub tab: usize,
    pub query: String,
    pub placeholder: &'static str,
    pub columns: [&'static str; 3],
    pub groups: Vec<ChangeGroup>,
    pub empty: Option<&'static str>,
    /// Under the list: files differing only in store paths, or why files could not be compared.
    pub footer: Option<String>,
    pub files_toggle: Option<&'static str>,
    pub files: Option<String>,
    pub closure: Closure,
    pub services: Option<Services>,
    pub activation: Option<ActivationCard>,
    pub buttons: Vec<ActivateButton>,
    pub minimize: Option<&'static str>,
    pub warnings: Vec<String>,
}

/// User-facing wording for one activation mode.
#[derive(Clone, Copy)]
pub struct ModeText {
    pub idle: &'static str,
    pub step: &'static str,
    pub running: &'static str,
    pub done: &'static str,
    pub failed: &'static str,
    pub tooltip: &'static str,
    pub cross_host_tooltip: &'static str,
    pub note: &'static str,
    pub complete_title: &'static str,
    pub failed_title: &'static str,
    pub conduit_running: &'static str,
    pub conduit_done: &'static str,
}

pub const fn mode_text(mode: ActivationMode) -> ModeText {
    match mode {
        ActivationMode::Switch => ModeText {
            idle: "Switch now",
            step: "Switch",
            running: "Activating",
            done: "Switched",
            failed: "Switch failed",
            tooltip: "Activate this configuration now",
            cross_host_tooltip: "This configuration targets another host; click twice to activate it",
            note: "Applying configuration and running activation scripts...",
            complete_title: "Configuration Activated",
            failed_title: "Switch Failed",
            conduit_running: "ACTIVATION IN PROGRESS",
            conduit_done: "SYSTEM ACTIVE",
        },
        ActivationMode::Boot => ModeText {
            idle: "Apply on reboot",
            step: "Boot",
            running: "Applying",
            done: "Applied",
            failed: "Apply failed",
            tooltip: "Make this configuration the boot default without activating it now",
            cross_host_tooltip: "This configuration targets another host; click twice to apply it on reboot",
            note: "Installing the boot entry; the running system is left untouched...",
            complete_title: "Applied — active after the next reboot",
            failed_title: "Apply Failed",
            conduit_running: "INSTALLING BOOT ENTRY",
            conduit_done: "ACTIVE AFTER REBOOT",
        },
    }
}

pub const FAILED_CONDUIT: &str = "ACTIVATION ENCOUNTERED AN ERROR";

/// A rebuild without a version change, e.g. after a dependency changed.
fn is_rebuild(change: &Change) -> bool {
    change.status == ChangeStatus::Changed && (change.old.is_empty() || change.old == change.new)
}

pub fn has_changelog(change: &Change) -> bool {
    !change.new.is_empty() && !is_rebuild(change)
}

/// Longer than this, an old and a new version show what they share once.
const LONG_CHANGE: usize = 28;

/// Where `version` may be split: after each `.`, `-`, `_` or `+`.
fn segments(version: &str) -> Vec<&str> {
    version.split_inclusive(['.', '-', '_', '+']).collect()
}

/// `1.0 → 2.0`; when that is long, what the versions share written once round what changed, as
/// `1.15.{18 → 19}-unstable-2026-11-24`.
pub fn version_change(old: &str, new: &str) -> Vec<(String, Ink)> {
    let whole = vec![
        (old.to_owned(), Ink::Old),
        (" → ".to_owned(), Ink::Muted),
        (new.to_owned(), Ink::New),
    ];
    if old.len() + new.len() + 3 <= LONG_CHANGE {
        return whole;
    }
    let (old_parts, new_parts) = (segments(old), segments(new));
    let shortest = old_parts.len().min(new_parts.len());
    let front = old_parts
        .iter()
        .zip(&new_parts)
        .take_while(|(old, new)| old == new)
        .count()
        .min(shortest.saturating_sub(1));
    let back = old_parts
        .iter()
        .rev()
        .zip(new_parts.iter().rev())
        .take_while(|(old, new)| old == new)
        .count()
        .min(shortest - front - 1);
    if front == 0 && back == 0 {
        return whole;
    }
    let join = |parts: &[&str]| parts.concat();
    let prefix = join(&old_parts[..front]);
    let suffix = join(&old_parts[old_parts.len() - back..]);
    let old_mid = join(&old_parts[front..old_parts.len() - back]);
    let new_mid = join(&new_parts[front..new_parts.len() - back]);
    // A separator closing the changed part stays outside the braces, with what follows it.
    let (old_mid, new_mid, suffix) = match (old_mid.chars().last(), new_mid.chars().last()) {
        (Some(a), Some(b)) if a == b && matches!(a, '.' | '-' | '_' | '+') => (
            old_mid[..old_mid.len() - 1].to_owned(),
            new_mid[..new_mid.len() - 1].to_owned(),
            format!("{a}{suffix}"),
        ),
        _ => (old_mid, new_mid, suffix),
    };
    let mut parts = Vec::new();
    if !prefix.is_empty() {
        parts.push((prefix, Ink::Muted));
    }
    parts.extend([
        (format!("{{{old_mid}"), Ink::Old),
        (" → ".to_owned(), Ink::Muted),
        (format!("{new_mid}}}"), Ink::New),
    ]);
    if !suffix.is_empty() {
        parts.push((suffix, Ink::Muted));
    }
    parts
}

/// The version column's parts, plus the same text unstyled. Never empty: a row without versions
/// still says why it is listed.
pub fn version_text(change: &Change) -> (Vec<(String, Ink)>, String) {
    let muted = |text: &str| (vec![(text.to_owned(), Ink::Muted)], text.to_owned());
    if is_rebuild(change) {
        return if change.new.is_empty() {
            muted("rebuilt")
        } else {
            (
                vec![
                    (change.new.clone(), Ink::Plain),
                    (" · rebuilt".to_owned(), Ink::Muted),
                ],
                format!("{} · rebuilt", change.new),
            )
        };
    }
    match (change.old.is_empty(), change.new.is_empty()) {
        (false, false) => (
            version_change(&change.old, &change.new),
            format!("{} → {}", change.old, change.new),
        ),
        (true, false) => (vec![(change.new.clone(), Ink::New)], change.new.clone()),
        (false, true) => (vec![(change.old.clone(), Ink::Old)], change.old.clone()),
        (true, true) => muted("no version"),
    }
}

fn signed(bytes: i64, units: &dyn Units) -> (String, Ink) {
    let sign = if bytes >= 0 { "+" } else { "-" };
    let ink = match bytes.cmp(&0) {
        std::cmp::Ordering::Greater => Ink::Grew,
        std::cmp::Ordering::Less => Ink::Shrank,
        std::cmp::Ordering::Equal => Ink::Muted,
    };
    (format!("{sign}{}", units.size(bytes.unsigned_abs())), ink)
}

/// The lines column of a file: `+2 −1`, or why there are no line counts.
fn line_counts(content: &ConfigContent) -> Vec<(String, Ink)> {
    match content {
        ConfigContent::Binary => vec![("binary".to_owned(), Ink::Muted)],
        ConfigContent::Text {
            added: 0,
            removed: 0,
            ..
        } => vec![("line endings".to_owned(), Ink::Muted)],
        ConfigContent::Text { added, removed, .. } => {
            let mut counts = Vec::with_capacity(2);
            if *added > 0 {
                counts.push((format!("+{added}"), Ink::Added));
            }
            if *removed > 0 {
                if !counts.is_empty() {
                    counts.push((" ".to_owned(), Ink::Plain));
                }
                counts.push((format!("−{removed}"), Ink::Removed));
            }
            counts
        }
    }
}

/// `3,238`: an integer with thousands separators.
pub fn grouped(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut text = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if value < 0 {
        text.push('-');
    }
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            text.push(',');
        }
        text.push(digit);
    }
    text
}

enum Entry<'a> {
    Package(usize, &'a Change),
    File(usize, &'a ConfigChange),
}

impl Entry<'_> {
    fn status(&self) -> ChangeStatus {
        match self {
            Self::Package(_, change) => change.status,
            Self::File(_, file) => file.status,
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Package(_, change) => &change.name,
            Self::File(_, file) => &file.path,
        }
    }

    fn row(&self, units: &dyn Units) -> ChangeRow {
        match self {
            Self::Package(index, change) => {
                let (version, version_plain) = version_text(change);
                let (size, ink) = signed(change.size, units);
                ChangeRow {
                    name: change.name.clone(),
                    version,
                    version_plain,
                    change: vec![(size, ink)],
                    path: false,
                    opens: has_changelog(change).then(|| {
                        (
                            Opens::Changelog(*index),
                            "Changelog ›",
                            format!("Release notes for {}", change.name),
                        )
                    }),
                }
            }
            Self::File(index, file) => ChangeRow {
                name: file.path.clone(),
                version: vec![("config file".to_owned(), Ink::Muted)],
                version_plain: "config file".to_owned(),
                change: line_counts(&file.content),
                path: true,
                opens: Some((
                    Opens::Diff(*index),
                    "Diff ›",
                    format!("Changes to {}", file.path),
                )),
            },
        }
    }
}

fn rank(status: ChangeStatus) -> usize {
    SECTION_ORDER
        .iter()
        .position(|candidate| *candidate == status)
        .unwrap_or(SECTION_ORDER.len())
}

fn closure(review: &Review, units: &dyn Units) -> Closure {
    let report = &review.report;
    let (delta, range) = match report.sizes {
        Some((old, new)) => (
            Some(signed(new - old, units)),
            Some(format!(
                "{} → {}",
                units.size(old.unsigned_abs()),
                units.size(new.unsigned_abs())
            )),
        ),
        None => (None, None),
    };
    let mut rows = Vec::new();
    if let Some((_, new, added, removed)) = report.paths {
        rows.push((
            "Paths",
            format!("{}  +{added} −{removed}", grouped(new)),
            true,
        ));
    }
    rows.push((
        "Packages",
        format!("{} changed", report.changes.len()),
        false,
    ));
    rows.push((
        "Config files",
        match &report.config {
            Ok(config) => format!("{} changed", config.changes.len()),
            Err(_) => "unavailable".to_owned(),
        },
        false,
    ));
    Closure {
        title: "Closure",
        delta,
        range,
        rows,
    }
}

fn services(review: &Review) -> Option<Services> {
    if review.report.target != Target::NixOs {
        return None;
    }
    let lines = match &review.report.operational_impact {
        Ok(lines) => lines,
        Err(error) => {
            return Some(Services {
                title: "Services",
                notice: Some(format!("Impact analysis unavailable: {error}")),
                empty: None,
                groups: Vec::new(),
                output: None,
            });
        }
    };
    let groups = service_impact(lines);
    Some(Services {
        title: "Services",
        notice: None,
        empty: groups.is_empty().then_some(if lines.is_empty() {
            "Dry activation reports no service changes"
        } else {
            "No units would change"
        }),
        groups: groups
            .iter()
            .map(|group| ServiceGroupView {
                heading: format!("{} · {}", group.action.label(), group.units.len()),
                hue: group.action.hue(),
                units: group
                    .units
                    .iter()
                    .map(|unit| {
                        let tip = if unit.user {
                            format!("{} (user unit)", unit.name)
                        } else {
                            unit.name.clone()
                        };
                        (unit.display_name().to_owned(), tip)
                    })
                    .collect(),
            })
            .collect(),
        output: (!lines.is_empty()).then(|| {
            if review.output_open {
                ("Hide dry-activate output ‹", Some(lines.join("\n")))
            } else {
                ("Full dry-activate output ›", None)
            }
        }),
    })
}

fn activation(review: &Review) -> Option<ActivationCard> {
    let activation = review.activation.as_ref()?;
    let text = mode_text(activation.mode);
    let report = &review.report;
    Some(match &activation.outcome {
        None => ActivationCard {
            title: match activation.mode {
                ActivationMode::Switch => {
                    format!("Activating {} #{}", report.target.name(), report.flake)
                }
                ActivationMode::Boot => {
                    format!(
                        "Applying {} #{} on reboot",
                        report.target.name(),
                        report.flake
                    )
                }
            },
            elapsed: format!("{:.1}s", activation.started.elapsed().as_secs_f64()),
            capsule: text.conduit_running,
            state: CapsuleState::Running,
            note: Some(text.note.to_owned()),
            chip: None,
            error: None,
            copy: None,
        },
        Some((Outcome::Done(generation), took)) => ActivationCard {
            title: text.complete_title.to_owned(),
            elapsed: format!("✓ in {:.1}s", took.as_secs_f64()),
            capsule: text.conduit_done,
            state: CapsuleState::Done,
            note: None,
            chip: generation.map(|generation| format!("Generation {generation}")),
            error: None,
            copy: None,
        },
        Some((Outcome::Failed(error), _)) => ActivationCard {
            title: text.failed_title.to_owned(),
            elapsed: "Failed".to_owned(),
            capsule: FAILED_CONDUIT,
            state: CapsuleState::Failed,
            note: None,
            chip: None,
            error: Some(error.clone()),
            copy: Some("Copy error"),
        },
    })
}

fn button(review: &Review, mode: ActivationMode) -> ActivateButton {
    let text = mode_text(mode);
    let current = review
        .activation
        .as_ref()
        .filter(|activation| activation.mode == mode);
    let running = current.is_some_and(|activation| activation.outcome.is_none());
    let failed = current.and_then(|activation| match &activation.outcome {
        Some((Outcome::Failed(error), _)) => Some(error.clone()),
        _ => None,
    });
    let done = review.done.contains(&mode);
    let confirming = review.confirming == Some(mode);
    let label = if running {
        text.running
    } else if done {
        text.done
    } else if confirming {
        "Are you sure?"
    } else if failed.is_some() {
        text.failed
    } else {
        text.idle
    };
    let tooltip = match (&failed, confirming, review.cross_host) {
        (Some(error), _, _) => error.clone(),
        (None, true, _) => "Click again to activate this configuration".to_owned(),
        (None, false, true) => text.cross_host_tooltip.to_owned(),
        (None, false, false) => text.tooltip.to_owned(),
    };
    ActivateButton {
        mode,
        label,
        tooltip,
        primary: mode == ActivationMode::Switch,
        confirming,
        enabled: review.can_activate(mode),
        busy: running,
        done,
        failed: failed.is_some(),
    }
}

/// The review screen; `built_in` words how long the build took.
pub fn view(review: &Review, units: &dyn Units) -> ReviewView {
    let report = &review.report;
    let (files, footer, store_paths) = match &report.config {
        Ok(config) => {
            let count = config.store_path_only.len();
            let noun = if count == 1 {
                "file differs"
            } else {
                "files differ"
            };
            (
                config.changes.as_slice(),
                (count > 0).then(|| format!("{count} {noun} only in store paths")),
                (count > 0).then(|| config.store_path_only.join("\n")),
            )
        }
        Err(error) => (
            &[][..],
            Some(format!("Configuration comparison unavailable: {error}")),
            None,
        ),
    };
    let mut entries: Vec<Entry> = report
        .changes
        .iter()
        .enumerate()
        .map(|(index, change)| Entry::Package(index, change))
        .chain(
            files
                .iter()
                .enumerate()
                .map(|(index, file)| Entry::File(index, file)),
        )
        .collect();
    entries.sort_by_key(|entry| rank(entry.status()));
    let mut totals = [0_usize; 5];
    for entry in &entries {
        totals[rank(entry.status())] += 1;
    }
    let mut tabs = vec![(None, "All", entries.len())];
    tabs.extend(
        SECTION_ORDER
            .iter()
            .enumerate()
            .filter(|(at, _)| totals[*at] > 0)
            .map(|(at, status)| (Some(*status), status.label(), totals[at])),
    );
    let tab = tabs
        .iter()
        .position(|(status, _, _)| *status == review.tab)
        .unwrap_or(0);
    let query = review.query.to_lowercase();
    let mut groups: Vec<ChangeGroup> = Vec::new();
    for entry in &entries {
        let status = entry.status();
        if review.tab.is_some_and(|tab| tab != status)
            || !entry.name().to_lowercase().contains(&query)
        {
            continue;
        }
        if groups.last().is_none_or(|group| group.status != status) {
            groups.push(ChangeGroup {
                status,
                hue: status.hue(),
                heading: String::new(),
                rows: Vec::new(),
            });
        }
        if let Some(group) = groups.last_mut() {
            group.rows.push(entry.row(units));
        }
    }
    for group in &mut groups {
        group.heading = format!("{} · {}", group.status.label(), group.rows.len());
    }
    let empty = if entries.is_empty() {
        Some("No package or configuration changes")
    } else if groups.is_empty() {
        Some("Nothing matches the filter")
    } else {
        None
    };
    let mut subtitle = format!("{} #{}", report.target.name(), report.flake);
    if let Some(built_in) = report.built_in {
        subtitle.push_str(&format!(" · built in {}", units.elapsed(built_in)));
    }
    let mut buttons = Vec::new();
    if report.target == Target::NixOs {
        buttons.push(button(review, ActivationMode::Boot));
    }
    buttons.push(button(review, ActivationMode::Switch));
    ReviewView {
        title: ready_title(report.target),
        subtitle,
        tabs,
        tab,
        query: review.query.clone(),
        placeholder: "Filter packages and files",
        columns: ["Name", "Version", "Change"],
        groups,
        empty,
        files_toggle: store_paths.as_ref().map(|_| {
            if review.files_open {
                "Hide files ‹"
            } else {
                "Show files ›"
            }
        }),
        files: store_paths.filter(|_| review.files_open),
        footer,
        closure: closure(review, units),
        services: services(review),
        activation: activation(review),
        buttons,
        minimize: review.activating().then_some("Minimize"),
        warnings: report.warnings.clone(),
    }
}

/// What the closed panel shows while activating, or once done.
pub fn mini(review: &Review, now: Instant) -> Option<crate::build::present::Mini> {
    let activation = review.activation.as_ref()?;
    let text = mode_text(activation.mode);
    let report = &review.report;
    Some(match &activation.outcome {
        None => crate::build::present::Mini {
            step: text.step,
            title: format!("{} {}", text.running, report.target.name()),
            brief: String::new(),
            note: text.note.to_owned(),
            elapsed: now.saturating_duration_since(activation.started),
            progress: None,
            failed: false,
            ready: false,
        },
        Some((Outcome::Done(_), took)) => crate::build::present::Mini {
            step: text.done,
            title: text.done.to_owned(),
            brief: String::new(),
            note: format!("{} #{}", report.target.name(), report.flake),
            elapsed: *took,
            progress: Some(1.0),
            failed: false,
            ready: true,
        },
        Some((Outcome::Failed(error), took)) => crate::build::present::Mini {
            step: text.step,
            title: text.failed.to_owned(),
            brief: String::new(),
            note: error.lines().next().unwrap_or_default().to_owned(),
            elapsed: *took,
            progress: None,
            failed: true,
            ready: false,
        },
    })
}

#[cfg(test)]
#[path = "present_test.rs"]
mod tests;
