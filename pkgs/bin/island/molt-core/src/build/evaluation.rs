//! Flake-input rows during evaluation, and how long the last evaluation of each target took.

use std::time::{Duration, Instant};

use crate::build::Target;
use crate::build::rows::{RowContent, RowLook};
use crate::cache;
use crate::inputs::FlakeInput;
use crate::nix::FlakeFetch;
use crate::units::Units;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputState {
    Fetching(Instant),
    Fetched(Duration),
    Cached,
    /// Still being fetched when evaluation failed.
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputRow {
    pub key: String,
    pub name: String,
    pub source: String,
    /// The fetched flake reference, without its query.
    pub reference: String,
    pub state: InputState,
}

impl InputRow {
    pub fn content(&self, now: Instant, units: &dyn Units) -> RowContent {
        let (group, look, value) = match self.state {
            InputState::Failed => (0, RowLook::Failed, "failed".to_owned()),
            InputState::Fetching(started) => (
                1,
                RowLook::Active,
                units.timing(now.saturating_duration_since(started), false),
            ),
            InputState::Fetched(took) => (2, RowLook::Done, units.timing(took, true)),
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
pub fn fail_open_fetches(rows: &mut [InputRow]) {
    for row in rows {
        if matches!(row.state, InputState::Fetching(_)) {
            row.state = InputState::Failed;
        }
    }
}

/// Direct inputs with what nix did to each. Fetches matching no direct input (the flake itself,
/// transitive inputs) get rows of their own. A fetch still open when evaluation ended counts as
/// finished then.
pub fn input_rows(
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
            Some(finished) => {
                InputState::Fetched(finished.saturating_duration_since(fetch.started))
            }
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
pub fn last_evaluation(target: Target) -> Option<Duration> {
    let seconds = cache::read(&history_name(target))?
        .trim()
        .parse::<f64>()
        .ok()?;
    (seconds.is_finite() && seconds > 0.0).then(|| Duration::from_secs_f64(seconds))
}

pub fn remember_evaluation(target: Target, took: Duration) {
    cache::write(
        &history_name(target),
        &format!("{:.1}\n", took.as_secs_f64()),
    );
}

#[cfg(test)]
#[path = "evaluation_test.rs"]
mod tests;
