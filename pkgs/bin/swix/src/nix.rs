use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::process::{Command, Output};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use serde_json::Value;

use swix::command::{self, OutputLimits};
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct NixBuildProgress {
    pub(crate) flake_fetches: Vec<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) builds: NixProgressMetric,
    pub(crate) downloads: NixProgressMetric,
    pub(crate) copy_paths: NixProgressMetric,
    pub(crate) copy_bytes: NixProgressMetric,
    pub(crate) planned: HashSet<String>,
    pub(crate) completed: HashSet<String>,
    pub(crate) failed: HashSet<String>,
    pub(crate) items: Vec<NixActivityProgress>,
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
}

#[derive(Clone, Debug)]
struct NixActivity {
    kind: u64,
    parent: u64,
    path: Option<String>,
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
}

impl NixProgressTracker {
    pub(crate) fn update(&mut self, line: &str) -> Option<()> {
        let json = line.strip_prefix("@nix ")?;
        let event: Value = serde_json::from_str(json).ok()?;
        let action = event.get("action")?.as_str()?;
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
                let path = event
                    .get("fields")
                    .and_then(Value::as_array)
                    .and_then(|fields| fields.first())
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                self.activities.insert(
                    id,
                    NixActivity {
                        kind: activity_type,
                        parent,
                        path: path.clone(),
                        metric: NixProgressMetric::default(),
                    },
                );
                let work_path = if activity_type == 101 {
                    self.ancestor_path(parent, 108)
                } else {
                    path
                };
                if activity_type == 105 || activity_type == 101 && work_path.is_some() {
                    self.work_order.push(id);
                    self.work_items.insert(
                        id,
                        NixActivityProgress {
                            kind: activity_type,
                            path: work_path,
                            ..NixActivityProgress::default()
                        },
                    );
                }
            }
            "stop" => {
                let activity = self.activities.remove(&id)?;
                if activity.kind != 0 {
                    let completed = self.completed.entry(activity.kind).or_default();
                    completed.done += activity.metric.done;
                    completed.failed += activity.metric.failed;
                    if let Some(item) = self.work_items.remove(&id) {
                        self.work_order.retain(|item_id| *item_id != id);
                        if item.kind == 105
                            && let Some(path) = item.path
                        {
                            if activity.metric.failed > 0 {
                                self.progress.failed.insert(path);
                            } else {
                                self.progress.completed.insert(path);
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
                if event.get("level").and_then(Value::as_u64) == Some(1) {
                    let warning = event
                        .get("raw_msg")
                        .or_else(|| event.get("msg"))
                        .and_then(Value::as_str)
                        .map(strip_ansi)
                        .unwrap_or_default();
                    let warning = warning.trim();
                    if !warning.is_empty()
                        && !self
                            .progress
                            .warnings
                            .iter()
                            .any(|existing| existing == warning)
                    {
                        self.progress.warnings.push(warning.to_owned());
                        changed = true;
                    }
                }
                for line in message.lines() {
                    let line = strip_ansi(line);
                    let path = line.trim();
                    if let Some(input) = flake_fetch_input(path)
                        && !self.progress.flake_fetches.contains(&input)
                    {
                        self.progress.flake_fetches.push(input);
                        changed = true;
                    }
                    if path.starts_with("/nix/store/") && path.ends_with(".drv") {
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

    fn ancestor_path(&self, mut id: u64, kind: u64) -> Option<String> {
        while id != 0 {
            let activity = self.activities.get(&id)?;
            if activity.kind == kind {
                return activity.path.clone();
            }
            id = activity.parent;
        }
        None
    }
}

fn flake_fetch_input(message: &str) -> Option<String> {
    message
        .strip_prefix("fetching ")?
        .split_once(" input '")?
        .1
        .strip_suffix('\'')
        .map(str::to_owned)
}
pub(crate) fn run_nix_command(
    command: &mut Command,
    name: &str,
    cancellation: &AtomicBool,
    timeout: Duration,
    progress: impl Fn(NixBuildProgress),
) -> Result<Output, String> {
    let mut tracker = NixProgressTracker::default();
    let mut dirty = false;
    let mut last_progress = Instant::now() - Duration::from_millis(100);
    let best_error = RefCell::new(None::<(u8, String)>);
    let output = command::run(
        command,
        name,
        cancellation,
        timeout,
        OutputLimits {
            stdout: 64 * 1024,
            stderr: 4 * 1024 * 1024,
        },
        |line| {
            update_best_error(&best_error, line);
            if tracker.update(line).is_some() {
                dirty = true;
                if last_progress.elapsed() >= Duration::from_millis(100) {
                    progress(tracker.snapshot());
                    dirty = false;
                    last_progress = Instant::now();
                }
            }
        },
        |stderr| {
            best_error
                .borrow()
                .as_ref()
                .map(|(_, message)| message.clone())
                .or_else(|| nix_error_message(stderr))
                .unwrap_or_else(|| String::from_utf8_lossy(stderr).trim().to_owned())
        },
    );
    if dirty {
        progress(tracker.snapshot());
    }
    output
}

pub(crate) fn nix_error_message(stderr: &[u8]) -> Option<String> {
    let best = RefCell::new(None);
    for line in String::from_utf8_lossy(stderr).lines() {
        update_best_error(&best, line);
    }
    best.into_inner().map(|(_, message)| message)
}

fn update_best_error(best: &RefCell<Option<(u8, String)>>, line: &str) {
    let Some(event) = line
        .strip_prefix("@nix ")
        .and_then(|json| serde_json::from_str::<Value>(json).ok())
    else {
        return;
    };
    if event.get("action").and_then(Value::as_str) != Some("msg")
        || event
            .get("level")
            .and_then(Value::as_u64)
            .is_some_and(|level| level != 0)
    {
        return;
    }
    let Some(message) = event
        .get("raw_msg")
        .or_else(|| event.get("msg"))
        .and_then(Value::as_str)
    else {
        return;
    };
    let message = strip_ansi(message).trim().to_owned();
    if message.is_empty() {
        return;
    }
    let priority = error_priority(&message);
    if best
        .borrow()
        .as_ref()
        .is_none_or(|(best_priority, _)| priority > *best_priority)
    {
        best.replace(Some((priority, message)));
    }
}

fn error_priority(message: &str) -> u8 {
    if is_dependency_failure(message) {
        0
    } else if message.contains("builder for '") && message.contains(" log lines:") {
        4
    } else if message.contains("builder for '") {
        3
    } else if message
        .lines()
        .any(|line| line.trim_start().starts_with("error:"))
    {
        2
    } else {
        1
    }
}

fn is_dependency_failure(message: &str) -> bool {
    let message = message
        .trim()
        .strip_prefix("error:")
        .unwrap_or(message)
        .trim();
    message.contains("dependencies of derivation '") && message.ends_with("failed to build")
}

pub(crate) fn strip_ansi(text: &str) -> String {
    let mut output = String::new();
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' && chars.next_if_eq(&'[').is_some() {
            for code in chars.by_ref() {
                if ('@'..='~').contains(&code) {
                    break;
                }
            }
        } else {
            output.push(character);
        }
    }
    output
}

pub(crate) fn run(
    command: &mut Command,
    name: &str,
    cancellation: &AtomicBool,
    timeout: Duration,
    stdout_limit: usize,
) -> Result<Output, String> {
    command::run(
        command,
        name,
        cancellation,
        timeout,
        OutputLimits {
            stdout: stdout_limit,
            stderr: 1024 * 1024,
        },
        |_| {},
        |stderr| {
            strip_ansi(String::from_utf8_lossy(stderr).trim())
                .trim()
                .to_owned()
        },
    )
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
        assert_eq!(tracker.snapshot().flake_fetches, ["git+file:///dotnix"]);
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
        tracker.update(r#"@nix {"action":"stop","id":3}"#);

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
        assert!(progress.completed.contains("/nix/store/hash-demo-1.0.drv"));
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
    fn extracts_originating_nix_build_error() {
        let stderr = br#"@nix {"action":"msg","level":1,"raw_msg":"warning: deprecated option"}
@nix {"action":"msg","level":0,"raw_msg":"linking '/nix/store/system_fish-completions/uptime.fish' to '/nix/store/.links/content-address' not allowed"}
@nix {"action":"msg","level":0,"raw_msg":"\u001b[31;1merror:\u001b[0m builder for '/nix/store/chomp.drv' failed with exit code 101;\n       last 25 log lines:\n       > error[E0583]: file not found for module `video`\n       > error: could not compile `chomp` due to 1 previous error\n       For full logs, run:\n               nix log /nix/store/chomp.drv"}
@nix {"action":"msg","level":0,"raw_msg":"error: 2 dependencies of derivation '/nix/store/home-manager.drv' failed to build"}
@nix {"action":"msg","level":0,"raw_msg":"error: 1 dependencies of derivation '/nix/store/nixos-system-odin.drv' failed to build"}"#;
        assert_eq!(
            nix_error_message(stderr).as_deref(),
            Some(
                "error: builder for '/nix/store/chomp.drv' failed with exit code 101;\n       last 25 log lines:\n       > error[E0583]: file not found for module `video`\n       > error: could not compile `chomp` due to 1 previous error\n       For full logs, run:\n               nix log /nix/store/chomp.drv"
            )
        );
        assert_eq!(
            strip_ansi("Resolving package\n\u{1b}[1;31m✗\u{1b}[0m failed"),
            "Resolving package\n✗ failed"
        );
    }
}
