//! Molt's build, as molt-core words it: the flake inputs or the work nix does on the left, how far
//! it has come and nix's warnings on the right, and once it fails, the error with its build log.

use std::time::Instant;

use molt_core::build::{
    flow::{Build, Msg},
    present::{
        self, Action, ActionId, BuildView, ListView, LogView, Side, StepState, WarningsCard,
    },
    rows::{RowContent, RowLook},
};

use super::{
    format::IslandUnits,
    inputs::{body_h, row},
    parts::{frame, hue, views},
    runtime::Molt,
    view::{Focus, Screen, View},
};
use crate::{
    components::{
        card::{Look, Motion, RAISED_GAP, RAISED_PAD},
        code::Scroll,
    },
    features::PanelCx,
    panel::{
        Block, Cell, ChartView, Code, CodeLine, CodeMotion, Column, Field, Filters, FrameView,
        HeaderLead, IconButton, Item, Mark, RowLead, Series, Split, Stop, StopState, TabLabel,
        TableRow, Table, Trail, TrailButton, Views, Width, on,
    },
    theme::{ACCENT, BLUE, CRITICAL, GREEN, OVERLAY0, OVERLAY1, OVERLAY2, SAPPHIRE, SUBTEXT1, TEXT, YELLOW},
    ui::{DISPLAY_SIZE, FIGURE_SIZE, TITLE_SIZE},
};

const LEAD: f32 = 36.0;
/// The side column, beside the list.
const SIDE_W: f32 = 380.0;
/// The most rows listed at once; the filter finds the rest.
const LISTED: usize = 300;
/// The end of a build log shown.
const LOG_LINES: usize = 800;
/// The list's line of tabs and filter, and the rule under it.
const TOOLBAR_H: f32 = 36.0 + 1.0;
const FILTER_W: f32 = 200.0;
const MORE_H: f32 = 20.0;
const SPARK_H: f32 = 48.0;
/// Between the side column's cards.
const GRID_GAP: f32 = RAISED_GAP;

