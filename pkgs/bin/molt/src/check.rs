//! `molt check`: compares the flake repository with its upstream without starting GTK, so
//! scripts can poll it.

use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};

use gtk::glib::ExitCode;
use serde::Serialize;
use molt::command::CANCELLED;
use crate::config::load_config;
use crate::home::repository::{self, RepositoryKind};
use crate::watchlist::{PrState, StepState};
use crate::watchlist::db::{list_prs, open_db, save_pr};
use crate::watchlist::github::fetch_pr;

const USAGE: &str = "usage: molt check [--json] [--no-fetch]";
const HELP: &str = "\
Report incoming and local changes between the flake repository and its upstream.

  --json      print one JSON object: {\"vcs\",\"upstream\",\"incoming\",\"local\",\"fetched\",\"commits\"}
              where commits lists every incoming commit, newest first, as {\"id\",\"author\",\"time\",\"summary\"}
  --no-fetch  compare against the last fetched upstream instead of fetching (no network)
  -h, --help  show this help

Exits 0 on success, 1 when the check fails, 2 on invalid arguments, 130 when interrupted.";

/// Set by SIGINT/SIGTERM; the running git/jj child is in its own process group, so it only
/// stops when command execution sees this flag.
static CANCELLATION: AtomicBool = AtomicBool::new(false);

struct Options {
    json: bool,
    fetch: bool,
}

#[derive(Serialize)]
struct Report<'a> {
    vcs: &'a str,
    upstream: &'a str,
    incoming: usize,
    local: usize,
    fetched: bool,
    /// Every incoming commit, newest first.
    commits: Vec<Commit<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    watchlist: Option<WatchlistReport>,
}

#[derive(Serialize)]
struct WatchlistReport {
    total: usize,
    ready: usize,
    prs: Vec<WatchedPrReport>,
}

#[derive(Serialize)]
struct WatchedPrReport {
    pr_number: u64,
    repo: String,
    title: String,
    ready: bool,
    state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    merge_commit: Option<String>,
    branches: Vec<BranchReport>,
}

#[derive(Serialize)]
struct BranchReport {
    name: String,
    reached: bool,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    ahead_by: Option<usize>,
}

#[derive(Serialize)]
struct Commit<'a> {
    id: &'a str,
    author: &'a str,
    time: i64,
    summary: &'a str,
}

