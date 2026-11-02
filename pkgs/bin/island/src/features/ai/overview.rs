use std::{cmp::Ordering, path::Path};

use super::{
    Ai, format, logos,
    state::{Credits, Limit, Provider, Severity, Source, State},
    view::{Account, View},
};
use crate::{
    components::{
        badge::Mark,
        card,
        ledger::{Bar, Columns, Entry, Folded, Ledger},
        timeline::{Pin, Side, Span, Timeline},
    },
    features::PanelCx,
    panel::{Block, SharedString},
    theme::{
        BLUE, GREEN, MAUVE, OVERLAY1, OVERLAY2, PEACH, RED, SUBTEXT1, TEAL, TEXT, YELLOW, app_color,
    },
};
use chrono::{Local, TimeZone, Timelike};

const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const WEEK_MS: i64 = 7 * DAY_MS;
const SOON_MS: i64 = 6 * 60 * 60 * 1000;
const SHORT_MS: i64 = 5 * 60 * 60 * 1000;
const MAX_SWIMLANES: usize = 2;
const SPAN_GAP: f32 = 0.025;
const PIN_CLUSTER_GAP: f32 = 0.22;

pub(super) struct Seen<'a> {
    pub(super) provider: &'a Provider,
    pub(super) sources: Vec<Source>,
}

pub(super) struct FleetCounts {
    pub(super) total: usize,
    pub(super) ready: usize,
    pub(super) with_limits: usize,
    pub(super) ready_with_limits: usize,
    pub(super) hot: usize,
}

fn same_account(one: &Provider, other: &Provider) -> bool {
    if one.account.is_empty() || other.account.is_empty() {
        return one.id == other.id;
    }
    one.provider_id == other.provider_id && one.account == other.account
}

fn fresher(one: &Provider, other: &Provider) -> bool {
    (one.freshness.is_fresh(), one.fetched_at) > (other.freshness.is_fresh(), other.fetched_at)
}

pub(super) fn accounts<'a>(providers: &[&'a Provider]) -> Vec<Seen<'a>> {
    let mut seen: Vec<Seen<'a>> = Vec::new();
    for &provider in providers {
        match seen
            .iter_mut()
            .find(|known| same_account(known.provider, provider))
        {
            Some(known) => {
                if fresher(provider, known.provider) {
                    known.provider = provider;
                }
                if !known.sources.contains(&provider.source) {
                    known.sources.push(provider.source);
                }
            }
            None => seen.push(Seen {
                provider,
                sources: vec![provider.source],
            }),
        }
    }
    seen
}

fn summarize_fleet(seen: &[Seen<'_>]) -> FleetCounts {
    let mut counts = FleetCounts {
        total: seen.len(),
        ready: 0,
        with_limits: 0,
        ready_with_limits: 0,
        hot: 0,
    };
    for item in seen {
        let provider = item.provider;
        let has_limits = !provider.limits.is_empty();
        if has_limits {
            counts.with_limits += 1;
        }
        let exhausted = provider
            .limits
            .iter()
            .any(|limit| limit.severity == Severity::Exhausted);
        if !exhausted {
            counts.ready += 1;
            if has_limits {
                counts.ready_with_limits += 1;
            }
            if provider.freshness.is_fresh()
                && provider
                    .limits
                    .iter()
                    .any(|limit| limit.exhausts_at.is_some())
            {
                counts.hot += 1;
            }
        }
    }
    counts
}

pub(super) fn fleet_counts(sources: &[Source], state: &State) -> FleetCounts {
    let providers: Vec<&Provider> = state
        .providers
        .iter()
        .filter(|provider| !provider.is_empty() && sources.contains(&provider.source))
        .collect();
    summarize_fleet(&accounts(&providers))
}

pub(super) fn all_fleet_counts(state: &State) -> FleetCounts {
    let providers: Vec<&Provider> = state
        .providers
        .iter()
        .filter(|provider| !provider.is_empty())
        .collect();
    summarize_fleet(&accounts(&providers))
}

fn unwrap_leading_parens(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(after_open) = trimmed.strip_prefix('(')
        && let Some(close) = after_open.find(')')
    {
        let inner = after_open[..close].trim();
        let rest = after_open[close + 1..].trim_start();
        if !inner.is_empty() {
            return if rest.is_empty() {
                inner.to_owned()
            } else {
                format!("{inner} {rest}")
            };
        }
    }
    trimmed.to_owned()
}

fn first_word(text: &str) -> &str {
    text.split_whitespace().next().unwrap_or_default()
}

/// Strips a word every limit of the account starts with, then unwraps an orphaned leading `(…)`
/// qualifier such as `"(Argon) · Weekly"` into `"Argon · Weekly"`.
fn short_label(label: &str, limits: &[Limit]) -> String {
    let shared = first_word(label);
    let all_share = limits.len() > 1
        && !shared.is_empty()
        && limits
            .iter()
            .all(|limit| first_word(&limit.label) == shared);
    let stripped = if all_share {
        let rest = label[shared.len()..]
            .trim_start_matches(|c: char| c.is_whitespace() || c == '•' || c == '·');
        if rest.is_empty() { label } else { rest }
    } else {
        label
    };
    unwrap_leading_parens(stripped)
}

fn compact(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_owned(),
    }
}

