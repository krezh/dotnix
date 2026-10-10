use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;
use swix::command::{self, OutputLimits};

const NIX: &str = match option_env!("SWIX_NIX") {
    Some(path) => path,
    None => "nix",
};
const GIT: &str = match option_env!("SWIX_GIT") {
    Some(path) => path,
    None => "git",
};
const UPDATE_TIMEOUT: Duration = Duration::from_secs(20 * 60);
const HISTORY_TIMEOUT: Duration = Duration::from_secs(2 * 60);
const MAX_OUTPUT: usize = 4 * 1024 * 1024;
const HISTORY_LIMIT: usize = 48;
static HISTORY_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FlakeInput {
    pub(crate) name: String,
    pub(crate) source: String,
    pub(crate) source_kind: String,
    pub(crate) revision: String,
    pub(crate) last_modified: Option<i64>,
    pub(crate) follows: Option<String>,
    pub(crate) flake_ref: Option<String>,
    remote_url: Option<String>,
    remote_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RevisionRecord {
    pub(crate) revision: String,
    pub(crate) flake_ref: String,
    pub(crate) date: String,
    pub(crate) summary: String,
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
    let (_, inputs) = load(directory)?;
    let input = inputs
        .into_iter()
        .find(|input| input.name == input_name)
        .ok_or_else(|| format!("input {input_name:?} is no longer in flake.lock"))?;
    let remote_url = input
        .remote_url
        .as_deref()
        .ok_or_else(|| format!("{} does not expose a Git remote", input.name))?;
    let target = input.remote_ref.as_deref().unwrap_or("HEAD");
    let repository = TemporaryRepository::new()?;

    let mut init = Command::new(GIT);
    init.args(["init", "--bare", "--quiet"])
        .arg(&repository.path);
    run_output(
        &mut init,
        "remote revision workspace",
        cancellation,
        HISTORY_TIMEOUT,
    )?;

    let mut fetch = Command::new(GIT);
    fetch.current_dir(&repository.path).args([
        "fetch",
        "--quiet",
        "--depth",
        &HISTORY_LIMIT.to_string(),
        "--filter=blob:none",
        "--no-tags",
        remote_url,
        target,
    ]);
    run_output(
        &mut fetch,
        "remote flake revision fetch",
        cancellation,
        HISTORY_TIMEOUT,
    )?;

    let mut log = Command::new(GIT);
    log.current_dir(&repository.path).args([
        "log",
        "--format=%H%x1f%cI%x1f%s",
        "--max-count",
        &HISTORY_LIMIT.to_string(),
        "FETCH_HEAD",
    ]);
    let output = run_output(
        &mut log,
        "remote flake revision history",
        cancellation,
        HISTORY_TIMEOUT,
    )?;
    let text = String::from_utf8(output.stdout)
        .map_err(|_| "remote revision history returned invalid UTF-8".to_owned())?;
    parse_remote_history(&text, &input)
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

struct TemporaryRepository {
    path: PathBuf,
}

impl TemporaryRepository {
    fn new() -> Result<Self, String> {
        let id = HISTORY_ID.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("swix-remote-history-{}-{id}", std::process::id()));
        if path.exists() {
            fs::remove_dir_all(&path)
                .map_err(|error| format!("failed to reset {}: {error}", path.display()))?;
        }
        Ok(Self { path })
    }
}

impl Drop for TemporaryRepository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn parse_remote_history(text: &str, input: &FlakeInput) -> Result<Vec<RevisionRecord>, String> {
    text.lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let mut fields = line.splitn(3, '\x1f');
            let revision = fields
                .next()
                .filter(|field| !field.is_empty())
                .ok_or_else(|| "remote history omitted a commit revision".to_owned())?;
            let date = fields
                .next()
                .ok_or_else(|| "remote history omitted a commit date".to_owned())?;
            let summary = fields.next().unwrap_or("Remote commit");
            Ok(RevisionRecord {
                revision: revision.to_owned(),
                flake_ref: revision_ref(input, revision)?,
                date: date.to_owned(),
                summary: summary.to_owned(),
            })
        })
        .collect()
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
    let original = node.original.as_ref();
    let locked = node.locked.as_ref().or(original);
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
        remote_url: locked.and_then(remote_url),
        remote_ref: original
            .and_then(remote_ref)
            .or_else(|| locked.and_then(remote_ref)),
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

fn remote_url(value: &Value) -> Option<String> {
    match value.get("type")?.as_str()? {
        "github" => Some(format!(
            "https://github.com/{}/{}.git",
            value.get("owner")?.as_str()?,
            value.get("repo")?.as_str()?
        )),
        "gitlab" => Some(format!(
            "https://gitlab.com/{}/{}.git",
            value.get("owner")?.as_str()?,
            value.get("repo")?.as_str()?
        )),
        "sourcehut" => Some(format!(
            "https://git.sr.ht/{}/{}",
            value.get("owner")?.as_str()?,
            value.get("repo")?.as_str()?
        )),
        "git" => value
            .get("url")?
            .as_str()
            .map(|url| url.strip_prefix("git+").unwrap_or(url).to_owned()),
        _ => None,
    }
}

fn remote_ref(value: &Value) -> Option<String> {
    value.get("ref")?.as_str().map(str::to_owned)
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
        assert_eq!(
            inputs[1].remote_url.as_deref(),
            Some("https://github.com/owner/repo.git")
        );
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
    fn reads_revisions_from_the_flake_remote() {
        let root =
            std::env::temp_dir().join(format!("swix-input-remote-test-{}", std::process::id()));
        let remote = root.join("remote");
        let flake = root.join("flake");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&remote).unwrap();
        fs::create_dir_all(&flake).unwrap();
        let run_git = |arguments: &[&str]| {
            let output = Command::new(GIT)
                .current_dir(&remote)
                .args(arguments)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8(output.stdout).unwrap().trim().to_owned()
        };
        run_git(&["init", "--initial-branch=main"]);
        run_git(&["config", "user.name", "Swix Fixture"]);
        run_git(&["config", "user.email", "swix@example.invalid"]);
        fs::write(remote.join("value"), "first").unwrap();
        run_git(&["add", "value"]);
        run_git(&["commit", "-m", "first remote revision"]);
        let first = run_git(&["rev-parse", "HEAD"]);
        fs::write(remote.join("value"), "second").unwrap();
        run_git(&["commit", "-am", "second remote revision"]);
        let second = run_git(&["rev-parse", "HEAD"]);
        fs::write(
            flake.join("flake.lock"),
            remote_lock_fixture(&format!("file://{}", remote.display()), &second),
        )
        .unwrap();

        let records = history(&flake, "demo", &AtomicBool::new(false)).unwrap();
        assert_eq!(records[0].revision, second);
        assert_eq!(records[0].summary, "second remote revision");
        assert!(records.iter().any(|record| record.revision == first));
        assert!(records[0].flake_ref.ends_with(&records[0].revision));
        fs::remove_dir_all(root).unwrap();
    }

    fn remote_lock_fixture(url: &str, revision: &str) -> String {
        format!(
            r#"{{
                "nodes": {{
                    "root": {{ "inputs": {{ "demo": "demo" }} }},
                    "demo": {{
                        "locked": {{
                            "type": "git",
                            "url": "{url}",
                            "rev": "{revision}"
                        }},
                        "original": {{
                            "type": "git",
                            "url": "{url}",
                            "ref": "main"
                        }}
                    }}
                }},
                "root": "root",
                "version": 7
            }}"#
        )
    }
}
