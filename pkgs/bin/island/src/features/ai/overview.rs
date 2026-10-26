use std::{borrow::Cow, cmp::Ordering};

use gpui::SharedString;

use super::{
    Ai, format,
    state::{Credits, Limit, Provider, Severity, Source, State},
    view::{Account, View},
};
use crate::{
    components::{
        card::{self, Title},
        ring::Ring,
        tile::{Part, Row, Tile},
        timeline::{Lane, Timeline},
    },
    panel::Block,
    theme::{OVERLAY2, RED, SUBTEXT1, TEAL, TEXT, YELLOW},
};

const WEEK_MS: i64 = 7 * 24 * 60 * 60 * 1000;
/// Windows this short reset several times a week, so the timeline gives them one lane together.
const SHORT_MS: i64 = 5 * 60 * 60 * 1000;
/// Limits a tile names under its rings; more fold into "+N more".
const MAX_ROWS: usize = 3;
const MAX_LANES: usize = 6;
const AXIS: [&str; 8] = ["now", "1d", "2d", "3d", "4d", "5d", "6d", "7d"];

fn compact(text: &str, max: usize) -> Cow<'_, str> {
    let Some((end, _)) = text.char_indices().nth(max) else {
        return Cow::Borrowed(text);
    };
    Cow::Owned(format!("{}…", &text[..end]))
}

/// A limit's label without the word every limit of the account starts with, such as "Claude" in
/// "Claude 5 Hour" beside "Claude 7 Day", since the tile already names the account.
fn short_label(label: &str, limits: &[Limit]) -> String {
    let first = |label: &str| {
        label
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_owned()
    };
    let shared = first(label);
    let all_share = limits.len() > 1
        && !shared.is_empty()
        && limits.iter().all(|limit| first(&limit.label) == shared);
    if !all_share {
        return label.to_owned();
    }
    let rest = label[shared.len()..]
        .trim_start_matches(|c: char| c.is_whitespace() || c == '•' || c == '·');
    if rest.is_empty() {
        label.to_owned()
    } else {
        rest.to_owned()
    }
}

/// The resets you have, and how long until they expire, such as "2 resets · 3d 4h left".
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

/// One account however many harnesses report it: the freshest reading of it, and each harness
/// that saw it.
struct Seen<'a> {
    provider: &'a Provider,
    sources: Vec<Source>,
}

/// The same account seen through two harnesses, such as Claude through Claude Code and omp, is
/// one; an account with no address is told apart only by its own id.
fn same_account(one: &Provider, other: &Provider) -> bool {
    if one.account.is_empty() || other.account.is_empty() {
        return one.id == other.id;
    }
    one.provider_id == other.provider_id && one.account == other.account
}

fn fresher(one: &Provider, other: &Provider) -> bool {
    (one.freshness.is_fresh(), one.fetched_at) > (other.freshness.is_fresh(), other.fetched_at)
}

fn accounts<'a>(providers: &[&'a Provider]) -> Vec<Seen<'a>> {
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

/// Columns that leave no row with a lone tile where they can: four go side by side, five and six
/// make two rows of three.
fn columns(count: usize) -> usize {
    match count {
        0..=3 | 5 | 6 => 3,
        _ => 4,
    }
}

/// The limit to look at first: one used up, then the one that runs out soonest, then the most
/// used.
fn urgency(one: &Limit, other: &Limit) -> Ordering {
    let spent = |limit: &Limit| limit.severity == Severity::Exhausted;
    spent(other)
        .cmp(&spent(one))
        .then_with(|| match (one.exhausts_at, other.exhausts_at) {
            (Some(one), Some(other)) => one.cmp(&other),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => other.used.total_cmp(&one.used),
        })
}

/// A limit on course to run out before it resets shows it in yellow.
fn color(limit: &Limit, fresh: bool) -> u32 {
    if !fresh {
        OVERLAY2
    } else if limit.exhausts_at.is_some() && limit.severity != Severity::Exhausted {
        YELLOW
    } else {
        format::quota_color(limit)
    }
}

fn detail(limit: &Limit, now: i64) -> String {
    match (limit.exhausts_at, limit.resets_at) {
        (Some(at), _) => format!("likely out {}", format::when(at, now)),
        (None, Some(at)) => format!("resets in {}", format::countdown(at, now)),
        (None, None) => String::new(),
    }
}

