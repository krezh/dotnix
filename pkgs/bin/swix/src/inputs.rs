use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;
use swix::command::{self, OutputLimits};

const NIX: &str = match option_env!("SWIX_NIX") {
    Some(path) => path,
    None => "nix",
};
const JJ: &str = match option_env!("SWIX_JJ") {
    Some(path) => path,
    None => "jj",
};
const GIT: &str = match option_env!("SWIX_GIT") {
    Some(path) => path,
    None => "git",
};
const UPDATE_TIMEOUT: Duration = Duration::from_secs(20 * 60);
const HISTORY_TIMEOUT: Duration = Duration::from_secs(2 * 60);
const MAX_OUTPUT: usize = 4 * 1024 * 1024;
const HISTORY_LIMIT: usize = 48;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FlakeInput {
    pub(crate) name: String,
    pub(crate) source: String,
    pub(crate) source_kind: String,
    pub(crate) revision: String,
    pub(crate) last_modified: Option<i64>,
    pub(crate) follows: Option<String>,
    pub(crate) flake_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RevisionRecord {
    pub(crate) revision: String,
    pub(crate) flake_ref: String,
    pub(crate) date: String,
    pub(crate) summary: String,
    pub(crate) commit: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LockFile {
    nodes: HashMap<String, LockNode>,
    root: String,
}

#[derive(Debug, Deserialize)]
struct LockNode {
    #[serde(default)]
    inputs: HashMap<String, Value>,
    #[serde(default)]
    locked: Option<Value>,
    #[serde(default)]
    original: Option<Value>,
}

#[derive(Clone, Debug)]
struct HistoryMetadata {
    commit: String,
    date: String,
    summary: String,
}

#[derive(Clone, Copy)]
enum HistoryKind {
    Jj,
    Git,
}

pub(crate) fn load(directory: &Path) -> Result<(String, Vec<FlakeInput>), String> {
    let path = lock_path(directory);
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let inputs = parse_inputs(&content, &path)?;
    Ok((content, inputs))
}

pub(crate) fn history(
    directory: &Path,
    input_name: &str,
    cancellation: &AtomicBool,
) -> Result<Vec<RevisionRecord>, String> {
    let (_, current_inputs) = load(directory)?;
    let current = current_inputs
        .into_iter()
        .find(|input| input.name == input_name)
        .ok_or_else(|| format!("input {input_name:?} is no longer in flake.lock"))?;
    let current_ref = current
        .flake_ref
        .clone()
        .ok_or_else(|| format!("{} does not expose a selectable revision", current.name))?;
    let (root, path, metadata, kind) = history_metadata(directory, cancellation)?;
    let mut records: Vec<RevisionRecord> = Vec::new();
    for metadata in metadata.into_iter().take(HISTORY_LIMIT) {
        let snapshot = historical_lock(&root, &path, &metadata.commit, kind, cancellation)?;
        let inputs = parse_inputs(&snapshot, &path)?;
        let Some(input) = inputs.into_iter().find(|input| input.name == input_name) else {
            continue;
        };
        let Some(flake_ref) = input.flake_ref else {
            continue;
        };
        if records
            .last()
            .is_some_and(|record| record.revision == input.revision)
        {
            continue;
        }
        records.push(RevisionRecord {
            revision: input.revision,
            flake_ref,
            date: metadata.date,
            summary: metadata.summary,
            commit: Some(metadata.commit),
        });
    }
    if !records
        .iter()
        .any(|record| record.revision == current.revision)
    {
        records.insert(
            0,
            RevisionRecord {
                revision: current.revision,
                flake_ref: current_ref,
                date: current
                    .last_modified
                    .map(|timestamp| timestamp.to_string())
                    .unwrap_or_default(),
                summary: "Current lock file".to_owned(),
                commit: None,
            },
        );
    }
    Ok(records)
}

pub(crate) fn update(
    directory: &Path,
    input: Option<&str>,
    cancellation: &AtomicBool,
) -> Result<(), String> {
    let mut command = Command::new(NIX);
    command.current_dir(directory).args(["flake", "update"]);
    if let Some(input) = input {
        command.arg(input);
    }
    let label = input.map_or("flake inputs update", |_| "flake input update");
    run(&mut command, label, cancellation)?;
    Ok(())
}

pub(crate) fn update_at_revision(
    directory: &Path,
    input: &str,
    flake_ref: &str,
    cancellation: &AtomicBool,
) -> Result<(), String> {
    let lock = lock_path(directory);
    let temporary = lock.with_extension(format!("lock.swix-{}", std::process::id()));
    let mut command = Command::new(NIX);
    command
        .current_dir(directory)
        .args(["flake", "update", input, "--flake"])
        .arg(directory)
        .args(["--override-input", input, flake_ref, "--output-lock-file"])
        .arg(&temporary);
    let result = run(&mut command, "flake input revision change", cancellation);
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    fs::rename(&temporary, &lock).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("failed to replace {}: {error}", lock.display())
    })
}

