//! Molt's Home, as molt-core words it: three figures over a card for each tool, each tool opening
//! over Home.

use std::time::SystemTime;

use molt_core::{
    build::{Target, flow::AfterBuild},
    home::{
        flow::Msg,
        present::{self, Action, ActionId, Card, CardKind, Figure, Tone},
    },
};

use super::{
    format::IslandUnits,
    parts::{frame, tone},
    runtime::Molt,
    state::Updates,
    view::{Screen, View},
};
use crate::{
    features::PanelCx,
    panel::{
        Block, ButtonRow, Chip, HeaderLead, IconButton, Item, Lead, Mark, RowLead, Tint, Views,
    },
    theme::{ACCENT, BLUE, CRITICAL, GREEN, MAUVE, PEACH, PINK, SAPPHIRE, SUBTEXT1, TEAL, YELLOW},
    ui::{HEADING_SIZE, TITLE_SIZE},
};

const LEAD: f32 = 36.0;
/// A tool's card, as tall as molt's so their buttons line up.
const CARD_H: f32 = 142.0;
/// One whose status sits over its buttons.
const WRAPPED_CARD_H: f32 = 151.0;

/// Its glyph's key and its color.
fn look(kind: CardKind) -> (&'static str, u32) {
    match kind {
        CardKind::Target(Target::NixOs) => ("molt-nixos", BLUE),
        CardKind::Target(Target::HomeManager) => ("molt-home-manager", GREEN),
        CardKind::Repository => ("molt-repository", PINK),
        CardKind::Inputs => ("molt-inputs", MAUVE),
        CardKind::Search => ("molt-search", SAPPHIRE),
        CardKind::Cleanup => ("molt-cleanup", PEACH),
        CardKind::Watchlist => ("molt-watchlist", TEAL),
    }
}

fn button(action: &Action, p: &PanelCx<'_>) -> IconButton {
    let on_click = match action.id {
        ActionId::Check => p.call(|molt: &Molt| molt.home(Msg::Check)),
        ActionId::Pull => p.call(|molt: &Molt| molt.home(Msg::Pull)),
        ActionId::Inputs => p.on_local::<Molt>(|molt, view: &mut View| {
            view.open(Screen::Inputs);
            molt.open_inputs();
        }),
        ActionId::Watchlist => p.on_local::<Molt>(|molt, view: &mut View| {
            view.open(Screen::Watchlist);
            molt.open_watchlist();
        }),
        ActionId::Search => p.on_local::<Molt>(|molt, view: &mut View| {
            view.open(Screen::Search);
            molt.open_search();
        }),
        ActionId::Cleanup => p.on_local::<Molt>(|molt, view: &mut View| {
            view.open(Screen::Cleanup);
            molt.open_cleanup();
        }),
        ActionId::Build(target) => p.on_local::<Molt>(move |molt, view: &mut View| {
            view.open(Screen::Build(target));
            molt.start_build(target, AfterBuild::Review);
        }),
        ActionId::BuildAndSwitch(target) => p.on_local::<Molt>(move |molt, view: &mut View| {
            view.open(Screen::Build(target));
            molt.start_build(target, AfterBuild::Switch);
        }),
        ActionId::Review(target) => p.on_local::<Molt>(move |molt, view: &mut View| {
            molt.seen();
            view.open(Screen::Build(target));
        }),
    };
    let color = if action.primary {
        Some(ACCENT)
    } else if action.failed {
        Some(CRITICAL)
    } else {
        None
    };
    IconButton {
        glyph: None,
        color,
        label: action.label.into(),
        active: action.primary,
        badge: None,
        enabled: action.enabled,
        busy: action.busy,
        on_click,
    }
}

/// A status beside three or more buttons does not fit one line of a card, so it goes over them.
fn wraps(card: &Card) -> bool {
    card.status.is_some() && card.actions.len() >= 3
}

fn card(card: &Card, height: f32, p: &PanelCx<'_>) -> Block {
    let (glyph, color) = look(card.kind);
    let item = Item {
        id: format!("molt-home-{:?}", card.kind).into(),
        lead: Some(Lead {
            mark: p.glyph(glyph).map(|path| Mark::Glyph(path, color)),
            tint: Some(color),
            top: true,
            ..Default::default()
        }),
        title: card.title.into(),
        title_size: Some(TITLE_SIZE),
        ..Default::default()
    };
    let lead = card.status.as_ref().map(|status| match status.tone {
        Tone::Muted => RowLead::Note(status.text.clone().into()),
        _ => RowLead::Chip(Chip {
            busy: status.busy,
            ..Chip::new(status.text.clone(), tone(status.tone))
        }),
    });
    Block::Frame(
        frame(format!("molt-home-card-{:?}", card.kind), Some(height)),
        vec![
            Block::Item(item, vec![Block::caption(card.detail.clone(), SUBTEXT1)]),
            Block::Buttons(ButtonRow {
                lead,
                buttons: card.actions.iter().map(|action| button(action, p)).collect(),
                compact: false,
                indent: 0.0,
                menu: None,
                end: true,
                fill: false,
            }),
        ],
    )
}

fn figure(figure: &Figure) -> Block {
    Block::Item(
        Item {
            id: format!("molt-home-figure-{}", figure.label).into(),
            overline: Some(figure.label.into()),
            title: figure.value.clone().into(),
            title_size: Some(HEADING_SIZE),
            detail: figure.detail.clone().into(),
            tinted: figure
                .chip
                .iter()
                .map(|(text, _)| Tint::Title(text.clone().into(), YELLOW))
                .collect(),
            ..Default::default()
        },
        Vec::new(),
    )
}

pub(super) fn home(state: &Updates, p: &PanelCx<'_>) -> (Views, Vec<Block>) {
    let Some(home) = &state.home else {
        return (
            super::parts::views(None, None, p),
            vec![Block::note("Reading Molt…")],
        );
    };
    let view = present::view(home, SystemTime::now(), &IslandUnits);
    let lead = HeaderLead {
        mark: p
            .glyph("molt")
            .map_or(Mark::Letter("M".into(), ACCENT), |path| {
                Mark::Glyph(path, ACCENT)
            }),
        size: LEAD,
        overline: view.subtitle.clone().into(),
        title: view.title.into(),
        title_color: None,
        on_click: None,
    };
    let mut blocks = Vec::new();
    if let Some(error) = &view.error {
        blocks.push(Block::note(error.clone()));
    } else {
        blocks.push(Block::Frame(
            frame("molt-home-figures".to_owned(), None),
            vec![Block::Divided(view.figures.iter().map(figure).collect())],
        ));
        let cards = view
            .cards
            .chunks(2)
            .flat_map(|pair| {
                let height = if pair.iter().any(wraps) {
                    WRAPPED_CARD_H
                } else {
                    CARD_H
                };
                pair.iter().map(move |each| card(each, height, p))
            })
            .collect();
        blocks.push(Block::Grid(2, cards));
    }
    (super::parts::views(Some(lead), None, p), blocks)
}
