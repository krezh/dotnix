//! Molt's review, as molt-core words it: what the build changes, grouped and filtered, beside the
//! closure and the services a switch touches, and switching or applying it on reboot.

use molt_core::{
    build::Target,
    report::{
        flow::{Msg, Review},
        present::{self, ActivateButton, ActivationCard, CapsuleState, Ink, Opens, ReviewView},
    },
};

use super::{
    build::{header, stops},
    format::IslandUnits,
    inputs::{body_h, row},
    parts::{frame, hue},
    runtime::Molt,
    view::{Focus, Screen, View},
};
use crate::{
    components::{
        card::{RAISED_GAP, RAISED_PAD},
        table::Line,
    },
    features::PanelCx,
    panel::{
        Block, Cell, Chip, Code, CodeLine, CodeMotion, Column, Field, Filters, IconButton, Item,
        Split, TabLabel, Table, TableRow, Trail, TrailButton, Views, Width, on,
    },
    theme::{
        ACCENT, CRITICAL, GREEN, MAUVE, OVERLAY1, OVERLAY2, SUBTEXT1, TEXT, YELLOW,
    },
    ui::{DISPLAY_SIZE, TITLE_SIZE},
};

const SIDE_W: f32 = 380.0;
const TOOLBAR_H: f32 = 36.0;
const FILTER_W: f32 = 220.0;
const FOOTER_H: f32 = 20.0;
const FILES_H: f32 = 170.0;

const fn ink(ink: Ink) -> u32 {
    match ink {
        Ink::Plain => TEXT,
        Ink::Old => SUBTEXT1,
        Ink::Muted => OVERLAY1,
        Ink::New => MAUVE,
        Ink::Grew | Ink::Removed => CRITICAL,
        Ink::Shrank | Ink::Added => GREEN,
    }
}

fn changes(
    shown: &ReviewView,
    target: Target,
    height: f32,
    view: &View,
    p: &PanelCx<'_>,
) -> Block {
    let tabs = Filters {
        tabs: shown
            .tabs
            .iter()
            .map(|(_, name, count)| TabLabel {
                label: (*name).into(),
                glyph: None,
                count: Some(count.to_string().into()),
            })
            .collect(),
        selected: shown.tab,
        on_select: {
            let statuses: Vec<_> = shown.tabs.iter().map(|(status, _, _)| *status).collect();
            p.on_local_with::<Molt, usize>(move |molt, _, tab| {
                molt.review(Msg::Tab(statuses.get(tab).copied().flatten()));
            })
        },
    };
    let toolbar = Block::Item(
        Item {
            id: "molt-review-toolbar".into(),
            before: vec![Trail::Strip(tabs)],
            title: "".into(),
            trail: vec![Trail::Field(Field {
                text: shown.query.clone().into(),
                placeholder: shown.placeholder.into(),
                width: FILTER_W,
                focused: view.focus == Focus::Filter,
                on_focus: Some(p.on_local::<Molt>(|_, view: &mut View| view.focus = Focus::Filter)),
            })],
            ..Default::default()
        },
        Vec::new(),
    );
    let mut rows = Vec::new();
    for group in &shown.groups {
        rows.push(TableRow::Heading(Chip::new(group.heading.clone(), hue(group.hue))));
        for change in &group.rows {
            let opens = change.opens.as_ref().map(|(opens, _, _)| opens.clone());
            let (name, folder) = match change.name.rsplit_once('/') {
                Some((folder, file)) if change.path => (file, Some(folder)),
                _ => (change.name.as_str(), None),
            };
            let name = Cell::strong(name.to_owned(), TEXT);
            rows.push(TableRow::Line(Line {
                under: folder.map(|folder| (0, folder.to_owned().into())),
                cells: vec![
                    name,
                    Cell::Parts(
                        change
                            .version
                            .iter()
                            .map(|(text, part)| (text.clone().into(), ink(*part)))
                            .collect(),
                        true,
                    ),
                    Cell::Parts(
                        change
                            .change
                            .iter()
                            .map(|(text, part)| (text.clone().into(), ink(*part)))
                            .collect(),
                        true,
                    ),
                ],
                tip: Some(
                    change
                        .opens
                        .as_ref()
                        .map_or_else(|| change.version_plain.clone(), |(_, _, tip)| tip.clone())
                        .into(),
                ),
                on_click: opens.map(|opens| {
                    p.on_local::<Molt>(move |molt, view: &mut View| match &opens {
                        Opens::Changelog(index) => {
                            molt.open_notes(*index);
                            view.open(Screen::Notes(target));
                        }
                        Opens::Diff(index) => view.open(Screen::Diff(target, *index)),
                    })
                }),
                ..Line::default()
            }));
        }
    }
    let mut room = height - 2.0 * RAISED_PAD - TOOLBAR_H - RAISED_GAP;
    let mut under = Vec::new();
    if let Some(footer) = &shown.footer {
        room -= FOOTER_H + RAISED_GAP;
        let toggle = shown.files_toggle.map(|toggle| {
            Trail::Button(TrailButton {
                label: Some(toggle.into()),
                glyph: None,
                turned: false,
                tint: Some(ACCENT),
                on_click: p.call(|molt: &Molt| molt.review(Msg::ToggleFiles)),
            })
        });
        under.push(Block::Item(
            Item {
                id: "molt-review-files".into(),
                title: footer.clone().into(),
                title_color: Some(OVERLAY2),
                trail: toggle.into_iter().collect(),
                ..Default::default()
            },
            Vec::new(),
        ));
    }
    if let Some(files) = &shown.files {
        room -= FILES_H + RAISED_GAP;
        under.push(Block::Code(Code {
            heading: None,
            lines: files.lines().map(CodeLine::plain).collect(),
            wrap: false,
            copy: None,
            motion: CodeMotion::Still,
            scroll: Some(crate::components::code::Scroll {
                key: "molt-review-files-list".into(),
                height: FILES_H,
                follows: false,
            }),
        }));
    }
    let listed = match shown.empty {
        Some(empty) => vec![Block::note(empty)],
        None => {
            let [name, version, change] = shown.columns;
            vec![Block::Table(Table {
                columns: vec![
                    Column {
                        title: name.into(),
                        width: Width::Share(1.0),
                        end: false,
                    },
                    Column {
                        title: version.into(),
                        width: Width::Share(1.3),
                        end: false,
                    },
                    Column {
                        title: change.into(),
                        width: Width::Fixed(90.0),
                        end: true,
                    },
                ],
                rows,
            })]
        }
    };
    let mut blocks = vec![
        toolbar,
        Block::Scrolled("molt-review-rows".into(), room.max(0.0), listed),
    ];
    blocks.extend(under);
    Block::Frame(frame("molt-review-changes".to_owned(), Some(height)), blocks)
}

