use std::time::Duration;

use super::{
    Ai, asking, chat, format, jelly, overview, sessions,
    state::{Severity, State},
    view::{Tab, View},
    widget,
};
use crate::{
    features::PanelCx,
    island::{Mode, Panel},
    panel::{Block, Button, Header, IconButton, PanelView, TabLabel, Views, on},
};

fn usage(ai: &Ai, state: &State, view: &View, p: &PanelCx<'_>) -> (Vec<Button>, Vec<Block>) {
    let buttons = vec![Button {
        id: "ai-refresh".into(),
        label: if state.refreshing {
            "Refreshing…"
        } else {
            "Refresh"
        }
        .into(),
        active: state.refreshing,
        toggle: false,
        fits: vec!["Refresh".into(), "Refreshing…".into()],
        busy: state.refreshing,
        enabled: !ai.settings.sources.is_empty() && !state.refreshing,
        on_click: p.call(Ai::refresh),
    }];
    let now = format::now();
    if let Some(after) = format::usage_next_change(&ai.settings.sources, state, now) {
        p.redraw_in(after);
    }
    (buttons, overview::blocks(ai, state, view, now))
}

pub(super) fn panel(ai: &Ai, state: &State, p: &PanelCx<'_>) -> Option<PanelView> {
    let view = p.local::<View>()?;
    let (buttons, blocks) = match view.tab {
        Tab::Sessions => {
            let now = format::now();
            if let Some(ms) = state
                .sessions
                .iter()
                .flat_map(|session| {
                    [
                        format::next_elapsed_ms(session.active_at, now),
                        format::next_elapsed_ms(session.started_at, now),
                    ]
                })
                .min()
            {
                p.redraw_in(Duration::from_millis(ms));
            }
            (Vec::new(), sessions::blocks(state, view, p))
        }
        Tab::Chat => chat::panel(ai, state, view, p),
        Tab::Usage => usage(ai, state, view, p),
    };
    Some(PanelView {
        header: Header::Views(Views {
            lead: jelly::lead(state, view, p),
            tabs: Tab::ALL
                .iter()
                .map(|tab| TabLabel {
                    label: tab.label().into(),
                    glyph: p.glyph(tab.glyph()),
                })
                .collect(),
            selected: Tab::ALL
                .iter()
                .position(|tab| *tab == view.tab)
                .unwrap_or(0),
            on_select: p.on_local_with::<Ai, usize>(|_, view, index| {
                view.tab = Tab::ALL[index.min(Tab::ALL.len() - 1)];
            }),
            buttons,
        }),
        blocks,
    })
}

/// The requests waiting on you, oldest on top, each until it is answered or runs out of time.
pub(super) fn popup(state: &State, p: &PanelCx<'_>) -> Option<PanelView> {
    let view = p.local::<View>()?;
    if state.requests.is_empty() && view.leaving.is_empty() {
        return None;
    }
    let now = format::now();
    if let Some(ms) = state
        .requests
        .iter()
        .chain(view.leaving.iter().map(|(request, _)| request))
        .filter_map(|request| format::next_ago_ms(request.received_at, now))
        .min()
    {
        p.redraw_in(Duration::from_millis(ms));
    }
    let waiting = state
        .requests
        .iter()
        .map(|request| asking::frame(request, view, p, Some(state.hold), false));
    let leaving = view
        .leaving
        .iter()
        .map(|(request, _)| asking::frame(request, view, p, None, true));
    Some(PanelView {
        header: Header::None,
        blocks: waiting.chain(leaving).collect(),
    })
}

pub(super) fn system_button(ai: &Ai, state: &State, p: &PanelCx<'_>) -> IconButton {
    let now = format::now();
    if let Some(after) = format::usage_next_change(&ai.settings.sources, state, now) {
        p.redraw_in(after);
    }
    let worst = state.worst();
    let badge = if ai.settings.widget.show_badge {
        worst
            .map(|(provider, limit)| format::badge(limit, provider.freshness, now, true).into())
            .or_else(|| (!state.issues.is_empty()).then(|| "!".into()))
    } else {
        None
    };
    IconButton {
        id: "open-ai".into(),
        glyph: p.glyph("ai"),
        rising: None,
        label: widget::tooltip(ai, state).into(),
        active: worst.is_some_and(|(_, limit)| limit.severity != Severity::Ok)
            || !state.issues.is_empty(),
        badge,
        on_click: on(|island, cx| island.set_mode(Mode::Open(Panel::Feature("ai")), cx)),
    }
}
