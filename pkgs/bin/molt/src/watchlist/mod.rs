//! Upstream pull request tracking: monitor PRs through Nixpkgs channel promotion.

pub(crate) mod db;
pub(crate) mod github;
pub(crate) mod ui;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum StepState {
    Reached,
    Pending { ahead_by: usize },
    Open,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ChannelStep {
    pub(crate) name: String,
    pub(crate) state: StepState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum PrState {
    Open,
    Merged,
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WatchedPr {
    pub(crate) id: i64,
    pub(crate) owner: String,
    pub(crate) repo: String,
    pub(crate) pr_number: u64,
    pub(crate) title: String,
    pub(crate) html_url: String,
    pub(crate) pr_state: PrState,
    pub(crate) merge_commit: Option<String>,
    pub(crate) merged_at: Option<String>,
    pub(crate) channels: Vec<ChannelStep>,
    pub(crate) added_at: i64,
    pub(crate) last_checked: i64,
}

impl WatchedPr {
    pub(crate) fn is_ready_in_channel(&self, channel: &str) -> bool {
        self.channels
            .iter()
            .any(|c| c.name == channel && c.state == StepState::Reached)
    }

    pub(crate) fn is_ready(&self) -> bool {
        if self.channels.iter().any(|c| c.name == "nixos-unstable") {
            self.is_ready_in_channel("nixos-unstable")
        } else {
            self.channels
                .last()
                .is_some_and(|c| c.state == StepState::Reached)
        }
    }
}
