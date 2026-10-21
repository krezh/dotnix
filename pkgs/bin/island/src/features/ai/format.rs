use chrono::{Local, TimeZone, Timelike, Utc};

use super::state::{Freshness, Limit, Provider, Severity, Source};
use crate::theme::{BLUE, CRITICAL, GREEN, MAUVE, YELLOW};

pub(super) fn now() -> i64 {
    Utc::now().timestamp_millis()
}

pub(super) fn percent(value: f64) -> String {
    if (0.0..1.0).contains(&value) && value > 0.0 {
        format!("{value:.1}%")
    } else {
        format!("{value:.0}%")
    }
}

/// How much of the limit's window has passed, from 0 to 1, when it says how long its window is.
pub(super) fn elapsed(limit: &Limit, now: i64) -> Option<f32> {
    let (reset, duration) = (limit.resets_at?, limit.duration_ms?);
    if duration <= 0 {
        return None;
    }
    let start = reset - duration;
    Some(((now - start) as f64 / duration as f64).clamp(0.0, 1.0) as f32)
}

pub(super) fn countdown(at: i64, now: i64) -> String {
    let minutes = at.saturating_sub(now).max(0).saturating_add(59_999) / 60_000;
    match minutes {
        0 => "due now".to_owned(),
        1..60 => format!("{minutes}m"),
        60..1440 => format!("{}h {:02}m", minutes / 60, minutes % 60),
        _ => format!("{}d {}h", minutes / 1440, minutes % 1440 / 60),
    }
}

/// When something is due, as you would say it: soon as a countdown, such as "in 1h 27m"; later as
/// a day and a rounded local hour, such as "tomorrow 09:00" or "Fri 15:00".
pub(super) fn when(at: i64, now: i64) -> String {
    const SOON_MS: i64 = 6 * 60 * 60 * 1000;
    const HALF_HOUR_MS: i64 = 30 * 60 * 1000;
    if at - now < SOON_MS {
        return format!("in {}", countdown(at, now));
    }
    let local = |time: i64| Local.timestamp_millis_opt(time).single();
    let (Some(due), Some(today)) = (local(at + HALF_HOUR_MS), local(now)) else {
        return format!("in {}", countdown(at, now));
    };
    let day = match (due.date_naive() - today.date_naive()).num_days() {
        0 => "today".to_owned(),
        1 => "tomorrow".to_owned(),
        _ => due.format("%a").to_string(),
    };
    format!("{day} {:02}:00", due.hour())
}

pub(super) fn ago(at: i64, now: i64) -> String {
    if at <= 0 {
        return "not refreshed yet".to_owned();
    }
    let minutes = now.saturating_sub(at).max(0) / 60_000;
    match minutes {
        0 => "just now".to_owned(),
        1..60 => format!("{minutes}m ago"),
        60..1440 => format!("{}h ago", minutes / 60),
        _ => format!("{}d ago", minutes / 1440),
    }
}

pub(super) fn color(severity: Severity) -> u32 {
    match severity {
        Severity::Ok => GREEN,
        Severity::High => YELLOW,
        Severity::Exhausted => CRITICAL,
    }
}

pub(super) fn quota_color(limit: &Limit) -> u32 {
    match (limit.severity, limit.duration_ms) {
        (Severity::Exhausted, _) => CRITICAL,
        (Severity::High, _) => YELLOW,
        (Severity::Ok, Some(duration)) if duration > 86_400_000 => MAUVE,
        _ => BLUE,
    }
}

pub(super) fn status(severity: Severity) -> &'static str {
    match severity {
        Severity::Ok => "Within quota",
        Severity::High => "High usage",
        Severity::Exhausted => "Quota exhausted",
    }
}

pub(super) fn glyph(provider: &Provider) -> &'static str {
    let named = |needle: &str| {
        provider
            .name
            .as_bytes()
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
    };
    if provider.source == Source::Claude || named("claude") || named("anthropic") {
        "ai-claude"
    } else if named("openai") || named("codex") || named("chatgpt") {
        "ai-openai"
    } else if named("antigravity") {
        "ai-antigravity"
    } else if named("google") || named("gemini") {
        "ai-google"
    } else if named("github") || named("copilot") {
        "ai-github"
    } else {
        "ai"
    }
}

pub(super) fn freshness(freshness: Freshness) -> &'static str {
    match freshness {
        Freshness::Cached => "Cached · pending verification",
        Freshness::Fresh => "Fresh",
        Freshness::Stale => "Stale · refresh failed",
    }
}

pub(super) fn reset(at: Option<i64>, freshness: Freshness, now: i64) -> String {
    match at {
        Some(at) if at <= now && !freshness.is_fresh() => {
            "Reset passed · refresh required".to_owned()
        }
        Some(at) if at <= now => "Reset due now".to_owned(),
        Some(at) if freshness.is_fresh() => format!("Resets in {}", countdown(at, now)),
        Some(at) => format!("Recorded reset in {} · unverified", countdown(at, now)),
        None => "Reset time not reported".to_owned(),
    }
}

pub(super) fn badge(limit: &Limit, freshness: Freshness, now: i64, countdown: bool) -> String {
    if !freshness.is_fresh() {
        return format!("{}?", percent(limit.used));
    }
    if countdown {
        if let Some(at) = limit.resets_at {
            return self::countdown(at, now);
        }
    }
    percent(limit.used)
}
