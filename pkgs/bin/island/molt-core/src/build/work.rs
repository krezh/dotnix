//! Build work rows: what nix plans, builds, and downloads, and the throughput and time left.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use crate::build::rows::{RowContent, RowLook, activity_name};
use crate::nix::{NixBuildProgress, NixProgressMetric};
use crate::units::Units;

/// Finished builds needed before their mean is trusted for a time estimate.
const ESTIMATE_AFTER: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkState {
    Queued,
    Building,
    Downloading,
    Done,
    Failed,
}

impl WorkState {
    const fn priority(self) -> u8 {
        match self {
            Self::Failed => 5,
            Self::Downloading => 4,
            Self::Building => 3,
            Self::Done => 2,
            Self::Queued => 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkItem {
    pub state: WorkState,
    pub detail: String,
    pub started: Option<Instant>,
    pub took: Option<Duration>,
    pub bytes: (u64, u64),
}

impl WorkItem {
    const fn new(state: WorkState) -> Self {
        Self {
            state,
            detail: String::new(),
            started: None,
            took: None,
            bytes: (0, 0),
        }
    }

    pub fn content(&self, path: &str, now: Instant, units: &dyn Units) -> RowContent {
        let running = || {
            self.started.map_or_else(
                || "—".to_owned(),
                |started| units.timing(now.saturating_duration_since(started), false),
            )
        };
        let (group, look, value) = match self.state {
            WorkState::Failed => (0, RowLook::Failed, "failed".to_owned()),
            WorkState::Building => (1, RowLook::Active, running()),
            WorkState::Downloading => (
                2,
                RowLook::Transfer,
                units.transfer(self.bytes.0, self.bytes.1),
            ),
            WorkState::Queued => (3, RowLook::Idle, String::new()),
            WorkState::Done => (
                4,
                RowLook::Done,
                self.took
                    .map(|took| units.timing(took, false))
                    .unwrap_or_default(),
            ),
        };
        let detail = match (self.state, self.detail.is_empty()) {
            (_, false) => self.detail.clone(),
            (WorkState::Failed, true) if path.ends_with(".drv") => "build failed".to_owned(),
            (WorkState::Failed, true) => "download failed".to_owned(),
            (WorkState::Downloading, true) => "downloading".to_owned(),
            (_, true) => "building".to_owned(),
        };
        RowContent {
            group,
            look,
            title: activity_name(path),
            tag: String::new(),
            column: String::new(),
            detail,
            value,
            fraction: if self.bytes.1 > 0 {
                self.bytes.0 as f64 / self.bytes.1 as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkSummary {
    pub total: usize,
    pub done: usize,
    pub failed: usize,
    pub in_flight: usize,
    /// Mean duration of finished builds, once enough have finished to trust it.
    pub mean_build: Option<Duration>,
}

/// Every planned or seen piece of build work, kept across progress snapshots so work that stops
/// without a completion record still lands in Done. Items are keyed by store path, so a
/// derivation nix reports through several activities still gets one row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkPlan {
    items: HashMap<String, WorkItem>,
    /// The paths holding each display name; shared names get a store-hash tag.
    names: HashMap<String, Vec<String>>,
}

impl WorkPlan {
    /// Folds a snapshot in and returns the paths whose item changed, new ones sorted by name.
    pub fn update(&mut self, progress: &NixBuildProgress, now: Instant) -> Vec<String> {
        let mut next = HashMap::<&str, WorkItem>::new();
        for path in &progress.planned {
            merge(&mut next, path, WorkItem::new(WorkState::Queued));
        }
        for (path, took) in &progress.completed {
            let mut item = WorkItem::new(WorkState::Done);
            item.took = Some(*took);
            merge(&mut next, path, item);
        }
        for path in &progress.failed {
            merge(&mut next, path, WorkItem::new(WorkState::Failed));
        }
        // Snapshots list the newest activity first; folding it in last lets it win ties, so a
        // derivation's row shows its latest log line.
        for activity in progress.items.iter().rev() {
            let Some(path) = activity.path.as_deref() else {
                continue;
            };
            let state = if activity.failed > 0 {
                WorkState::Failed
            } else if activity.kind == 101 {
                WorkState::Downloading
            } else {
                WorkState::Building
            };
            merge(
                &mut next,
                path,
                WorkItem {
                    state,
                    detail: activity.detail.clone().unwrap_or_default(),
                    started: activity.started,
                    took: None,
                    bytes: (activity.done, activity.expected),
                },
            );
        }

        let mut changed = Vec::new();
        let mut added = Vec::new();
        for (path, item) in &mut self.items {
            // Planned paths stay in `planned` for the whole build, so work that stopped without a
            // completion record reappears only as Queued; it has finished, not gone back in line.
            let still_running = next
                .get(path.as_str())
                .is_some_and(|next| next.state != WorkState::Queued);
            if matches!(item.state, WorkState::Building | WorkState::Downloading) && !still_running
            {
                item.state = WorkState::Done;
                item.took = item
                    .started
                    .map(|started| now.saturating_duration_since(started));
                changed.push(path.clone());
            }
        }
        for (path, mut item) in next {
            match self.items.get_mut(path) {
                Some(current) if *current == item => {}
                // Planning never demotes work that already started or finished.
                Some(current)
                    if item.state == WorkState::Queued && current.state != WorkState::Queued => {}
                Some(current) => {
                    if item.state == WorkState::Failed && item.detail.is_empty() {
                        item.detail = std::mem::take(&mut current.detail);
                        item.started = current.started;
                    }
                    *current = item;
                    changed.push(path.to_owned());
                }
                None => {
                    self.items.insert(path.to_owned(), item);
                    added.push(path.to_owned());
                }
            }
        }
        added.sort_by_cached_key(|path| (activity_name(path), path.clone()));
        for path in &added {
            let holders = self.names.entry(activity_name(path)).or_default();
            // The first holder of a name now needs its tag too.
            if let [first] = holders.as_slice() {
                changed.push(first.clone());
            }
            holders.push(path.clone());
        }
        changed.extend(added);
        changed
    }

    pub fn get(&self, path: &str) -> Option<&WorkItem> {
        self.items.get(path)
    }

    /// Every path the plan holds.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.items.keys().map(String::as_str)
    }

    /// The row for `path`, tagged with its store hash when another item shares its name.
    pub fn content(&self, path: &str, now: Instant, units: &dyn Units) -> Option<RowContent> {
        let mut content = self.items.get(path)?.content(path, now, units);
        if self
            .names
            .get(&content.title)
            .is_some_and(|holders| holders.len() > 1)
        {
            content.tag = store_hash(path)
                .map(|hash| format!("· {hash}"))
                .unwrap_or_default();
        }
        Some(content)
    }

    /// Work still in flight when nix stopped never finished: it goes back to Queued. Returns
    /// the paths that moved.
    pub fn interrupt(&mut self) -> Vec<String> {
        let mut moved = Vec::new();
        for (path, item) in &mut self.items {
            if matches!(item.state, WorkState::Building | WorkState::Downloading) {
                item.state = WorkState::Queued;
                moved.push(path.clone());
            }
        }
        moved
    }

    pub fn failed(&self) -> impl Iterator<Item = &str> {
        self.items
            .iter()
            .filter(|(_, item)| item.state == WorkState::Failed)
            .map(|(path, _)| path.as_str())
    }

    /// Paths of builds in flight, the only rows whose text changes with time alone.
    pub fn running(&self) -> impl Iterator<Item = &str> {
        self.items
            .iter()
            .filter(|(_, item)| item.state == WorkState::Building)
            .map(|(path, _)| path.as_str())
    }

    pub fn summary(&self) -> WorkSummary {
        let mut summary = WorkSummary {
            total: self.items.len(),
            ..WorkSummary::default()
        };
        let mut builds = Vec::new();
        for (path, item) in &self.items {
            match item.state {
                WorkState::Done => {
                    summary.done += 1;
                    if path.ends_with(".drv")
                        && let Some(took) = item.took
                    {
                        builds.push(took);
                    }
                }
                WorkState::Failed => summary.failed += 1,
                WorkState::Building | WorkState::Downloading => summary.in_flight += 1,
                WorkState::Queued => {}
            }
        }
        if builds.len() >= ESTIMATE_AFTER {
            summary.mean_build = Some(builds.iter().sum::<Duration>() / builds.len() as u32);
        }
        summary
    }
}

fn merge<'a>(items: &mut HashMap<&'a str, WorkItem>, path: &'a str, item: WorkItem) {
    if items
        .get(path)
        .is_none_or(|current| current.state.priority() <= item.state.priority())
    {
        items.insert(path, item);
    }
}

/// `3k9f2a1` from `/nix/store/3k9f2a1…-molt-0.1.0.drv`.
fn store_hash(path: &str) -> Option<&str> {
    path.strip_prefix("/nix/store/")?
        .split('-')
        .next()?
        .get(..7)
}

/// Time left as remaining work times the mean finished build, once that mean exists.
pub fn remaining_estimate(summary: &WorkSummary) -> Option<Duration> {
    let remaining = summary.total.saturating_sub(summary.done + summary.failed);
    Some(summary.mean_build? * u32::try_from(remaining).ok()?)
}

/// Download throughput over a sliding window, plus a per-second history for the sparkline.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Throughput {
    samples: VecDeque<(Instant, u64)>,
    pub history: VecDeque<f64>,
    last_point: Option<Instant>,
}

impl Throughput {
    const WINDOW: Duration = Duration::from_secs(3);
    const POINT_EVERY: Duration = Duration::from_secs(1);
    pub const POINTS: usize = 40;

    /// Records the byte counter at `now` and returns bytes per second over the window.
    pub fn sample(&mut self, now: Instant, bytes: u64) -> f64 {
        self.samples.push_back((now, bytes));
        while self.samples.len() > 2
            && now.saturating_duration_since(self.samples[1].0) >= Self::WINDOW
        {
            self.samples.pop_front();
        }
        let rate = self.rate();
        if self
            .last_point
            .is_none_or(|at| now.saturating_duration_since(at) >= Self::POINT_EVERY)
        {
            self.history.push_back(rate);
            if self.history.len() > Self::POINTS {
                self.history.pop_front();
            }
            self.last_point = Some(now);
        }
        rate
    }

    /// Bytes per second over the window, as last sampled.
    pub fn rate(&self) -> f64 {
        match (self.samples.front(), self.samples.back()) {
            (Some(&(first_at, first)), Some(&(last_at, last))) if last_at > first_at => {
                last.saturating_sub(first) as f64 / (last_at - first_at).as_secs_f64()
            }
            _ => 0.0,
        }
    }
}

/// Bytes moved so far and expected in total, from file transfers or, failing that, store copies.
pub fn download_bytes(progress: &NixBuildProgress) -> (u64, u64) {
    let pick = |metric: &NixProgressMetric| (metric.done, metric.expected.max(metric.done));
    if progress.downloads.expected > 0 || progress.copy_bytes.expected == 0 {
        pick(&progress.downloads)
    } else {
        pick(&progress.copy_bytes)
    }
}

#[cfg(test)]
#[path = "work_test.rs"]
mod tests;
