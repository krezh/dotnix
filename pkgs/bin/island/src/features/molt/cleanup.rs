//! Molt's Clean up, as molt-core words it: the disks and what cleaning would free on them over the
//! open category's items beside the removal plan, the cleanup armed and run from the bottom.

use std::rc::Rc;

use molt_core::cleanup::{
    CleanupKind,
    flow::{Msg, Phase},
    present::{self, ActionLook, CleanupView, Items, Pick, PlanRow, Retention, Setting, Tone},
};

use super::{
    format::IslandUnits,
    inputs::{body_h, row},
    parts::{frame, hue, views},
    runtime::Molt,
    state::Updates,
    view::{Focus, Screen, View},
};
use crate::{
    components::card::{Look, Motion, RAISED_GAP, RAISED_PAD},
    features::PanelCx,
    panel::{
        Block, Cell, Column, Field, FrameView, HeaderLead, HeaderStatus, IconButton, Item, Lead,
        Mark, SearchView, Split, Stepper, Table, TableRow, Trail, Views, Width,
    },
    theme::{
        ACCENT, CRITICAL, GREEN, OVERLAY0, OVERLAY2, PEACH, SUBTEXT1, TEXT, YELLOW,
    },
    ui::{BLOCK_GAP, DISPLAY_SIZE, FIGURE_SIZE, HEADING_SIZE, TITLE_SIZE},
};

const LEAD: f32 = 36.0;
/// The removal plan, beside the items.
const PLAN_W: f32 = 470.0;
/// The figure to reclaim, beside the disks.
const FIGURE_W: f32 = 240.0;
/// The most items listed at once; the filter finds the rest.
const LISTED: usize = 400;
const ROW_H: f32 = 28.0;
/// The figure over the disks, and the line under it.
const STORAGE_HEAD: f32 = 38.0;
const FIELD_H: f32 = 34.0;
const FIELD_GAP: f32 = 10.0;
const NOTE_H: f32 = 34.0;
/// A category's mark in the plan, and its row.
const MARK: f32 = 24.0;
const PLAN_ROW_H: f32 = MARK + 4.0;
const ROWS_GAP: f32 = 2.0;
/// The plan's title and its sum, over its rows.
const PLAN_HEAD: f32 = 22.0 + 24.0;
/// A retention setting: its name over its hint.
const RETENTION_H: f32 = 56.0;
const JOURNAL_FIELD_W: f32 = 80.0;

fn glyph_key(kind: CleanupKind) -> &'static str {
    match kind {
        CleanupKind::Nix => "molt-clean-nix",
        CleanupKind::Docker => "molt-clean-docker",
        CleanupKind::Development => "molt-clean-development",
        CleanupKind::Caches => "molt-clean-caches",
        CleanupKind::Games | CleanupKind::GamePrefixes => "molt-clean-games",
        CleanupKind::Trash => "molt-clean-trash",
        CleanupKind::Journals => "molt-clean-journals",
    }
}

fn storage(shown: &CleanupView) -> (Block, f32) {
    let (under, rows) = match &shown.volumes {
        Err(why) => (Block::caption(*why, SUBTEXT1), 1),
        Ok(volumes) => {
            let rows = volumes
                .iter()
                .map(|volume| {
                    let parts = volume
                        .parts
                        .iter()
                        .map(|(share, part)| (*share, part.map_or(OVERLAY0, hue)))
                        .collect();
                    let tip = (!volume.breakdown.is_empty()).then(|| volume.breakdown.join("\n").into());
                    TableRow::cells(
                        vec![
                            Cell::strong(volume.mount.clone(), SUBTEXT1),
                            Cell::Bar(parts),
                            Cell::text(volume.used.clone(), OVERLAY2),
                            Cell::text(volume.after.clone(), SUBTEXT1),
                        ],
                        tip,
                    )
                })
                .collect::<Vec<_>>();
            let count = rows.len();
            let column = |width, end| Column {
                title: "".into(),
                width,
                end,
            };
            (
                Block::Table(Table {
                    columns: vec![
                        column(Width::Fixed(120.0), false),
                        column(Width::Share(1.0), false),
                        column(Width::Fixed(190.0), true),
                        column(Width::Fixed(80.0), true),
                    ],
                    rows,
                }),
                count,
            )
        }
    };
    let available = Block::Item(
        Item {
            id: "molt-cleanup-available".into(),
            title: shown.available.clone().into(),
            title_size: Some(FIGURE_SIZE),
            ..Default::default()
        },
        vec![under],
    );
    let reclaim = Block::Item(
        Item {
            id: "molt-cleanup-reclaim".into(),
            title: shown.reclaim.clone().into(),
            title_size: Some(DISPLAY_SIZE),
            ..Default::default()
        },
        vec![Block::caption(shown.reclaim_detail.clone(), SUBTEXT1)],
    );
    let height = 2.0 * RAISED_PAD + STORAGE_HEAD + rows as f32 * ROW_H;
    let card = Block::Frame(
        frame("molt-cleanup-storage".to_owned(), Some(height)),
        vec![Block::Columns(
            Split::Trail { width: FIGURE_W },
            vec![available, reclaim],
        )],
    );
    (card, height)
}

