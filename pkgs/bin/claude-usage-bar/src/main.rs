use std::io::{self, Read};
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const GREEN: &str = "\x1b[32m";
const ORANGE: &str = "\x1b[38;5;208m";
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const MAGENTA: &str = "\x1b[35m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

const FILLED: char = '\u{2588}';
const EMPTY: char = '\u{2591}';

// ── Rendering ─────────────────────────────────────────────────────────────────

/// Renders a block progress bar of the given character length.
fn bar(pct: f64, length: usize) -> String {
    let filled = ((pct / 100.0) * length as f64) as usize;
    let filled = filled.min(length);
    std::iter::repeat(FILLED)
        .take(filled)
        .chain(std::iter::repeat(EMPTY).take(length - filled))
        .collect()
}

/// Formats a rate-limit usage percentage with a colored bar.
///
/// Thresholds: green below 80%, orange 80–89%, red 90%+.
fn color_rate(pct: Option<f64>) -> String {
    match pct {
        None => format!("{DIM}N/A{RESET}"),
        Some(v) => {
            let color = if v >= 90.0 { RED } else if v >= 80.0 { ORANGE } else { GREEN };
            format!("{color}{} {:.0}%{RESET}", bar(v, 8), v)
        }
    }
}

/// Formats a context-window usage percentage with a colored bar.
///
/// Thresholds are tighter than rate limits (orange 70%+, red 80%+) because
/// approaching the context limit degrades model quality before hitting the hard cap.
fn color_ctx(pct: Option<f64>) -> String {
    match pct {
        None => format!("{DIM}N/A{RESET}"),
        Some(v) => {
            let color = if v > 80.0 { RED } else if v >= 70.0 { ORANGE } else { GREEN };
            format!("{color}{} {:.0}%{RESET}", bar(v, 8), v)
        }
    }
}

/// Formats a unix timestamp as "Day HH:MM" in local time using the system `date` command.
fn format_reset_ts(reset_ts: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if reset_ts <= now {
        return format!("{GREEN}now{RESET}");
    }

    let today = Command::new("date")
        .args(["+%Y-%m-%d"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(o) } else { None })
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let reset_day = Command::new("date")
        .args(["-d", &format!("@{reset_ts}"), "+%Y-%m-%d"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(o) } else { None })
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let fmt = if reset_day == today { "+%H:%M" } else { "+%a %H:%M" };

    let result = Command::new("date")
        .args(["-d", &format!("@{reset_ts}"), fmt])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(o) } else { None })
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    match result {
        Some(s) => format!("{CYAN}{s}{RESET}"),
        None => format!("{CYAN}?{RESET}"),
    }
}

/// Shortens an absolute path by replacing the home directory prefix with `~`.
fn display_path(path_str: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    if !home.is_empty() && path_str.starts_with(&home) {
        format!("~{}", &path_str[home.len()..])
    } else {
        path_str.to_string()
    }
}

// ── Git ───────────────────────────────────────────────────────────────────────

struct GitStatus {
    branch: String,
    staged: u32,
    unstaged: u32,
    untracked: u32,
    conflicts: u32,
    ahead: u32,
    behind: u32,
    operation: Option<String>,
}

