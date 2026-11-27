//! Molt's Inputs, as molt-core words it: the inputs to filter and pick on the left, the picked
//! one's lock and remote history on the right, lock changes at the bottom.

use std::{rc::Rc, time::SystemTime};

use molt_core::{
    build::{Target, flow::AfterBuild},
    inputs::{
        flow::Msg,
        present::{self, Action, ActionId, HistoryRow, Place},
    },
};

use super::{
    format::IslandUnits,
    parts::{frame, views},
    runtime::Molt,
    state::Updates,
    view::{Focus, Screen, View},
};
use crate::{
    components::card::{Look, Motion, RAISED_GAP, RAISED_PAD},
    features::PanelCx,
    panel::{
        Block, ButtonRow, Chip, FrameView, HeaderLead, IconButton, InputView, Item, Lead, Mark,
        RowIcon, RowLead, SearchRow, SearchView, Segments, Split, TabLabel, Tint, Trail, Views,
    },
    theme::{ACCENT, BLUE, CRITICAL, LAVENDER, MAUVE, OVERLAY2, SUBTEXT1, TEAL, YELLOW},
    ui::{BLOCK_GAP, CONTROL_H, HEADER_GAP, HEADER_H, HEADING_SIZE, INSET, TITLE_SIZE},
};

const LEAD: f32 = 36.0;
const LIST_W: f32 = 300.0;
const FIELD_H: f32 = 34.0;
const FIELD_GAP: f32 = 10.0;
/// The detail's parts over its history: the name and source, the lock line, the History line.
const DETAIL_HEAD: f32 = 46.0 + 20.0 + 24.0;
const PIN_H: f32 = 40.0;
/// A commit's mark on the history.
const DOT: f32 = 12.0;

/// A tool's header, its lead's two lines beside its mark.
const TOOL_HEADER_H: f32 = HEADER_H + 13.0;
/// The footer's row of buttons.
const FOOTER_H: f32 = CONTROL_H + 8.0;

/// Between the header and the footer, as molt's panes are.
pub(super) fn body_h() -> f32 {
    let (_, height) = Screen::Inputs.size();
    height - 2.0 * INSET - TOOL_HEADER_H - HEADER_GAP - BLOCK_GAP - FOOTER_H
}

fn button(action: &Action, p: &PanelCx<'_>) -> IconButton {
    let msg = match action.id {
        ActionId::TogglePin => Some(Msg::TogglePin),
        ActionId::ApplyPin => Some(Msg::ApplyPin),
        ActionId::UpdateSelected => Some(Msg::UpdateSelected),
        ActionId::UpdateAll => Some(Msg::UpdateAll),
        ActionId::Undo => Some(Msg::Undo),
        ActionId::BuildHomeManager | ActionId::BuildNixOs => None,
    };
    let builds = match action.id {
        ActionId::BuildHomeManager => Some(Target::HomeManager),
        ActionId::BuildNixOs => Some(Target::NixOs),
        _ => None,
    };
    let toggles_pin = action.id == ActionId::TogglePin;
    let on_click = match msg {
        Some(msg) => p.on_local::<Molt>(move |molt, view: &mut View| {
            if toggles_pin {
                view.focus = if view.focus == Focus::Pin {
                    Focus::None
                } else {
                    Focus::Pin
                };
            }
            molt.inputs(msg.clone());
        }),
        None => p.on_local::<Molt>(move |molt, view: &mut View| {
            if let Some(target) = builds {
                view.open(Screen::Build(target));
                molt.start_build(target, AfterBuild::Review);
            }
        }),
    };
    IconButton {
        glyph: None,
        color: action.primary.then_some(ACCENT),
        label: action.label.into(),
        active: action.primary,
        badge: None,
        enabled: action.enabled,
        busy: action.busy,
        on_click,
    }
}

pub(super) fn row(button_list: Vec<IconButton>, lead: Option<RowLead>) -> Block {
    Block::Buttons(ButtonRow {
        lead,
        buttons: button_list,
        compact: false,
        indent: 0.0,
        menu: None,
        end: true,
        fill: false,
    })
}

