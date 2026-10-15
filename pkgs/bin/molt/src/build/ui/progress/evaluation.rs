//! Flake-input rows during evaluation, and how long the last evaluation of each target took.

use std::time::{Duration, Instant};

use crate::build::Target;
use crate::build::ui::activity::{GroupSpec, RowContent, RowLook};
use crate::cache;
use crate::inputs::FlakeInput;
use crate::nix::FlakeFetch;

use super::format::short_duration;

pub(crate) const INPUT_GROUPS: &[GroupSpec] = &[
    GroupSpec {
        name: "Failed",
        tone: crate::ui::theme::RED,
        newest_first: false,
        optional: true,
    },
    GroupSpec {
        name: "Fetching",
        tone: crate::ui::theme::SAPPHIRE,
        newest_first: false,
        optional: false,
    },
    GroupSpec {
        name: "Fetched",
        tone: crate::ui::theme::GREEN,
        newest_first: false,
        optional: false,
    },
    GroupSpec {
        name: "Cached",
        tone: crate::ui::theme::OVERLAY1,
        newest_first: false,
        optional: false,
    },
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum InputState {
    Fetching(Instant),
    Fetched(Duration),
    Cached,
    /// Still being fetched when evaluation failed.
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InputRow {
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) source: String,
    /// The fetched flake reference, without its query.
    pub(crate) reference: String,
    pub(crate) state: InputState,
}

impl InputRow {
    pub(crate) fn content(&self, now: Instant) -> RowContent {
        let (group, look, value) = match self.state {
            InputState::Failed => (0, RowLook::Failed, "failed".to_owned()),
            InputState::Fetching(started) => (
                1,
                RowLook::Active,
                short_duration(now.saturating_duration_since(started), false),
            ),
            InputState::Fetched(took) => (2, RowLook::Done, short_duration(took, true)),
            InputState::Cached => (3, RowLook::Idle, "local".to_owned()),
        };
        RowContent {
            group,
            look,
            title: self.name.clone(),
            tag: String::new(),
            column: self.source.clone(),
            detail: self.reference.clone(),
            value,
            fraction: 0.0,
        }
    }
}

/// Marks fetches still open when evaluation failed as the inputs that failed.
pub(crate) fn fail_open_fetches(rows: &mut [InputRow]) {
    for row in rows {
        if matches!(row.state, InputState::Fetching(_)) {
            row.state = InputState::Failed;
        }
    }
}

/// Direct inputs with what nix did to each. Fetches matching no direct input (the flake itself,
/// transitive inputs) get rows of their own. A fetch still open when evaluation ended counts as
/// finished then.
pub(crate) fn input_rows(
    inputs: &[FlakeInput],
    fetches: &[FlakeFetch],
    evaluation_done: Option<Instant>,
) -> Vec<InputRow> {
    let mut rows = inputs
        .iter()
        .map(|input| InputRow {
            key: format!("input:{}", input.name),
            name: input.name.clone(),
            source: input.source.clone(),
            reference: input.flake_ref.clone().unwrap_or_default(),
            state: InputState::Cached,
        })
        .collect::<Vec<_>>();
    let mut taken = vec![false; inputs.len()];
    for fetch in fetches {
        let state = match fetch.finished.or(evaluation_done) {
            Some(finished) => InputState::Fetched(finished.saturating_duration_since(fetch.started)),
            None => InputState::Fetching(fetch.started),
        };
        let reference = fetch
            .input
            .split('?')
            .next()
            .unwrap_or(&fetch.input)
            .to_owned();
        match match_fetch(&fetch.input, inputs, &taken) {
            Some(index) => {
                taken[index] = true;
                rows[index].reference = reference;
                rows[index].state = state;
            }
            None => rows.push(InputRow {
                key: format!("fetch:{}", fetch.input),
                name: fetch_name(&reference),
                source: reference.clone(),
                reference,
                state,
            }),
        }
    }
    rows
}

/// The direct input a fetch announcement refers to: its source must appear as a whole path in
/// the fetched reference; a matching locked revision, then a non-`follows` input, break ties.
fn match_fetch(fetch: &str, inputs: &[FlakeInput], taken: &[bool]) -> Option<usize> {
    let fetch = fetch.to_lowercase();
    inputs
        .iter()
        .enumerate()
        .filter(|(index, input)| !taken[*index] && mentions(&fetch, &input.source.to_lowercase()))
        .rev()
        .max_by_key(|(_, input)| {
            (
                fetch.contains(&input.revision.to_lowercase()),
                input.follows.is_none(),
            )
        })
        .map(|(index, _)| index)
}

fn mentions(haystack: &str, needle: &str) -> bool {
    let name_char = |c: char| c.is_alphanumeric() || matches!(c, '-' | '_' | '.');
    !needle.is_empty()
        && haystack.match_indices(needle).any(|(at, _)| {
            !haystack[..at].chars().next_back().is_some_and(name_char)
                && !haystack[at + needle.len()..]
                    .chars()
                    .next()
                    .is_some_and(name_char)
        })
}

/// `github:owner/repo/rev` → `repo`; URLs and paths → their last segment.
fn fetch_name(reference: &str) -> String {
    for scheme in ["github:", "gitlab:", "sourcehut:"] {
        if let Some(rest) = reference.strip_prefix(scheme)
            && let Some(repo) = rest.split('/').nth(1)
        {
            return repo.to_owned();
        }
    }
    reference
        .trim_end_matches('/')
        .rsplit(['/', ':'])
        .find(|segment| !segment.is_empty())
        .map_or(reference, |segment| segment.trim_end_matches(".git"))
        .to_owned()
}

fn history_name(target: Target) -> String {
    format!("evaluation-{}", target.slug())
}

/// How long the last successful evaluation of `target` took.
pub(crate) fn last_evaluation(target: Target) -> Option<Duration> {
    let seconds = cache::read(&history_name(target))?
        .trim()
        .parse::<f64>()
        .ok()?;
    (seconds.is_finite() && seconds > 0.0).then(|| Duration::from_secs_f64(seconds))
}

pub(crate) fn remember_evaluation(target: Target, took: Duration) {
    cache::write(
        &history_name(target),
        &format!("{:.1}\n", took.as_secs_f64()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    /// Direct inputs as `inputs::load` reads them from a lock file with these GitHub nodes.
    fn inputs(nodes: &[(&str, &str, &str, &str)]) -> Vec<FlakeInput> {
        let directory = env::temp_dir().join(format!(
            "molt-build-progress-test-{}-{}",
            std::process::id(),
            nodes.len()
        ));
        fs::create_dir_all(&directory).unwrap();
        let mut lock = serde_json::json!({
            "nodes": { "root": { "inputs": {} } },
            "root": "root",
            "version": 7,
        });
        for (name, owner, repo, rev) in nodes {
            lock["nodes"]["root"]["inputs"][name] = serde_json::json!(name);
            lock["nodes"][name] = serde_json::json!({
                "locked": { "type": "github", "owner": owner, "repo": repo, "rev": rev },
            });
        }
        fs::write(directory.join("flake.lock"), lock.to_string()).unwrap();
        let (_, inputs) = crate::inputs::load(&directory).unwrap();
        fs::remove_dir_all(&directory).unwrap();
        inputs
    }

    fn fetch(input: &str, started: Instant, finished: Option<Instant>) -> FlakeFetch {
        FlakeFetch {
            input: input.to_owned(),
            started,
            finished,
        }
    }

    #[test]
    fn fetches_map_to_direct_inputs_by_source_and_revision() {
        let now = Instant::now();
        let inputs = inputs(&[
            ("nixpkgs", "NixOS", "nixpkgs", "aaa111"),
            ("nixpkgs-stable", "NixOS", "nixpkgs", "bbb222"),
            ("nixpkgs-wayland", "nix-community", "nixpkgs-wayland", "ccc"),
        ]);
        let fetches = [
            fetch("github:NixOS/nixpkgs/bbb222?narHash=x", now, None),
            fetch(
                "git+file:///home/me/dotnix",
                now,
                Some(now + Duration::from_millis(800)),
            ),
        ];
        let rows = input_rows(&inputs, &fetches, None);
        assert_eq!(rows[0].state, InputState::Cached);
        assert_eq!(rows[1].state, InputState::Fetching(now));
        assert_eq!(rows[1].reference, "github:NixOS/nixpkgs/bbb222");
        assert_eq!(rows[2].state, InputState::Cached);
        assert_eq!(rows[3].name, "dotnix");
        assert_eq!(
            rows[3].state,
            InputState::Fetched(Duration::from_millis(800))
        );

        let done = now + Duration::from_secs(2);
        let rows = input_rows(&inputs, &fetches, Some(done));
        assert_eq!(rows[1].state, InputState::Fetched(Duration::from_secs(2)));
    }

    #[test]
    fn source_matches_need_whole_path_segments() {
        assert!(mentions("github:nixos/nixpkgs/rev", "nixos/nixpkgs"));
        assert!(!mentions("github:nixos/nixpkgs-wayland/rev", "nixos/nixpkgs"));
        assert!(!mentions("github:xnixos/nixpkgs/rev", "nixos/nixpkgs"));
        assert_eq!(fetch_name("github:catppuccin/nix/abc"), "nix");
        assert_eq!(fetch_name("git+https://example.com/a/repo.git"), "repo");
    }

    #[test]
    fn open_fetches_fail_with_evaluation() {
        let now = Instant::now();
        let fetches = [
            fetch("github:NixOS/nixpkgs/aaa", now, None),
            fetch("git+file:///home/me/dotnix", now, Some(now)),
        ];
        let mut rows = input_rows(&[], &fetches, None);
        fail_open_fetches(&mut rows);
        assert_eq!(rows[0].state, InputState::Failed);
        assert_eq!(rows[0].content(now).group, 0);
        assert_eq!(rows[1].state, InputState::Fetched(Duration::ZERO));
    }
}
