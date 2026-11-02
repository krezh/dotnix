use std::time::Duration;

use super::{
    Ai, asking, chat, format, jelly, overview, sessions,
    state::State,
    view::{Tab, View},
};
use crate::{
    features::PanelCx,
    panel::{Block, Button, Header, PanelView, Passing, TabLabel, Views},
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
        glyph: p.glyph("ai-refresh"),
        bare: true,
        active: state.refreshing,
        toggle: false,
        fits: Vec::new(),
        busy: false,
        enabled: !ai.settings.sources.is_empty() && !state.refreshing,
        on_click: p.call(Ai::refresh),
    }];
    let now = format::now();
    if let Some(after) = format::usage_next_change(&ai.settings.sources, state, now) {
        p.redraw_in(after);
    }
    (buttons, overview::blocks(ai, state, view, p, now))
}

fn tab_view(
    ai: &Ai,
    state: &State,
    view: &View,
    tab: Tab,
    p: &PanelCx<'_>,
) -> (Vec<Button>, Vec<Block>) {
    match tab {
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
    }
}

fn tab_index(tab: Tab) -> usize {
    Tab::ALL.iter().position(|known| *known == tab).unwrap_or(0)
}

pub(super) fn panel(ai: &Ai, state: &State, p: &PanelCx<'_>) -> Option<PanelView> {
    let view = p.local::<View>()?;
    let (buttons, blocks) = tab_view(ai, state, view, view.tab, p);
    let passing = view.passing.from.map(|from| Passing {
        blocks: tab_view(ai, state, view, from, p).1,
        at: view.passing.at.value().clamp(0.0, 1.0),
        rightwards: tab_index(view.tab) > tab_index(from),
        width: view.passing.width.clone(),
    });
    Some(PanelView {
        scroll: view.tab == Tab::Chat,
        header: Header::Views(Views {
            lead: jelly::lead(ai, state, view, p),
            tabs: Tab::ALL
                .iter()
                .map(|tab| TabLabel {
                    label: tab.label().into(),
                    glyph: p.glyph(tab.glyph()),
                    count: None,
                })
                .collect(),
            selected: tab_index(view.tab),
            on_select: p.on_local_with::<Ai, usize>(|_, view, index| {
                view.pick_tab(Tab::ALL[index.min(Tab::ALL.len() - 1)]);
            }),
            pick: None,
            buttons,
            icons: Vec::new(),
            passing,
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
        scroll: false,
        header: Header::None,
        blocks: waiting.chain(leaving).collect(),
    })
}
