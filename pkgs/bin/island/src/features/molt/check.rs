//! The flake repository against its upstream, and the pull requests molt watches.

use std::sync::atomic::AtomicBool;

use molt_core::{
    config::load_config,
    home::repository::{self, RepositoryKind},
    watchlist::{
        PrState, StepState,
        db::{list_prs, open_db, save_pr},
        github::fetch_pr,
    },
};

use super::state::{Branch, Commit, Report, WatchedPr, Watchlist};

/// Nothing stops a check once started; molt-core asks for a flag all the same.
static NEVER: AtomicBool = AtomicBool::new(false);

/// Only with `fetch` does it touch the network, fetching the upstream and refreshing the watchlist
/// from GitHub.
pub(super) fn check(fetch: bool) -> Result<Report, String> {
    let config = load_config()?;
    let status = repository::check(&config.flake_dir, fetch, &NEVER)?;
    Ok(Report {
        incoming: u32::try_from(status.incoming).unwrap_or(u32::MAX),
        local: u32::try_from(status.local).unwrap_or(u32::MAX),
        vcs: match status.kind {
            RepositoryKind::Git => "git",
            RepositoryKind::Jj => "jj",
        }
        .to_owned(),
        upstream: status.upstream,
        commits: status
            .commits
            .into_iter()
            .map(|commit| Commit {
                id: commit.id,
                author: commit.author,
                time: commit.time,
                summary: commit.summary,
            })
            .collect(),
        watchlist: watchlist(fetch),
    })
}

fn watchlist(fetch: bool) -> Option<Watchlist> {
    let conn = open_db().ok()??;
    if fetch && let Ok(watched) = list_prs(&conn) {
        for pr in watched {
            if let Ok(refreshed) = fetch_pr(&pr.owner, &pr.repo, pr.pr_number, Some(pr.id), &NEVER)
            {
                let _ = save_pr(&conn, &refreshed);
            }
        }
    }
    let prs = list_prs(&conn).ok()?;
    Some(Watchlist {
        total: prs.len(),
        ready: prs.iter().filter(|pr| pr.is_ready()).count(),
        prs: prs
            .into_iter()
            .map(|pr| WatchedPr {
                ready: pr.is_ready(),
                pr_number: pr.pr_number,
                repo: format!("{}/{}", pr.owner, pr.repo),
                title: pr.title,
                state: match pr.pr_state {
                    PrState::Merged => "merged",
                    PrState::Open => "open",
                    PrState::Closed => "closed",
                }
                .to_owned(),
                merge_commit: pr.merge_commit,
                branches: pr
                    .channels
                    .into_iter()
                    .map(|channel| {
                        let (status, ahead_by) = match channel.state {
                            StepState::Reached => ("reached", None),
                            StepState::Pending { ahead_by } => ("pending", Some(ahead_by)),
                            StepState::Open => ("open", None),
                        };
                        Branch {
                            name: channel.name,
                            reached: channel.state == StepState::Reached,
                            status: status.to_owned(),
                            ahead_by,
                        }
                    })
                    .collect(),
            })
            .collect(),
    })
}