pub(super) fn stops(steps: &[(&'static str, StepState)]) -> Vec<Stop> {
    steps
        .iter()
        .map(|(name, state)| Stop {
            label: (*name).into(),
            state: match state {
                StepState::Done => StopState::Reached,
                StepState::Now => StopState::Pending(None),
                StepState::Failed => StopState::Failed,
                StepState::Waiting => StopState::NotYet,
            },
        })
        .collect()
}

fn marker(row: &RowContent, tint: u32) -> Cell {
    match row.look {
        RowLook::Active | RowLook::Transfer => Cell::Dot(tint),
        RowLook::Idle => Cell::text("•", OVERLAY1),
        RowLook::Done => Cell::text("✓", GREEN),
        RowLook::Failed => Cell::text("✕", CRITICAL),
    }
}

fn table(list: &ListView, inputs: bool) -> Table {
    let mut rows = Vec::new();
    for group in &list.groups {
        let tint = hue(group.hue);
        rows.push(TableRow::Group(group.heading.clone().into(), tint));
        for content in &group.rows {
            let busy = content.look == RowLook::Active;
            let failed = content.look == RowLook::Failed;
            let two = (busy || failed) && !content.detail.is_empty() && !inputs;
            let mut title = vec![(content.title.clone().into(), TEXT)];
            if !content.tag.is_empty() {
                title.push((format!(" {}", content.tag).into(), OVERLAY2));
            }
            let mut cells = vec![marker(content, tint), Cell::Parts(title, false)];
            if inputs {
                cells.push(Cell::text(content.column.clone(), OVERLAY2));
            } else if !two {
                cells.push(Cell::mono(content.detail.clone(), OVERLAY2));
            } else {
                cells.push(Cell::text("", OVERLAY2));
            }
            cells.push(if content.look == RowLook::Transfer {
                Cell::Bar(vec![(content.fraction as f32, SAPPHIRE)])
            } else {
                Cell::Bar(Vec::new())
            });
            cells.push(Cell::text(
                content.value.clone(),
                if failed { CRITICAL } else { OVERLAY2 },
            ));
            rows.push(TableRow::Line(crate::components::table::Line {
                cells,
                tip: (!content.detail.is_empty()).then(|| content.detail.clone().into()),
                under: two.then(|| (1, content.detail.clone().into())),
                wash: failed.then_some(CRITICAL),
                busy,
                on_click: None,
            }));
        }
    }
    let column = |width, end| Column {
        title: "".into(),
        width,
        end,
    };
    Table {
        columns: vec![
            column(Width::Fixed(14.0), false),
            column(
                if inputs {
                    Width::Fixed(210.0)
                } else {
                    Width::Share(1.0)
                },
                false,
            ),
            column(Width::Share(1.0), false),
            column(Width::Fixed(70.0), false),
            column(Width::Fixed(64.0), true),
        ],
        rows,
    }
}

fn list_card(shown: &BuildView, inputs: bool, height: f32, view: &View, p: &PanelCx<'_>) -> Block {
    if let Some(log) = &shown.log {
        return log_card(log, height, p);
    }
    let list = &shown.list;
    let tabs = Filters {
        tabs: list
            .tabs
            .iter()
            .map(|(name, count)| TabLabel {
                label: (*name).into(),
                glyph: None,
                count: Some(count.to_string().into()),
            })
            .collect(),
        selected: list.tab,
        on_select: p.on_local_with::<Molt, usize>(|molt, _, tab| molt.build(Msg::Tab(tab))),
    };
    let toolbar = Block::Item(
        Item {
            id: "molt-build-toolbar".into(),
            label: Some(list.title.clone().into()),
            before: vec![Trail::Strip(tabs)],
            title: "".into(),
            trail: vec![Trail::Field(Field {
                text: list.query.clone().into(),
                placeholder: list.placeholder.into(),
                width: FILTER_W,
                focused: view.focus == Focus::Filter,
                on_focus: Some(p.on_local::<Molt>(|_, view: &mut View| view.focus = Focus::Filter)),
            })],
            ..Default::default()
        },
        Vec::new(),
    );
    let mut room = height - 2.0 * RAISED_PAD - TOOLBAR_H - 2.0 * RAISED_GAP;
    if list.more.is_some() {
        room -= MORE_H + RAISED_GAP;
    }
    let listed = match list.empty {
        Some(empty) => vec![Block::note(empty)],
        None => vec![Block::Table(table(list, inputs))],
    };
    let mut blocks = vec![
        toolbar,
        Block::Rule,
        Block::Scrolled("molt-build-rows".into(), room.max(0.0), listed),
    ];
    if let Some(more) = &list.more {
        blocks.push(Block::caption(more.clone(), OVERLAY2));
    }
    Block::Frame(frame("molt-build-list".to_owned(), Some(height)), blocks)
}

fn log_card(log: &LogView, height: f32, p: &PanelCx<'_>) -> Block {
    let copy_text = log.text.clone().unwrap_or_default();
    let mut trail = Vec::new();
    if log.text.is_some() {
        trail.push(Trail::Button(TrailButton {
            label: Some(log.copy.into()),
            glyph: None,
            turned: false,
            tint: None,
            on_click: on(move |_| crate::copy::text(copy_text.clone())),
        }));
    }
    let header = Block::Item(
        Item {
            id: "molt-build-log-header".into(),
            before: vec![Trail::Button(TrailButton {
                label: Some(format!("‹ {}", log.back).into()),
                glyph: None,
                turned: false,
                tint: None,
                on_click: p.call(|molt: &Molt| molt.build(Msg::HideLog)),
            })],
            title: log.title.clone().into(),
            title_color: Some(OVERLAY2),
            trail,
            ..Default::default()
        },
        Vec::new(),
    );
    let room = height - 2.0 * RAISED_PAD - TOOLBAR_H - 2.0 * RAISED_GAP;
    let mut blocks = vec![header, Block::Rule];
    if let Some((status, error)) = &log.status {
        blocks.push(Block::caption(
            status.clone(),
            if *error { CRITICAL } else { SUBTEXT1 },
        ));
    }
    if let Some(note) = &log.note {
        blocks.push(Block::caption(note.clone(), OVERLAY2));
    }
    if let Some(text) = &log.text {
        let used = if log.note.is_some() { 20.0 + RAISED_GAP } else { 0.0 };
        blocks.push(Block::Code(Code {
            heading: None,
            lines: text.lines().map(CodeLine::plain).collect(),
            wrap: true,
            copy: None,
            motion: CodeMotion::Still,
            scroll: Some(Scroll {
                key: "molt-build-log".into(),
                height: (room - used).max(0.0),
                follows: true,
            }),
        }));
    }
    Block::Frame(frame("molt-build-log".to_owned(), Some(height)), blocks)
}

fn card(id: &str, title: &str, title_color: Option<u32>, trail: Vec<Trail>, mut blocks: Vec<Block>, height: Option<f32>) -> Block {
    blocks.insert(
        0,
        Block::Item(
            Item {
                id: format!("{id}-title").into(),
                title: title.to_owned().into(),
                title_size: Some(TITLE_SIZE),
                title_color,
                trail,
                ..Default::default()
            },
            Vec::new(),
        ),
    );
    Block::Frame(frame(id.to_owned(), height), blocks)
}

fn figure(id: &str, value: &str, size: f32, detail: &str) -> Block {
    Block::Item(
        Item {
            id: id.to_owned().into(),
            title: value.to_owned().into(),
            title_size: Some(size),
            detail: detail.to_owned().into(),
            ..Default::default()
        },
        Vec::new(),
    )
}

fn side(shown: &BuildView, height: f32, p: &PanelCx<'_>) -> Vec<Block> {
    match &shown.side {
        Side::Evaluation(card_view) => vec![card(
            "molt-build-evaluation",
            card_view.title,
            None,
            Vec::new(),
            vec![
                figure(
                    "molt-build-evaluation-elapsed",
                    &card_view.elapsed,
                    DISPLAY_SIZE,
                    &card_view.estimate,
                ),
                Block::Bar(match card_view.fraction {
                    Some(fraction) => vec![(fraction, ACCENT)],
                    None => Vec::new(),
                }),
                Block::caption(card_view.note, OVERLAY2),
                Block::Rule,
                Block::Item(
                    Item {
                        id: "molt-build-evaluation-inputs".into(),
                        title: card_view.inputs_label.into(),
                        title_color: Some(SUBTEXT1),
                        trail: vec![Trail::Text(card_view.inputs.clone().into(), TEXT)],
                        ..Default::default()
                    },
                    Vec::new(),
                ),
            ],
            None,
        )],
        Side::Building(progress, downloads) => {
            let peak = downloads.history.iter().copied().fold(1.0_f64, f64::max) as f32;
            let mut progress_blocks = vec![
                figure("molt-build-count", &progress.count, DISPLAY_SIZE, ""),
                Block::Bar(vec![(progress.done, ACCENT), (progress.pending, OVERLAY0)]),
                Block::Item(
                    Item {
                        id: "molt-build-elapsed".into(),
                        title: progress.elapsed.clone().into(),
                        title_color: Some(OVERLAY2),
                        trail: vec![Trail::Faint(progress.remaining.clone().into())],
                        ..Default::default()
                    },
                    Vec::new(),
                ),
            ];
            if !progress.note.is_empty() {
                progress_blocks.push(Block::caption(progress.note.clone(), OVERLAY2));
            }
            vec![
                card(
                    "molt-build-progress",
                    progress.title,
                    None,
                    Vec::new(),
                    progress_blocks,
                    None,
                ),
                card(
                    "molt-build-downloads",
                    downloads.title,
                    None,
                    vec![Trail::Faint(downloads.rate.clone().into())],
                    vec![
                        figure(
                            "molt-build-downloaded",
                            &downloads.done,
                            FIGURE_SIZE,
                            &downloads.total,
                        ),
                        Block::Chart(ChartView {
                            series: vec![Series {
                                values: downloads.history.iter().map(|rate| *rate as f32).collect(),
                                max: peak,
                                color: SAPPHIRE,
                                label: downloads.title.into(),
                                format: |rate| crate::units::rate(f64::from(rate)),
                            }],
                            height: SPARK_H,
                            phase: None,
                        }),
                    ],
                    None,
                ),
            ]
        }
        Side::Failed(error) => {
            let message = error.message.clone();
            let mut buttons = vec![IconButton {
                glyph: None,
                color: None,
                label: error.copy.into(),
                active: false,
                badge: None,
                enabled: true,
                busy: false,
                on_click: on(move |_| crate::copy::text(message.clone())),
            }];
            if let Some(view_log) = error.view_log {
                buttons.push(IconButton {
                    glyph: None,
                    color: None,
                    label: view_log.into(),
                    active: false,
                    badge: None,
                    enabled: true,
                    busy: false,
                    on_click: p.call(|molt: &Molt| molt.build(Msg::ShowLog)),
                });
            }
            let room = height - 2.0 * RAISED_PAD - 24.0 - 3.0 * RAISED_GAP - 30.0;
            vec![card(
                "molt-build-error",
                error.title,
                Some(CRITICAL),
                error
                    .derivation
                    .iter()
                    .map(|(name, _)| Trail::Faint(name.clone().into()))
                    .collect(),
                vec![
                    Block::Code(Code {
                        heading: None,
                        lines: error.message.lines().map(CodeLine::plain).collect(),
                        wrap: true,
                        copy: None,
                        motion: CodeMotion::Still,
                        scroll: Some(Scroll {
                            key: "molt-build-error-text".into(),
                            height: room.max(60.0),
                            follows: false,
                        }),
                    }),
                    row(buttons, None),
                ],
                Some(height),
            )]
        }
    }
}

fn warnings(card_view: &WarningsCard, p: &PanelCx<'_>) -> Block {
    let mut blocks: Vec<Block> = card_view
        .shown
        .iter()
        .map(|warning| {
            Block::Frame(
                FrameView {
                    id: format!("molt-build-warning-{warning}").into(),
                    look: Look::Washed(YELLOW),
                    on_click: None,
                    on_dismiss: None,
                    countdown: None,
                    selected: false,
                    motion: Motion::Appear,
                    shown_height: None,
                    min_height: None,
                },
                vec![Block::caption(warning.clone(), YELLOW)],
            )
        })
        .collect();
    if let Some(empty) = card_view.empty {
        blocks.push(Block::caption(empty, SUBTEXT1));
    }
    if let Some(more) = &card_view.more {
        blocks.push(Block::caption(more.clone(), OVERLAY2));
    }
    let trail = card_view
        .toggle
        .map(|toggle| {
            Trail::Button(TrailButton {
                label: Some(toggle.into()),
                glyph: None,
                turned: false,
                tint: None,
                on_click: p.call(|molt: &Molt| molt.build(Msg::ToggleWarnings)),
            })
        })
        .into_iter()
        .collect();
    card("molt-build-warnings", &card_view.title, None, trail, blocks, None)
}

fn action(action: &Action, build: &Build, p: &PanelCx<'_>) -> IconButton {
    let (target, after) = (build.target, build.after);
    let on_click = match action.id {
        ActionId::Minimize => on(|ui| ui.collapse()),
        ActionId::Cancel => p.on_local::<Molt>(|molt, view: &mut View| {
            molt.build(Msg::Cancel);
            view.open(Screen::Home);
        }),
        ActionId::Retry => p.call(move |molt: &Molt| molt.start_build(target, after)),
    };
    IconButton {
        glyph: None,
        color: action.primary.then_some(ACCENT),
        label: action.label.into(),
        active: action.primary,
        badge: None,
        enabled: true,
        busy: false,
        on_click,
    }
}

pub(super) fn header(
    title: String,
    subtitle: String,
    failed: bool,
    tint: u32,
    back: bool,
    p: &PanelCx<'_>,
) -> Views {
    views(
        Some(HeaderLead {
            mark: p
                .glyph("molt-nixos")
                .map_or(Mark::Letter("N".into(), tint), |path| Mark::Glyph(path, tint)),
            size: LEAD,
            overline: subtitle.into(),
            title: title.into(),
            title_color: failed.then_some(CRITICAL),
            on_click: None,
        }),
        back.then(|| p.on_local::<Molt>(|_, view: &mut View| view.open(Screen::Home))),
        p,
    )
}

pub(super) fn build(
    build: &Build,
    screen: Screen,
    view: &View,
    p: &PanelCx<'_>,
) -> (Views, Vec<Block>) {
    let shown = present::view(build, LISTED, LOG_LINES, Instant::now(), &IslandUnits);
    let mut head = header(
        shown.title.clone(),
        shown.subtitle.clone(),
        shown.failed,
        BLUE,
        shown.failed,
        p,
    );
    head.route = Some(stops(&shown.steps));
    let body = body_h();
    let (width, _) = screen.size();
    let list_w = width - 2.0 * crate::ui::INSET - crate::components::card::GAP - SIDE_W;
    let shown_warnings = shown.warnings.shown.len().max(1) as f32;
    let warnings_h = 2.0 * RAISED_PAD + 24.0 + shown_warnings * (34.0 + RAISED_GAP);
    let mut column = side(&shown, body - warnings_h - GRID_GAP, p);
    column.push(warnings(&shown.warnings, p));
    let lead = shown.hint.map(|hint| RowLead::Note(hint.into()));
    let buttons = shown
        .actions
        .iter()
        .map(|each| action(each, build, p))
        .collect();
    (
        head,
        vec![
            Block::Columns(
                Split::Lead {
                    width: list_w,
                    height: body,
                },
                vec![
                    list_card(&shown, build.lists_inputs(), body, view, p),
                    Block::Scrolled("molt-build-side".into(), body, column),
                ],
            ),
            row(buttons, lead),
        ],
    )
}