fn resets(credits: &Credits, now: i64) -> Option<(Option<u32>, SharedString)> {
    if credits.count == 0 {
        return None;
    }
    let count = format!(
        "{} reset{}",
        credits.count,
        if credits.count == 1 { "" } else { "s" }
    );
    Some(match credits.expires_at {
        Some(at) if at <= now => (Some(YELLOW), format!("{count} expired").into()),
        Some(at) => (
            Some(TEAL),
            format!("{count} · {} left", format::countdown(at, now)).into(),
        ),
        None => (Some(TEAL), count.into()),
    })
}

fn credit_chip(credits: &Credits, now: i64) -> Option<(SharedString, u32)> {
    if credits.count == 0 {
        return None;
    }
    let noun = if credits.count == 1 {
        "reset"
    } else {
        "resets"
    };
    Some(match credits.expires_at {
        Some(at) if at <= now => (format!("{} {noun} exp", credits.count).into(), YELLOW),
        _ => (format!("{} {noun}", credits.count).into(), GREEN),
    })
}

fn pool_order(one: &Limit, other: &Limit) -> Ordering {
    let spent = |limit: &Limit| limit.severity == Severity::Exhausted;
    spent(other)
        .cmp(&spent(one))
        .then_with(|| match (one.exhausts_at, other.exhausts_at) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        })
        .then_with(|| other.used.total_cmp(&one.used))
        .then_with(|| {
            other
                .duration_ms
                .unwrap_or(0)
                .cmp(&one.duration_ms.unwrap_or(0))
        })
}

fn bar_color(limit: &Limit, fresh: bool) -> u32 {
    if !fresh {
        OVERLAY2
    } else if limit.exhausts_at.is_some() && limit.severity != Severity::Exhausted {
        YELLOW
    } else {
        format::quota_color(limit)
    }
}

fn brand_color(provider: &Provider) -> u32 {
    match format::glyph(provider) {
        "ai-claude" => PEACH,
        "ai-openai" => GREEN,
        "ai-antigravity" | "ai-google" => BLUE,
        "ai-github" => MAUVE,
        _ => app_color(&provider.name),
    }
}

fn short_provider(name: &str) -> &str {
    name.strip_prefix("Google ")
        .or_else(|| name.strip_prefix("OpenAI "))
        .or_else(|| name.strip_prefix("GitHub "))
        .unwrap_or(name)
}

fn weekday_short(at: i64, now: i64) -> String {
    let local = |ms: i64| Local.timestamp_millis_opt(ms).single();
    match (local(at), local(now)) {
        (Some(due), Some(today)) if due.date_naive() == today.date_naive() => "today".to_owned(),
        (Some(due), _) => due.format("%a").to_string(),
        (None, _) => format::countdown(at, now),
    }
}

fn weekday_time(at: i64, now: i64) -> String {
    if at - now < SOON_MS {
        return format!("in {}", format::countdown(at, now));
    }
    let local = |ms: i64| Local.timestamp_millis_opt(ms).single();
    let Some(due) = local(at) else {
        return format!("in {}", format::countdown(at, now));
    };
    let day = match local(now) {
        Some(today) if due.date_naive() == today.date_naive() => "today".to_owned(),
        _ => due.format("%a").to_string(),
    };
    format!("{day} {:02}:{:02}", due.hour(), due.minute())
}

