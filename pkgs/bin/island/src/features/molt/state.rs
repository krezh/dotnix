use serde::{Deserialize, Serialize};

use crate::{
    panel::{Route, Stop, StopState},
    units::plural,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub incoming: u32,
    pub local: u32,
    #[serde(default)]
    pub vcs: String,
    #[serde(default)]
    pub upstream: String,
    /// Newest incoming commits first; may be fewer than `incoming`.
    #[serde(default)]
    pub commits: Vec<Commit>,
    /// The pull requests molt watches; absent before molt had a watchlist.
    #[serde(default)]
    pub watchlist: Option<Watchlist>,
}

impl Report {
    fn ready(&self) -> usize {
        self.watchlist
            .as_ref()
            .map_or(0, |watchlist| watchlist.ready)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Watchlist {
    pub total: usize,
    pub ready: usize,
    #[serde(default)]
    pub prs: Vec<WatchedPr>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchedPr {
    pub pr_number: u64,
    pub repo: String,
    pub title: String,
    /// Reached the branch the system follows.
    pub ready: bool,
    /// `open`, `merged` or `closed`.
    pub state: String,
    #[serde(default)]
    pub merge_commit: Option<String>,
    #[serde(default)]
    pub branches: Vec<Branch>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub reached: bool,
    /// `reached`, `pending` or `open`.
    pub status: String,
    /// Commits the branch still has to take in before it has the pull request.
    #[serde(default)]
    pub ahead_by: Option<usize>,
}

impl WatchedPr {
    pub(super) fn row(&self) -> Route {
        let state = match self.state.as_str() {
            "merged" => "Merged",
            "open" => "Open",
            "closed" => "Closed",
            other => other,
        };
        let commit = self
            .merge_commit
            .as_deref()
            .map(|sha| format!(" · {}", &sha[..sha.len().min(7)]))
            .unwrap_or_default();
        Route {
            title: format!("#{} {}", self.pr_number, self.title).into(),
            detail: format!("{} · {state}{commit}", self.repo).into(),
            done: self.ready.then(|| "Ready".into()),
            vertical: false,
            stops: self
                .branches
                .iter()
                .map(|branch| Stop {
                    label: branch.name.clone().into(),
                    state: match (branch.reached, branch.status.as_str()) {
                        (true, _) => StopState::Reached,
                        (false, "pending") => StopState::Pending(
                            branch
                                .ahead_by
                                .filter(|ahead| *ahead > 0)
                                .map(|ahead| format!("+{ahead}").into()),
                        ),
                        _ => StopState::NotYet,
                    },
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub author: String,
    /// Unix seconds.
    pub time: i64,
    pub summary: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Updates {
    pub report: Option<Report>,
    pub checking: bool,
    /// The check under way was asked for, by Check now or a right-click, rather than due; only
    /// then does the energy border run round the button and the widget.
    pub refreshing: bool,
    pub error: Option<String>,
    /// Unix seconds of the last check attempt.
    pub checked_at: Option<u64>,
}

impl Updates {
    pub(super) fn incoming(&self) -> u32 {
        self.report.as_ref().map_or(0, |report| report.incoming)
    }

    pub(super) fn ready(&self) -> usize {
        self.report.as_ref().map_or(0, Report::ready)
    }

    pub(super) fn summary(&self) -> String {
        let incoming = self.incoming();
        let ready = self.ready();
        let ready = (ready > 0).then(|| {
            format!(
                "{} ready",
                plural(
                    u32::try_from(ready).unwrap_or(u32::MAX),
                    "watched PR",
                    "watched PRs"
                )
            )
        });
        match (incoming, ready) {
            (0, None) => "Up to date".to_owned(),
            (0, Some(ready)) => ready,
            (incoming, None) => plural(incoming, "incoming change", "incoming changes"),
            (incoming, Some(ready)) => format!(
                "{} · {ready}",
                plural(incoming, "incoming change", "incoming changes")
            ),
        }
    }
}

#[cfg(test)]
#[path = "state_test.rs"]
mod tests;
