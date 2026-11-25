//! Keys in molt's panel, which it only takes while a clicked field takes typing: they go into that
//! field, and Escape leaves it. Typed keys go to the worker in order, which applies them to the
//! text it holds, so fast typing loses nothing.

use molt_core::build::{Target, flow::Stage};

use super::{
    runtime::{Field, Molt},
    state::Updates,
    view::{Focus, Screen, View},
};
use crate::{features::Outcome, key::Key};

/// The build's list filter while it builds or failed; the review's once it is built.
fn build_filter(state: &Updates, target: Target) -> Field {
    let building = state
        .build
        .as_ref()
        .is_some_and(|build| build.target == target && !matches!(build.stage, Stage::Ready(_)));
    if building || state.review.is_none() {
        Field::BuildFilter
    } else {
        Field::ReviewFilter
    }
}

fn field(state: &Updates, view: &View) -> Option<Field> {
    Some(match (view.focus, view.screen) {
        (Focus::None, _) => return None,
        (Focus::Pin, _) => Field::InputsPin,
        (Focus::Track, _) => Field::WatchTrack,
        (Focus::Journals, _) => Field::CleanupJournals,
        (Focus::Filter, Screen::Inputs) => Field::InputsFilter,
        (Focus::Filter, Screen::Watchlist) => Field::WatchFilter,
        (Focus::Filter, Screen::Search) => Field::SearchQuery,
        (Focus::Filter, Screen::Cleanup) => Field::CleanupFilter,
        (Focus::Filter, Screen::Build(target)) => build_filter(state, target),
        (Focus::Filter, Screen::Home | Screen::Notes(_) | Screen::Diff(..)) => return None,
    })
}

pub(super) fn key(molt: &Molt, state: &Updates, view: &mut View, key: &Key) -> Outcome {
    match field(state, view) {
        Some(Field::InputsPin) => {
            if key.name == "escape" {
                view.focus = Focus::None;
            }
            molt.typed(Field::InputsPin, key);
        }
        Some(_) if key.name == "escape" => view.focus = Focus::None,
        Some(Field::CleanupJournals) if matches!(key.name.as_str(), "enter" | "return") => {
            view.focus = Focus::None;
        }
        Some(field) => molt.typed(field, key),
        None => view.focus = Focus::None,
    }
    Outcome::Open
}