/// Runs a git command in `cwd` with `GIT_OPTIONAL_LOCKS=0`.
fn git_cmd(cwd: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(["-C", &cwd.to_string_lossy()])
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

/// Parses `git status --porcelain=v1 -b` output into a GitStatus.
fn parse_git_status(cwd: &Path) -> Option<GitStatus> {
    let output = git_cmd(cwd, &["status", "--porcelain=v1", "-b"])?;

    let mut branch = String::from("HEAD");
    let mut ahead = 0u32;
    let mut behind = 0u32;
    let mut staged = 0u32;
    let mut unstaged = 0u32;
    let mut untracked = 0u32;
    let mut conflicts = 0u32;

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            // Branch line: "main...origin/main [ahead 2, behind 1]" or just "main"
            let (branch_part, tracking_part) = rest
                .split_once("...")
                .map(|(b, t)| (b, Some(t)))
                .unwrap_or((rest, None));

            // Handle detached HEAD: "HEAD (no branch)"
            if branch_part == "HEAD (no branch)" || branch_part == "No Commits" {
                branch = git_cmd(cwd, &["rev-parse", "--short", "HEAD"])
                    .map(|h| format!("({h})"))
                    .unwrap_or_else(|| String::from("(detached)"));
            } else {
                branch = branch_part.to_string();
            }

            if let Some(tracking) = tracking_part {
                // Parse "[ahead N, behind M]" or "[ahead N]" or "[behind M]"
                if let Some(bracket) = tracking.find('[') {
                    let info = &tracking[bracket + 1..];
                    let info = info.trim_end_matches(']');
                    for part in info.split(',') {
                        let part = part.trim();
                        if let Some(n) = part.strip_prefix("ahead ") {
                            ahead = n.parse().unwrap_or(0);
                        } else if let Some(n) = part.strip_prefix("behind ") {
                            behind = n.parse().unwrap_or(0);
                        }
                    }
                }
            }
            continue;
        }

        if line.len() < 2 {
            continue;
        }

        // Porcelain v1: two-character XY status code where X = index, Y = worktree.
        let idx = line.as_bytes()[0] as char;
        let wt = line.as_bytes()[1] as char;

        // Conflict markers
        if matches!((idx, wt), ('D','D') | ('A','U') | ('U','D') | ('U','A') | ('D','U') | ('A','A') | ('U','U')) {
            conflicts += 1;
            continue;
        }

        if idx == '?' && wt == '?' {
            untracked += 1;
            continue;
        }

        if idx != ' ' && idx != '?' {
            staged += 1;
        }
        if wt != ' ' && wt != '?' {
            unstaged += 1;
        }
    }

    // Detect ongoing operations by inspecting the .git dir
    let operation = detect_git_operation(cwd);

    Some(GitStatus { branch, staged, unstaged, untracked, conflicts, ahead, behind, operation })
}

/// Checks `.git/` for in-progress operations (MERGE, REBASE, etc.).
fn detect_git_operation(cwd: &Path) -> Option<String> {
    // Resolve real .git dir (handles worktrees where .git is a file)
    let dot_git = cwd.join(".git");
    let git_dir = if dot_git.is_file() {
        let content = std::fs::read_to_string(&dot_git).ok()?;
        let rel = content.trim().strip_prefix("gitdir: ")?;
        cwd.join(rel.trim())
    } else {
        dot_git
    };

    if git_dir.join("MERGE_HEAD").exists() {
        Some("MERGE".into())
    } else if git_dir.join("CHERRY_PICK_HEAD").exists() {
        Some("CHERRY-PICK".into())
    } else if git_dir.join("REVERT_HEAD").exists() {
        Some("REVERT".into())
    } else if git_dir.join("BISECT_LOG").exists() {
        Some("BISECT".into())
    } else if git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists() {
        Some("REBASE".into())
    } else {
        None
    }
}

/// Builds the git status segment string, or returns `None` when `cwd` is not a git repo.
fn format_git_segment(cwd: &Path) -> Option<String> {
    let s = parse_git_status(cwd)?;

    let mut parts = vec![format!("{BOLD}{}{RESET}", s.branch)];

    // Ongoing operation badge
    if let Some(op) = &s.operation {
        parts.push(format!("{RED}|{op}{RESET}"));
    }

    // Ahead/behind
    if s.ahead > 0 {
        parts.push(format!("{GREEN}↑{}{RESET}", s.ahead));
    }
    if s.behind > 0 {
        parts.push(format!("{RED}↓{}{RESET}", s.behind));
    }

    // Working tree counts
    if s.conflicts > 0 {
        parts.push(format!("{RED}!{}{RESET}", s.conflicts));
    }
    if s.staged > 0 {
        parts.push(format!("{GREEN}+{}{RESET}", s.staged));
    }
    if s.unstaged > 0 {
        parts.push(format!("{YELLOW}~{}{RESET}", s.unstaged));
    }
    if s.untracked > 0 {
        parts.push(format!("{DIM}?{}{RESET}", s.untracked));
    }

    Some(parts.join(" "))
}

