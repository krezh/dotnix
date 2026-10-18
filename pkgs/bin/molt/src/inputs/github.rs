//! Revision history for GitHub inputs from the branch activity API: one request returns every
//! update of the tracked branch (pushes, merges), each of which is a lockable tip. For nixpkgs
//! channels this is the list of channel advances, without fetching any of the Git history. One
//! batched GraphQL request then supplies each tip's commit headline and commit date, the date
//! `flake.lock` records as `lastModified`.

use std::collections::HashMap;
use std::process::Command;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use molt::command::CANCELLED;
use serde::Deserialize;
use serde_json::Value;

use super::{FlakeInput, HISTORY_LIMIT, NIX, RevisionRecord, revision_ref};

const API: &str = "https://api.github.com";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const TOKEN_LOOKUP_TIMEOUT: Duration = Duration::from_secs(30);
const DELETED: &str = "0000000000000000000000000000000000000000";

#[derive(Deserialize)]
struct Repository {
    default_branch: String,
}

#[derive(Deserialize)]
struct Activity {
    after: String,
    timestamp: String,
    activity_type: String,
    actor: Option<Actor>,
}

#[derive(Deserialize)]
struct Actor {
    login: String,
}

#[derive(Deserialize)]
struct GraphResponse {
    data: Option<GraphData>,
}

#[derive(Deserialize)]
struct GraphData {
    repository: Option<HashMap<String, Option<CommitDetails>>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitDetails {
    message_headline: Option<String>,
    committed_date: Option<String>,
}

/// Branch updates of a GitHub input, newest first. `None` when the ref has no branch activity
/// (a tag, or a branch GitHub reports nothing for), so the caller reads Git history instead.
pub(super) fn branch_history(
    input: &FlakeInput,
    cancellation: &AtomicBool,
) -> Result<Option<Vec<RevisionRecord>>, String> {
    let repository = &input.source;
    let branch = match input.remote_ref.as_deref() {
        Some(reference) => reference.to_owned(),
        None => {
            check_cancelled(cancellation)?;
            let body = get(&format!("{API}/repos/{repository}"))?;
            serde_json::from_str::<Repository>(&body)
                .map_err(|error| format!("invalid GitHub repository response: {error}"))?
                .default_branch
        }
    };
    let reference = if branch.starts_with("refs/") {
        branch
    } else {
        format!("refs/heads/{branch}")
    };
    check_cancelled(cancellation)?;
    let body = get(&format!(
        "{API}/repos/{repository}/activity?ref={}&per_page={HISTORY_LIMIT}",
        encode_query(&reference)
    ))?;
    let activity = serde_json::from_str::<Vec<Activity>>(&body)
        .map_err(|error| format!("invalid GitHub activity response: {error}"))?;
    let mut records = records_from_activity(activity, input)?;
    if records.is_empty() {
        return Ok(None);
    }
    check_cancelled(cancellation)?;
    // Details only refine the rows: without them the push labels still identify each tip, and
    // the push times stand in for the commit dates.
    if let Ok(mut details) = commit_details(repository, &records) {
        for record in &mut records {
            let Some(commit) = details.remove(&record.revision) else {
                continue;
            };
            if let Some(headline) = commit.message_headline {
                record.summary = headline;
            }
            if let Some(date) = commit.committed_date {
                record.date = date;
            }
        }
    }
    Ok(Some(records))
}

fn records_from_activity(
    activity: Vec<Activity>,
    input: &FlakeInput,
) -> Result<Vec<RevisionRecord>, String> {
    let mut records: Vec<RevisionRecord> = Vec::with_capacity(activity.len());
    for entry in activity {
        if entry.after == DELETED || records.iter().any(|record| record.revision == entry.after) {
            continue;
        }
        let action = match entry.activity_type.as_str() {
            "push" => "Pushed",
            "force_push" => "Force-pushed",
            "pr_merge" => "Pull request merged",
            "merge_queue_merge" => "Merged from queue",
            "branch_creation" => "Branch created",
            other => other,
        };
        let summary = match entry.actor {
            Some(actor) => format!("{action} by {}", actor.login),
            None => action.to_owned(),
        };
        records.push(RevisionRecord {
            flake_ref: revision_ref(input, &entry.after)?,
            revision: entry.after,
            date: entry.timestamp,
            summary,
        });
    }
    Ok(records)
}

/// Commit headlines and dates keyed by revision, fetched for all records in one GraphQL request.
fn commit_details(
    repository: &str,
    records: &[RevisionRecord],
) -> Result<HashMap<String, CommitDetails>, String> {
    let (owner, name) = repository
        .split_once('/')
        .ok_or_else(|| format!("{repository} is not an owner/repository pair"))?;
    let mut query = String::from(
        "query($owner: String!, $name: String!) { repository(owner: $owner, name: $name) {",
    );
    for (index, record) in records.iter().enumerate() {
        // Revisions come from the API as hex object IDs; anything else stays out of the query.
        if record.revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            query.push_str(&format!(
                " c{index}: object(oid: \"{}\") {{ ... on Commit {{ messageHeadline committedDate }} }}",
                record.revision
            ));
        }
    }
    query.push_str(" } }");
    let body = serde_json::json!({
        "query": query,
        "variables": { "owner": owner, "name": name },
    })
    .to_string();
    let response = serde_json::from_str::<GraphResponse>(&post(&format!("{API}/graphql"), &body)?)
        .map_err(|error| format!("invalid GitHub GraphQL response: {error}"))?;
    let mut commits = response
        .data
        .and_then(|data| data.repository)
        .ok_or("GitHub GraphQL response has no repository")?;
    Ok(records
        .iter()
        .enumerate()
        .filter_map(|(index, record)| {
            let commit = commits.remove(&format!("c{index}"))??;
            Some((record.revision.clone(), commit))
        })
        .collect())
}