fn titled(id: &str, title: String, color: Option<u32>, trail: Vec<Trail>, mut blocks: Vec<Block>) -> Block {
    blocks.insert(
        0,
        Block::Item(
            Item {
                id: format!("{id}-title").into(),
                title: title.into(),
                title_size: Some(TITLE_SIZE),
                title_color: color,
                trail,
                ..Default::default()
            },
            Vec::new(),
        ),
    );
    Block::Frame(frame(id.to_owned(), None), blocks)
}

fn side(shown: &ReviewView, p: &PanelCx<'_>) -> Vec<Block> {
    if let Some(card) = &shown.activation {
        return vec![activation(card)];
    }
    let closure = &shown.closure;
    let mut blocks = Vec::new();
    if let Some((delta, part)) = &closure.delta {
        blocks.push(Block::Item(
            Item {
                id: "molt-review-delta".into(),
                title: delta.clone().into(),
                title_size: Some(DISPLAY_SIZE),
                title_color: Some(ink(*part)),
                ..Default::default()
            },
            closure
                .range
                .iter()
                .map(|range| Block::caption(range.clone(), SUBTEXT1))
                .collect(),
        ));
    }
    for (name, value, mono) in &closure.rows {
        blocks.push(Block::Item(
            Item {
                id: format!("molt-review-closure-{name}").into(),
                title: (*name).into(),
                title_color: Some(SUBTEXT1),
                trail: vec![if *mono {
                    Trail::Faint(value.clone().into())
                } else {
                    Trail::Text(value.clone().into(), TEXT)
                }],
                ..Default::default()
            },
            Vec::new(),
        ));
    }
    let mut cards = vec![titled(
        "molt-review-closure",
        closure.title.to_owned(),
        None,
        Vec::new(),
        blocks,
    )];
    if let Some(services) = &shown.services {
        let mut blocks = Vec::new();
        if let Some(notice) = &services.notice {
            blocks.push(Block::caption(notice.clone(), YELLOW));
        }
        if let Some(empty) = services.empty {
            blocks.push(Block::caption(empty, SUBTEXT1));
        }
        for group in &services.groups {
            let units = group
                .units
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>()
                .join(" · ");
            blocks.push(Block::Item(
                Item {
                    id: format!("molt-review-services-{}", group.heading).into(),
                    title: "".into(),
                    before: vec![Trail::Chip(Chip::new(group.heading.clone(), hue(group.hue)))],
                    ..Default::default()
                },
                vec![Block::Text(crate::panel::Text {
                    lines: None,
                    ..crate::panel::Text::new(crate::panel::Tone::Caption(OVERLAY2), units)
                })],
            ));
        }
        let mut trail = Vec::new();
        if let Some((toggle, _)) = &services.output {
            trail.push(Trail::Button(TrailButton {
                label: Some((*toggle).into()),
                glyph: None,
                turned: false,
                tint: Some(ACCENT),
                on_click: p.call(|molt: &Molt| molt.review(Msg::ToggleOutput)),
            }));
        }
        if let Some((_, Some(output))) = &services.output {
            blocks.push(Block::Code(Code {
                heading: None,
                lines: output.lines().map(CodeLine::plain).collect(),
                wrap: true,
                copy: None,
                motion: CodeMotion::Still,
                scroll: Some(crate::components::code::Scroll {
                    key: "molt-review-dry-activate".into(),
                    height: 180.0,
                    follows: false,
                }),
            }));
        }
        cards.push(titled(
            "molt-review-services",
            services.title.to_owned(),
            None,
            trail,
            blocks,
        ));
    }
    cards
}

