//! Jelly, the Claude Code mascot: its mood from the sessions and Claude's limits, and the lead that
//! shows it large in the panel's header.

use gpui::rgb;

use super::{
    Ai,
    session::Activity as Work,
    state::{Severity, Source, State},
    view::View,
};
use crate::{
    components::mascot::{Activity, Mood},
    features::PanelCx,
    panel::{HeaderLead, Mark},
    theme::{GREEN, LAVENDER, OVERLAY1, OVERLAY2, PEACH, YELLOW},
};

/// The header's Jelly.
const LARGE: f32 = 52.0;
/// Past this much of a limit, it starts to tire.
const TIRES_FROM: f64 = 60.0;

fn mix(from: u32, to: u32, t: f32) -> u32 {
    let (from, to) = (rgb(from), rgb(to));
    let at = |a: f32, b: f32| ((a + (b - a) * t).clamp(0.0, 1.0) * 255.0).round() as u32;
    (at(from.r, to.r) << 16) | (at(from.g, to.g) << 8) | at(from.b, to.b)
}

fn activity(state: &State) -> Activity {
    let claude = || {
        state
            .providers
            .iter()
            .filter(|provider| provider.source == Source::Claude)
            .flat_map(|provider| &provider.limits)
    };
    if !state.requests.is_empty() {
        Activity::Asking
    } else if state
        .sessions
        .iter()
        .any(|session| session.activity == Work::Working)
    {
        Activity::Busy
    } else if claude().any(|limit| limit.severity == Severity::Exhausted) {
        Activity::Asleep
    } else {
        Activity::Rest
    }
}

pub(super) fn mood(state: &State, near: bool) -> Mood {
    let used = state
        .providers
        .iter()
        .filter(|provider| provider.source == Source::Claude)
        .flat_map(|provider| &provider.limits)
        .map(|limit| limit.used)
        .fold(0.0, f64::max);
    let tired = ((used - TIRES_FROM) / (100.0 - TIRES_FROM)).clamp(0.0, 1.0) as f32;
    let activity = activity(state);
    let color = match activity {
        Activity::Rest => LAVENDER,
        Activity::Busy => PEACH,
        Activity::Asking => YELLOW,
        Activity::Asleep => OVERLAY1,
    };
    Mood {
        activity,
        tired,
        color: if activity == Activity::Asleep {
            color
        } else {
            mix(color, OVERLAY2, tired * 0.55)
        },
        cheer_color: GREEN,
        near,
        loading: state.refreshing,
        warning: !state.issues.is_empty(),
    }
}

/// What Claude Code is up to, in a few words.
pub(super) fn status(state: &State) -> String {
    match activity(state) {
        Activity::Asking => match state.requests.first() {
            Some(request) => format!("Waiting on you · {}", request.project),
            None => "Waiting on you".to_owned(),
        },
        Activity::Busy => state
            .sessions
            .iter()
            .find(|session| session.activity == Work::Working)
            .map_or_else(
                || "Working".to_owned(),
                |session| {
                    if session.doing.is_empty() {
                        format!("Working · {}", session.project)
                    } else {
                        format!("Working · {}", session.doing)
                    }
                },
            ),
        Activity::Asleep => "Limit reached".to_owned(),
        Activity::Rest => match state.sessions.len() {
            0 => "No sessions".to_owned(),
            1 => "1 session".to_owned(),
            count => format!("{count} sessions"),
        },
    }
}

/// Jelly large, with what Claude Code is doing; clicking it pokes it.
pub(super) fn lead(state: &State, view: &View, p: &PanelCx<'_>) -> Option<HeaderLead> {
    let pose = view.pose()?;
    Some(HeaderLead {
        mark: Mark::Mascot(pose),
        size: LARGE,
        overline: "Claude Code".into(),
        title: status(state).into(),
        on_click: Some(p.on_local::<Ai>(|_, view| view.poke())),
    })
}