fn weekday_axis(now: i64) -> Vec<SharedString> {
    (0..=7)
        .map(|day| {
            if day == 0 {
                return SharedString::from("Now");
            }
            Local
                .timestamp_millis_opt(now + day * DAY_MS)
                .single()
                .map_or_else(
                    || SharedString::from(format!("{day}d")),
                    |dt| SharedString::from(dt.format("%a").to_string()),
                )
        })
        .collect()
}

fn ceiling_blocker(limit: &Limit, limits: &[Limit]) -> Option<String> {
    if limit.used > 0.0 || limit.severity == Severity::Exhausted {
        return None;
    }
    let my_dur = limit.duration_ms.unwrap_or(0);
    let blocker = limits
        .iter()
        .filter(|other| {
            other.severity == Severity::Exhausted && other.duration_ms.unwrap_or(i64::MAX) > my_dur
        })
        .min_by_key(|other| other.duration_ms.unwrap_or(i64::MAX))?;
    let label = short_label(&blocker.label, limits).to_lowercase();
    let tag = if label.contains('7') || label.contains("day") {
        "blocked by 7-day"
    } else if label.contains("week") {
        "blocked by weekly"
    } else {
        "blocked by limit"
    };
    Some(tag.to_owned())
}

fn pool_note(limit: &Limit, limits: &[Limit], fresh: bool, now: i64) -> (String, u32) {
    if !fresh {
        return ("unverified".to_owned(), YELLOW);
    }
    if limit.severity == Severity::Exhausted {
        let note = match limit.resets_at {
            Some(at) if at <= now => "reset due".to_owned(),
            Some(at) if at - now < SOON_MS => format!("in {}", format::countdown(at, now)),
            Some(at) => format!("{} · {}", weekday_time(at, now), format::countdown(at, now)),
            None => "exhausted".to_owned(),
        };
        return (note, RED);
    }
    if let Some(out) = limit.exhausts_at.filter(|at| *at > now) {
        return (format!("out {}", weekday_time(out, now)), YELLOW);
    }
    if let Some(blocked) = ceiling_blocker(limit, limits) {
        return (blocked, OVERLAY2);
    }
    match limit.resets_at {
        Some(at) if at <= now => ("reset due".to_owned(), SUBTEXT1),
        Some(at) if at - now < DAY_MS => (format!("in {}", format::countdown(at, now)), OVERLAY2),
        Some(at) if at - now <= WEEK_MS => (weekday_time(at, now), OVERLAY2),
        Some(at) => {
            let cd = format::countdown(at, now);
            let clean = cd.strip_suffix(" 0h").unwrap_or(&cd);
            (format!("{clean} left"), OVERLAY2)
        }
        None if limit.used <= 0.0 => ("starts on use".to_owned(), OVERLAY1),
        None => (String::new(), OVERLAY2),
    }
}

fn is_idle_pool(limit: &Limit, limits: &[Limit]) -> bool {
    limit.used <= 0.0
        && limit.severity == Severity::Ok
        && limit.exhausts_at.is_none()
        && ceiling_blocker(limit, limits).is_none()
}

fn build_bar(limit: &Limit, limits: &[Limit], fresh: bool, appear: bool, now: i64) -> Bar {
    let blocked = fresh && ceiling_blocker(limit, limits).is_some();
    let (note, note_color) = pool_note(limit, limits, fresh, now);
    let value_color = if !fresh || blocked {
        OVERLAY2
    } else if limit.severity == Severity::Exhausted {
        RED
    } else if limit.exhausts_at.is_some() || limit.severity == Severity::High {
        YELLOW
    } else if limit.used <= 0.0 {
        OVERLAY2
    } else {
        TEXT
    };
    Bar {
        key: limit.id.clone().into(),
        label: short_label(&limit.label, limits).into(),
        label_color: if blocked || !fresh { OVERLAY2 } else { TEXT },
        fraction: (limit.used / 100.0).clamp(0.0, 1.0) as f32,
        ahead: limit
            .expected
            .filter(|_| fresh && limit.severity != Severity::Exhausted)
            .map(|expected| (expected / 100.0).clamp(0.0, 1.0) as f32),
        mark: fresh.then(|| format::elapsed(limit, now)).flatten(),
        color: bar_color(limit, fresh),
        value: if fresh {
            format::percent(limit.used).into()
        } else {
            format!("{}?", format::percent(limit.used)).into()
        },
        value_color,
        note: note.into(),
        note_color,
        appear,
    }
}

