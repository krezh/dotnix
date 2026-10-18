//! Build work rows: what nix plans, builds, and downloads, and the throughput and time left.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use crate::build::ui::activity::{GroupSpec, RowContent, RowLook};
use crate::nix::{NixBuildProgress, NixProgressMetric};

use super::format::{activity_name, short_duration, transfer_text};

/// Finished builds needed before their mean is trusted for a time estimate.
const ESTIMATE_AFTER: usize = 3;

pub(crate) const WORK_GROUPS: &[GroupSpec] = &[
    GroupSpec {
        name: "Failed",
        tone: crate::ui::theme::RED,
        newest_first: false,
        optional: true,
    },
    GroupSpec {
        name: "Building",
        tone: crate::ui::theme::YELLOW,
        newest_first: false,
        optional: false,
    },
    GroupSpec {
        name: "Downloading",
        tone: crate::ui::theme::SAPPHIRE,
        newest_first: false,
        optional: false,
    },
    GroupSpec {
        name: "Queued",
        tone: crate::ui::theme::OVERLAY1,
        newest_first: false,
        optional: false,
    },
    GroupSpec {
        name: "Done",
        tone: crate::ui::theme::GREEN,
        newest_first: true,
        optional: false,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkState {
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
pub(crate) struct WorkItem {
    pub(crate) state: WorkState,
    pub(crate) detail: String,
    pub(crate) started: Option<Instant>,
    pub(crate) took: Option<Duration>,
    pub(crate) bytes: (u64, u64),
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

    pub(crate) fn content(&self, path: &str, now: Instant) -> RowContent {
        let running = || {
            self.started.map_or_else(
                || "—".to_owned(),
                |started| short_duration(now.saturating_duration_since(started), false),
            )
        };
        let (group, look, value) = match self.state {
            WorkState::Failed => (0, RowLook::Failed, "failed".to_owned()),
            WorkState::Building => (1, RowLook::Active, running()),
            WorkState::Downloading => (
                2,
                RowLook::Transfer,
                transfer_text(self.bytes.0, self.bytes.1),
            ),
            WorkState::Queued => (3, RowLook::Idle, String::new()),
            WorkState::Done => (
                4,
                RowLook::Done,
                self.took
                    .map(|took| short_duration(took, false))
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
pub(crate) struct WorkSummary {
    pub(crate) total: usize,
    pub(crate) done: usize,
    pub(crate) failed: usize,
    pub(crate) in_flight: usize,
    /// Mean duration of finished builds, once enough have finished to trust it.
    pub(crate) mean_build: Option<Duration>,
}

/// Every planned or seen piece of build work, kept across progress snapshots so work that stops
/// without a completion record still lands in Done. Items are keyed by store path, so a
/// derivation nix reports through several activities still gets one row.
#[derive(Default)]
pub(crate) struct WorkPlan {
    items: HashMap<String, WorkItem>,
    /// How many items share each display name; shared names get a store-hash tag.
    names: HashMap<String, usize>,
}

impl WorkPlan {
    /// Folds a snapshot in and returns the paths whose item changed, new ones sorted by name.
    pub(crate) fn update(&mut self, progress: &NixBuildProgress, now: Instant) -> Vec<String> {
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
            let name = activity_name(path);
            let count = self.names.entry(name.clone()).or_default();
            *count += 1;
            // The first holder of a name now needs its tag too.
            if *count == 2 {
                changed.extend(
                    self.items
                        .keys()
                        .filter(|other| *other != path && activity_name(other) == name)
                        .cloned(),
                );
            }
        }
        changed.extend(added);
        changed
    }

    #[cfg(test)]
    pub(crate) fn get(&self, path: &str) -> Option<&WorkItem> {
        self.items.get(path)
    }

    /// The row for `path`, tagged with its store hash when another item shares its name.
    pub(crate) fn content(&self, path: &str, now: Instant) -> Option<RowContent> {
        let mut content = self.items.get(path)?.content(path, now);
        if self
            .names
            .get(&content.title)
            .is_some_and(|count| *count > 1)
        {
            content.tag = store_hash(path)
                .map(|hash| format!("· {hash}"))
                .unwrap_or_default();
        }
        Some(content)
    }

    /// Work still in flight when nix stopped never finished: it goes back to Queued. Returns
    /// the paths that moved.
    pub(crate) fn interrupt(&mut self) -> Vec<String> {
        let mut moved = Vec::new();
        for (path, item) in &mut self.items {
            if matches!(item.state, WorkState::Building | WorkState::Downloading) {
                item.state = WorkState::Queued;
                moved.push(path.clone());
            }
        }
        moved
    }

    /// Paths of builds in flight, the only rows whose text changes with time alone.
    pub(crate) fn running(&self) -> impl Iterator<Item = &str> {
        self.items
            .iter()
            .filter(|(_, item)| item.state == WorkState::Building)
            .map(|(path, _)| path.as_str())
    }

    pub(crate) fn summary(&self) -> WorkSummary {
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
pub(crate) fn remaining_estimate(summary: &WorkSummary) -> Option<Duration> {
    let remaining = summary.total.saturating_sub(summary.done + summary.failed);
    Some(summary.mean_build? * u32::try_from(remaining).ok()?)
}

/// Download throughput over a sliding window, plus a per-second history for the sparkline.
#[derive(Default)]
pub(crate) struct Throughput {
    samples: VecDeque<(Instant, u64)>,
    pub(super) history: VecDeque<f64>,
    last_point: Option<Instant>,
}

impl Throughput {
    const WINDOW: Duration = Duration::from_secs(3);
    const POINT_EVERY: Duration = Duration::from_secs(1);
    pub(super) const POINTS: usize = 40;

    /// Records the byte counter at `now` and returns bytes per second over the window.
    pub(crate) fn sample(&mut self, now: Instant, bytes: u64) -> f64 {
        self.samples.push_back((now, bytes));
        while self.samples.len() > 2
            && now.saturating_duration_since(self.samples[1].0) >= Self::WINDOW
        {
            self.samples.pop_front();
        }
        let rate = match (self.samples.front(), self.samples.back()) {
            (Some(&(first_at, first)), Some(&(last_at, last))) if last_at > first_at => {
                last.saturating_sub(first) as f64 / (last_at - first_at).as_secs_f64()
            }
            _ => 0.0,
        };
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
}

/// Bytes moved so far and expected in total, from file transfers or, failing that, store copies.
pub(crate) fn download_bytes(progress: &NixBuildProgress) -> (u64, u64) {
    let pick = |metric: &NixProgressMetric| (metric.done, metric.expected.max(metric.done));
    if progress.downloads.expected > 0 || progress.copy_bytes.expected == 0 {
        pick(&progress.downloads)
    } else {
        pick(&progress.copy_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nix::NixActivityProgress;

    #[test]
    fn work_plan_tracks_states_and_finishes_vanished_work() {
        let started = Instant::now();
        let mut progress = NixBuildProgress::default();
        progress.planned.insert("/nix/store/h-bravo.drv".to_owned());
        progress.planned.insert("/nix/store/h-alpha.drv".to_owned());
        progress.items.push(NixActivityProgress {
            kind: 105,
            path: Some("/nix/store/h-bravo.drv".to_owned()),
            started: Some(started),
            ..NixActivityProgress::default()
        });
        let mut plan = WorkPlan::default();
        let changed = plan.update(&progress, started);
        assert_eq!(
            changed,
            ["/nix/store/h-alpha.drv", "/nix/store/h-bravo.drv"]
        );
        assert_eq!(
            plan.get("/nix/store/h-bravo.drv").unwrap().state,
            WorkState::Building
        );
        assert!(plan.update(&progress, started).is_empty());

        progress.items.clear();
        let later = started + Duration::from_secs(5);
        assert_eq!(plan.update(&progress, later), ["/nix/store/h-bravo.drv"]);
        let bravo = plan.get("/nix/store/h-bravo.drv").unwrap();
        assert_eq!(bravo.state, WorkState::Done);
        assert_eq!(bravo.took, Some(Duration::from_secs(5)));
        let summary = plan.summary();
        assert_eq!((summary.total, summary.done, summary.in_flight), (2, 1, 0));
        assert_eq!(summary.mean_build, None);
    }

    fn building(path: &str, detail: &str, started: Instant) -> NixActivityProgress {
        NixActivityProgress {
            kind: 105,
            path: Some(path.to_owned()),
            detail: Some(detail.to_owned()),
            started: Some(started),
            ..NixActivityProgress::default()
        }
    }

    #[test]
    fn work_plan_keeps_one_row_per_derivation_with_its_latest_line() {
        let started = Instant::now();
        let path = "/nix/store/3k9f2a1abc-molt-0.1.0.drv";
        let mut progress = NixBuildProgress::default();
        // Newest activity first, as snapshots list them.
        progress
            .items
            .push(building(path, "Compiling gtk4 v0.11.5", started));
        progress
            .items
            .push(building(path, "Compiling glib v0.22.0", started));
        let mut plan = WorkPlan::default();
        assert_eq!(plan.update(&progress, started), [path]);
        let content = plan.content(path, started).unwrap();
        assert_eq!(content.detail, "Compiling gtk4 v0.11.5");
        assert_eq!(content.tag, "");
        assert_eq!(plan.summary().total, 1);

        // The failure record keeps the row's last line and moves it to the Failed group.
        progress.items.clear();
        progress.failed.insert(path.to_owned());
        assert_eq!(plan.update(&progress, started), [path]);
        let content = plan.content(path, started).unwrap();
        assert_eq!(content.group, 0);
        assert_eq!(content.look, RowLook::Failed);
        assert_eq!(content.detail, "Compiling gtk4 v0.11.5");

        // Work nix was still doing when it stopped returns to Queued.
        let other = "/nix/store/bbbbbbbbbb-glib-2.84.drv";
        progress
            .items
            .push(building(other, "Compiling glib", started));
        plan.update(&progress, started);
        assert_eq!(plan.interrupt(), [other]);
        assert_eq!(plan.get(other).unwrap().state, WorkState::Queued);
        assert_eq!(plan.get(path).unwrap().state, WorkState::Failed);
    }

    #[test]
    fn work_plan_tags_distinct_derivations_sharing_a_name() {
        let started = Instant::now();
        let first = "/nix/store/3k9f2a1abc-molt-0.1.0.drv";
        let second = "/nix/store/zz11yy2def-molt-0.1.0.drv";
        let other = "/nix/store/aaaaaaaaaa-glib-2.84.drv";
        let mut progress = NixBuildProgress::default();
        progress.planned.insert(first.to_owned());
        progress.planned.insert(other.to_owned());
        let mut plan = WorkPlan::default();
        plan.update(&progress, started);
        assert_eq!(plan.content(first, started).unwrap().tag, "");

        progress
            .items
            .push(building(second, "Compiling molt", started));
        let mut changed = plan.update(&progress, started);
        changed.sort();
        assert_eq!(changed, [first, second]);
        assert_eq!(plan.content(first, started).unwrap().tag, "· 3k9f2a1");
        assert_eq!(plan.content(second, started).unwrap().tag, "· zz11yy2");
        assert_eq!(plan.content(other, started).unwrap().tag, "");
    }

    #[test]
    fn estimate_needs_three_finished_builds() {
        let mut progress = NixBuildProgress::default();
        for (name, seconds) in [("a", 10), ("b", 20)] {
            progress.completed.insert(
                format!("/nix/store/h-{name}.drv"),
                Duration::from_secs(seconds),
            );
        }
        progress.planned.insert("/nix/store/h-d.drv".to_owned());
        progress.planned.insert("/nix/store/h-e.drv".to_owned());
        let mut plan = WorkPlan::default();
        plan.update(&progress, Instant::now());
        assert_eq!(remaining_estimate(&plan.summary()), None);
        progress
            .completed
            .insert("/nix/store/h-c.drv".to_owned(), Duration::from_secs(30));
        plan.update(&progress, Instant::now());
        assert_eq!(
            remaining_estimate(&plan.summary()),
            Some(Duration::from_secs(40))
        );
    }

    #[test]
    fn throughput_uses_a_sliding_window() {
        let start = Instant::now();
        let mut throughput = Throughput::default();
        assert!(throughput.sample(start, 0).abs() < 1e-6);
        let rate = throughput.sample(start + Duration::from_secs(1), 1024);
        assert!((rate - 1024.0).abs() < 1e-6);
        throughput.sample(start + Duration::from_secs(5), 1024);
        let rate = throughput.sample(start + Duration::from_secs(6), 4096);
        // The window reaches back to the last sample before it (at one second), so the rate
        // spans five seconds.
        assert!((rate - 3072.0 / 5.0).abs() < 1e-6);
        assert_eq!(throughput.history.len(), 4);
    }
}