// ── Cost / session ────────────────────────────────────────────────────────────

/// Formats the session's net line additions and deletions, or returns `None` when both are absent or zero.
fn format_lines(data: &serde_json::Value) -> Option<String> {
    let added = data["cost"]["total_lines_added"].as_i64();
    let removed = data["cost"]["total_lines_removed"].as_i64();
    match (added, removed) {
        (None, None) => None,
        (a, r) => {
            let mut s = String::new();
            if let Some(n) = a {
                if n > 0 {
                    s.push_str(&format!("{GREEN}+{n}{RESET}"));
                }
            }
            if let Some(n) = r {
                if n > 0 {
                    if !s.is_empty() { s.push(' '); }
                    s.push_str(&format!("{RED}-{n}{RESET}"));
                }
            }
            if s.is_empty() { None } else { Some(s) }
        }
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).ok();

    let data: serde_json::Value =
        serde_json::from_str(input.trim()).unwrap_or(serde_json::Value::Null);

    let cwd: std::path::PathBuf = data["cwd"]
        .as_str()
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let fh = &data["rate_limits"]["five_hour"];
    let sd = &data["rate_limits"]["seven_day"];
    let ctx = &data["context_window"];

    let sep = format!(" {DIM}|{RESET} ");
    let mut parts: Vec<String> = Vec::new();

    // ── Location + git ───────────────────────────────────────────────────────
    let dir_str = display_path(&cwd.to_string_lossy());
    let location = match format_git_segment(&cwd) {
        Some(git) => format!("{BOLD}{dir_str}{RESET} {git}"),
        None => format!("{BOLD}{dir_str}{RESET}"),
    };
    parts.push(location);

    // ── Model ────────────────────────────────────────────────────────────────
    if let Some(model) = data["model"]["display_name"].as_str() {
        parts.push(format!("{CYAN}{model}{RESET}"));
    }

    // ── Session lines ────────────────────────────────────────────────────────
    if let Some(lines) = format_lines(&data) {
        parts.push(lines);
    }

    // ── Rate limits ──────────────────────────────────────────────────────────
    let fh_pct = fh["used_percentage"].as_f64();
    let fh_reset = fh["resets_at"].as_u64();
    let sd_pct = sd["used_percentage"].as_f64();
    let sd_reset = sd["resets_at"].as_u64();

    let fh_reset_str = fh_reset
        .map(|ts| format!(" {}", format_reset_ts(ts)))
        .unwrap_or_default();
    parts.push(format!("{BOLD}5h:{RESET} {}{fh_reset_str}", color_rate(fh_pct)));

    let sd_reset_str = sd_reset
        .map(|ts| format!(" {}", format_reset_ts(ts)))
        .unwrap_or_default();
    parts.push(format!("{BOLD}7d:{RESET} {}{sd_reset_str}", color_rate(sd_pct)));

    // ── Context window ───────────────────────────────────────────────────────
    let ctx_pct = ctx["used_percentage"].as_f64();
    if ctx_pct.is_some() {
        // Show token count if available
        let token_str = ctx["current_usage"]["input_tokens"].as_u64().map(|t| {
            let cache_create = ctx["current_usage"]["cache_creation_input_tokens"].as_u64().unwrap_or(0);
            let cache_read = ctx["current_usage"]["cache_read_input_tokens"].as_u64().unwrap_or(0);
            let total = t + cache_create + cache_read;
            format!(" {MAGENTA}{}k{RESET}", total / 1000)
        }).unwrap_or_default();
        parts.push(format!("{BOLD}ctx:{RESET} {}{token_str}", color_ctx(ctx_pct)));
    }

    println!("{}", parts.join(&sep));
}
