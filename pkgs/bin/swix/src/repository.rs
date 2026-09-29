use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use swix::command::{self, OutputLimits};

const GIT: &str = match option_env!("SWIX_GIT") {
    Some(path) => path,
    None => "git",
};
const JJ: &str = match option_env!("SWIX_JJ") {
    Some(path) => path,
    None => "jj",
};
const NETWORK_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const COMMAND_TIMEOUT: Duration = Duration::from_secs(2 * 60);
const MAX_OUTPUT: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RepositoryKind {
    Git,
    Jj,
}

impl RepositoryKind {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Git => "Git",
            Self::Jj => "jj",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RepositoryStatus {
    pub(crate) kind: RepositoryKind,
    pub(crate) upstream: String,
    pub(crate) incoming: usize,
    pub(crate) local: usize,
}

impl RepositoryStatus {
    pub(crate) fn detail(&self) -> String {
        match (self.incoming, self.local) {
            (0, 0) => format!("{} · up to date", self.upstream),
            (0, local) => format!("{} · {local} local", self.upstream),
            (incoming, 0) => format!("{} · {incoming} incoming", self.upstream),
            (incoming, local) => {
                format!("{} · {incoming} incoming · {local} local", self.upstream)
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Repository {
    kind: RepositoryKind,
    root: PathBuf,
}

pub(crate) fn check(
    directory: &Path,
    cancellation: &AtomicBool,
) -> Result<RepositoryStatus, String> {
    let repository = detect(directory)?;
    match repository.kind {
        RepositoryKind::Git => {
            git_fetch(&repository.root, cancellation)?;
            git_status(&repository.root, cancellation)
        }
        RepositoryKind::Jj => {
            jj_fetch(&repository.root, cancellation, true)?;
            jj_status(&repository.root, cancellation, true)
        }
    }
}

pub(crate) fn update(
    directory: &Path,
    cancellation: &AtomicBool,
) -> Result<RepositoryStatus, String> {
    let repository = detect(directory)?;
    match repository.kind {
        RepositoryKind::Git => git_update(&repository.root, cancellation)?,
        RepositoryKind::Jj => jj_update(&repository.root, cancellation)?,
    }
    match repository.kind {
        RepositoryKind::Git => git_status(&repository.root, cancellation),
        RepositoryKind::Jj => jj_status(&repository.root, cancellation, false),
    }
}

fn detect(directory: &Path) -> Result<Repository, String> {
    let directory = directory
        .canonicalize()
        .map_err(|error| format!("could not resolve {}: {error}", directory.display()))?;
    for ancestor in directory.ancestors() {
        if ancestor.join(".jj").is_dir() {
            return Ok(Repository {
                kind: RepositoryKind::Jj,
                root: ancestor.to_owned(),
            });
        }
        if ancestor.join(".git").exists() {
            return Ok(Repository {
                kind: RepositoryKind::Git,
                root: ancestor.to_owned(),
            });
        }
    }
    Err(format!(
        "{} is not inside a Git or jj repository",
        directory.display()
    ))
}

fn git_fetch(root: &Path, cancellation: &AtomicBool) -> Result<(), String> {
    let mut command = git(root);
    command.args(["fetch", "--quiet"]);
    run(&mut command, "git fetch", cancellation, NETWORK_TIMEOUT).map(|_| ())
}

fn git_status(root: &Path, cancellation: &AtomicBool) -> Result<RepositoryStatus, String> {
    let upstream = git_text(
        root,
        [
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
        "git upstream lookup",
        cancellation,
    )?;
    let counts = git_text(
        root,
        ["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
        "git revision comparison",
        cancellation,
    )?;
    let (local, incoming) = parse_counts(&counts, "git revision comparison")?;
    Ok(RepositoryStatus {
        kind: RepositoryKind::Git,
        upstream,
        incoming,
        local,
    })
}

fn git_update(root: &Path, cancellation: &AtomicBool) -> Result<(), String> {
    let changes = git_text(
        root,
        ["status", "--porcelain"],
        "git working tree check",
        cancellation,
    )?;
    if !changes.is_empty() {
        return Err(
            "Git working tree has local changes; commit or stash them before updating".to_owned(),
        );
    }
    git_fetch(root, cancellation)?;
    let mut command = git(root);
    command.args(["merge", "--ff-only", "@{upstream}"]);
    run(
        &mut command,
        "git fast-forward",
        cancellation,
        COMMAND_TIMEOUT,
    )
    .map(|_| ())
}

fn git_text<const N: usize>(
    root: &Path,
    arguments: [&str; N],
    name: &str,
    cancellation: &AtomicBool,
) -> Result<String, String> {
    let mut command = git(root);
    command.args(arguments);
    let output = run(&mut command, name, cancellation, COMMAND_TIMEOUT)?;
    text(&output.stdout, name)
}

fn git(root: &Path) -> Command {
    let mut command = Command::new(GIT);
    command
        .args(["-c", "color.ui=false", "-C"])
        .arg(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null());
    command
}

fn jj_fetch(
    root: &Path,
    cancellation: &AtomicBool,
    ignore_working_copy: bool,
) -> Result<(), String> {
    let mut command = jj(root);
    if ignore_working_copy {
        command.arg("--ignore-working-copy");
    }
    command.args([
        "--config",
        "git.abandon-unreachable-commits=false",
        "git",
        "fetch",
    ]);
    run(&mut command, "jj git fetch", cancellation, NETWORK_TIMEOUT).map(|_| ())
}

fn jj_status(
    root: &Path,
    cancellation: &AtomicBool,
    ignore_working_copy: bool,
) -> Result<RepositoryStatus, String> {
    let incoming = jj_revision_count(root, "@..trunk()", cancellation, ignore_working_copy)?;
    let local = jj_revision_count(root, "trunk()..@", cancellation, ignore_working_copy)?;
    Ok(RepositoryStatus {
        kind: RepositoryKind::Jj,
        upstream: "trunk".to_owned(),
        incoming,
        local,
    })
}

fn jj_update(root: &Path, cancellation: &AtomicBool) -> Result<(), String> {
    if jj_has_conflicts(root, None, cancellation)? {
        return Err(
            "jj working branch already contains conflicts; resolve them before updating".to_owned(),
        );
    }
    jj_fetch(root, cancellation, false)?;
    let mut rebase = jj(root);
    rebase.args([
        "--no-integrate-operation",
        "rebase",
        "--branch",
        "@",
        "--onto",
        "trunk()",
    ]);
    let output = run(
        &mut rebase,
        "jj rebase preview",
        cancellation,
        COMMAND_TIMEOUT,
    )?;
    let operation = std::str::from_utf8(&output.stderr)
        .ok()
        .and_then(|stderr| {
            stderr.lines().find_map(|line| {
                line.strip_prefix(
                    "Operation left uncommitted because --no-integrate-operation was requested: ",
                )
            })
        })
        .filter(|value| {
            value.len() >= 12 && value.chars().all(|character| character.is_ascii_hexdigit())
        })
        .ok_or("jj rebase preview returned an invalid operation ID")?;
    if jj_has_conflicts(root, Some(operation), cancellation)? {
        return Err(
            "Updating would introduce jj conflicts; the repository was left unchanged".to_owned(),
        );
    }
    let mut integrate = jj(root);
    integrate.args(["operation", "integrate", operation]);
    run(
        &mut integrate,
        "jj operation integration",
        cancellation,
        COMMAND_TIMEOUT,
    )?;
    let mut workspace = jj(root);
    workspace.args(["workspace", "update-stale"]);
    run(
        &mut workspace,
        "jj workspace update",
        cancellation,
        COMMAND_TIMEOUT,
    )?;
    Ok(())
}

fn jj_has_conflicts(
    root: &Path,
    operation: Option<&str>,
    cancellation: &AtomicBool,
) -> Result<bool, String> {
    let mut command = jj(root);
    if let Some(operation) = operation {
        command.args(["--at-operation", operation]);
    }
    command.args([
        "log",
        "--no-graph",
        "--revisions",
        "conflicts() & ((trunk()..@)::)",
        "--template",
        "commit_id ++ \"\\n\"",
    ]);
    let output = run(
        &mut command,
        "jj conflict check",
        cancellation,
        COMMAND_TIMEOUT,
    )?;
    Ok(!text(&output.stdout, "jj conflict check")?.is_empty())
}

fn jj_revision_count(
    root: &Path,
    revset: &str,
    cancellation: &AtomicBool,
    ignore_working_copy: bool,
) -> Result<usize, String> {
    let mut command = jj(root);
    if ignore_working_copy {
        command.arg("--ignore-working-copy");
    }
    command.args([
        "log",
        "--no-graph",
        "--revisions",
        revset,
        "--template",
        "\"revision\\\\n\"",
    ]);
    let output = run(
        &mut command,
        "jj revision lookup",
        cancellation,
        COMMAND_TIMEOUT,
    )?;
    Ok(std::str::from_utf8(&output.stdout)
        .map_err(|_| "jj revision lookup returned invalid UTF-8")?
        .lines()
        .count())
}

fn jj(root: &Path) -> Command {
    let mut command = Command::new(JJ);
    command
        .args(["--no-pager", "--color", "never", "--repository"])
        .arg(root)
        .stdin(Stdio::null());
    command
}

fn run(
    command: &mut Command,
    name: &str,
    cancellation: &AtomicBool,
    timeout: Duration,
) -> Result<Output, String> {
    command::run(
        command,
        name,
        cancellation,
        timeout,
        OutputLimits {
            stdout: MAX_OUTPUT,
            stderr: MAX_OUTPUT,
        },
        |_| {},
        |detail| String::from_utf8_lossy(detail).trim().to_owned(),
    )
}

fn text(bytes: &[u8], name: &str) -> Result<String, String> {
    std::str::from_utf8(bytes)
        .map(str::trim)
        .map(str::to_owned)
        .map_err(|_| format!("{name} returned invalid UTF-8"))
}

fn parse_counts(value: &str, name: &str) -> Result<(usize, usize), String> {
    let mut values = value.split_whitespace();
    let left = values
        .next()
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| format!("{name} returned invalid revision counts"))?;
    let right = values
        .next()
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| format!("{name} returned invalid revision counts"))?;
    if values.next().is_some() {
        return Err(format!("{name} returned invalid revision counts"));
    }
    Ok((left, right))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
        remote: PathBuf,
        seed: PathBuf,
        local: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir()
                .join(format!("swix-repository-test-{}-{id}", std::process::id()));
            if root.exists() {
                fs::remove_dir_all(&root).unwrap();
            }
            fs::create_dir_all(&root).unwrap();
            let fixture = Self {
                remote: root.join("remote.git"),
                seed: root.join("seed"),
                local: root.join("local"),
                root,
            };
            succeed(
                Command::new(GIT)
                    .args(["init", "--bare", "--initial-branch=main"])
                    .arg(&fixture.remote),
            );
            succeed(
                Command::new(GIT)
                    .args(["init", "--initial-branch=main"])
                    .arg(&fixture.seed),
            );
            succeed(
                Command::new(GIT)
                    .args(["config", "user.name", "Swix Fixture"])
                    .current_dir(&fixture.seed),
            );
            succeed(
                Command::new(GIT)
                    .args(["config", "user.email", "swix@example.test"])
                    .current_dir(&fixture.seed),
            );
            fs::write(fixture.seed.join("base"), "base\n").unwrap();
            succeed(
                Command::new(GIT)
                    .args(["add", "base"])
                    .current_dir(&fixture.seed),
            );
            succeed(
                Command::new(GIT)
                    .args(["commit", "-m", "base"])
                    .current_dir(&fixture.seed),
            );
            succeed(
                Command::new(GIT)
                    .arg("remote")
                    .arg("add")
                    .arg("origin")
                    .arg(&fixture.remote)
                    .current_dir(&fixture.seed),
            );
            succeed(
                Command::new(GIT)
                    .args(["push", "--set-upstream", "origin", "main"])
                    .current_dir(&fixture.seed),
            );
            fixture
        }

        fn advance(&self, file: &str, content: &str, message: &str) {
            fs::write(self.seed.join(file), content).unwrap();
            succeed(
                Command::new(GIT)
                    .args(["add", file])
                    .current_dir(&self.seed),
            );
            succeed(
                Command::new(GIT)
                    .args(["commit", "-m", message])
                    .current_dir(&self.seed),
            );
            succeed(Command::new(GIT).arg("push").current_dir(&self.seed));
        }

        fn clone_git(&self) {
            succeed(
                Command::new(GIT)
                    .arg("clone")
                    .arg(&self.remote)
                    .arg(&self.local),
            );
        }

        fn clone_jj(&self) {
            succeed(
                Command::new(JJ)
                    .args(["git", "clone"])
                    .arg(&self.remote)
                    .arg(&self.local),
            );
            succeed(
                Command::new(JJ)
                    .args(["config", "set", "--repo", "user.name", "Swix Fixture"])
                    .current_dir(&self.local),
            );
            succeed(
                Command::new(JJ)
                    .args(["config", "set", "--repo", "user.email", "swix@example.test"])
                    .current_dir(&self.local),
            );
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }

    fn succeed(command: &mut Command) {
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn cancellation() -> AtomicBool {
        AtomicBool::new(false)
    }

    #[test]
    fn git_update_fast_forwards_an_incoming_commit() {
        let fixture = Fixture::new();
        fixture.clone_git();
        fixture.advance("remote", "remote\n", "remote");

        let status = check(&fixture.local, &cancellation()).unwrap();
        assert_eq!(status.kind, RepositoryKind::Git);
        assert_eq!(status.incoming, 1);

        let status = update(&fixture.local, &cancellation()).unwrap();
        assert_eq!(status.incoming, 0);
        assert_eq!(
            fs::read_to_string(fixture.local.join("remote")).unwrap(),
            "remote\n"
        );
    }

    #[test]
    fn jj_update_rebases_local_work_onto_the_remote_trunk() {
        let fixture = Fixture::new();
        fixture.clone_jj();
        fs::write(fixture.local.join("local"), "local\n").unwrap();
        succeed(
            Command::new(JJ)
                .args(["commit", "-m", "local"])
                .current_dir(&fixture.local),
        );
        fixture.advance("remote", "remote\n", "remote");

        let status = check(&fixture.local, &cancellation()).unwrap();
        assert_eq!(status.kind, RepositoryKind::Jj);
        assert_eq!(status.incoming, 1);

        let status = update(&fixture.local, &cancellation()).unwrap();
        assert_eq!(status.incoming, 0);
        assert_eq!(
            fs::read_to_string(fixture.local.join("local")).unwrap(),
            "local\n"
        );
        assert_eq!(
            fs::read_to_string(fixture.local.join("remote")).unwrap(),
            "remote\n"
        );
    }

    #[test]
    fn jj_update_refuses_a_conflicting_rebase_without_changing_the_working_copy() {
        let fixture = Fixture::new();
        fixture.clone_jj();
        fs::write(fixture.local.join("base"), "local\n").unwrap();
        succeed(
            Command::new(JJ)
                .args(["commit", "-m", "local"])
                .current_dir(&fixture.local),
        );
        fixture.advance("base", "remote\n", "remote");

        let error = update(&fixture.local, &cancellation()).unwrap_err();
        assert!(error.contains("would introduce jj conflicts"));
        assert_eq!(
            fs::read_to_string(fixture.local.join("base")).unwrap(),
            "local\n"
        );
    }

    #[test]
    fn parses_git_divergence_counts_strictly() {
        assert_eq!(parse_counts("2\t5", "fixture").unwrap(), (2, 5));
        assert!(parse_counts("2", "fixture").is_err());
        assert!(parse_counts("2 5 extra", "fixture").is_err());
    }

    #[test]
    fn status_detail_distinguishes_repository_relationships() {
        let mut status = RepositoryStatus {
            kind: RepositoryKind::Jj,
            upstream: "trunk".to_owned(),
            incoming: 0,
            local: 0,
        };
        assert_eq!(status.detail(), "trunk · up to date");
        status.incoming = 3;
        status.local = 2;
        assert_eq!(status.detail(), "trunk · 3 incoming · 2 local");
    }
}