fn items(items: &Items, height: f32, view: &View, p: &PanelCx<'_>) -> Block {
    let tint = hue(items.kind.hue());
    let mut blocks = vec![Block::Item(
        Item {
            id: "molt-cleanup-items-title".into(),
            title: items.title.into(),
            title_color: Some(tint),
            title_size: Some(TITLE_SIZE),
            trail: vec![Trail::Faint(items.summary.clone().into())],
            ..Default::default()
        },
        Vec::new(),
    )];
    let mut used = 2.0 * RAISED_PAD + 24.0 + RAISED_GAP + FIELD_H + FIELD_GAP;
    if let Some(note) = &items.note {
        used += NOTE_H + RAISED_GAP;
        blocks.push(Block::Frame(
            FrameView {
                id: "molt-cleanup-items-note".into(),
                look: Look::Washed(YELLOW),
                on_click: None,
                on_dismiss: None,
                countdown: None,
                selected: false,
                motion: Motion::Still,
                shown_height: None,
                min_height: None,
            },
            vec![Block::caption(note.clone(), YELLOW)],
        ));
    }
    if items.more.is_some() {
        used += 20.0 + RAISED_GAP;
    }
    let [item, detail, bar, size] = items.columns;
    let rows = items
        .rows
        .iter()
        .map(|entry| {
            TableRow::cells(
                vec![
                    Cell::text(entry.label.clone(), TEXT),
                    Cell::text(entry.detail.clone(), OVERLAY2),
                    Cell::Bar(entry.fraction.map(|share| (share, tint)).into_iter().collect()),
                    if entry.pending {
                        Cell::text(entry.size.clone(), OVERLAY2)
                    } else {
                        Cell::strong(entry.size.clone(), TEXT)
                    },
                ],
                Some(format!("{}\n{}", entry.label, entry.detail).into()),
            )
        })
        .collect();
    blocks.push(Block::Search(SearchView {
        glyph: p.glyph("molt-search"),
        query: items.query.clone().into(),
        placeholder: items.filter_placeholder.into(),
        empty: items.empty.unwrap_or_default().into(),
        filters: None,
        rows: Vec::new(),
        pictures: false,
        on_scroll: Rc::new(|_, _| {}),
        preview: None,
        fill: true,
        compact: true,
        list_height: Some((height - used).max(ROW_H)),
        status: None,
        list_width: None,
        beside: Vec::new(),
        focused: view.focus == Focus::Filter,
        on_focus: Some(p.on_local::<Molt>(|_, view: &mut View| view.focus = Focus::Filter)),
        table: Some(Table {
            columns: vec![
                Column {
                    title: item.into(),
                    width: Width::Share(1.25),
                    end: false,
                },
                Column {
                    title: detail.into(),
                    width: Width::Share(1.3),
                    end: false,
                },
                Column {
                    title: bar.into(),
                    width: Width::Fixed(110.0),
                    end: false,
                },
                Column {
                    title: size.into(),
                    width: Width::Fixed(80.0),
                    end: true,
                },
            ],
            rows,
        }),
    }));
    if let Some(more) = &items.more {
        blocks.push(Block::caption(more.clone(), OVERLAY2));
    }
    Block::Frame(frame("molt-cleanup-items".to_owned(), None), blocks)
}

