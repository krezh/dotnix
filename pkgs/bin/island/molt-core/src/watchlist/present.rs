//! The Watchlist screen as it is shown: every word on it, for any front end to lay out.

use crate::watchlist::flow::Watchlist;
use crate::watchlist::{PrState, StepState, WatchedPr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Green,
    Yellow,
    Mauve,
    Teal,
    Peach,
}

pub struct WatchlistView {
    pub title: &'static str,
    pub subtitle: String,
    pub filter_placeholder: &'static str,
    pub query: String,
    pub empty: &'static str,
    pub rows: Vec<Row>,
    pub track_placeholder: &'static str,
    pub track: String,
    pub track_enabled: bool,
    pub detail: Option<Detail>,
    /// In place of the detail when nothing is picked.
    pub nothing: &'static str,
    pub status: Option<(String, bool)>,
    pub refreshing: bool,
}

pub struct Row {
    pub id: i64,
    pub title: String,
    pub sub: String,
    pub badge: (&'static str, Tone),
    pub selected: bool,
}

pub struct Detail {
    pub title: String,
    pub chips: Vec<(String, Tone)>,
    pub url: String,
    /// The merge commit, short, and where it opens.
    pub commit: Option<(String, String)>,
    pub meta: String,
    pub summary: String,
    pub steps: Vec<Step>,
}

pub struct Step {
    pub name: String,
    pub description: String,
    pub reached: bool,
    pub pill: (String, Tone),
    pub url: Option<String>,
}

fn badge(pr: &WatchedPr) -> (&'static str, Tone) {
    if pr.is_ready() {
        ("Ready", Tone::Green)
    } else if pr.pr_state == PrState::Merged {
        ("Waiting", Tone::Yellow)
    } else if pr.pr_state == PrState::Open {
        ("Open", Tone::Mauve)
    } else {
        ("Closed", Tone::Plain)
    }
}

fn commits(ahead_by: usize) -> &'static str {
    if ahead_by == 1 { "commit" } else { "commits" }
}

fn pill(state: &StepState) -> (String, Tone) {
    match state {
        StepState::Reached => ("✓ In branch".to_owned(), Tone::Green),
        StepState::Pending { ahead_by } if *ahead_by > 0 => (
            format!("⏳ Waiting (+{ahead_by} {})", commits(*ahead_by)),
            Tone::Yellow,
        ),
        StepState::Pending { .. } => ("⏳ Waiting".to_owned(), Tone::Yellow),
        StepState::Open => ("⋯ Pending".to_owned(), Tone::Plain),
    }
}

fn detail(pr: &WatchedPr) -> Detail {
    let (state, tone) = match pr.pr_state {
        PrState::Merged => ("Merged", Tone::Green),
        PrState::Open => ("Open", Tone::Mauve),
        PrState::Closed => ("Closed", Tone::Peach),
    };
    let commit_url =
        |sha: &str| format!("https://github.com/{}/{}/commit/{sha}", pr.owner, pr.repo);
    let (commit, meta) = match (pr.pr_state, pr.merge_commit.as_deref()) {
        (PrState::Merged, Some(sha)) => (
            Some((sha[..sha.len().min(12)].to_owned(), commit_url(sha))),
            pr.merged_at.as_deref().map_or_else(
                || "· Merged into master".to_owned(),
                |date| format!("· Merged on {}", &date[..date.len().min(10)]),
            ),
        ),
        (state, _) => (
            None,
            match state {
                PrState::Open => "Open for review upstream",
                PrState::Closed => "Closed without merge",
                PrState::Merged => "Merged into master",
            }
            .to_owned(),
        ),
    };
    let (pr_desc, pr_reached) = match pr.pr_state {
        PrState::Merged => ("Merged upstream into trunk", true),
        PrState::Open => ("Under review / open", false),
        PrState::Closed => ("Closed without merge", false),
    };
    let pr_state = if pr_reached {
        StepState::Reached
    } else if pr.pr_state == PrState::Open {
        StepState::Open
    } else {
        StepState::Pending { ahead_by: 0 }
    };
    let mut steps = vec![Step {
        name: format!("Pull Request #{}", pr.pr_number),
        description: pr_desc.to_owned(),
        reached: pr_reached,
        pill: pill(&pr_state),
        url: Some(pr.html_url.clone()),
    }];
    for channel in &pr.channels {
        let compare = |sha: &str| {
            format!(
                "https://github.com/{}/{}/compare/{}...{sha}",
                pr.owner, pr.repo, channel.name
            )
        };
        let (description, url) = match &pr.merge_commit {
            Some(sha) => match channel.state {
                StepState::Reached => (
                    format!("Landed in branch at {}", &sha[..sha.len().min(7)]),
                    Some(commit_url(sha)),
                ),
                StepState::Pending { ahead_by } if ahead_by > 0 => (
                    format!(
                        "Waiting on Hydra channel advancement (+{ahead_by} {})",
                        commits(ahead_by)
                    ),
                    Some(compare(sha)),
                ),
                StepState::Pending { .. } => (
                    "Waiting on branch advancement".to_owned(),
                    Some(compare(sha)),
                ),
                StepState::Open => ("Pending PR merge".to_owned(), Some(pr.html_url.clone())),
            },
            None => (
                match channel.state {
                    StepState::Open => "Pending PR merge",
                    _ => "Waiting on branch advancement",
                }
                .to_owned(),
                Some(pr.html_url.clone()),
            ),
        };
        steps.push(Step {
            name: channel.name.clone(),
            description,
            reached: channel.state == StepState::Reached,
            pill: pill(&channel.state),
            url,
        });
    }
    let reached = pr
        .channels
        .iter()
        .filter(|channel| channel.state == StepState::Reached)
        .count();
    Detail {
        title: pr.title.clone(),
        chips: vec![
            (format!("#{}", pr.pr_number), Tone::Mauve),
            (format!("{}/{}", pr.owner, pr.repo), Tone::Teal),
            (state.to_owned(), tone),
        ],
        url: pr.html_url.clone(),
        commit,
        meta,
        summary: format!(
            "{reached} reached · {} pending",
            pr.channels.len().saturating_sub(reached)
        ),
        steps,
    }
}

pub fn view(screen: &Watchlist) -> WatchlistView {
    let total = screen.prs.len();
    let ready = screen.prs.iter().filter(|pr| pr.is_ready()).count();
    let subtitle = if total == 0 {
        "No pull requests tracked yet".to_owned()
    } else if ready > 0 {
        format!("{total} direct pull requests · {ready} ready in nixos-unstable")
    } else {
        format!("{total} direct pull requests · waiting on branch promotion")
    };
    WatchlistView {
        title: "Watchlist",
        subtitle,
        filter_placeholder: "Filter pull requests",
        query: screen.query.clone(),
        empty: "No pull requests tracked yet",
        rows: screen
            .shown()
            .into_iter()
            .map(|pr| Row {
                id: pr.id,
                title: pr.title.clone(),
                sub: format!("#{} · {}/{}", pr.pr_number, pr.owner, pr.repo),
                badge: badge(pr),
                selected: screen.selected == Some(pr.id),
            })
            .collect(),
        track_placeholder: "Paste PR URL or #number…",
        track: screen.track.clone(),
        track_enabled: !screen.tracking && !screen.track.trim().is_empty(),
        detail: screen.current().map(detail),
        nothing: "Select a pull request from the list, or track a new PR using the input above.",
        status: screen.status.clone(),
        refreshing: screen.refreshing,
    }
}