fn subtitle(seen: &Seen<'_>, named_twice: bool) -> String {
    let provider = seen.provider;
    let mut parts: Vec<String> = Vec::new();
    if !provider.freshness.is_fresh() {
        parts.push(
            match provider.freshness {
                super::state::Freshness::Cached => "cached",
                super::state::Freshness::Stale => "stale",
                super::state::Freshness::Fresh => "fresh",
            }
            .to_owned(),
        );
    }
    if !provider.plan.is_empty() {
        let mut chars = provider.plan.chars();
        let plan = match chars.next() {
            Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
            None => String::new(),
        };
        parts.push(plan);
    }
    if named_twice && !provider.account.is_empty() {
        parts.push(compact(&provider.account, 22));
    }
    parts.extend(seen.sources.iter().map(|source| source.name().to_owned()));
    parts.join(" · ")
}

fn ledger_entry(
    seen: &Seen<'_>,
    all: &[Seen<'_>],
    view: &View,
    p: &PanelCx<'_>,
    now: i64,
) -> Entry {
    let provider = seen.provider;
    let fresh = provider.freshness.is_fresh();
    let named_twice = all
        .iter()
        .filter(|other| other.provider.name == provider.name)
        .count()
        > 1;
    let account = Account::of(provider);
    let expanded = view.is_expanded(&account);
    let toggled = view.was_toggled(&account);

    let any_exhausted = provider
        .limits
        .iter()
        .any(|limit| limit.severity == Severity::Exhausted);
    let any_hot = fresh
        && provider
            .limits
            .iter()
            .any(|limit| limit.exhausts_at.is_some() || limit.severity == Severity::High);
    let accent = if !fresh {
        Some(YELLOW)
    } else if any_exhausted {
        Some(RED)
    } else if any_hot {
        Some(YELLOW)
    } else {
        None
    };
    let glyph_color = if !fresh {
        OVERLAY2
    } else if any_exhausted {
        RED
    } else {
        brand_color(provider)
    };

    let key = format::glyph(provider);
    let glyph = if key == "ai" {
        None
    } else {
        p.glyph(key)
            .or_else(|| logos::glyph(key).map(Path::to_path_buf))
    };

    let mut ordered: Vec<&Limit> = provider.limits.iter().collect();
    ordered.sort_by(|a, b| pool_order(a, b));

    let all_idle = fresh
        && ordered.len() > 1
        && ordered
            .iter()
            .all(|limit| is_idle_pool(limit, &provider.limits));

    let idle_tail_count = ordered
        .iter()
        .rev()
        .take_while(|limit| is_idle_pool(limit, &provider.limits))
        .count();
    let fold_idle_tail = !all_idle && fresh && ordered.len() > 3 && idle_tail_count >= 2;

    let (faded, bars, folded, on_toggle) = if ordered.is_empty() {
        let note = provider
            .credits
            .as_ref()
            .and_then(|credits| resets(credits, now))
            .map_or_else(String::new, |(_, text)| text.to_string());
        (
            false,
            Vec::new(),
            Some(Folded {
                label: "Saved reset credits".into(),
                value: "—".into(),
                note: note.into(),
            }),
            None,
        )
    } else if all_idle {
        let toggle_account = account.clone();
        let handler = Some(p.on_local::<Ai>(move |_, view| {
            view.toggle_account(toggle_account.clone());
        }));
        if expanded {
            let bars = ordered
                .iter()
                .map(|limit| build_bar(limit, &provider.limits, fresh, toggled, now))
                .collect();
            (false, bars, None, handler)
        } else {
            (
                true,
                Vec::new(),
                Some(Folded {
                    label: format!("{} pools unused", ordered.len()).into(),
                    value: "0%".into(),
                    note: "idle".into(),
                }),
                handler,
            )
        }
    } else if fold_idle_tail {
        let active_len = ordered.len() - idle_tail_count;
        let toggle_account = account.clone();
        let handler = Some(p.on_local::<Ai>(move |_, view| {
            view.toggle_account(toggle_account.clone());
        }));
        if expanded {
            let bars = ordered
                .iter()
                .enumerate()
                .map(|(idx, limit)| {
                    build_bar(
                        limit,
                        &provider.limits,
                        fresh,
                        toggled && idx >= active_len,
                        now,
                    )
                })
                .collect();
            (
                false,
                bars,
                Some(Folded {
                    label: format!("Hide {idle_tail_count} idle pools").into(),
                    value: "".into(),
                    note: "hide".into(),
                }),
                handler,
            )
        } else {
            let bars = ordered[..active_len]
                .iter()
                .map(|limit| build_bar(limit, &provider.limits, fresh, false, now))
                .collect();
            (
                false,
                bars,
                Some(Folded {
                    label: format!("+{idle_tail_count} idle pools").into(),
                    value: "0%".into(),
                    note: "show".into(),
                }),
                handler,
            )
        }
    } else {
        let bars = ordered
            .iter()
            .map(|limit| build_bar(limit, &provider.limits, fresh, false, now))
            .collect();
        (false, bars, None, None)
    };

    let mark_color = if faded { OVERLAY2 } else { glyph_color };
    let mark = match glyph {
        Some(path) => Mark::Glyph(path, mark_color),
        None => Mark::Letter(provider.name.clone().into(), mark_color),
    };

    let chip = (!ordered.is_empty())
        .then(|| {
            provider
                .credits
                .as_ref()
                .and_then(|credits| credit_chip(credits, now))
        })
        .flatten();

    Entry {
        key: account.element_id().into(),
        mark,
        tint: (!faded).then_some(glyph_color),
        accent,
        faded,
        title: provider.name.clone().into(),
        subtitle: subtitle(seen, named_twice).into(),
        chip,
        bars,
        folded,
        expanded,
        on_toggle,
    }
}

fn in_week(at: i64, now: i64) -> f32 {
    ((at - now) as f32 / WEEK_MS as f32).clamp(0.0, 1.0)
}

fn pack_outages(mut spans: Vec<Span>) -> (Vec<Vec<Span>>, usize) {
    let total = spans.len();
    spans.sort_by(|a, b| a.from.total_cmp(&b.from).then(a.to.total_cmp(&b.to)));
    let mut lanes: Vec<Vec<Span>> = Vec::new();
    let mut ends: Vec<f32> = Vec::new();
    for span in spans {
        if let Some((idx, end)) = ends
            .iter_mut()
            .enumerate()
            .find(|(_, end)| **end + SPAN_GAP <= span.from)
        {
            *end = span.to;
            lanes[idx].push(span);
        } else if lanes.len() < MAX_SWIMLANES {
            ends.push(span.to);
            lanes.push(vec![span]);
        }
    }
    (lanes, total)
}

struct ResetCandidate {
    key: String,
    provider: String,
    short_provider: String,
    pool: String,
    reset: i64,
    at: f32,
    color: u32,
}

fn approx_pin_width(label: &str) -> f32 {
    22.0 + label.chars().count() as f32 * 5.0
}

fn cluster_pins(mut items: Vec<ResetCandidate>, now: i64) -> (Vec<Pin>, usize) {
    let total = items.len();
    if items.is_empty() {
        return (Vec::new(), 0);
    }
    items.sort_by_key(|item| item.reset);
    let mut groups: Vec<Vec<ResetCandidate>> = Vec::new();
    for item in items {
        match groups.last_mut() {
            Some(group) if item.at - group[0].at < PIN_CLUSTER_GAP => group.push(item),
            _ => groups.push(vec![item]),
        }
    }

    let pins = groups
        .into_iter()
        .map(|group| {
            if group.len() == 1 {
                let item = &group[0];
                let label = format!(
                    "{} · {}",
                    item.short_provider,
                    weekday_time(item.reset, now)
                );
                let tooltip = format!(
                    "{} · {} — resets {} (in {})",
                    item.provider,
                    item.pool,
                    weekday_time(item.reset, now),
                    format::countdown(item.reset, now)
                );
                let width = approx_pin_width(&label);
                Pin {
                    key: item.key.clone().into(),
                    at: item.at,
                    label: label.into(),
                    tooltip: tooltip.into(),
                    color: item.color,
                    width,
                }
            } else {
                let count = group.len();
                let first = &group[0];
                let avg_at = group.iter().map(|item| item.at).sum::<f32>() / count as f32;
                let day = weekday_short(first.reset, now);
                let label = format!("+{count} resets · {day}");
                let tooltip = group
                    .iter()
                    .map(|item| {
                        format!(
                            "{} · {} — {}",
                            item.provider,
                            item.pool,
                            weekday_time(item.reset, now)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let width = approx_pin_width(&label);
                Pin {
                    key: format!("cluster-{}", first.key).into(),
                    at: avg_at,
                    label: label.into(),
                    tooltip: tooltip.into(),
                    color: MAUVE,
                    width,
                }
            }
        })
        .collect();

    (pins, total)
}

fn horizon(seen: &[Seen<'_>], now: i64) -> Option<Timeline> {
    let mut outages: Vec<Span> = Vec::new();
    let mut resets: Vec<ResetCandidate> = Vec::new();

    for account in seen {
        let provider = account.provider;
        if !provider.freshness.is_fresh() {
            continue;
        }
        let short_prov = short_provider(&provider.name);
        for limit in &provider.limits {
            let pool = short_label(&limit.label, &provider.limits);
            if limit.severity == Severity::Exhausted {
                let Some(reset) = limit.resets_at.filter(|at| *at > now) else {
                    continue;
                };
                let resets_in_week = reset - now <= WEEK_MS;
                let when_text = if resets_in_week {
                    weekday_time(reset, now)
                } else {
                    format::countdown(reset, now)
                };
                let label = if provider.limits.len() > 2 {
                    format!("{short_prov} {pool} · {when_text}")
                } else {
                    format!("{short_prov} · {when_text}")
                };
                let tooltip = format!(
                    "{} · {pool} — quota exhausted, unlocks {} (in {})",
                    provider.name,
                    weekday_time(reset, now),
                    format::countdown(reset, now)
                );
                outages.push(Span {
                    key: format!("{}-{}-out", provider.id, limit.id).into(),
                    label: label.into(),
                    tooltip: tooltip.into(),
                    from: 0.0,
                    to: in_week(reset, now),
                    color: RED,
                    start_tick: false,
                    end_dot: resets_in_week,
                });
                continue;
            }

            if let Some(out) = limit
                .exhausts_at
                .filter(|at| *at > now && *at - now < WEEK_MS)
            {
                let reset = limit
                    .resets_at
                    .filter(|at| *at > out)
                    .unwrap_or(now + WEEK_MS);
                let resets_in_week = reset - now <= WEEK_MS;
                let span_days = if resets_in_week {
                    format!("{}–{}", weekday_short(out, now), weekday_short(reset, now))
                } else {
                    format!("out {}", weekday_short(out, now))
                };
                let label = if provider.limits.len() > 1 {
                    format!("{short_prov} {pool} · {span_days}")
                } else {
                    format!("{short_prov} · {span_days}")
                };
                let tooltip = format!(
                    "{} · {pool} — projected out {} (resets {})",
                    provider.name,
                    weekday_time(out, now),
                    weekday_time(reset, now)
                );
                outages.push(Span {
                    key: format!("{}-{}-proj", provider.id, limit.id).into(),
                    label: label.into(),
                    tooltip: tooltip.into(),
                    from: in_week(out, now),
                    to: in_week(reset, now),
                    color: YELLOW,
                    start_tick: true,
                    end_dot: resets_in_week,
                });
                continue;
            }

            let is_long = limit.duration_ms.is_none_or(|dur| dur > SHORT_MS);
            if is_long
                && limit.used > 0.0
                && let Some(reset) = limit
                    .resets_at
                    .filter(|at| *at > now && *at - now <= WEEK_MS)
            {
                resets.push(ResetCandidate {
                    key: format!("{}-{}-pin", provider.id, limit.id),
                    provider: provider.name.clone(),
                    short_provider: short_prov.to_owned(),
                    pool,
                    reset,
                    at: in_week(reset, now),
                    color: brand_color(provider),
                });
            }
        }
    }

    if outages.is_empty() && resets.is_empty() {
        return None;
    }

    let (lanes, outage_count) = pack_outages(outages);
    let (pins, reset_count) = cluster_pins(resets, now);

    let mut summary = Vec::new();
    if outage_count > 0 {
        summary.push(format!(
            "{outage_count} {}",
            if outage_count == 1 {
                "outage"
            } else {
                "outages"
            }
        ));
    }
    if reset_count > 0 {
        summary.push(format!(
            "{reset_count} {}",
            if reset_count == 1 { "reset" } else { "resets" }
        ));
    }

    Some(Timeline {
        side: Some(Side {
            title: "Week horizon".into(),
            detail: summary.join(" · ").into(),
            striped_key: (outage_count > 0).then(|| "no quota".into()),
            dot_key: Some("reset".into()),
        }),
        lanes,
        pins,
        axis: weekday_axis(now),
    })
}

fn entry_heading(
    ai: &Ai,
    state: &State,
    providers: &[&Provider],
    count: usize,
    now: i64,
) -> String {
    let noun = if count == 1 { "account" } else { "accounts" };
    let mut parts = vec![format!("{count} {noun}")];
    if state.refreshing {
        parts.push("refreshing…".to_owned());
        return parts.join(" · ");
    }
    let mut ages: Vec<(Source, i64, bool)> = Vec::new();
    for &source in &ai.settings.sources {
        let mine: Vec<&&Provider> = providers
            .iter()
            .filter(|provider| provider.source == source)
            .collect();
        if let Some(oldest) = mine.iter().map(|provider| provider.fetched_at).min() {
            let stale = mine.iter().any(|provider| !provider.freshness.is_fresh());
            ages.push((source, oldest, stale));
        }
    }
    if ages.iter().any(|(_, _, stale)| *stale) {
        parts.push("stale".to_owned());
    } else if let Some(oldest) = ages.iter().map(|(_, oldest, _)| *oldest).min() {
        parts.push(format!("updated {}", format::ago(oldest, now)));
    }
    parts.join(" · ")
}

pub(super) fn blocks(ai: &Ai, state: &State, view: &View, p: &PanelCx<'_>, now: i64) -> Vec<Block> {
    let providers = view.ordered(state);
    let mut blocks = Vec::new();
    if ai.settings.sources.is_empty() {
        blocks.push(Block::Card(
            None,
            card::Size::Full,
            vec![Block::note(
                "AI usage sources are disabled. Enable omp or Claude Code in features.ai settings to show quotas.",
            )],
        ));
        return blocks;
    }
    if providers.is_empty() {
        let message = if state.refreshing {
            "Fetching quota information…"
        } else if state.attempted_at.is_none() {
            "No usage data yet. Select Refresh to fetch quotas."
        } else {
            "No AI usage data available."
        };
        blocks.push(Block::Card(
            None,
            card::Size::Full,
            vec![Block::note(message)],
        ));
    } else {
        let seen = accounts(&providers);
        let heading = entry_heading(ai, state, &providers, seen.len(), now);
        let entries = seen
            .iter()
            .map(|account| ledger_entry(account, &seen, view, p, now))
            .collect();
        blocks.push(Block::Card(
            None,
            card::Size::Full,
            vec![Block::Ledger(Ledger {
                columns: Some(Columns {
                    entry: heading.into(),
                    label: "Pool".into(),
                    bar: "Pace".into(),
                    mark_key: Some("elapsed".into()),
                    ahead_key: Some("forecast".into()),
                    value: "Used".into(),
                    note: "Reset / forecast".into(),
                }),
                chevron: p.glyph("ai-chevron"),
                entries,
            })],
        ));
        if let Some(week) = horizon(&seen, now) {
            blocks.push(Block::Card(
                None,
                card::Size::Full,
                vec![Block::Timeline(week)],
            ));
        }
    }

    if !state.issues.is_empty() {
        let mut issue_blocks = vec![Block::caption("Refresh issues", YELLOW)];
        for issue in state.issues.iter() {
            let account = if issue.account.is_empty() {
                issue.source.name().to_owned()
            } else {
                format!("{} · {}", issue.source.name(), issue.account)
            };
            issue_blocks.push(Block::note(format!("{account}: {}", issue.message)));
        }
        blocks.push(Block::Card(None, card::Size::Full, issue_blocks));
    }
    blocks
}

#[cfg(test)]
#[path = "overview_test.rs"]
mod tests;