fn plan_row(entry: &PlanRow, p: &PanelCx<'_>) -> Block {
    let tint = hue(entry.kind.hue());
    let kind = entry.kind;
    let live = !matches!(entry.pick, Pick::Unavailable);
    let mut trail = vec![Trail::Text(
        entry.size.clone().into(),
        match &entry.pick {
            Pick::Unavailable => OVERLAY2,
            Pick::Cleaned => GREEN,
            Pick::Failed(_) => CRITICAL,
            _ => TEXT,
        },
    )];
    trail.push(match &entry.pick {
        Pick::Ticked(picked) => Trail::Tick(
            *picked,
            entry
                .enabled
                .then(|| p.call(move |molt: &Molt| molt.cleanup(Msg::Toggle(kind)))),
        ),
        Pick::Unavailable => Trail::Tick(false, None),
        Pick::Cleaning => Trail::Tick(true, None),
        Pick::Cleaned => Trail::Text("✓".into(), GREEN),
        Pick::Failed(_) => Trail::Text("!".into(), CRITICAL),
    });
    let item = Item {
        id: format!("molt-cleanup-plan-{kind:?}").into(),
        lead: Some(Lead {
            mark: p.glyph(glyph_key(kind)).map(|path| Mark::Glyph(path, tint)),
            tint: Some(tint),
            faded: !live,
            size: Some(MARK),
            ..Default::default()
        }),
        title: entry.title.into(),
        title_color: (!live).then_some(OVERLAY2),
        trail,
        ..Default::default()
    };
    Block::Frame(
        FrameView {
            id: format!("molt-cleanup-plan-row-{kind:?}").into(),
            look: Look::Row,
            on_click: Some(p.call(move |molt: &Molt| molt.cleanup(Msg::Open(kind)))),
            on_dismiss: None,
            countdown: None,
            selected: entry.open,
            motion: Motion::Still,
            shown_height: None,
            min_height: None,
        },
        vec![Block::Item(item, Vec::new())],
    )
}

fn retention(setting: &Retention, view: &View, p: &PanelCx<'_>) -> Block {
    let control = match setting.setting {
        Setting::Generations => {
            let count = setting.value.trim().parse::<usize>().ok();
            let editable = setting.enabled;
            Trail::Stepper(Stepper {
                value: setting.value.clone().into(),
                on_less: (editable && count.is_some_and(|count| count > 0))
                    .then(|| p.call(|molt: &Molt| molt.cleanup(Msg::StepGenerations(-1)))),
                on_more: (editable
                    && count.is_some_and(|count| {
                        count < molt_core::cleanup::flow::MAX_GENERATIONS
                    }))
                .then(|| p.call(|molt: &Molt| molt.cleanup(Msg::StepGenerations(1)))),
            })
        }
        Setting::Journals => Trail::Field(Field {
            text: setting.value.clone().into(),
            placeholder: setting.placeholder.into(),
            width: JOURNAL_FIELD_W,
            focused: view.focus == Focus::Journals,
            on_focus: setting
                .enabled
                .then(|| p.on_local::<Molt>(|_, view: &mut View| view.focus = Focus::Journals)),
        }),
    };
    Block::Frame(
        FrameView {
            id: format!("molt-cleanup-retention-{:?}", setting.setting).into(),
            look: Look::Inset,
            on_click: None,
            on_dismiss: None,
            countdown: None,
            selected: false,
            motion: Motion::Appear,
            shown_height: None,
            min_height: None,
        },
        vec![Block::Item(
            Item {
                id: format!("molt-cleanup-retention-item-{:?}", setting.setting).into(),
                title: setting.title.into(),
                trail: vec![control],
                trail_beside: true,
                ..Default::default()
            },
            vec![Block::caption(setting.hint, OVERLAY2)],
        )],
    )
}

