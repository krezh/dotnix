//! GitHub API integration for tracking pull request promotion across branches.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use super::{ChannelStep, PrState, StepState, WatchedPr};
use crate::inputs::github::get;
use molt::command::CANCELLED;

const API: &str = "https://api.github.com";

pub(crate) const NIXPKGS_CHANNELS: &[&str] = &[
    "master",
    "nixpkgs-unstable",
    "nixos-unstable-small",
    "nixos-unstable",
];

#[derive(Deserialize)]
struct PullResponse {
    title: Option<String>,
    state: Option<String>,
    html_url: Option<String>,
    merged_at: Option<String>,
    merge_commit_sha: Option<String>,
}

#[derive(Deserialize)]
struct CompareResponse {
    status: Option<String>,
    ahead_by: Option<usize>,
}

/// Parses a user-supplied string into (owner, repo, pr_number).
///
/// Accepts:
/// - `https://github.com/NixOS/nixpkgs/pull/573942`
/// - `NixOS/nixpkgs#573942`
/// - `nixpkgs#573942`
/// - `573942` or `#573942` (defaults to NixOS/nixpkgs)
pub(crate) fn parse_pr_ref(input: &str) -> Result<(String, String, u64), String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("enter a PR URL or number".to_owned());
    }

    // Full URL: https://github.com/owner/repo/pull/1234
    if let Some(rest) = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("http://github.com/"))
        .or_else(|| trimmed.strip_prefix("github.com/"))
    {
        let parts: Vec<&str> = rest.split('/').filter(|p| !p.is_empty()).collect();
        if parts.len() >= 4 && parts[2] == "pull" {
            let number = parts[3]
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .unwrap_or("")
                .parse::<u64>()
                .map_err(|_| "invalid PR number in URL".to_owned())?;
            return Ok((parts[0].to_owned(), parts[1].to_owned(), number));
        }
        return Err("expected a URL like https://github.com/owner/repo/pull/123".to_owned());
    }

    // Shorthand: owner/repo#123
    if let Some((repo_part, number_part)) = trimmed.split_once('#') {
        let number = number_part
            .trim()
            .parse::<u64>()
            .map_err(|_| "invalid PR number".to_owned())?;
        let repo_part = repo_part.trim();
        if repo_part.is_empty() {
            // #123 -> NixOS/nixpkgs#123
            return Ok(("NixOS".to_owned(), "nixpkgs".to_owned(), number));
        }
        if let Some((owner, repo)) = repo_part.split_once('/') {
            return Ok((owner.trim().to_owned(), repo.trim().to_owned(), number));
        }
        if repo_part == "nixpkgs" {
            return Ok(("NixOS".to_owned(), "nixpkgs".to_owned(), number));
        }
        return Err("expected format owner/repo#123 or just #123".to_owned());
    }

    // Bare number: 12345
    if let Ok(number) = trimmed.parse::<u64>() {
        return Ok(("NixOS".to_owned(), "nixpkgs".to_owned(), number));
    }

    Err("invalid PR reference: enter a URL, owner/repo#123, or a PR number".to_owned())
}

pub(crate) fn fetch_pr(
    owner: &str,
    repo: &str,
    pr_number: u64,
    existing_id: Option<i64>,
    cancellation: &AtomicBool,
) -> Result<WatchedPr, String> {
    check_cancelled(cancellation)?;

    let pull_url = format!("{API}/repos/{owner}/{repo}/pulls/{pr_number}");
    let response_body =
        get(&pull_url).map_err(|err| format!("failed to fetch PR #{pr_number}: {err}"))?;
    let pull: PullResponse = serde_json::from_str(&response_body)
        .map_err(|err| format!("invalid JSON from GitHub pulls API: {err}"))?;

    let title = pull.title.unwrap_or_else(|| format!("PR #{pr_number}"));
    let html_url = pull
        .html_url
        .unwrap_or_else(|| format!("https://github.com/{owner}/{repo}/pull/{pr_number}"));

    let is_merged = pull.merged_at.is_some();
    let pr_state = if is_merged {
        PrState::Merged
    } else if pull.state.as_deref() == Some("closed") {
        PrState::Closed
    } else {
        PrState::Open
    };

    let target_channels: &[&str] =
        if owner.eq_ignore_ascii_case("NixOS") && repo.eq_ignore_ascii_case("nixpkgs") {
            NIXPKGS_CHANNELS
        } else {
            &["master", "main"]
        };

    let mut channels = Vec::new();

    if let (PrState::Merged, Some(merge_commit)) = (pr_state, pull.merge_commit_sha.as_deref()) {
        for &channel in target_channels {
            check_cancelled(cancellation)?;
            let compare_url =
                format!("{API}/repos/{owner}/{repo}/compare/{channel}...{merge_commit}");
            let step_state = match get(&compare_url) {
                Ok(comp_body) => {
                    if let Ok(comp) = serde_json::from_str::<CompareResponse>(&comp_body) {
                        match comp.status.as_deref() {
                            Some("behind" | "identical") => StepState::Reached,
                            Some("ahead") => StepState::Pending {
                                ahead_by: comp.ahead_by.unwrap_or(0),
                            },
                            _ => StepState::Pending { ahead_by: 0 },
                        }
                    } else {
                        StepState::Pending { ahead_by: 0 }
                    }
                }
                Err(_) => StepState::Pending { ahead_by: 0 },
            };
            channels.push(ChannelStep {
                name: channel.to_owned(),
                state: step_state,
            });
        }
    } else {
        let initial_state = if pr_state == PrState::Open {
            StepState::Open
        } else {
            StepState::Pending { ahead_by: 0 }
        };
        for &channel in target_channels {
            channels.push(ChannelStep {
                name: channel.to_owned(),
                state: initial_state.clone(),
            });
        }
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    Ok(WatchedPr {
        id: existing_id.unwrap_or(0),
        owner: owner.to_owned(),
        repo: repo.to_owned(),
        pr_number,
        title,
        html_url,
        pr_state,
        merge_commit: pull.merge_commit_sha,
        merged_at: pull.merged_at,
        channels,
        added_at: now,
        last_checked: now,
    })
}

fn check_cancelled(cancellation: &AtomicBool) -> Result<(), String> {
    if cancellation.load(Ordering::Relaxed) {
        Err(CANCELLED.to_owned())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_various_pr_reference_formats() {
        assert_eq!(
            parse_pr_ref("573942").unwrap(),
            ("NixOS".to_owned(), "nixpkgs".to_owned(), 573942)
        );
        assert_eq!(
            parse_pr_ref("#573942").unwrap(),
            ("NixOS".to_owned(), "nixpkgs".to_owned(), 573942)
        );
        assert_eq!(
            parse_pr_ref("nixpkgs#573942").unwrap(),
            ("NixOS".to_owned(), "nixpkgs".to_owned(), 573942)
        );
        assert_eq!(
            parse_pr_ref("nix-community/home-manager#123").unwrap(),
            ("nix-community".to_owned(), "home-manager".to_owned(), 123)
        );
        assert_eq!(
            parse_pr_ref("https://github.com/NixOS/nixpkgs/pull/573942").unwrap(),
            ("NixOS".to_owned(), "nixpkgs".to_owned(), 573942)
        );
        assert_eq!(
            parse_pr_ref("https://github.com/NixOS/nixpkgs/pull/573942/files").unwrap(),
            ("NixOS".to_owned(), "nixpkgs".to_owned(), 573942)
        );
        assert!(parse_pr_ref("").is_err());
        assert!(parse_pr_ref("not-a-pr").is_err());
    }
}
