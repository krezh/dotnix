use std::fmt::Write as _;

use super::{
    Ai, format, logos, mascot,
    session::Activity,
    settings::Display,
    state::{Provider, Severity, State},
    view::View,
};
use crate::{
    bar_widget::{BarWidget, Part},
    features::{Surface, WidgetCx},
    theme::{ACCENT, CRITICAL, OVERLAY2, SUBTEXT1, YELLOW},
};

/// Jelly in the bar: a little larger than a glyph, as its face needs the room.
const JELLY: f32 = 22.0;

pub(super) fn tooltip(ai: &Ai, state: &State) -> String {
    if ai.settings.sources.is_empty() {
        return "AI usage · disabled".to_owned();
    }
    let now = format::now();
    let mut text = String::with_capacity(ai.settings.sources.len() * 96);
    for &source in &ai.settings.sources {
        for provider in state
            .providers
            .iter()
            .filter(|provider| provider.source == source && !provider.is_empty())
        {
            if !text.is_empty() {
                text.push('\n');
            }
            let _ = write!(text, "{} · {}", source.name(), provider.name);
            if !provider.account.is_empty() {
                let _ = write!(text, " · {}", provider.account);
            }
            let _ = write!(
                text,
                " · {} · Last success: {}",
                format::freshness(provider.freshness),
                format::ago(provider.fetched_at, now),
            );
            if provider.freshness.is_fresh() {
                if provider.limits.is_empty() {
                    text.push_str(" · saved resets");
                } else {
                    let _ = write!(text, " · {}", format::status(provider.severity()));
                }
            } else {
                for limit in &provider.limits {
                    let _ = write!(
                        text,
                        "\n  {} · {} last reported",
                        limit.label,
                        format::percent(limit.used),
                    );
                    if limit.resets_at.is_some_and(|at| at <= now) {
                        let _ = write!(
                            text,
                            " · {}",
                            format::reset(limit.resets_at, provider.freshness, now),
                        );
                    }
                }
            }
        }
        for issue in state.issues.iter().filter(|issue| issue.source == source) {
            if !text.is_empty() {
                text.push('\n');
            }
            let _ = write!(text, "{} · {}", source.name(), issue.message);
        }
    }
    if text.is_empty() {
        text.push_str(if state.refreshing {
            "AI usage · refreshing…"
        } else if !state.issues.is_empty() {
            "AI usage · unavailable"
        } else if state.attempted_at.is_none() {
            "AI usage · not refreshed yet"
        } else {
            "AI usage · no usage data"
        });
    }
    text
}

fn account_icon(provider: &Provider, tint: u32, w: &WidgetCx<'_>) -> Part {
    let key = format::glyph(provider);
    let glyph = if key == "ai" {
        None
    } else {
        w.glyph(key).or_else(|| logos::glyph(key))
    };
    match glyph {
        Some(path) => Part::glyph(path, tint),
        None => Part::plain(provider.name.chars().next().unwrap_or('A').to_string()),
    }
}

/// How many Claude Code requests wait on you, as a yellow count on a mark's corner.
fn waiting(icon: Part, state: &State) -> Part {
    let count = state.requests.len();
    Part::Badged {
        id: "bar-ai-waiting",
        part: Box::new(icon),
        badge: (count > 0).then(|| (count.to_string().into(), YELLOW)),
    }
}

fn provider_badge(provider: &Provider, now: i64, state: Option<&State>, w: &WidgetCx<'_>) -> Part {
    let tint = if !provider.freshness.is_fresh() {
        OVERLAY2
    } else {
        format::color(provider.severity())
    };
    let icon = account_icon(provider, tint, w);
    let mut parts = vec![match state {
        Some(state) => waiting(icon, state),
        None => icon,
    }];
    parts.extend(provider.headline().map(|limit| {
        Part::plain(format::badge(
            limit,
            provider.freshness,
            now,
            limit.severity == Severity::Exhausted,
        ))
    }));
    Part::Cluster { color: tint, parts }
}

/// One mark for usage and Claude Code alike: a session at work runs the moving border, and
/// requests waiting on you show as a count on the mark.
pub(super) fn widget(ai: &Ai, state: &State, w: &WidgetCx<'_>) -> BarWidget {
    let now = format::now();
    if let Some(after) = format::usage_next_change(&ai.settings.sources, state, now) {
        w.redraw_in(after);
    }
    let working = state
        .sessions
        .iter()
        .any(|session| session.activity == Activity::Working);
    let mut base = w.compact("bar-ai").tip(tooltip(ai, state));
    let mut providers = state
        .providers
        .iter()
        .filter(|provider| !provider.is_empty())
        .peekable();
    // Jelly leads the widget when it is on, and carries the count of requests waiting.
    let jelly = w
        .local::<View>()
        .and_then(View::pose)
        .map(|pose| waiting(Part::Drawing(mascot::drawing(&pose, JELLY)), state));
    let counted = jelly.is_none();
    base = base.parts(jelly);
    if ai.settings.widget.display == Display::Providers && providers.peek().is_some() {
        base = base.parts(providers.enumerate().map(|(index, provider)| {
            provider_badge(provider, now, (counted && index == 0).then_some(state), w)
        }));
    } else {
        let worst = state.worst();
        let tint = worst.map_or_else(
            || {
                if !state.issues.is_empty() {
                    CRITICAL
                } else if state.refreshing {
                    ACCENT
                } else {
                    SUBTEXT1
                }
            },
            |(provider, limit)| {
                if provider.freshness.is_fresh() {
                    format::color(limit.severity)
                } else {
                    OVERLAY2
                }
            },
        );
        // Jelly stands in for the account's icon.
        if counted {
            let selected_icon = match worst {
                Some((provider, _)) => account_icon(provider, tint, w),
                None => match w.glyph("ai") {
                    Some(path) => Part::glyph(path, tint),
                    None => Part::plain("AI"),
                },
            };
            base = base.part(waiting(selected_icon, state));
        }
        base = base.color(tint);
        if ai.settings.widget.show_badge {
            if let Some((provider, limit)) = worst {
                base = base.part(Part::plain(format::badge(
                    limit,
                    provider.freshness,
                    now,
                    true,
                )));
            } else if counted && !state.issues.is_empty() {
                // Jelly wears the warning itself.
                base = base.part(Part::plain("!"));
            }
        }
    }
    // Jelly shows work and a refresh itself, so the energy border is only for the widget without it.
    if counted {
        base = base.part(Part::Busy(state.refreshing || working));
    }
    base.on_click(w.toggle(Ai::panel_handle()))
        .on_right(w.call(Ai::refresh))
}
