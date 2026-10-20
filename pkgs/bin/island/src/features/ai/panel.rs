use super::{
    Ai, detail, format, overview,
    state::{Severity, State},
    view::View,
    widget,
};
use crate::{
    features::PanelCx,
    island::{Mode, Panel},
    panel::{Block, Button, Header, IconButton, PanelView, on},
};

pub(super) fn panel(ai: &Ai, state: &State, p: &PanelCx<'_>) -> Option<PanelView> {
    let view = p.local::<View>()?;
    let selected = view.selected(state);
    let mut buttons = Vec::with_capacity(2);
    if selected.is_some() {
        buttons.push(Button {
            id: "ai-back".into(),
            label: "Back".into(),
            active: false,
            toggle: false,
            fits: Vec::new(),
            busy: false,
            enabled: true,
            on_click: p.on_local::<Ai>(|_, view| view.back()),
        });
    }
    buttons.push(Button {
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
    });
    let now = format::now();
    let (title, blocks) = match selected {
        Some(provider) => {
            let mut blocks = detail::blocks(provider, p, now);
            for issue in state.issues.iter().filter(|issue| {
                issue.source == provider.source
                    && (issue.account.is_empty() || issue.account == provider.account)
            }) {
                blocks.push(Block::note(issue.message.clone()));
            }
            (provider.name.clone(), blocks)
        }
        None => (
            "AI usage".to_owned(),
            overview::blocks(ai, state, view, p, now),
        ),
    };
    Some(PanelView {
        header: Header::Title(title.into(), buttons),
        blocks,
    })
}

pub(super) fn system_button(ai: &Ai, state: &State, p: &PanelCx<'_>) -> IconButton {
    let worst = state.worst();
    let badge = if ai.settings.widget.show_badge {
        worst
            .map(|(provider, limit)| {
                format::badge(limit, provider.freshness, format::now(), true).into()
            })
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
