use std::borrow::Cow;

use gpui::SharedString;

use super::{
    Ai, format, harness,
    state::{Provider, Severity, Source, State},
    view::{Account, View},
};
use crate::{
    components::{
        badge::Mark,
        item::{Item, Lead},
        tile::{Part, Tile, Volume},
    },
    features::PanelCx,
    panel::Block,
    theme::{OVERLAY2, RED, SUBTEXT1, TEAL, YELLOW},
};

fn compact(text: &str, max: usize) -> Cow<'_, str> {
    let Some((end, _)) = text.char_indices().nth(max) else {
        return Cow::Borrowed(text);
    };
    Cow::Owned(format!("{}…", &text[..end]))
}

fn identity(provider: &Provider) -> String {
    if provider.plan.is_empty() {
        "Plan not reported".to_owned()
    } else {
        provider.plan.clone()
    }
}

fn footer(provider: &Provider, now: i64) -> Vec<Part> {
    let mut cues: Vec<(Option<u32>, SharedString)> = Vec::new();
    cues.push((
        if provider.freshness.is_fresh() { None } else { Some(YELLOW) },
        format::freshness(provider.freshness).into(),
    ));
    let exhausted = provider
        .limits
        .iter()
        .filter(|limit| limit.severity == Severity::Exhausted)
        .count();
    if exhausted > 0 && provider.freshness.is_fresh() {
        cues.push((
            Some(RED),
            if exhausted == 1 {
                "Quota exhausted".into()
            } else {
                format!("{exhausted} limits exhausted").into()
            },
        ));
    } else if provider.freshness.is_fresh() {
        if let Some(at) = provider
            .limits
            .iter()
            .filter_map(|limit| limit.exhausts_at)
            .min()
        {
            cues.push((
                Some(YELLOW),
                format!("Limit in {}", format::countdown(at, now)).into(),
            ));
        } else if provider.severity() == Severity::High {
            cues.push((Some(YELLOW), "High usage".into()));
        }
    }
    if let Some(credits) = provider
        .credits
        .as_ref()
        .filter(|credits| credits.count > 0)
    {
        let expired = credits.expires_at.is_some_and(|at| at <= now);
        cues.push((
            Some(if expired { YELLOW } else { TEAL }),
            format!(
                "{} reset{}{}",
                credits.count,
                if credits.count == 1 { "" } else { "s" },
                if expired { " expired" } else { "" },
            )
            .into(),
        ));
    }
    let mut parts = Vec::new();
    let mut cues = cues.into_iter().peekable();
    while cues.peek().is_some() {
        parts.push(Part::Legend(cues.by_ref().take(2).collect()));
    }
    parts
}

fn tile(provider: &Provider, providers: &[&Provider], p: &PanelCx<'_>, now: i64) -> Tile {
    let account = Account::of(provider);
    let mut parts = vec![Part::Legend(vec![(None, identity(provider).into())])];
    let same_provider =
        |other: &&Provider| other.source == provider.source && other.name == provider.name;
    if providers.iter().copied().filter(same_provider).count() > 1 {
        let label = if provider.account.is_empty() {
            provider.id.as_str()
        } else {
            provider.account.as_str()
        };
        let ordinal = providers
            .iter()
            .copied()
            .filter(same_provider)
            .position(|other| other.id == provider.id)
            .unwrap_or(0)
            + 1;
        parts.push(Part::Legend(vec![(
            None,
            format!("{ordinal} · {}", compact(label, 28)).into(),
        )]));
    }
    if provider.limits.is_empty() {
        parts.push(Part::Legend(vec![(
            None,
            "No quota limits reported".into(),
        )]));
        parts.push(Part::Spacer);
    } else {
        parts.push(Part::Volumes(
            provider
                .limits
                .iter()
                .map(|limit| {
                    let label = compact(&limit.label, 16);
                    let mount = format!(
                        "{label} · {}",
                        format::reset(limit.resets_at, provider.freshness, now),
                    );
                    Volume {
                        mount: mount.into(),
                        usage: format::percent(limit.used).into(),
                        fraction: (limit.used / 100.0).clamp(0.0, 1.0) as f32,
                        color: if provider.freshness.is_fresh() {
                            format::quota_color(limit)
                        } else {
                            OVERLAY2
                        },
                        id: limit.id.clone().into(),
                    }
                })
                .collect(),
        ));
    }
    parts.extend(footer(provider, now));
    Tile {
        id: account.element_id().into(),
        span: 1,
        title: Some((provider.name.clone().into(), None)),
        parts,
        on_click: Some(p.on_local::<Ai>(move |_, view| view.select(account.clone()))),
    }
}

fn harness_heading(source: Source) -> Block {
    Block::Item(
        Item {
            id: format!("ai-harness-{}", source.name()).into(),
            lead: Some(Lead {
                mark: Some(Mark::Glyph(harness::glyph(source).to_path_buf(), SUBTEXT1)),
                tint: None,
                glow: None,
                faded: false,
                size: Some(28.0),
                on_click: None,
            }),
            overline: None,
            title: source.name().into(),
            title_color: Some(SUBTEXT1),
            detail: "".into(),
            trail: Vec::new(),
        },
        Vec::new(),
    )
}

pub(super) fn blocks(ai: &Ai, state: &State, view: &View, p: &PanelCx<'_>, now: i64) -> Vec<Block> {
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
    for &source in &ai.settings.sources {
        let mut accounts = providers
            .iter()
            .copied()
            .filter(|provider| provider.source == source)
            .peekable();
        if accounts.peek().is_none() {
            continue;
        }
        blocks.push(harness_heading(source));
        let mut issues = state
            .issues
            .iter()
            .filter(|issue| issue.source == source)
            .peekable();
        let mut count = 0;
        let mut refreshed = i64::MAX;
        let mut stale = false;
        while accounts.peek().is_some() {
            let row = accounts
                .by_ref()
                .take(3)
                .map(|provider| {
                    count += 1;
                    refreshed = refreshed.min(provider.fetched_at);
                    stale |= !provider.freshness.is_fresh();
                    vec![tile(provider, &providers, p, now)]
                })
                .collect();
            blocks.push(Block::Board(row));
        }
        let status = if state.refreshing {
            format!(
                "Refreshing… · Oldest successful refresh {}",
                format::ago(refreshed, now)
            )
        } else if stale {
            format!("Oldest successful refresh {}", format::ago(refreshed, now))
        } else {
            format!("Updated {}", format::ago(refreshed, now))
        };
        blocks.push(Block::caption(
            format!(
                "{count} account{} · {status}",
                if count == 1 { "" } else { "s" }
            ),
            SUBTEXT1,
        ));
        if issues.peek().is_some() {
            blocks.push(Block::caption("Refresh issues", YELLOW));
            for issue in issues {
                let message = if issue.account.is_empty() {
                    issue.message.clone()
                } else {
                    format!("{}: {}", issue.account, issue.message)
                };
                blocks.push(Block::note(message));
            }
        }
    }
    let issues = state.issues.iter().filter(|issue| {
        !providers
            .iter()
            .any(|provider| provider.source == issue.source)
    });
    let mut issues = issues.peekable();
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