fn history_row(entry: &HistoryRow, p: &PanelCx<'_>) -> Block {
    let current = entry.place == Place::Current;
    let index = entry.index;
    let mut trail = vec![Trail::Faint(entry.date.clone().into())];
    if current {
        trail.push(Trail::Chip(Chip::new(entry.action, LAVENDER)));
    }
    let item = Item {
        id: format!("molt-inputs-commit-{index}").into(),
        lead: Some(Lead {
            mark: Some(Mark::Choice {
                picked: current,
                many: false,
            }),
            size: Some(DOT),
            top: true,
            ..Default::default()
        }),
        title: entry.revision.clone().into(),
        title_mono: true,
        title_color: Some(MAUVE),
        trail,
        ..Default::default()
    };
    Block::Frame(
        FrameView {
            id: format!("molt-inputs-commit-{index}").into(),
            look: Look::Plain,
            on_click: (!current)
                .then(|| p.call(move |molt: &Molt| molt.inputs(Msg::LockAt(index)))),
            on_dismiss: None,
            countdown: None,
            selected: current,
            motion: Motion::Still,
            shown_height: None,
            min_height: None,
        },
        vec![Block::Item(
            item,
            vec![Block::caption(entry.summary.clone(), SUBTEXT1)],
        )],
    )
}

