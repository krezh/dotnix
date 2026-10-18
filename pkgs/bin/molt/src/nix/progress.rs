//! Nix internal-json progress tracking: build, download and flake fetch state.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use serde_json::Value;

use super::{failed_derivations, strip_ansi};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct NixBuildProgress {
    pub(crate) flake_fetches: Vec<FlakeFetch>,
    pub(crate) warnings: Vec<String>,
    pub(crate) builds: NixProgressMetric,
    pub(crate) downloads: NixProgressMetric,
    /// File transfers still running; zero once every download has finished.
    pub(crate) active_transfers: usize,
    pub(crate) copy_paths: NixProgressMetric,
    pub(crate) copy_bytes: NixProgressMetric,
    pub(crate) planned: HashSet<String>,
    /// Finished builds and downloads by store path, with how long each ran.
    pub(crate) completed: HashMap<String, Duration>,
    pub(crate) failed: HashSet<String>,
    pub(crate) items: Vec<NixActivityProgress>,
}

/// A flake input nix announced it is fetching. Nix logs no matching "fetched" message, so a fetch
/// ends when the first activity started after it stops, or when the next fetch begins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FlakeFetch {
    pub(crate) input: String,
    pub(crate) started: Instant,
    pub(crate) finished: Option<Instant>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct NixProgressMetric {
    pub(crate) done: u64,
    pub(crate) expected: u64,
    pub(crate) running: u64,
    pub(crate) failed: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct NixActivityProgress {
    pub(crate) kind: u64,
    pub(crate) done: u64,
    pub(crate) expected: u64,
    pub(crate) failed: u64,
    pub(crate) detail: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) started: Option<Instant>,
}

#[derive(Clone, Debug)]
struct NixActivity {
    kind: u64,
    parent: u64,
    path: Option<String>,
    /// The binary cache a substitution (type 108) pulls from.
    substituter: Option<String>,
    metric: NixProgressMetric,
}

#[derive(Default)]
pub(crate) struct NixProgressTracker {
    activities: HashMap<u64, NixActivity>,
    completed: HashMap<u64, NixProgressMetric>,
    expected: HashMap<u64, u64>,
    work_items: HashMap<u64, NixActivityProgress>,
    work_order: Vec<u64>,
    progress: NixBuildProgress,
    fetch_activity_started: bool,
    /// Set when the last running file transfer stops; cleared by `take_transfers_finished`.
    transfers_finished: bool,
    /// `progress.warnings` as a set, for deduplication.
    known_warnings: HashSet<String>,
}

impl NixProgressTracker {
    pub(crate) fn update(&mut self, line: &str) -> Option<()> {
        let json = line.strip_prefix("@nix ")?;
        let event: Value = serde_json::from_str(json).ok()?;
        let action = event.get("action")?.as_str()?;
        let fetch_finished = self.track_open_fetch(action);
        self.apply(action, &event).or(fetch_finished.then_some(()))
    }

    /// Closes the open fetch once an activity started after it has stopped.
    fn track_open_fetch(&mut self, action: &str) -> bool {
        let Some(fetch) = self
            .progress
            .flake_fetches
            .last_mut()
            .filter(|fetch| fetch.finished.is_none())
        else {
            return false;
        };
        match action {
            "start" => {
                self.fetch_activity_started = true;
                false
            }
            "stop" if self.fetch_activity_started => {
                fetch.finished = Some(Instant::now());
                true
            }
            _ => false,
        }
    }

