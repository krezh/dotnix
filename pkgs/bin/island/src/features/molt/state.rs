use serde::{Deserialize, Serialize};

use crate::units::plural;

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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub author: String,
    /// Unix seconds.
    pub time: i64,
    pub summary: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Updates {
    pub report: Option<Report>,
    pub checking: bool,
    /// The check under way was asked for, by Check now or a right-click, rather than due; only
    /// then does the energy border run round the button and the widget.
    pub refreshing: bool,
    pub error: Option<String>,
    /// Unix seconds of the last check attempt.
    pub checked_at: Option<u64>,
    /// Molt's Home, read when the panel opens.
    pub home: Option<molt_core::home::flow::Home>,
    /// Molt's Inputs, read when it opens, kept after so its undo outlives the screen.
    pub inputs: Option<molt_core::inputs::flow::Inputs>,
    pub watchlist: Option<molt_core::watchlist::flow::Watchlist>,
    pub search: Option<molt_core::search::flow::Search>,
    pub cleanup: Option<molt_core::cleanup::flow::Cleanup>,
    /// The latest build, kept once it ends so its failure or report stays to see.
    pub build: Option<molt_core::build::flow::Build>,
    /// What the latest successful build changes, and switching to it.
    pub review: Option<molt_core::report::flow::Review>,
    pub notes: Option<molt_core::report::notes::Notes>,
    /// The build whose end has been seen, so its droplet melts back into the bar.
    pub seen_build: u64,
    /// The latest switch's end has been seen.
    pub seen_activation: bool,
}

impl Updates {
    pub(super) fn incoming(&self) -> u32 {
        self.report.as_ref().map_or(0, |report| report.incoming)
    }

    pub(super) fn ready(&self) -> usize {
        self.report.as_ref().map_or(0, Report::ready)
    }

    pub(super) fn summary(&self) -> String {
        let incoming = Some(self.incoming())
            .filter(|incoming| *incoming > 0)
            .map(|incoming| plural(incoming, "incoming change", "incoming changes"));
        let ready = Some(self.ready())
            .filter(|ready| *ready > 0)
            .map(|ready| format!("{} ready", plural(ready, "watched PR", "watched PRs")));
        match (incoming, ready) {
            (None, None) => "Up to date".to_owned(),
            (Some(one), None) | (None, Some(one)) => one,
            (Some(incoming), Some(ready)) => format!("{incoming} · {ready}"),
        }
    }
}

#[cfg(test)]
#[path = "state_test.rs"]
mod tests;