fn lock_path(directory: &Path) -> PathBuf {
    directory.join("flake.lock")
}

fn parse_inputs(content: &str, path: &Path) -> Result<Vec<FlakeInput>, String> {
    let lock: LockFile = serde_json::from_str(content)
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    let root = lock
        .nodes
        .get(&lock.root)
        .ok_or_else(|| format!("{} has no root node", path.display()))?;
    let mut inputs = root
        .inputs
        .iter()
        .map(|(name, reference)| input_from_reference(name, reference, &lock))
        .collect::<Result<Vec<_>, _>>()?;
    inputs.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(inputs)
}

pub(crate) fn revision_ref(input: &FlakeInput, revision: &str) -> Result<String, String> {
    let revision = revision.trim();
    if revision.is_empty() || revision.chars().any(char::is_whitespace) {
        return Err("Enter a commit, tag, or branch without whitespace".to_owned());
    }
    let current = input
        .flake_ref
        .as_ref()
        .ok_or_else(|| format!("{} does not support revision selection", input.name))?;
    if let Some(prefix) = current.strip_suffix(&format!("/{}", input.revision)) {
        return Ok(format!("{prefix}/{revision}"));
    }
    if let Some((prefix, _)) = current.split_once("?rev=") {
        return Ok(format!("{prefix}?rev={revision}"));
    }
    if let Some((prefix, _)) = current.split_once("&rev=") {
        return Ok(format!("{prefix}&rev={revision}"));
    }
    Err(format!(
        "{} does not support revision selection",
        input.name
    ))
}

fn history_metadata(
    directory: &Path,
    cancellation: &AtomicBool,
) -> Result<(PathBuf, PathBuf, Vec<HistoryMetadata>, HistoryKind), String> {
    let (root, kind) = if let Some(root) = repository_root(directory, ".jj") {
        (root, HistoryKind::Jj)
    } else if let Some(root) = repository_root(directory, ".git") {
        (root, HistoryKind::Git)
    } else {
        return Err("flake directory is not inside a jj or Git repository".to_owned());
    };
    let path = lock_path(directory)
        .strip_prefix(&root)
        .map_err(|_| "flake.lock is outside the repository root".to_owned())?
        .to_owned();
    let mut command = match kind {
        HistoryKind::Jj => {
            let mut command = Command::new(JJ);
            command
                .current_dir(&root)
                .args([
                    "log",
                    "-r",
                    "ancestors(@)",
                    "--no-graph",
                    "--limit",
                    &HISTORY_LIMIT.to_string(),
                    "--template",
                    "commit_id ++ \"\\x1f\" ++ author.timestamp() ++ \"\\x1f\" ++ description.first_line() ++ \"\\n\"",
                    "--",
                ])
                .arg(&path);
            command
        }
        HistoryKind::Git => {
            let mut command = Command::new(GIT);
            command
                .current_dir(&root)
                .args([
                    "log",
                    &format!("--max-count={HISTORY_LIMIT}"),
                    "--format=%H%x1f%aI%x1f%s",
                    "--",
                ])
                .arg(&path);
            command
        }
    };
    let output = run_output(
        &mut command,
        "flake.lock revision history",
        cancellation,
        HISTORY_TIMEOUT,
    )?;
    let text = String::from_utf8(output.stdout)
        .map_err(|_| "revision history returned invalid UTF-8".to_owned())?;
    let metadata = text
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\x1f');
            Some(HistoryMetadata {
                commit: fields.next()?.to_owned(),
                date: fields.next()?.to_owned(),
                summary: fields.next().unwrap_or("flake.lock update").to_owned(),
            })
        })
        .collect();
    Ok((root, path, metadata, kind))
}

