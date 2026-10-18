use std::fmt::Write as _;

use gpui::{AnyElement, FontWeight, MouseButton, div, prelude::*, px, rgb};

use super::{
    Ai, format, logos,
    settings::Display,
    state::{Provider, Severity, State},
};
use crate::{
    bar::{GLYPH, ICON_GAP},
    components::{icon, tooltip::Tip},
    features::WidgetCx,
    island::Panel,
    theme::{ACCENT, CRITICAL, OVERLAY2, SUBTEXT1},
    ui::SMALL_SIZE,
};

pub(super) fn tooltip(ai: &Ai, state: &State) -> String {
    if ai.settings.sources.is_empty() {
        return "AI usage · disabled".to_owned();
    }
    let mut text = String::with_capacity(ai.settings.sources.len() * 64);
    for &source in &ai.settings.sources {
        let mut count = 0;
        let mut severity = Severity::Ok;
        let mut has_limits = false;
        let mut stale = false;
        for provider in state
            .providers
            .iter()
            .filter(|provider| provider.source == source && !provider.is_empty())
        {
            count += 1;
            severity = severity.max(provider.severity());
            has_limits |= !provider.limits.is_empty();
            stale |= provider.stale;
        }
        if count == 0 {
            continue;
        }
        let has_issues = state.issues.iter().any(|issue| issue.source == source);
        let status = match severity {
            Severity::Exhausted => "quota exhausted",
            Severity::High => "high usage",
            Severity::Ok if has_limits => "within quota",
            Severity::Ok => "saved resets",
        };
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(source.name());
        if count > 1 {
            let _ = write!(text, " · {count} accounts");
        }
        text.push_str(" · ");
        text.push_str(status);
        if stale {
            text.push_str(" · stale");
        } else if has_issues {
            text.push_str(" · source issue");
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

fn provider_badge(provider: &Provider, now: i64, w: &WidgetCx<'_>) -> AnyElement {
    let tint = if provider.stale {
        OVERLAY2
    } else {
        format::color(provider.severity())
    };
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(ICON_GAP))
        .text_color(rgb(tint))
        .child(account_icon(provider, tint, w))
        .when_some(provider.headline(), |badge, limit| {
            badge.child(format::badge(
                limit,
                now,
                limit.severity == Severity::Exhausted,
            ))
        })
        .into_any_element()
}

pub(super) fn widget(ai: &Ai, state: &State, w: &WidgetCx<'_>) -> AnyElement {
    let now = format::now();
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
        base = base.children(providers.map(|provider| provider_badge(provider, now, w)));
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
            |(_, limit)| format::color(limit.severity),
        );
        let selected_icon = match worst {
            Some((provider, _)) => account_icon(provider, tint, w),
            None => match w.glyph("ai") {
                Some(path) => icon::glyph(path, GLYPH, tint).into_any_element(),
                None => div().child("AI").into_any_element(),
            },
        };
        base = base.text_color(rgb(tint)).child(selected_icon);
        if ai.settings.widget.show_badge {
            if let Some((_, limit)) = worst {
                base = base.child(format::badge(limit, now, true));
            } else if !state.issues.is_empty() {
                base = base.child("!");
            }
        }
    }
    base.child(w.energy(state.refreshing))
        .on_click(w.toggle(Panel::Feature("ai")))
        .on_mouse_down(MouseButton::Right, w.call(Ai::refresh))
        .into_any_element()
}
