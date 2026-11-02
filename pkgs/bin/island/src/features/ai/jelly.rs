//! Jelly, the panel's mascot: its mood from active sessions and the configured quota fleet, and
//! the header lead naming what the AI panel is doing across harnesses.

use super::{
    Ai,
    mascot::{self, Activity, Mood},
    overview,
    session::Activity as Work,
    state::{Source, State},
    view::{Tab, View},
};
use crate::{
    features::PanelCx,
    panel::HeaderLead,
    theme::{GREEN, LAVENDER, OVERLAY1, OVERLAY2, PEACH, YELLOW},
};

const LARGE: f32 = 52.0;
const TIRES_FROM: f64 = 60.0;

fn mix(from: u32, to: u32, t: f32) -> u32 {
    let at = |shift: u32| {
        let (a, b) = (
            ((from >> shift) & 0xff) as f32,
            ((to >> shift) & 0xff) as f32,
        );
        ((a + (b - a) * t).clamp(0.0, 255.0).round() as u32) << shift
    };
    at(16) | at(8) | at(0)
}

pub(super) fn activity(state: &State) -> Activity {
    if !state.requests.is_empty() {
        return Activity::Asking;
    }
    if state
        .sessions
        .iter()
        .any(|session| session.activity == Work::Working)
    {
        return Activity::Busy;
    }
    let fleet = overview::all_fleet_counts(state);
    if fleet.with_limits > 0 && fleet.ready_with_limits == 0 {
        Activity::Asleep
    } else {
        Activity::Rest
    }
}

pub(super) fn mood(state: &State, near: bool) -> Mood {
    let providers: Vec<_> = state
        .providers
        .iter()
        .filter(|provider| !provider.is_empty())
        .collect();
    let seen = overview::accounts(&providers);
    let with_limits: Vec<_> = seen
        .iter()
        .filter(|seen| !seen.provider.limits.is_empty())
        .collect();
    let tired = if with_limits.is_empty() {
        0.0
    } else {
        let sum: f32 = with_limits
            .iter()
            .map(|seen| {
                let peak = seen
                    .provider
                    .limits
                    .iter()
                    .map(|limit| limit.used)
                    .fold(0.0, f64::max);
                ((peak - TIRES_FROM) / (100.0 - TIRES_FROM)).clamp(0.0, 1.0) as f32
            })
            .sum();
        let any_out = with_limits
            .iter()
            .any(|seen| seen.provider.severity() == super::state::Severity::Exhausted);
        let avg = sum / with_limits.len() as f32;
        if any_out { avg.max(0.25) } else { avg }
    };
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

pub(super) fn overline(sources: &[Source]) -> &'static str {
    let has_claude = sources.contains(&Source::Claude);
    let has_omp = sources.contains(&Source::Omp);
    match (has_claude, has_omp) {
        (true, true) => "AI · Claude Code & omp",
        (true, false) => "AI · Claude Code",
        (false, true) => "AI · omp",
        (false, false) => "AI",
    }
}

pub(super) fn title(sources: &[Source], state: &State, tab: Tab) -> String {
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
        Activity::Asleep | Activity::Rest if tab == Tab::Usage => {
            let fleet = overview::fleet_counts(sources, state);
            match fleet.total {
                0 if sources.is_empty() => "Sources disabled".to_owned(),
                0 if state.refreshing => "Fetching quotas…".to_owned(),
                0 if !state.issues.is_empty() => "Refresh failed".to_owned(),
                0 => "No quota data".to_owned(),
                1 if fleet.ready == 0 => "Limit reached".to_owned(),
                1 if fleet.hot > 0 => "1 pool pacing hot".to_owned(),
                1 => "Within quota".to_owned(),
                _ if fleet.ready == 0 => "All limits exhausted".to_owned(),
                total if fleet.ready == total && fleet.hot > 0 => {
                    format!("{total} ready · {} hot", fleet.hot)
                }
                total if fleet.ready == total => format!("{total} accounts ready"),
                total => format!("{} of {total} accounts ready", fleet.ready),
            }
        }
        Activity::Asleep if state.sessions.is_empty() => {
            let fleet = overview::fleet_counts(sources, state);
            if fleet.total > 1 {
                "All limits exhausted".to_owned()
            } else {
                "Limit reached".to_owned()
            }
        }
        Activity::Asleep | Activity::Rest => match state.sessions.len() {
            0 => "No sessions".to_owned(),
            1 => "1 session".to_owned(),
            count => format!("{count} sessions"),
        },
    }
}

pub(super) fn lead(ai: &Ai, state: &State, view: &View, p: &PanelCx<'_>) -> Option<HeaderLead> {
    let pose = view.pose()?;
    Some(HeaderLead {
        mark: mascot::mark(pose),
        size: LARGE,
        overline: overline(&ai.settings.sources).into(),
        title: title(&ai.settings.sources, state, view.tab).into(),
        on_click: Some(p.on_local::<Ai>(|_, view| view.poke())),
    })
}