fn activation(card: &ActivationCard) -> Block {
    let (tint, capsule) = match card.state {
        CapsuleState::Running => (SUBTEXT1, Chip { busy: true, wide: true, ..Chip::new(card.capsule, SUBTEXT1) }),
        CapsuleState::Done => (GREEN, Chip { wide: true, ..Chip::new(card.capsule, GREEN) }),
        CapsuleState::Failed => (CRITICAL, Chip { wide: true, ..Chip::new(card.capsule, CRITICAL) }),
    };
    let mut trail = Vec::new();
    match (&card.chip, card.state) {
        (Some(chip), _) => trail.push(Trail::Chip(Chip::new(chip.clone(), GREEN))),
        (None, CapsuleState::Failed) => trail.push(Trail::Chip(Chip::new(card.elapsed.clone(), CRITICAL))),
        (None, _) => trail.push(Trail::Faint(card.elapsed.clone().into())),
    }
    let mut blocks = vec![Block::Chip(capsule)];
    if let Some(note) = &card.note {
        blocks.push(Block::caption(note.clone(), SUBTEXT1));
    }
    if card.state == CapsuleState::Done && card.chip.is_some() {
        blocks.push(Block::caption(card.elapsed.clone(), SUBTEXT1));
    }
    if let Some(error) = &card.error {
        blocks.push(Block::Code(Code {
            heading: None,
            lines: error.lines().map(CodeLine::plain).collect(),
            wrap: true,
            copy: None,
            motion: CodeMotion::Still,
            scroll: None,
        }));
        if let Some(copy) = card.copy {
            let error = error.clone();
            blocks.push(row(
                vec![IconButton {
                    glyph: None,
                    color: None,
                    label: copy.into(),
                    active: false,
                    badge: None,
                    enabled: true,
                    busy: false,
                    on_click: on(move |_| crate::copy::text(error.clone())),
                }],
                None,
            ));
        }
    }
    titled(
        "molt-review-activation",
        card.title.clone(),
        (card.state != CapsuleState::Running).then_some(tint),
        trail,
        blocks,
    )
}

fn button(shown: &ActivateButton, p: &PanelCx<'_>) -> IconButton {
    let mode = shown.mode;
    let color = if shown.confirming {
        Some(CRITICAL)
    } else if shown.done {
        Some(GREEN)
    } else if shown.primary {
        Some(ACCENT)
    } else {
        None
    };
    IconButton {
        glyph: None,
        color,
        label: shown.label.into(),
        active: shown.primary || shown.confirming,
        badge: None,
        enabled: shown.enabled,
        busy: shown.busy,
        on_click: p.call(move |molt: &Molt| molt.review(Msg::Activate(mode))),
    }
}

pub(super) fn review(review: &Review, screen: Screen, view: &View, p: &PanelCx<'_>) -> (Views, Vec<Block>) {
    let shown = present::view(review, &IslandUnits);
    let mut head = header(
        shown.title.to_owned(),
        shown.subtitle.clone(),
        false,
        GREEN,
        !review.activating(),
        p,
    );
    head.route = Some(stops(&[
        ("Evaluate", molt_core::build::present::StepState::Done),
        ("Build", molt_core::build::present::StepState::Done),
        ("Compare", molt_core::build::present::StepState::Done),
    ]));
    let body = body_h();
    let (width, _) = screen.size();
    let list_w = width - 2.0 * crate::ui::INSET - crate::components::card::GAP - SIDE_W;
    let mut buttons: Vec<IconButton> = Vec::new();
    if let Some(minimize) = shown.minimize {
        buttons.push(IconButton {
            glyph: None,
            color: None,
            label: minimize.into(),
            active: false,
            badge: None,
            enabled: true,
            busy: false,
            on_click: on(|ui| ui.collapse()),
        });
    }
    buttons.extend(shown.buttons.iter().map(|each| button(each, p)));
    (
        head,
        vec![
            Block::Columns(
                Split::Lead {
                    width: list_w,
                    height: body,
                },
                vec![
                    changes(&shown, review.report.target, body, view, p),
                    Block::Scrolled("molt-review-side".into(), body, side(&shown, p)),
                ],
            ),
            row(buttons, None),
        ],
    )
}