    fn apply(&mut self, action: &str, event: &Value) -> Option<()> {
        let id = event.get("id").and_then(Value::as_u64).unwrap_or_default();
        let activity_type = event
            .get("type")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let parent = event
            .get("parent")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        match action {
            "start"
                if matches!(activity_type, 100..=112)
                    || activity_type == 0
                        && event
                            .get("level")
                            .and_then(Value::as_u64)
                            .unwrap_or_default()
                            <= 4 =>
            {
                let fields = event.get("fields").and_then(Value::as_array);
                let field = |index: usize| {
                    fields
                        .and_then(|fields| fields.get(index))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                };
                let path = field(0);
                self.activities.insert(
                    id,
                    NixActivity {
                        kind: activity_type,
                        parent,
                        path: path.clone(),
                        substituter: if activity_type == 108 { field(1) } else { None },
                        metric: NixProgressMetric::default(),
                    },
                );
                let (work_path, detail) = if activity_type == 101 {
                    self.ancestor(parent, 108)
                        .map_or((None, None), |substitution| {
                            (
                                substitution.path.clone(),
                                substitution.substituter.as_deref().map(cache_label),
                            )
                        })
                } else {
                    (path, None)
                };
                if activity_type == 105 || activity_type == 101 && work_path.is_some() {
                    self.work_order.push(id);
                    self.work_items.insert(
                        id,
                        NixActivityProgress {
                            kind: activity_type,
                            path: work_path,
                            detail,
                            started: Some(Instant::now()),
                            ..NixActivityProgress::default()
                        },
                    );
                }
            }
            "stop" => {
                let activity = self.activities.remove(&id)?;
                self.transfers_finished |= activity.kind == 101
                    && !self.activities.values().any(|other| other.kind == 101);
                if activity.kind != 0 {
                    let completed = self.completed.entry(activity.kind).or_default();
                    completed.done += activity.metric.done;
                    completed.failed += activity.metric.failed;
                    if let Some(item) = self.work_items.remove(&id) {
                        self.work_order.retain(|item_id| *item_id != id);
                        if matches!(item.kind, 101 | 105)
                            && let Some(path) = item.path
                        {
                            if activity.metric.failed > 0 {
                                self.progress.failed.insert(path);
                            } else if !self.progress.failed.contains(&path) {
                                let ran = item.started.map_or(Duration::ZERO, |at| at.elapsed());
                                self.progress.completed.insert(path, ran);
                            }
                        }
                    }
                }
            }
            "result" if activity_type == 105 => {
                let fields = event.get("fields")?.as_array()?;
                let activity = self.activities.get_mut(&id)?;
                activity.metric.done = fields.first()?.as_u64()?;
                activity.metric.expected = fields.get(1)?.as_u64()?;
                activity.metric.running = fields.get(2)?.as_u64()?;
                activity.metric.failed = fields.get(3)?.as_u64()?;
                if let Some(item) = self.work_items.get_mut(&id) {
                    item.done = activity.metric.done;
                    item.expected = activity.metric.expected;
                    item.failed = activity.metric.failed;
                }
            }
            "result" if activity_type == 106 => {
                let fields = event.get("fields")?.as_array()?;
                self.expected
                    .insert(fields.first()?.as_u64()?, fields.get(1)?.as_u64()?);
            }
            "result" if activity_type == 101 => {
                if self.activities.contains_key(&id) {
                    let line = event
                        .get("fields")
                        .and_then(Value::as_array)
                        .and_then(|fields| fields.first())
                        .and_then(Value::as_str)
                        .map(strip_ansi);
                    if let Some(item) = self.work_items.get_mut(&id) {
                        item.detail = line;
                    }
                }
            }
            "result" if activity_type == 104 => {
                let phase = event
                    .get("fields")
                    .and_then(Value::as_array)
                    .and_then(|fields| fields.first())
                    .and_then(Value::as_str)
                    .map(strip_ansi)?;
                if let Some(item) = self.work_items.get_mut(&id) {
                    item.detail = Some(phase);
                }
            }
            "msg" => {
                let message = event
                    .get("msg")
                    .or_else(|| event.get("raw_msg"))
                    .and_then(Value::as_str)?;
                let mut changed = false;
                let level = event.get("level").and_then(Value::as_u64);
                if level == Some(0) {
                    for path in failed_derivations(&strip_ansi(message)) {
                        self.progress.completed.remove(&path);
                        changed |= self.progress.failed.insert(path);
                    }
                }
                if level == Some(1) {
                    let warning = event
                        .get("raw_msg")
                        .or_else(|| event.get("msg"))
                        .and_then(Value::as_str)
                        .map(strip_ansi)
                        .unwrap_or_default();
                    let warning = warning.trim();
                    if !warning.is_empty() && self.known_warnings.insert(warning.to_owned()) {
                        self.progress.warnings.push(warning.to_owned());
                        changed = true;
                    }
                }
                let fetched_paths = message.contains("will be fetched");
                for line in message.lines() {
                    let line = strip_ansi(line);
                    let path = line.trim();
                    if let Some(input) = flake_fetch_input(path)
                        && !self
                            .progress
                            .flake_fetches
                            .iter()
                            .any(|fetch| fetch.input == input)
                    {
                        let now = Instant::now();
                        if let Some(open) = self.progress.flake_fetches.last_mut()
                            && open.finished.is_none()
                        {
                            open.finished = Some(now);
                        }
                        self.fetch_activity_started = false;
                        self.progress.flake_fetches.push(FlakeFetch {
                            input,
                            started: now,
                            finished: None,
                        });
                        changed = true;
                    }
                    if path.starts_with("/nix/store/")
                        && !path.contains(char::is_whitespace)
                        && (fetched_paths || path.ends_with(".drv"))
                    {
                        changed |= self.progress.planned.insert(path.to_owned());
                    }
                }
                if !changed {
                    return None;
                }
            }
            _ => return None,
        }
        Some(())
    }

