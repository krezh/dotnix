use super::{
    format,
    state::{Freshness, Limit, Provider, Severity},
    view::Account,
};
use crate::{
    components::card,
    features::PanelCx,
    panel::{Block, Slider, Text, TextStyle, Tone},
    theme::{OVERLAY2, SUBTEXT1, YELLOW},
};

fn body(text: impl Into<gpui::SharedString>) -> Block {
    Block::Text(Text::new(Tone::Body, text))
}

fn quota(account_id: &str, limit: &Limit, freshness: Freshness, now: i64) -> Block {
    let color = if freshness.is_fresh() {
        format::quota_color(limit)
    } else {
        OVERLAY2
    };
    let mut label = Text::new(Tone::Body, limit.label.clone());
    label.spans.push((
        0..limit.label.len(),
        TextStyle {
            bold: true,
            ..TextStyle::default()
        },
    ));
    let mut content = vec![
        Block::Text(label),
        Block::caption(
            format!(
                "{} used · {} remaining",
                format::percent(limit.used),
                format::percent(limit.remaining)
            ),
            color,
        ),
        Block::Slider(Slider {
            id: format!("{account_id}-quota-{}", limit.id).into(),
            value: (limit.used / 100.0).clamp(0.0, 1.0) as f32,
            color,
            signal: None,
            on_change: None,
        }),
    ];
    if limit.severity != Severity::Ok {
        content.push(Block::caption(
            if freshness.is_fresh() {
                format::status(limit.severity).to_owned()
            } else {
                format!("Last reported: {}", format::status(limit.severity))
            },
            color,
        ));
    }
    let reset = format::reset(limit.resets_at, freshness, now);
    content.push(body(match limit.resets_at {
        Some(at) => format!("{reset} · {}", format::exact(at)),
        None => reset,
    }));
    if let Some(at) = limit.exhausts_at.filter(|_| freshness.is_fresh()) {
        content.push(body(if at <= now {
            format!("Projected exhaustion due now · {}", format::exact(at))
        } else {
            format!(
                "Projected exhaustion in {} · {}",
                format::countdown(at, now),
                format::exact(at)
            )
        }));
    }
    Block::Card(None, card::Size::Medium, content)
}

pub(super) fn blocks(provider: &Provider, _p: &PanelCx<'_>, now: i64) -> Vec<Block> {
    let size = if provider.limits.len() > 1 {
        card::Size::Wide
    } else {
        card::Size::Medium
    };
    let plan = if provider.plan.is_empty() {
        "Plan not reported"
    } else {
        &provider.plan
    };
    let account = if provider.account.is_empty() {
        "Account identity not reported"
    } else {
        &provider.account
    };
    let mut identity = vec![
        Block::caption(format!("{} · {plan}", provider.source.name()), SUBTEXT1),
        body(account.to_owned()),
        Block::caption(
            if provider.fetched_at > 0 {
                format!(
                    "Last successful refresh: {} · {}",
                    format::ago(provider.fetched_at, now),
                    format::exact(provider.fetched_at),
                )
            } else {
                "Last successful refresh not reported".to_owned()
            },
            OVERLAY2,
        ),
    ];
    identity.push(Block::caption(
        format::freshness(provider.freshness),
        if provider.freshness.is_fresh() { SUBTEXT1 } else { YELLOW },
    ));
    if !provider.freshness.is_fresh() {
        identity.push(body(
            "Quotas are from the last successful report and are not verified.",
        ));
    }
    if let Some(credits) = &provider.credits {
        let expiry = match credits.expires_at {
            Some(at) if at <= now => format!("expired · {}", format::exact(at)),
            Some(at) => format!(
                "expire in {} · {}",
                format::countdown(at, now),
                format::exact(at)
            ),
            None => "expiry not reported".to_owned(),
        };
        identity.push(body(format!(
            "Saved resets: {} credit{} · {expiry}",
            credits.count,
            if credits.count == 1 { "" } else { "s" },
        )));
    }
    let mut blocks = vec![Block::Card(None, size, identity)];
    if provider.limits.is_empty() {
        blocks.push(Block::Card(
            None,
            size,
            vec![body("This account did not report any quota limits.")],
        ));
    } else {
        let account_id = Account::of(provider).element_id();
        for limits in provider.limits.chunks(2) {
            blocks.push(Block::Row(
                limits
                    .iter()
                    .map(|limit| quota(&account_id, limit, provider.freshness, now))
                    .collect(),
            ));
        }
    }
    blocks
}