/// Shared so the activity and GraphQL requests reuse one TLS connection.
static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .build()
        .into()
});

pub(crate) fn get(url: &str) -> Result<String, String> {
    read_response(authorized(AGENT.get(url)).call())
}

fn post(url: &str, body: &str) -> Result<String, String> {
    read_response(
        authorized(AGENT.post(url))
            .header("Content-Type", "application/json")
            .send(body),
    )
}

fn authorized<B>(request: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B> {
    let request = request
        .header("User-Agent", "molt")
        .header("Accept", "application/vnd.github+json");
    match GITHUB_TOKEN.as_deref() {
        Some(token) => request.header("Authorization", &format!("Bearer {token}")),
        None => request,
    }
}

fn read_response(
    result: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<String, String> {
    let mut response = result.map_err(|error| match error {
        ureq::Error::StatusCode(status @ (401 | 403 | 429)) => {
            format!("GitHub API refused the request (HTTP {status}): check the access token or rate limit")
        }
        error => format!("GitHub API request failed: {error}"),
    })?;
    response
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("failed to read GitHub API response: {error}"))
}

/// The `github.com` token from Nix's own `access-tokens` setting, read once per process.
static GITHUB_TOKEN: LazyLock<Option<String>> = LazyLock::new(nix_github_token);

fn nix_github_token() -> Option<String> {
    // Process-wide and shared by every request, so no single view's cancellation applies.
    let output = crate::nix::run(
        Command::new(NIX).args(["config", "show", "--json"]),
        "nix config show",
        &AtomicBool::new(false),
        TOKEN_LOOKUP_TIMEOUT,
        1024 * 1024,
    )
    .ok()?;
    let config: Value = serde_json::from_slice(&output.stdout).ok()?;
    let token = config
        .get("access-tokens")?
        .get("value")?
        .get("github.com")?
        .as_str()?;
    // Nix allows a `type:` prefix such as `PAT:`; the API wants the bare token.
    Some(
        token
            .split_once(':')
            .map_or(token, |(_, token)| token)
            .to_owned(),
    )
}

fn check_cancelled(cancellation: &AtomicBool) -> Result<(), String> {
    if cancellation.load(Ordering::Relaxed) {
        Err(CANCELLED.to_owned())
    } else {
        Ok(())
    }
}

fn encode_query(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> FlakeInput {
        FlakeInput {
            name: "nixpkgs".to_owned(),
            source: "NixOS/nixpkgs".to_owned(),
            source_kind: "github".to_owned(),
            revision: "aaa".to_owned(),
            last_modified: None,
            follows: None,
            flake_ref: Some("github:NixOS/nixpkgs/aaa".to_owned()),
            remote_url: None,
            remote_ref: Some("nixos-unstable".to_owned()),
        }
    }

    #[test]
    fn activity_becomes_lockable_branch_tips() {
        let activity = serde_json::from_str(&format!(
            r#"[
                {{"after":"bbb","timestamp":"2026-10-12T19:19:15Z","activity_type":"push","actor":{{"login":"nixos-channel-bot"}}}},
                {{"after":"{DELETED}","timestamp":"2026-10-11T00:00:00Z","activity_type":"branch_deletion","actor":null}},
                {{"after":"bbb","timestamp":"2026-10-10T00:00:00Z","activity_type":"branch_creation","actor":null}},
                {{"after":"aaa","timestamp":"2026-10-09T00:00:00Z","activity_type":"pr_merge","actor":null}}
            ]"#
        ))
        .unwrap();
        let records = records_from_activity(activity, &input()).unwrap();
        let summary = records
            .iter()
            .map(|record| (record.revision.as_str(), record.summary.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            summary,
            [
                ("bbb", "Pushed by nixos-channel-bot"),
                ("aaa", "Pull request merged")
            ]
        );
        assert_eq!(records[0].flake_ref, "github:NixOS/nixpkgs/bbb");
    }
}