    pub(crate) fn snapshot(&mut self) -> NixBuildProgress {
        self.progress.builds = self.metric(104);
        self.progress.downloads = self.metric(101);
        self.progress.active_transfers = self
            .activities
            .values()
            .filter(|activity| activity.kind == 101)
            .count();
        self.progress.copy_paths = self.metric(103);
        self.progress.copy_bytes = self.metric(100);
        self.progress.items = self
            .work_order
            .iter()
            .rev()
            .filter_map(|id| self.work_items.get(id).cloned())
            .collect();
        self.progress.clone()
    }

    /// Whether the last running file transfer stopped since the last call.
    pub(crate) fn take_transfers_finished(&mut self) -> bool {
        std::mem::take(&mut self.transfers_finished)
    }

    fn metric(&self, kind: u64) -> NixProgressMetric {
        let completed = self.completed.get(&kind).cloned().unwrap_or_default();
        let mut metric = NixProgressMetric {
            done: completed.done,
            expected: completed.done,
            failed: completed.failed,
            ..NixProgressMetric::default()
        };
        for activity in self
            .activities
            .values()
            .filter(|activity| activity.kind == kind)
        {
            metric.done += activity.metric.done;
            metric.expected += activity.metric.expected;
            metric.running += activity.metric.running;
            metric.failed += activity.metric.failed;
        }
        metric.expected = metric
            .expected
            .max(self.expected.get(&kind).copied().unwrap_or_default());
        metric
    }

    fn ancestor(&self, mut id: u64, kind: u64) -> Option<&NixActivity> {
        while id != 0 {
            let activity = self.activities.get(&id)?;
            if activity.kind == kind {
                return Some(activity);
            }
            id = activity.parent;
        }
        None
    }
}

/// `https://cache.nixos.org/` → `from cache.nixos.org`: scheme and trailing slash dropped.
fn cache_label(uri: &str) -> String {
    let host = uri
        .split_once("://")
        .map_or(uri, |(_, rest)| rest)
        .trim_end_matches('/');
    format!("from {host}")
}