fn rings(provider: &Provider, now: i64) -> Part {
    let fresh = provider.freshness.is_fresh();
    let limits = &provider.limits;
    let long = limits
        .iter()
        .max_by_key(|limit| limit.duration_ms.unwrap_or(0))
        .expect("an account with limits");
    let mut rest: Vec<&Limit> = limits
        .iter()
        .filter(|limit| !std::ptr::eq(*limit, long))
        .collect();
    rest.sort_by(|one, other| urgency(one, other));
    let short = rest.first().copied();
    let tight = std::iter::once(long)
        .chain(short)
        .min_by(|one, other| urgency(one, other))
        .unwrap_or(long);
    let ring = |limit: &Limit| Ring {
        fraction: (limit.used / 100.0).clamp(0.0, 1.0) as f32,
        color: color(limit, fresh),
        mark: fresh.then(|| format::elapsed(limit, now)).flatten(),
        ahead: limit
            .expected
            .filter(|_| fresh)
            .map(|expected| (expected / 100.0).clamp(0.0, 1.0) as f32),
    };
    let listed: Vec<&Limit> = std::iter::once(long).chain(rest).collect();
    let room = if listed.len() > MAX_ROWS {
        MAX_ROWS - 1
    } else {
        MAX_ROWS
    };
    let mut rows: Vec<Row> = listed
        .iter()
        .take(room)
        .map(|limit| Row {
            color: Some(color(limit, fresh)),
            label: compact(&short_label(&limit.label, limits), 14)
                .into_owned()
                .into(),
            value: format::percent(limit.used).into(),
            detail: if fresh {
                detail(limit, now).into()
            } else {
                "".into()
            },
        })
        .collect();
    if listed.len() > room {
        rows.push(Row {
            color: None,
            label: format!("+{} more", listed.len() - room).into(),
            value: "".into(),
            detail: "".into(),
        });
    }
    Part::Rings {
        rings: std::iter::once(long).chain(short).map(ring).collect(),
        value: format::percent(tight.used).into(),
        value_color: if !fresh {
            OVERLAY2
        } else if tight.severity == Severity::Exhausted {
            RED
        } else if tight.exhausts_at.is_some() {
            YELLOW
        } else {
            TEXT
        },
        label: compact(&short_label(&tight.label, limits), 10)
            .into_owned()
            .into(),
        rows,
    }
}

/// What needs saying, then the plan, the account when two share a provider's name, and the
/// harnesses that report it.
fn footer(seen: &Seen<'_>, named_twice: bool, now: i64) -> Part {
    let provider = seen.provider;
    let mut items: Vec<(Option<u32>, SharedString)> = Vec::new();
    if !provider.freshness.is_fresh() {
        items.push((Some(YELLOW), format::freshness(provider.freshness).into()));
    }
    let exhausted = provider
        .limits
        .iter()
        .filter(|limit| limit.severity == Severity::Exhausted)
        .count();
    if exhausted > 0 && provider.freshness.is_fresh() {
        items.push((
            Some(RED),
            if exhausted == 1 {
                "Quota exhausted".into()
            } else {
                format!("{exhausted} limits exhausted").into()
            },
        ));
    }
    if let Some(cue) = provider
        .credits
        .as_ref()
        .and_then(|credits| resets(credits, now))
    {
        items.push(cue);
    }
    let mut about: Vec<String> = Vec::new();
    if !provider.plan.is_empty() {
        about.push(provider.plan.clone());
    }
    if named_twice && !provider.account.is_empty() {
        about.push(compact(&provider.account, 20).into_owned());
    }
    about.extend(seen.sources.iter().map(|source| source.name().to_owned()));
    items.push((None, about.join(" · ").into()));
    Part::Legend(items)
}

fn tile(seen: &Seen<'_>, all: &[Seen<'_>], now: i64) -> Tile {
    let provider = seen.provider;
    let named_twice = all
        .iter()
        .filter(|other| other.provider.name == provider.name)
        .count()
        > 1;
    let mut parts = Vec::new();
    if provider.limits.is_empty() {
        parts.push(Part::Legend(vec![(
            None,
            "No quota limits reported".into(),
        )]));
    } else {
        parts.push(rings(provider, now));
    }
    parts.push(Part::Spacer);
    parts.push(footer(seen, named_twice, now));
    Tile {
        id: Account::of(provider).element_id().into(),
        span: 1,
        title: Some((provider.name.clone().into(), None)),
        parts,
        on_click: None,
    }
}

/// Where in the coming week `at` falls, from now to its end.
fn in_week(at: i64, now: i64) -> f32 {
    ((at - now) as f32 / WEEK_MS as f32).clamp(0.0, 1.0)
}

