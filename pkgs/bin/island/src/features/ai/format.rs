use chrono::{Local, TimeZone, Utc};

use super::state::{Limit, Provider, Severity, Source};
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

pub(super) fn countdown(at: i64, now: i64) -> String {
    let minutes = at.saturating_sub(now).max(0).saturating_add(59_999) / 60_000;
    match minutes {
        0 => "due now".to_owned(),
        1..60 => format!("{minutes}m"),
        60..1440 => format!("{}h {:02}m", minutes / 60, minutes % 60),
        _ => format!("{}d {}h", minutes / 1440, minutes % 1440 / 60),
    }
}

pub(super) fn exact(at: i64) -> String {
    Local
        .timestamp_millis_opt(at)
        .single()
        .map(|time| time.format("%a %b %-d, %H:%M %Z").to_string())
        .unwrap_or_else(|| "unknown time".to_owned())
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

pub(super) fn badge(limit: &Limit, now: i64, countdown: bool) -> String {
    if countdown {
        if let Some(at) = limit.resets_at {
            return self::countdown(at, now);
        }
    }
    percent(limit.used)
}