fn flake_fetch_input(message: &str) -> Option<String> {
    message
        .strip_prefix("fetching ")?
        .split_once(" input '")?
        .1
        .strip_suffix('\'')
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tracks_nix_internal_json_progress() {
        let mut tracker = NixProgressTracker::default();
        tracker
            .update(
                r#"@nix {"action":"msg","level":3,"msg":"fetching \u001b[35;1mgit\u001b[0m input '\u001b[35;1mgit+file:///dotnix\u001b[0m'"}"#,
            )
            .unwrap();
        let fetches = tracker.snapshot().flake_fetches;
        assert_eq!(fetches.len(), 1);
        assert_eq!(fetches[0].input, "git+file:///dotnix");
        assert!(fetches[0].finished.is_none());
        assert!(tracker
            .update(
                r#"@nix {"action":"msg","level":3,"msg":"fetching git input 'git+file:///dotnix'"}"#,
            )
            .is_none());
        tracker
            .update(
                r#"@nix {"action":"start","id":0,"level":4,"text":"evaluating derivation 'git+file:///dotnix#nixosConfigurations.thor.config.system.build.toplevel'","type":0}"#,
            )
            .unwrap();
        assert!(tracker.snapshot().items.is_empty());
        // The first activity to stop after the fetch began closes it, even an untracked one.
        tracker.update(r#"@nix {"action":"stop","id":99}"#).unwrap();
        assert!(tracker.snapshot().flake_fetches[0].finished.is_some());
        assert!(
            tracker
                .update(r#"@nix {"action":"stop","id":99}"#)
                .is_none()
        );
        assert!(tracker
            .update(
                r#"@nix {"action":"start","id":99,"level":5,"text":"copying '/nix/store/hash-source/file' to the store","type":0}"#,
            )
            .is_none());
        tracker.update(r#"@nix {"action":"stop","id":0}"#);
        tracker
            .update(
                r#"@nix {"action":"msg","level":3,"msg":"these derivations will be built:\n  /nix/store/hash-demo-1.0.drv"}"#,
            )
            .unwrap();
        assert!(
            tracker
                .snapshot()
                .planned
                .contains("/nix/store/hash-demo-1.0.drv")
        );
        tracker
            .update(
                r#"@nix {"action":"msg","level":3,"msg":"these 1 paths will be fetched (0.5 MiB download):\n  /nix/store/hash-demo"}"#,
            )
            .unwrap();
        assert!(tracker.snapshot().planned.contains("/nix/store/hash-demo"));
        tracker.update(r#"@nix {"action":"start","id":1,"text":"","type":104}"#);
        tracker
            .update(r#"@nix {"action":"result","id":1,"type":105,"fields":[2,5,0,0]}"#)
            .unwrap();
        let progress = tracker.snapshot();
        assert_eq!((progress.builds.done, progress.builds.expected), (2, 5));

        for event in [
            r#"@nix {"action":"start","id":4,"fields":["/nix/store/hash-demo","https://cache.example"],"text":"fetching demo","type":108}"#,
            r#"@nix {"action":"start","id":5,"parent":4,"fields":["/nix/store/hash-demo"],"text":"copying demo","type":100}"#,
            r#"@nix {"action":"start","id":3,"parent":5,"text":"downloading https://cache.example/demo.nar","type":101}"#,
            r#"@nix {"action":"result","id":3,"type":105,"fields":[512,1024,0,0]}"#,
        ] {
            tracker.update(event).unwrap();
        }
        let progress = tracker.snapshot();
        assert_eq!(
            (progress.downloads.done, progress.downloads.expected),
            (512, 1024)
        );
        assert_eq!(
            (progress.items[0].done, progress.items[0].expected),
            (512, 1024)
        );
        assert_eq!(
            progress.items[0].path.as_deref(),
            Some("/nix/store/hash-demo")
        );
        assert_eq!(
            progress.items[0].detail.as_deref(),
            Some("from cache.example")
        );
        tracker.update(r#"@nix {"action":"stop","id":3}"#);
        assert!(
            tracker
                .snapshot()
                .completed
                .contains_key("/nix/store/hash-demo")
        );

        tracker
            .update(
                r#"@nix {"action":"start","id":2,"fields":["/nix/store/hash-demo-1.0.drv"],"text":"building '/nix/store/hash-demo-1.0.drv'","type":105}"#,
            )
            .unwrap();
        tracker
            .update(
                r#"@nix {"action":"result","id":2,"type":101,"fields":["\u001b[1m\u001b[92mChecking\u001b[0m \u001b[1mnum-complex\u001b[0m v0.4.6"]}"#,
            )
            .unwrap();
        assert_eq!(
            tracker.snapshot().items[0].detail.as_deref(),
            Some("Checking num-complex v0.4.6")
        );
        tracker.update(r#"@nix {"action":"stop","id":2}"#).unwrap();
        let progress = tracker.snapshot();
        assert!(progress.items.is_empty());
        assert!(
            progress
                .completed
                .contains_key("/nix/store/hash-demo-1.0.drv")
        );
    }

    #[test]
    fn tracks_unique_nix_warnings() {
        let mut tracker = NixProgressTracker::default();
        let warning = r#"@nix {"action":"msg","level":1,"raw_msg":"warning: 'system' has been renamed to/replaced by 'stdenv.hostPlatform.system'"}"#;
        tracker.update(warning).unwrap();
        assert!(tracker.update(warning).is_none());
        tracker
            .update(
                r#"@nix {"action":"msg","level":1,"raw_msg":"warning: The option `nix.nixPath' has been renamed to `nix.settings.nix-path'."}"#,
            )
            .unwrap();
        assert_eq!(
            tracker.snapshot().warnings,
            [
                "warning: 'system' has been renamed to/replaced by 'stdenv.hostPlatform.system'",
                "warning: The option `nix.nixPath' has been renamed to `nix.settings.nix-path'.",
            ]
        );
    }

    #[test]
    fn failed_builds_never_count_as_completed() {
        let mut tracker = NixProgressTracker::default();
        for event in [
            r#"@nix {"action":"start","id":2,"fields":["/nix/store/f-demo.drv"],"text":"building","type":105}"#,
            r#"@nix {"action":"msg","level":0,"msg":"\u001b[31;1merror:\u001b[0m builder for '\u001b[35;1m/nix/store/f-demo.drv\u001b[0m' failed with exit code 1"}"#,
        ] {
            tracker.update(event).unwrap();
        }
        tracker.update(r#"@nix {"action":"stop","id":2}"#).unwrap();
        let progress = tracker.snapshot();
        assert!(progress.failed.contains("/nix/store/f-demo.drv"));
        assert!(progress.completed.is_empty());
    }
}
