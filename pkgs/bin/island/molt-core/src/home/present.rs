//! Home as it is shown: every word on it, for any front end to lay out.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::build::Target;
use crate::home::flow::{Home, Repository};
use crate::home::repository::RepositoryStatus;
use crate::inputs::FlakeInput;
use crate::units::Units;

pub struct HomeView {
    pub title: &'static str,
    pub subtitle: String,
    /// Set when the configuration cannot be read: then there is nothing else.
    pub error: Option<String>,
    pub figures: Vec<Figure>,
    pub cards: Vec<Card>,
}

pub struct Figure {
    pub label: &'static str,
    pub value: String,
    pub detail: String,
    /// A note beside the value, such as a generation waiting for the next boot.
    pub chip: Option<(String, String)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardKind {
    Target(Target),
    Repository,
    Inputs,
    Search,
    Cleanup,
    Watchlist,
}

pub struct Card {
    pub kind: CardKind,
    pub title: &'static str,
    pub detail: String,
    pub detail_tip: Option<String>,
    pub status: Option<Status>,
    pub actions: Vec<Action>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// A plain chip.
    Neutral,
    /// Plain secondary text rather than a chip.
    Muted,
    Success,
    Attention,
    Error,
}

pub struct Status {
    pub text: String,
    pub tone: Tone,
    /// Work is under way behind it.
    pub busy: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionId {
    Review(Target),
    BuildAndSwitch(Target),
    Build(Target),
    Check,
    Pull,
    Inputs,
    Search,
    Cleanup,
    Watchlist,
}

pub struct Action {
    pub id: ActionId,
    pub label: &'static str,
    pub key: Option<char>,
    pub primary: bool,
    pub enabled: bool,
    pub busy: bool,
    pub failed: bool,
    pub tooltip: Option<String>,
}

impl Action {
    fn new(id: ActionId, label: &'static str, key: Option<char>) -> Self {
        Self {
            id,
            label,
            key,
            primary: false,
            enabled: true,
            busy: false,
            failed: false,
            tooltip: None,
        }
    }
}

fn status(text: impl Into<String>, tone: Tone) -> Option<Status> {
    Some(Status {
        text: text.into(),
        tone,
        busy: false,
    })
}

fn since(now: SystemTime, unix: i64) -> u64 {
    let then = UNIX_EPOCH + std::time::Duration::from_secs(unix.max(0) as u64);
    now.duration_since(then)
        .map_or(0, |elapsed| elapsed.as_secs())
}

pub fn view(home: &Home, now: SystemTime, units: &dyn Units) -> HomeView {
    let config = match &home.config {
        Ok(config) => config,
        Err(error) => {
            return HomeView {
                title: "Molt",
                subtitle: "System configuration and maintenance".to_owned(),
                error: Some(error.clone()),
                figures: Vec::new(),
                cards: Vec::new(),
            };
        }
    };
    let host = Target::NixOs.flake(config).ok();
    let version = home
        .system
        .version
        .as_deref()
        .map(|version| format!("NixOS {version}"));
    let subtitle = match (host, version) {
        (Some(host), Some(version)) => format!("{host} · {version}"),
        (Some(host), None) => host.to_owned(),
        (None, Some(version)) => version,
        (None, None) => "System configuration and maintenance".to_owned(),
    };

    let mut cards = Vec::new();
    let mut targets = vec![(Target::NixOs, 'N')];
    if config.home_flake.is_some() {
        targets.insert(0, (Target::HomeManager, 'M'));
    }
    for (target, key) in targets {
        let detail = target.flake(config).ok().map_or_else(
            || "User environment".to_owned(),
            |flake| format!("#{flake} · build, review, switch"),
        );
        let reviewed = home.reviewed.contains(&target);
        let mut actions = Vec::new();
        if reviewed {
            let mut review = Action::new(ActionId::Review(target), "Review", None);
            review.tooltip = Some("Reopen the latest reviewed build".to_owned());
            actions.push(review);
        }
        let mut switch = Action::new(ActionId::BuildAndSwitch(target), "Build & switch", None);
        switch.tooltip = Some(format!(
            "Build {} and switch to it without stopping to review",
            target.name()
        ));
        actions.push(switch);
        let mut build = Action::new(ActionId::Build(target), "Build", Some(key));
        build.primary = true;
        actions.push(build);
        cards.push(Card {
            kind: CardKind::Target(target),
            title: target.name(),
            detail,
            detail_tip: None,
            status: if reviewed {
                status("Reviewed build ready", Tone::Success)
            } else {
                None
            },
            actions,
        });
    }

    cards.push(repository_card(&home.repository));

    let inputs = home.inputs.as_deref();
    cards.push(Card {
        kind: CardKind::Inputs,
        title: "Inputs",
        detail: inputs.map_or_else(
            || "Manage flake.lock".to_owned(),
            |inputs| format!("{} direct inputs", inputs.len()),
        ),
        detail_tip: None,
        status: inputs
            .and_then(|inputs| input_age_summary(inputs, now, units))
            .and_then(|summary| status(summary, Tone::Muted)),
        actions: vec![Action::new(ActionId::Inputs, "Manage", Some('I'))],
    });

    cards.push(Card {
        kind: CardKind::Search,
        title: "Search",
        detail: "Packages, NixOS & Home Manager options".to_owned(),
        detail_tip: None,
        status: status("search.nixos.org", Tone::Muted),
        actions: vec![Action::new(ActionId::Search, "Search", Some('S'))],
    });

    cards.push(Card {
        kind: CardKind::Cleanup,
        title: "Cleanup",
        detail: "Generations, journals, caches, trash".to_owned(),
        detail_tip: Some(
            "Generations, journals, containers, build artifacts, caches, game leftovers, and trash"
                .to_owned(),
        ),
        status: status(
            home.cleanup.map_or_else(
                || "Not scanned yet".to_owned(),
                |estimate| {
                    format!(
                        "{} reclaimable · scanned {} ago",
                        units.size(estimate.bytes),
                        units.span(since(now, estimate.scanned_at))
                    )
                },
            ),
            Tone::Muted,
        ),
        actions: vec![Action::new(ActionId::Cleanup, "Clean up", Some('C'))],
    });

    let watchlist = home.watchlist;
    cards.push(Card {
        kind: CardKind::Watchlist,
        title: "Watchlist",
        detail: if watchlist.total == 0 {
            "Track PRs across branches".to_owned()
        } else if watchlist.ready_in_unstable > 0 {
            format!(
                "{} tracked · {} ready in nixos-unstable",
                watchlist.total, watchlist.ready_in_unstable
            )
        } else {
            format!("{} tracked · waiting on branches", watchlist.total)
        },
        detail_tip: None,
        status: if watchlist.ready_in_unstable > 0 {
            status(
                format!("{} ready", watchlist.ready_in_unstable),
                Tone::Success,
            )
        } else if watchlist.total > 0 {
            status("Waiting", Tone::Muted)
        } else {
            None
        },
        actions: vec![Action::new(ActionId::Watchlist, "Track", Some('W'))],
    });

    HomeView {
        title: "Molt",
        subtitle,
        error: None,
        figures: figures(home, inputs, now, units),
        cards,
    }
}

fn figures(
    home: &Home,
    inputs: Option<&[FlakeInput]>,
    now: SystemTime,
    units: &dyn Units,
) -> Vec<Figure> {
    let system = &home.system;
    let generation = Figure {
        label: "Generation",
        value: system
            .generation
            .map_or_else(|| "—".to_owned(), |generation| generation.to_string()),
        detail: "running".to_owned(),
        chip: system.staged.map(|staged| {
            (
                format!("{staged} on reboot"),
                format!("Generation {staged} is applied and becomes active after the next reboot"),
            )
        }),
    };
    let (value, detail) = system.activated.map_or_else(
        || ("Unknown".to_owned(), String::new()),
        |activated| {
            let elapsed = now.duration_since(activated).unwrap_or_default().as_secs();
            let unix = activated
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.as_secs() as i64);
            (format!("{} ago", units.span(elapsed)), units.moment(unix))
        },
    );
    let activated = Figure {
        label: "Activated",
        value,
        detail,
        chip: None,
    };
    let nixpkgs = inputs
        .into_iter()
        .flatten()
        .find(|input| input.name == "nixpkgs");
    let (value, detail) = nixpkgs.map_or_else(
        || ("—".to_owned(), String::new()),
        |input| {
            let age = input.last_modified.map_or_else(
                || "Unknown".to_owned(),
                |modified| format!("{} old", units.old(since(now, modified))),
            );
            (age, input.revision.chars().take(7).collect())
        },
    );
    let nixpkgs = Figure {
        label: "Nixpkgs",
        value,
        detail,
        chip: None,
    };
    vec![generation, activated, nixpkgs]
}

/// `nixpkgs 12d · oldest 41d`: how stale the lock is, from the inputs' locked timestamps.
fn input_age_summary(inputs: &[FlakeInput], now: SystemTime, units: &dyn Units) -> Option<String> {
    let nixpkgs = inputs
        .iter()
        .find(|input| input.name == "nixpkgs")
        .and_then(|input| input.last_modified);
    let oldest = inputs.iter().filter_map(|input| input.last_modified).min();
    match (nixpkgs, oldest) {
        (Some(nixpkgs), Some(oldest)) => Some(format!(
            "nixpkgs {} · oldest {}",
            units.age(since(now, nixpkgs)),
            units.age(since(now, oldest))
        )),
        (None, Some(oldest)) => Some(format!("oldest locked {}", units.age(since(now, oldest)))),
        _ => None,
    }
}

fn single_line(message: &str) -> String {
    message.lines().next().unwrap_or(message).trim().to_owned()
}

fn repository_card(repository: &Repository) -> Card {
    let mut check = Action::new(ActionId::Check, "Check", Some('R'));
    let mut pull = Action::new(ActionId::Pull, "Pull", Some('P'));
    pull.tooltip = Some("Fast-forward or rebase incoming changes".to_owned());
    let mut card = Card {
        kind: CardKind::Repository,
        title: "Repository",
        detail: String::new(),
        detail_tip: None,
        status: None,
        actions: Vec::new(),
    };
    let mut show_pull = false;
    let shown = match repository {
        Repository::Updating(before) => {
            pull.label = "Applying";
            pull.enabled = false;
            pull.busy = true;
            show_pull = true;
            before.as_ref()
        }
        other => other,
    };
    match shown {
        Repository::Checking { pull: shown } => {
            show_pull |= *shown;
            check.label = "Checking…";
            check.enabled = false;
            card.status = Some(Status {
                text: "Checking".to_owned(),
                tone: Tone::Neutral,
                busy: true,
            });
        }
        Repository::Updating(_) => {
            check.label = "Checking…";
            check.enabled = false;
            card.status = Some(Status {
                text: "Checking".to_owned(),
                tone: Tone::Neutral,
                busy: true,
            });
        }
        Repository::Ready(status) => ready(&mut card, status, &mut show_pull),
        Repository::CheckFailed(error) => {
            check.label = "Retry";
            card.detail = single_line(error);
            card.detail_tip = Some(error.clone());
            card.status = status("Check failed", Tone::Error);
        }
        Repository::UpdateFailed(error) => {
            card.detail = single_line(error);
            card.detail_tip = Some(error.clone());
            card.status = status("Update failed", Tone::Error);
            if !matches!(repository, Repository::Updating(_)) {
                pull.label = "Retry";
                pull.failed = true;
                pull.tooltip = Some(single_line(error));
            }
            show_pull = true;
        }
    }
    card.actions.push(check);
    if show_pull {
        card.actions.push(pull);
    }
    card
}

fn ready(card: &mut Card, repository: &RepositoryStatus, show_pull: &mut bool) {
    card.detail = format!("{} · {}", repository.kind.name(), repository.upstream);
    card.detail_tip = Some(format!(
        "{} incoming · {} local",
        repository.incoming, repository.local
    ));
    if repository.incoming > 0 {
        let pending = if repository.local > 0 {
            format!(
                "{} incoming · {} local",
                repository.incoming, repository.local
            )
        } else {
            format!("{} incoming", repository.incoming)
        };
        card.status = status(pending, Tone::Attention);
        *show_pull = true;
    } else if repository.local == 0 {
        card.status = status("Up to date", Tone::Success);
    } else {
        card.status = status(format!("{} local", repository.local), Tone::Neutral);
    }
}

#[cfg(test)]
#[path = "present_test.rs"]
mod tests;
