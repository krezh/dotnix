//! Molt's bar widget: incoming changes in the bar, and while a build or switch runs, or until its
//! end is seen, a droplet of its own split off the bar with a ring of how far it has come.

use std::time::Instant;

use molt_core::{
    build::{flow::Stage, present as build_present},
    report::present as review_present,
};

use super::{format::IslandUnits, runtime::Molt, state::Updates};
use crate::{
    bar_widget::{BarWidget, Emphasis, Part, RingMark},
    features::{Surface, WidgetCx},
    theme::{ACCENT, CRITICAL, GREEN, SUBTEXT1},
};

/// What the droplet shows, when it shows: a switch first, then the build.
fn mini(state: &Updates, now: Instant) -> Option<build_present::Mini> {
    if let Some(review) = &state.review
        && review.activation.is_some()
        && (review.activating() || !state.seen_activation)
    {
        return review_present::mini(review, now);
    }
    let build = state.build.as_ref()?;
    let ended = !matches!(build.stage, Stage::Running);
    if ended && state.seen_build == build.generation {
        return None;
    }
    Some(build_present::mini(build, now, &IslandUnits))
}

fn droplet(shown: build_present::Mini, w: &WidgetCx<'_>) -> BarWidget {
    let running = !shown.failed && !shown.ready;
    let (tint, mark) = if shown.failed {
        (CRITICAL, Some(RingMark::Failed))
    } else if shown.ready {
        (GREEN, Some(RingMark::Done))
    } else {
        (ACCENT, None)
    };
    let share = if running { shown.progress } else { Some(1.0) };
    let elapsed = running.then(|| molt_core::units::Units::elapsed(&IslandUnits, shown.elapsed));
    let brief: Vec<Part> = [Some(shown.brief.clone()), elapsed]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .map(|part| Part::text(part, Emphasis::Small))
        .collect();
    w.compact()
        .tip(format!("{} · {}", shown.title, shown.note))
        .part(
            Part::Ring {
                share,
                color: tint,
                mark,
            }
            .fixed(),
        )
        .part(Part::text(shown.title, Emphasis::Small))
        .parts((!brief.is_empty()).then(|| Part::Cluster {
            color: SUBTEXT1,
            parts: brief,
        }))
        .part(Part::Busy(running))
        .on_click(w.on_island::<Molt>(|molt, _, ui| {
            molt.seen();
            ui.toggle(Molt::panel_handle());
        }))
        .start()
        .droplet()
}

/// With no incoming changes, the ready watched pull requests show in green, since an update now
/// brings their fixes.
pub(super) fn widget(state: &Updates, w: &WidgetCx<'_>) -> Option<BarWidget> {
    if let Some(shown) = mini(state, Instant::now()) {
        return Some(droplet(shown, w));
    }
    let (text, color) = match (&state.error, state.incoming(), state.ready()) {
        (Some(_), _, _) => ("!".to_owned(), CRITICAL),
        (None, 0, ready) if ready > 0 => (ready.to_string(), GREEN),
        (None, incoming, _) if state.report.is_some() => (incoming.to_string(), ACCENT),
        _ => (String::new(), SUBTEXT1),
    };
    let tip = match (&state.error, &state.report) {
        (Some(error), _) => format!("cannot check for updates: {error}"),
        (None, Some(_)) => state.summary(),
        (None, None) => "Molt".to_owned(),
    };
    Some(
        w.compact()
            .color(color)
            .tip(tip)
            .glyph(w.glyph("molt"), color)
            .part(Part::count(text))
            .part(Part::Busy(state.refreshing))
            .on_click(w.toggle(Molt::panel_handle()))
            .on_right(w.call(Molt::refresh)),
    )
}