/// Runs `molt check`; `arguments` are those after the subcommand.
pub(crate) fn run(arguments: impl IntoIterator<Item = OsString>) -> ExitCode {
    let options = match parse(arguments) {
        Ok(Some(options)) => options,
        Ok(None) => {
            println!("{USAGE}\n\n{HELP}");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("molt check: {error}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    install_signal_handlers();
    let status = match load_config()
        .and_then(|config| repository::check(&config.flake_dir, options.fetch, &CANCELLATION))
    {
        Ok(status) => status,
        Err(error) if error == CANCELLED => return ExitCode::from(130),
        Err(error) => {
            eprintln!("molt check: {error}");
            return ExitCode::FAILURE;
        }
    };
    let vcs = match status.kind {
        RepositoryKind::Git => "git",
        RepositoryKind::Jj => "jj",
    };
    let watchlist_report = if let Ok(Some(conn)) = open_db() {
        if options.fetch {
            if let Ok(current_prs) = list_prs(&conn) {
                for pr in current_prs {
                    if let Ok(refreshed) = fetch_pr(&pr.owner, &pr.repo, pr.pr_number, Some(pr.id), &CANCELLATION) {
                        let _ = save_pr(&conn, &refreshed);
                    }
                }
            }
        }
        if let Ok(prs) = list_prs(&conn) {
            let total = prs.len();
            let ready = prs.iter().filter(|p| p.is_ready()).count();
            let pr_reports = prs
                .into_iter()
                .map(|p| {
                    let is_ready = p.is_ready();
                    let state = match p.pr_state {
                        PrState::Merged => "merged",
                        PrState::Open => "open",
                        PrState::Closed => "closed",
                    };
                    let branches = p
                        .channels
                        .iter()
                        .map(|c| {
                            let reached = c.state == StepState::Reached;
                            let (status, ahead_by) = match c.state {
                                StepState::Reached => ("reached", None),
                                StepState::Pending { ahead_by } => ("pending", Some(ahead_by)),
                                StepState::Open => ("open", None),
                            };
                            BranchReport {
                                name: c.name.clone(),
                                reached,
                                status,
                                ahead_by,
                            }
                        })
                        .collect();
                    WatchedPrReport {
                        pr_number: p.pr_number,
                        repo: format!("{}/{}", p.owner, p.repo),
                        title: p.title,
                        ready: is_ready,
                        state,
                        merge_commit: p.merge_commit,
                        branches,
                    }
                })
                .collect();
            Some(WatchlistReport {
                total,
                ready,
                prs: pr_reports,
            })
        } else {
            None
        }
    } else {
        None
    };

    if options.json {
        let report = Report {
            vcs,
            upstream: &status.upstream,
            incoming: status.incoming,
            local: status.local,
            fetched: options.fetch,
            commits: status
                .commits
                .iter()
                .map(|commit| Commit {
                    id: &commit.id,
                    author: &commit.author,
                    time: commit.time,
                    summary: &commit.summary,
                })
                .collect(),
            watchlist: watchlist_report,
        };
        match serde_json::to_string(&report) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("molt check: failed to encode JSON: {error}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        print_text_report(&status, vcs, watchlist_report.as_ref());
    }
    ExitCode::SUCCESS
}

fn print_text_report(
    status: &crate::home::repository::RepositoryStatus,
    vcs: &str,
    watchlist: Option<&WatchlistReport>,
) {
    use std::io::IsTerminal;
    use std::time::SystemTime;
    use crate::ui::{seconds_since, short_age};

    let color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();

    let (bold, dim, green, yellow, mauve, teal, reset) = if color {
        (
            "\x1b[1m",
            "\x1b[90m",
            "\x1b[32m",
            "\x1b[33m",
            "\x1b[35m",
            "\x1b[36m",
            "\x1b[0m",
        )
    } else {
        ("", "", "", "", "", "", "")
    };

    let now = SystemTime::now();

    // ── Repository ─────────────────────────────────────────────────────────────
    if status.incoming > 0 {
        let local_str = if status.local > 0 {
            format!(" · {dim}{status_local} local{reset}", status_local = status.local)
        } else {
            String::new()
        };
        println!(
            "{bold}Repository{reset} ({mauve}{vcs}{reset} {dim}· {}{reset} · {yellow}{bold}{} incoming{reset}{local_str}):",
            status.upstream, status.incoming
        );

        for commit in status.commits.iter().take(5) {
            let short_id = &commit.id[..commit.id.len().min(7)];
            let elapsed = short_age(seconds_since(now, commit.time));
            println!();
            println!("  • {mauve}{short_id}{reset}  {bold}{}{reset}", commit.summary);
            println!("    {dim}{}{reset} · {dim}{elapsed} ago{reset}", commit.author);
        }

        if status.incoming > 5 {
            println!();
            println!("  {dim}… and {} more incoming commits{reset}", status.incoming - 5);
        }
    } else {
        let local_str = if status.local > 0 {
            let unit = if status.local == 1 { "change" } else { "changes" };
            format!(" · {dim}{status_local} local {unit}{reset}", status_local = status.local)
        } else {
            String::new()
        };
        println!(
            "{bold}Repository{reset} ({mauve}{vcs}{reset} {dim}· {}{reset} · {green}{bold}✓ in sync{reset}{local_str})",
            status.upstream
        );
    }

    // ── Watchlist ──────────────────────────────────────────────────────────────
    let Some(wl) = watchlist.filter(|wl| wl.total > 0) else {
        return;
    };

    println!();
    if wl.ready > 0 {
        let unit = if wl.ready == 1 { "PR" } else { "PRs" };
        println!(
            "{bold}Watchlist{reset} ({dim}{} tracked · {green}{bold}{} {unit} ready in nixos-unstable{reset}):",
            wl.total, wl.ready
        );
    } else {
        println!(
            "{bold}Watchlist{reset} ({dim}{} tracked · waiting on branches{reset}):",
            wl.total
        );
    }

    for pr in &wl.prs {
        println!();
        println!("  • {mauve}{bold}#{}{reset}  {bold}{}{reset}", pr.pr_number, pr.title);

        let state_str = match pr.state {
            "merged" => format!("{green}Merged{reset}"),
            "open" => format!("{mauve}Open{reset}"),
            _ => format!("{dim}Closed{reset}"),
        };

        let commit_str = pr.merge_commit.as_deref().map_or_else(String::new, |sha| {
            let short = &sha[..sha.len().min(7)];
            format!(" · {dim}{short}{reset}")
        });

        let ready_badge = if pr.ready {
            format!(" · {green}{bold}✓ Ready in nixos-unstable{reset}")
        } else {
            String::new()
        };

        println!("    {teal}{}{reset} · {state_str}{commit_str}{ready_badge}", pr.repo);

        let pipeline = pr
            .branches
            .iter()
            .map(|b| {
                if b.reached {
                    format!("{green}{} ✓{reset}", b.name)
                } else if let Some(ahead) = b.ahead_by {
                    if ahead > 0 {
                        let unit = if ahead == 1 { "commit" } else { "commits" };
                        format!("{yellow}{} ⏳ (+{ahead} {unit}){reset}", b.name)
                    } else {
                        format!("{yellow}{} ⏳{reset}", b.name)
                    }
                } else {
                    format!("{dim}{} ⋯{reset}", b.name)
                }
            })
            .collect::<Vec<_>>()
            .join(&format!(" {dim}──{reset} "));

        println!("    {pipeline}");
    }
}

/// `Ok(None)` asks for help.
fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Option<Options>, String> {
    let mut options = Options {
        json: false,
        fetch: true,
    };
    for argument in arguments {
        match argument.to_str() {
            Some("--json") => options.json = true,
            Some("--no-fetch") => options.fetch = false,
            Some("-h" | "--help") => return Ok(None),
            _ => {
                return Err(format!(
                    "unknown argument: {}",
                    argument.to_string_lossy()
                ));
            }
        }
    }
    Ok(Some(options))
}

fn install_signal_handlers() {
    extern "C" fn cancel(_: libc::c_int) {
        CANCELLATION.store(true, Ordering::Relaxed);
    }
    let handler = cancel as extern "C" fn(libc::c_int) as libc::sighandler_t;
    // SAFETY: the handler only stores to an atomic, which is async-signal-safe.
    unsafe {
        libc::signal(libc::SIGINT, handler);
        libc::signal(libc::SIGTERM, handler);
    }
}