fn plan(shown: &CleanupView, height: f32, view: &View, p: &PanelCx<'_>) -> Block {
    let mut blocks = vec![
        Block::Item(
            Item {
                id: "molt-cleanup-plan-title".into(),
                title: shown.plan_title.into(),
                title_size: Some(TITLE_SIZE),
                ..Default::default()
            },
            Vec::new(),
        ),
        Block::Item(
            Item {
                id: "molt-cleanup-plan-summary".into(),
                title: shown.plan_summary.clone().into(),
                title_size: Some(HEADING_SIZE),
                ..Default::default()
            },
            Vec::new(),
        ),
    ];
    let count = shown.plan.len() as f32;
    let listed = count * PLAN_ROW_H + (count - 1.0).max(0.0) * ROWS_GAP;
    let settings = shown.retention.len() as f32 * (RETENTION_H + RAISED_GAP);
    let room = height - 2.0 * RAISED_PAD - PLAN_HEAD - 2.0 * RAISED_GAP - settings;
    blocks.push(Block::Scrolled(
        "molt-cleanup-plan-rows".into(),
        listed.min(room.max(PLAN_ROW_H)),
        vec![Block::Rows(
            shown.plan.iter().map(|entry| plan_row(entry, p)).collect(),
        )],
    ));
    blocks.extend(shown.retention.iter().map(|setting| retention(setting, view, p)));
    Block::Frame(frame("molt-cleanup-plan".to_owned(), None), blocks)
}

fn act(shown: &CleanupView, cleaning: bool, p: &PanelCx<'_>) -> IconButton {
    let action = &shown.action;
    let color = match action.look {
        ActionLook::Primary => Some(ACCENT),
        ActionLook::Danger => Some(CRITICAL),
        ActionLook::Plain => None,
    };
    IconButton {
        glyph: None,
        color,
        label: action.label.clone().into(),
        active: color.is_some(),
        badge: None,
        enabled: action.enabled,
        busy: action.busy,
        on_click: p.on_local::<Molt>(move |molt, view: &mut View| {
            if cleaning {
                view.open(Screen::Home);
            }
            molt.cleanup(Msg::Act);
        }),
    }
}

pub(super) fn cleanup(state: &Updates, view: &View, p: &PanelCx<'_>) -> (Views, Vec<Block>) {
    let back = p.on_local::<Molt>(|_, view: &mut View| view.open(Screen::Home));
    let Some(screen) = &state.cleanup else {
        return (
            views(None, Some(back), p),
            vec![Block::note("Inspecting every available cleaner…")],
        );
    };
    let shown = present::view(screen, LISTED, &IslandUnits);
    let mut header = views(
        Some(HeaderLead {
            mark: p
                .glyph("molt-cleanup")
                .map_or(Mark::Letter("C".into(), PEACH), |path| {
                    Mark::Glyph(path, PEACH)
                }),
            size: LEAD,
            overline: shown.subtitle.into(),
            title: shown.title.into(),
            title_color: None,
            on_click: None,
        }),
        shown.back_enabled.then_some(back),
        p,
    );
    header.status = Some(HeaderStatus {
        text: shown.status.text.clone().into(),
        color: match shown.status.tone {
            Tone::Plain => SUBTEXT1,
            Tone::Error => CRITICAL,
            Tone::Success => GREEN,
        },
        busy: shown.status.busy,
    });
    let (storage, storage_h) = storage(&shown);
    let columns_h = body_h() - storage_h - BLOCK_GAP;
    let (width, _) = Screen::Cleanup.size();
    let items_w = width - 2.0 * crate::ui::INSET - crate::components::card::GAP - PLAN_W;
    let cleaning = screen.phase == Phase::Cleaning;
    (
        header,
        vec![
            storage,
            Block::Columns(
                Split::Lead {
                    width: items_w,
                    height: columns_h,
                },
                vec![
                    items(&shown.items, columns_h, view, p),
                    plan(&shown, columns_h, view, p),
                ],
            ),
            row(vec![act(&shown, cleaning, p)], None),
        ],
    )
}
