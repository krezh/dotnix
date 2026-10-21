use std::fmt::Write as _;

use gpui::{AnyElement, FontWeight, MouseButton, div, prelude::*, px, rgb};

use super::{
    Ai, format, logos,
    session::Activity,
    settings::Display,
    state::{Provider, Severity, State},
};
use crate::{
    bar::{GLYPH, ICON_GAP},
    components::{
        button::{self, Badge},
        icon,
        tooltip::Tip,
    },
    features::WidgetCx,
    island::Panel,
    theme::{ACCENT, CRITICAL, OVERLAY2, SUBTEXT1, YELLOW},
    ui::SMALL_SIZE,
};

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

fn account_icon(provider: &Provider, tint: u32, w: &WidgetCx<'_>) -> AnyElement {
    let key = format::glyph(provider);
    let glyph = if key == "ai" {
        None
    } else {
        w.glyph(key).or_else(|| logos::glyph(key))
    };
    match glyph {
        Some(path) => icon::glyph(path, GLYPH, tint).into_any_element(),
        None => div()
            .child(provider.name.chars().next().unwrap_or('A').to_string())
            .into_any_element(),
    }
}

/// How many Claude Code requests wait on you, as a yellow count on a mark's corner.
fn waiting(icon: AnyElement, state: &State) -> AnyElement {
    let count = state.requests.len();
    let badge = (count > 0).then(|| Badge {
        text: count.to_string().into(),
        color: YELLOW,
    });
    button::badged("bar-ai-waiting", icon, badge, false).into_any_element()
}

fn provider_badge(
    provider: &Provider,
    now: i64,
    state: Option<&State>,
    w: &WidgetCx<'_>,
) -> AnyElement {
    let tint = if !provider.freshness.is_fresh() {
        OVERLAY2
    } else {
        format::color(provider.severity())
    };
    let icon = account_icon(provider, tint, w);
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(ICON_GAP))
        .text_color(rgb(tint))
        .child(match state {
            Some(state) => waiting(icon, state),
            None => icon,
        })
        .when_some(provider.headline(), |badge, limit| {
            badge.child(format::badge(
                limit,
                provider.freshness,
                now,
                limit.severity == Severity::Exhausted,
            ))
        })
        .into_any_element()
}

/// One mark for usage and Claude Code alike: a session at work runs the moving border, and
/// requests waiting on you show as a count on the mark.
pub(super) fn widget(ai: &Ai, state: &State, w: &WidgetCx<'_>) -> AnyElement {
    let now = format::now();
    let working = state
        .sessions
        .iter()
        .any(|session| session.activity == Activity::Working);
    let mut base = w
        .base("bar-ai")
        .gap(px(ICON_GAP))
        .text_size(px(SMALL_SIZE))
        .font_weight(FontWeight::SEMIBOLD)
        .tip(w.tips(), "bar-ai", tooltip(ai, state));
    let mut providers = state
        .providers
        .iter()
        .filter(|provider| !provider.is_empty())
        .peekable();
    if ai.settings.widget.display == Display::Providers && providers.peek().is_some() {
        base = base.children(providers.enumerate().map(|(index, provider)| {
            provider_badge(provider, now, (index == 0).then_some(state), w)
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
        let selected_icon = match worst {
            Some((provider, _)) => account_icon(provider, tint, w),
            None => match w.glyph("ai") {
                Some(path) => icon::glyph(path, GLYPH, tint).into_any_element(),
                None => div().child("AI").into_any_element(),
            },
        };
        base = base
            .text_color(rgb(tint))
            .child(waiting(selected_icon, state));
        if ai.settings.widget.show_badge {
            if let Some((provider, limit)) = worst {
                base = base.child(format::badge(limit, provider.freshness, now, true));
            } else if !state.issues.is_empty() {
                base = base.child("!");
            }
        }
    }
    base.child(w.energy(state.refreshing || working))
        .on_click(w.toggle(Panel::Feature("ai")))
        .on_mouse_down(MouseButton::Right, w.call(Ai::refresh))
        .into_any_element()
}
