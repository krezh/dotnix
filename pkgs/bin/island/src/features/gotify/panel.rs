use std::time::Duration;

use chrono::{Local, TimeZone};
use gpui::SharedString;

use super::{
    runtime::{Gotify, Request},
    state::{Entry, State},
};
use crate::{
    components::card::{self, FrameView, Look as FrameLook},
    features::PanelCx,
    markdown,
    panel::{Block, Button, Header, Item, Lead, Mark, PanelView, Text, TextStyle, Tone, Trail},
    theme::{ACCENT, CRITICAL, PEACH, RED},
};

const REFRESH: &str = "Refresh";
const REFRESHING: &str = "Refreshing…";
const BODY_LINES: usize = 6;
const ICON_SIZE: f32 = 28.0;

fn age_label(timestamp: Option<i64>, now: i64) -> String {
    let Some(timestamp) = timestamp else {
        return String::new();
    };
    let minutes = (now - timestamp).max(0) / 60;
    match minutes {
        0 => "now".to_owned(),
        1..60 => format!("{minutes}m"),
        60..1440 => format!("{}h", minutes / 60),
        _ => Local
            .timestamp_opt(timestamp, 0)
            .single()
            .map(|dt| dt.format("%b %-d").to_string())
            .unwrap_or_default(),
    }
}

fn next_age_change(timestamp: Option<i64>, now: i64) -> Option<Duration> {
    let elapsed = u64::try_from((now - timestamp?).max(0)).ok()?;
    let step = match elapsed {
        ..3600 => 60,
        ..86_400 => 3600,
        _ => return None,
    };
    Some(Duration::from_secs(step - (elapsed % step)))
}

fn body_text(raw: &str) -> Text {
    let parsed = markdown::parse(raw);
    let links = parsed.link_ranges().into_iter().zip(parsed.links).collect();
    let spans = parsed
        .spans
        .into_iter()
        .map(|(range, style)| {
            let style = TextStyle {
                bold: style.bold,
                italic: style.italic,
                underline: style.underline,
                strike: style.strike,
                code: style.code,
                covered: style.spoiler,
                link: style.link.is_some(),
            };
            (range, style)
        })
        .collect();
    Text {
        spans,
        links,
        lines: Some(BODY_LINES),
        ..Text::new(Tone::Body, parsed.text)
    }
}

fn message_card(entry: &Entry, now: i64, p: &PanelCx<'_>) -> Block {
    if let Some(after) = next_age_change(entry.timestamp, now) {
        p.redraw_in(after);
    }
    let id = entry.id;
    let frame_id: SharedString = format!("gotify-msg-{id}").into();
    let age = age_label(entry.timestamp, now);
    let overline = if age.is_empty() {
        entry.app.clone()
    } else {
        format!("{} · {age}", entry.app)
    };
    let tint = if entry.critical() { CRITICAL } else { ACCENT };
    let item = Item {
        id: format!("gotify-item-{id}").into(),
        lead: Some(Lead {
            mark: Some(Mark::Letter(entry.app.clone().into(), tint)),
            tint: None,
            glow: None,
            faded: false,
            size: Some(ICON_SIZE),
            on_click: None,
            top: false,
        }),
        overline: Some(overline.into()),
        title: markdown::parse(&entry.title).text.into(),
        title_color: entry.critical().then_some(CRITICAL),
        wrap: false,
        detail: SharedString::default(),
        trail: vec![Trail::Close {
            group: frame_id.clone(),
            on_click: p.call::<Gotify>(move |gotify| gotify.request(Request::Delete(id))),
        }],
    };
    let under = if entry.body.trim().is_empty() {
        Vec::new()
    } else {
        vec![Block::Text(body_text(&entry.body))]
    };
    Block::Frame(
        FrameView {
            id: frame_id,
            look: FrameLook::Raised {
                urgent: entry.critical(),
            },
            on_click: None,
            on_dismiss: Some(p.call::<Gotify>(move |gotify| gotify.request(Request::Delete(id)))),
            countdown: None,
            selected: false,
            motion: card::Motion::Still,
            shown_height: None,
        },
        vec![Block::Item(item, under)],
    )
}

pub fn panel(state: &State, p: &PanelCx<'_>) -> PanelView {
    if !state.configured() {
        return PanelView {
            header: Header::Title("Gotify".into(), Vec::new()),
            blocks: vec![Block::Card(
                None,
                card::Size::Large,
                vec![Block::note("Gotify is not configured.")],
            )],
        };
    }

    let now = chrono::Utc::now().timestamp();
    let buttons = vec![
        Button {
            id: "gotify-refresh".into(),
            label: if state.busy { REFRESHING } else { REFRESH }.into(),
            active: state.busy,
            toggle: false,
            fits: vec![REFRESH.into(), REFRESHING.into()],
            busy: state.busy,
            enabled: !state.busy,
            on_click: p.call::<Gotify>(|gotify| gotify.request(Request::Refresh)),
        },
        Button {
            id: "gotify-clear".into(),
            label: "Clear".into(),
            active: false,
            toggle: false,
            fits: Vec::new(),
            busy: false,
            enabled: !state.messages.is_empty(),
            on_click: p.call::<Gotify>(|gotify| gotify.request(Request::Clear)),
        },
    ];

    let mut blocks = Vec::new();
    if let Some(error) = &state.error {
        blocks.push(Block::caption(
            format!("Disconnected: {error}"),
            if state.connected { PEACH } else { RED },
        ));
    }
    let list = if state.messages.is_empty() {
        Block::Card(None, card::Size::Large, vec![Block::note("No messages")])
    } else {
        let rows = state
            .messages
            .iter()
            .map(|entry| message_card(entry, now, p))
            .collect();
        Block::Card(None, card::Size::Large, rows)
    };
    blocks.push(list);
    let title = match state.messages.len() {
        0 => "Gotify".to_owned(),
        1 => "Gotify · 1 message".to_owned(),
        count => format!("Gotify · {count} messages"),
    };
    PanelView {
        header: Header::Title(title.into(), buttons),
        blocks,
    }
}