fn historical_lock(
    root: &Path,
    path: &Path,
    commit: &str,
    kind: HistoryKind,
    cancellation: &AtomicBool,
) -> Result<String, String> {
    let mut command = match kind {
        HistoryKind::Jj => {
            let mut command = Command::new(JJ);
            command
                .current_dir(root)
                .args(["file", "show", "-r", commit])
                .arg(path);
            command
        }
        HistoryKind::Git => {
            let mut command = Command::new(GIT);
            command
                .current_dir(root)
                .args(["show", &format!("{commit}:{}", path.display())]);
            command
        }
    };
    let output = run_output(
        &mut command,
        "historical flake.lock read",
        cancellation,
        HISTORY_TIMEOUT,
    )?;
    String::from_utf8(output.stdout)
        .map_err(|_| "historical flake.lock contains invalid UTF-8".to_owned())
}

fn repository_root(directory: &Path, marker: &str) -> Option<PathBuf> {
    directory
        .ancestors()
        .find(|ancestor| ancestor.join(marker).exists())
        .map(Path::to_owned)
}

fn run(
    command: &mut Command,
    label: &str,
    cancellation: &AtomicBool,
) -> Result<std::process::Output, String> {
    run_output(command, label, cancellation, UPDATE_TIMEOUT)
}

fn run_output(
    command: &mut Command,
    label: &str,
    cancellation: &AtomicBool,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    command::run(
        command,
        label,
        cancellation,
        timeout,
        OutputLimits {
            stdout: MAX_OUTPUT,
            stderr: MAX_OUTPUT,
        },
        |_| {},
        |stderr| {
            let detail = String::from_utf8_lossy(stderr).trim().to_owned();
            if detail.is_empty() {
                format!("{label} failed")
            } else {
                detail
            }
        },
    )
}

fn input_from_reference(
    name: &str,
    reference: &Value,
    lock: &LockFile,
) -> Result<FlakeInput, String> {
    let follows = reference.as_array().map(|path| {
        path.iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join("/")
    });
    let node = resolve_reference(reference, lock, &mut HashSet::new())
        .ok_or_else(|| format!("input {name:?} points to a missing lock node"))?;
    let locked = node.locked.as_ref().or(node.original.as_ref());
    Ok(FlakeInput {
        name: name.to_owned(),
        source: locked.map_or_else(|| "indirect input".to_owned(), source_label),
        source_kind: locked
            .and_then(|value| value.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("indirect")
            .to_owned(),
        revision: locked
            .and_then(revision_label)
            .unwrap_or_else(|| "unlocked".to_owned()),
        last_modified: locked
            .and_then(|value| value.get("lastModified"))
            .and_then(Value::as_i64),
        follows,
        flake_ref: locked.and_then(flake_reference),
    })
}

fn resolve_reference<'a>(
    reference: &Value,
    lock: &'a LockFile,
    visited: &mut HashSet<String>,
) -> Option<&'a LockNode> {
    if let Some(node_name) = reference.as_str() {
        return lock.nodes.get(node_name);
    }
    let path = reference.as_array()?;
    let mut node_name = lock.root.clone();
    for segment in path {
        let segment = segment.as_str()?;
        if !visited.insert(format!("{node_name}/{segment}")) {
            return None;
        }
        let next = lock.nodes.get(&node_name)?.inputs.get(segment)?;
        if let Some(next_name) = next.as_str() {
            node_name = next_name.to_owned();
        } else {
            return resolve_reference(next, lock, visited);
        }
    }
    lock.nodes.get(&node_name)
}

fn source_label(value: &Value) -> String {
    let owner = value.get("owner").and_then(Value::as_str);
    let repo = value.get("repo").and_then(Value::as_str);
    if let (Some(owner), Some(repo)) = (owner, repo) {
        return format!("{owner}/{repo}");
    }
    if let Some(url) = value.get("url").and_then(Value::as_str) {
        return url.to_owned();
    }
    if let Some(path) = value.get("path").and_then(Value::as_str) {
        return path.to_owned();
    }
    value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown source")
        .to_owned()
}