/// What resets or runs out in the coming week, soonest first, with every short window on one
/// lane; and how many more there are than fit.
fn lanes(seen: &[Seen<'_>], now: i64) -> (Vec<Lane>, usize) {
    let mut long: Vec<(i64, Lane)> = Vec::new();
    let mut short: Vec<(i64, Option<i64>, u32)> = Vec::new();
    for account in seen {
        let provider = account.provider;
        if !provider.freshness.is_fresh() {
            continue;
        }
        for limit in &provider.limits {
            let Some(reset) = limit.resets_at.filter(|reset| *reset > now) else {
                continue;
            };
            let out = limit.exhausts_at.filter(|at| *at > now && *at < reset);
            let soonest = out.unwrap_or(reset);
            if soonest - now > WEEK_MS {
                continue;
            }
            let tint = color(limit, true);
            if limit
                .duration_ms
                .is_some_and(|duration| duration <= SHORT_MS)
            {
                short.push((reset, out, tint));
                continue;
            }
            long.push((
                soonest,
                Lane {
                    key: format!("{}-{}", provider.id, limit.id).into(),
                    label: format!(
                        "{} · {}",
                        provider.name,
                        short_label(&limit.label, &provider.limits)
                    )
                    .into(),
                    to: in_week(reset, now),
                    color: tint,
                    cut: out.map(|at| in_week(at, now)),
                    note: match out {
                        Some(at) => format!("out in {}", format::countdown(at, now)),
                        None => format!("resets in {}", format::countdown(reset, now)),
                    }
                    .into(),
                    note_color: if out.is_some() { YELLOW } else { SUBTEXT1 },
                },
            ));
        }
    }
    long.sort_by_key(|(soonest, _)| *soonest);
    let mut lanes = Vec::new();
    // Short windows share one bar, up to whichever of them resets next.
    if let Some(&(next, _, tint)) = short.iter().min_by_key(|(reset, ..)| *reset) {
        let first_out = short.iter().filter_map(|(_, out, _)| *out).min();
        lanes.push(Lane {
            key: "short-windows".into(),
            label: format!("5-hour windows · {}", short.len()).into(),
            to: in_week(next, now),
            color: tint,
            cut: first_out.filter(|at| *at < next).map(|at| in_week(at, now)),
            note: match first_out {
                Some(at) => format!("out in {}", format::countdown(at, now)),
                None => format!("next in {}", format::countdown(next, now)),
            }
            .into(),
            note_color: if first_out.is_some() {
                YELLOW
            } else {
                SUBTEXT1
            },
        });
    }
    let room = MAX_LANES - lanes.len();
    let hidden = long.len().saturating_sub(room);
    lanes.extend(long.into_iter().take(room).map(|(_, lane)| lane));
    (lanes, hidden)
}

/// Each harness and when it last refreshed, on one line.
fn status(state: &State, ai: &Ai, providers: &[&Provider], now: i64) -> Option<Block> {
    let parts: Vec<String> = ai
        .settings
        .sources
        .iter()
        .filter_map(|&source| {
            let mine: Vec<&&Provider> = providers
                .iter()
                .filter(|provider| provider.source == source)
                .collect();
            let oldest = mine.iter().map(|provider| provider.fetched_at).min()?;
            let stale = mine.iter().any(|provider| !provider.freshness.is_fresh());
            Some(format!(
                "{} {} {}",
                source.name(),
                if stale { "last refreshed" } else { "updated" },
                format::ago(oldest, now)
            ))
        })
        .collect();
    if parts.is_empty() {
        return None;
    }
    let line = parts.join("  ·  ");
    Some(Block::caption(
        if state.refreshing {
            format!("Refreshing…  ·  {line}")
        } else {
            line
        },
        SUBTEXT1,
    ))
}

pub(super) fn blocks(ai: &Ai, state: &State, view: &View, now: i64) -> Vec<Block> {
    let providers = view.ordered(state);
    let mut blocks = Vec::new();
    if ai.settings.sources.is_empty() {
        blocks.push(Block::note("AI usage sources are disabled. Enable omp or Claude Code in the features.ai settings to show quotas."));
    } else if providers.is_empty() {
        blocks.push(Block::note(if state.refreshing {
            "Fetching quota information…"
        } else if state.attempted_at.is_none() {
            "No usage data yet. Select Refresh to fetch quotas."
        } else {
            "No AI usage data available."
        }));
    }
    let seen = accounts(&providers);
    let wide = columns(seen.len());
    let mut tiles = seen
        .iter()
        .map(|account| tile(account, &seen, now))
        .peekable();
    while tiles.peek().is_some() {
        blocks.push(Block::Board(
            tiles.by_ref().take(wide).map(|tile| vec![tile]).collect(),
        ));
    }
    let (lanes, hidden) = lanes(&seen, now);
    if !lanes.is_empty() {
        let mut inside = vec![Block::Timeline(Timeline {
            lanes,
            axis: AXIS
                .iter()
                .map(|label| SharedString::from(*label))
                .collect(),
        })];
        if hidden > 0 {
            inside.push(Block::caption(
                format!("+{hidden} more later this week"),
                SUBTEXT1,
            ));
        }
        blocks.push(Block::Card(
            Some(Title::with_detail(
                "Next 7 days",
                "bars end at the reset · ticks mark running out · striped: without quota",
            )),
            card::Size::Tiles(wide),
            inside,
        ));
    }
    blocks.extend(status(state, ai, &providers, now));
    let mut issues = state.issues.iter().peekable();
    if issues.peek().is_some() {
        blocks.push(Block::caption("Refresh issues", YELLOW));
        for issue in issues {
            let account = if issue.account.is_empty() {
                issue.source.name().to_owned()
            } else {
                format!("{} · {}", issue.source.name(), issue.account)
            };
            blocks.push(Block::note(format!("{account}: {}", issue.message)));
        }
    }
    blocks
}

#[cfg(test)]
#[path = "overview_test.rs"]
mod tests;