pub(super) fn inputs(state: &Updates, view: &View, p: &PanelCx<'_>) -> (Views, Vec<Block>) {
    let back = p.on_local::<Molt>(|_, view: &mut View| view.open(Screen::Home));
    let Some(screen) = &state.inputs else {
        return (
            views(None, Some(back), p),
            vec![Block::note("Reading flake.lock…")],
        );
    };
    let shown = present::view(screen, SystemTime::now(), &IslandUnits);
    let mut header = views(
        Some(HeaderLead {
            mark: p
                .glyph("molt-inputs")
                .map_or(Mark::Letter("I".into(), MAUVE), |path| {
                    Mark::Glyph(path, MAUVE)
                }),
            size: LEAD,
            overline: shown.subtitle.clone().into(),
            title: shown.title.into(),
            title_color: None,
            on_click: None,
            on_point: None,
        }),
        Some(back),
        p,
    );
    header.segments = Some(Segments {
        labels: shown
            .sort
            .iter()
            .map(|label| TabLabel {
                label: (*label).into(),
                glyph: None,
                count: None,
            })
            .collect(),
        selected: usize::from(shown.oldest_first),
        on_select: p.on_local_with::<Molt, usize>(|molt, _, index| {
            molt.inputs(Msg::OldestFirst(index == 1));
        }),
    });
    if let Some(error) = &shown.error {
        return (header, vec![Block::note(error.clone())]);
    }
    let body = body_h();
    let list_h = body - 2.0 * RAISED_PAD - FIELD_H - FIELD_GAP;
    let rows = shown
        .rows
        .iter()
        .map(|row| {
            let name = row.name.clone();
            SearchRow {
                name: row.name.clone().into(),
                detail: row.source.clone().into(),
                icon: RowIcon::None,
                mono: false,
                section: None,
                mark: None,
                trail: row
                    .age
                    .iter()
                    .map(|(age, stale)| {
                        if *stale {
                            Trail::Chip(Chip::new(age.clone(), YELLOW))
                        } else {
                            Trail::Text(age.clone().into(), OVERLAY2)
                        }
                    })
                    .collect(),
                selected: row.selected,
                on_click: p.call(move |molt: &Molt| molt.inputs(Msg::Select(name.clone()))),
                on_hover: crate::panel::on(|_| {}),
            }
        })
        .collect();
    let list = Block::Frame(
        frame("molt-inputs-list".to_owned(), Some(body)),
        vec![Block::Search(SearchView {
            glyph: p.glyph("molt-search"),
            query: shown.query.clone().into(),
            placeholder: shown.filter_placeholder.into(),
            empty: "No inputs".into(),
            filters: None,
            rows,
            pictures: false,
            on_scroll: Rc::new(|_, _| {}),
            preview: None,
            fill: true,
            compact: true,
            list_height: Some(list_h),
            status: None,
            list_width: None,
            beside: Vec::new(),
            focused: view.focus == Focus::Filter,
            on_focus: Some(p.on_local::<Molt>(|_, view: &mut View| view.focus = Focus::Filter)),
            table: None,
        })],
    );

    let detail = &shown.detail;
    let mut tinted = Vec::new();
    if let Some(kind) = &detail.kind {
        tinted.push(Tint::Chip(kind.clone().into(), TEAL));
    }
    if let Some(follows) = &detail.follows {
        tinted.push(Tint::Chip(follows.clone().into(), OVERLAY2));
    }
    let name = Item {
        id: "molt-inputs-name".into(),
        title: detail.name.clone().into(),
        title_size: Some(HEADING_SIZE),
        tinted,
        ..Default::default()
    };
    let mut blocks = vec![row(
        vec![button(&detail.pin, p), button(&detail.update, p)],
        Some(RowLead::Item(
            Box::new(name),
            vec![Block::caption(detail.source.clone(), SUBTEXT1)],
        )),
    )];
    if detail.picked {
        let (upstream, newer) = &detail.upstream;
        let (detail_words, tinted) = if *newer {
            (
                detail.age.clone(),
                vec![Tint::Title(upstream.clone().into(), BLUE)],
            )
        } else {
            (format!("{} {upstream}", detail.age), Vec::new())
        };
        blocks.push(Block::Item(
            Item {
                id: "molt-inputs-lock".into(),
                label: Some("Locked".into()),
                title: detail.revision.clone().into(),
                title_mono: true,
                title_color: Some(MAUVE),
                detail: detail_words.into(),
                tinted,
                ..Default::default()
            },
            Vec::new(),
        ));
    }
    let mut used = DETAIL_HEAD;
    if let Some(pin) = &detail.pin_field {
        used += PIN_H;
        let apply = pin.apply.enabled;
        blocks.push(Block::Input(InputView {
            text: pin.text.clone().into(),
            placeholder: pin.placeholder.into(),
            max_lines: 1,
            send_glyph: p.glyph("molt-pin"),
            on_send: apply.then(|| p.call(|molt: &Molt| molt.inputs(Msg::ApplyPin))),
            busy: false,
            secret: false,
            focused: view.focus == Focus::Pin,
            on_focus: Some(p.on_local::<Molt>(|_, view: &mut View| view.focus = Focus::Pin)),
        }));
    }
    let history = &shown.history;
    let (status, warns) = &history.status;
    blocks.push(Block::Item(
        Item {
            id: "molt-inputs-history".into(),
            title: "History".into(),
            title_size: Some(TITLE_SIZE),
            trail: vec![if *warns {
                Trail::Text(status.clone().into(), YELLOW)
            } else {
                Trail::Faint(status.clone().into())
            }],
            ..Default::default()
        },
        Vec::new(),
    ));
    let mut listed = Vec::new();
    if let Some(notice) = &history.notice {
        listed.push(Block::caption(notice.clone(), YELLOW));
    }
    match &history.message {
        Some(message) => listed.push(Block::note(message.clone())),
        None => listed.extend(history.rows.iter().map(|entry| history_row(entry, p))),
    }
    let room = body - 2.0 * RAISED_PAD - used - 3.0 * RAISED_GAP;
    blocks.push(Block::Scrolled(
        "molt-inputs-history-rows".into(),
        room.max(0.0),
        listed,
    ));
    let detail_card = Block::Frame(frame("molt-inputs-detail".to_owned(), Some(body)), blocks);

    let footer_lead = shown.status.as_ref().map(|status| {
        if status.error.is_some() {
            RowLead::Chip(Chip::new(status.text.clone(), CRITICAL))
        } else {
            RowLead::Note(status.text.clone().into())
        }
    });
    let mut actions: Vec<IconButton> = shown
        .transaction
        .iter()
        .map(|action| button(action, p))
        .collect();
    actions.push(button(&shown.update_all, p));
    (
        header,
        vec![
            Block::Columns(
                Split::Lead {
                    width: LIST_W,
                    height: body,
                },
                vec![list, detail_card],
            ),
            row(actions, footer_lead),
        ],
    )
}
