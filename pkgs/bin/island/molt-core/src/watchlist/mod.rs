//! Upstream pull request tracking: monitor PRs through Nixpkgs channel promotion.

pub mod db;
pub mod flow;
pub mod github;
pub mod present;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepState {
    Reached,
    Pending { ahead_by: usize },
    Open,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelStep {
    pub name: String,
    pub state: StepState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrState {
    Open,
    Merged,
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchedPr {
    pub id: i64,
    pub owner: String,
    pub repo: String,
    pub pr_number: u64,
    pub title: String,
    pub html_url: String,
    pub pr_state: PrState,
    pub merge_commit: Option<String>,
    pub merged_at: Option<String>,
    pub channels: Vec<ChannelStep>,
    pub added_at: i64,
    pub last_checked: i64,
}

impl WatchedPr {
    pub fn is_ready_in_channel(&self, channel: &str) -> bool {
        self.channels
            .iter()
            .any(|c| c.name == channel && c.state == StepState::Reached)
    }

    pub fn is_ready(&self) -> bool {
        if self.channels.iter().any(|c| c.name == "nixos-unstable") {
            self.is_ready_in_channel("nixos-unstable")
        } else {
            self.channels
                .last()
                .is_some_and(|c| c.state == StepState::Reached)
        }
    }
}