fn revision_label(value: &Value) -> Option<String> {
    ["rev", "ref", "narHash"]
        .into_iter()
        .find_map(|key| value.get(key).and_then(Value::as_str).map(str::to_owned))
}

fn flake_reference(value: &Value) -> Option<String> {
    let kind = value.get("type")?.as_str()?;
    let revision = revision_label(value)?;
    match kind {
        "github" | "gitlab" | "sourcehut" => {
            let owner = value.get("owner")?.as_str()?;
            let repo = value.get("repo")?.as_str()?;
            Some(format!("{kind}:{owner}/{repo}/{revision}"))
        }
        "git" => {
            let url = value.get("url")?.as_str()?;
            let separator = if url.contains('?') { '&' } else { '?' };
            let prefix = if url.starts_with("git+") {
                url.to_owned()
            } else {
                format!("git+{url}")
            };
            Some(format!("{prefix}{separator}rev={revision}"))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(content: &str) -> Vec<FlakeInput> {
        parse_inputs(content, Path::new("flake.lock")).unwrap()
    }

    #[test]
    fn lists_direct_inputs_and_resolves_follows() {
        let inputs = parse(
            r#"{
                "nodes": {
                    "root": { "inputs": { "zeta": "zeta", "alias": ["zeta"] } },
                    "zeta": {
                        "locked": {
                            "type": "github",
                            "owner": "owner",
                            "repo": "repo",
                            "rev": "abcdef123456",
                            "lastModified": 42
                        }
                    }
                },
                "root": "root",
                "version": 7
            }"#,
        );
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].name, "alias");
        assert_eq!(inputs[0].follows.as_deref(), Some("zeta"));
        assert_eq!(inputs[0].source, "owner/repo");
        assert_eq!(inputs[1].revision, "abcdef123456");
    }

    #[test]
    fn builds_an_explicit_revision_reference() {
        let input = parse(
            r#"{
                "nodes": {
                    "root": { "inputs": { "demo": "demo" } },
                    "demo": {
                        "locked": {
                            "type": "github",
                            "owner": "owner",
                            "repo": "repo",
                            "rev": "abcdef123456"
                        }
                    }
                },
                "root": "root",
                "version": 7
            }"#,
        )
        .remove(0);
        assert_eq!(revision_ref(&input, "v2").unwrap(), "github:owner/repo/v2");
        assert!(revision_ref(&input, "invalid revision").is_err());
    }

    #[test]
    fn reads_input_revisions_from_jj_history() {
        let directory =
            std::env::temp_dir().join(format!("swix-input-jj-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let run_jj = |arguments: &[&str]| {
            let status = Command::new(JJ)
                .current_dir(&directory)
                .args(arguments)
                .status()
                .unwrap();
            assert!(status.success());
        };
        run_jj(&["git", "init"]);
        run_jj(&["config", "set", "--repo", "user.name", "Swix Fixture"]);
        run_jj(&[
            "config",
            "set",
            "--repo",
            "user.email",
            "swix@example.invalid",
        ]);
        fs::write(directory.join("flake.lock"), lock_fixture("aaaaaaaa")).unwrap();
        run_jj(&["commit", "-m", "first lock"]);
        fs::write(directory.join("flake.lock"), lock_fixture("bbbbbbbb")).unwrap();
        run_jj(&["commit", "-m", "second lock"]);

        let cancellation = AtomicBool::new(false);
        let records = history(&directory, "demo", &cancellation).unwrap();
        assert_eq!(records[0].revision, "bbbbbbbb");
        assert!(records.iter().any(|record| record.revision == "aaaaaaaa"));
        fs::remove_dir_all(directory).unwrap();
    }

    fn lock_fixture(revision: &str) -> String {
        format!(
            r#"{{
                "nodes": {{
                    "root": {{ "inputs": {{ "demo": "demo" }} }},
                    "demo": {{
                        "locked": {{
                            "type": "github",
                            "owner": "owner",
                            "repo": "repo",
                            "rev": "{revision}"
                        }}
                    }}
                }},
                "root": "root",
                "version": 7
            }}"#
        )
    }
}
