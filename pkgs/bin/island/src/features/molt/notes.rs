//! A changed package's release notes and a changed file's diff, as molt-core words them, each a
//! screen of its own over the review.

use molt_core::build::Target;
use molt_core::report::{
    config_diff::DiffLineKind,
    flow::Review,
    markdown::{MarkdownBlockKind, spans},
    notes::{self, Notes},
};

use super::{
    format::IslandUnits,
    inputs::body_h,
    parts::{frame, hue, views},
    runtime::Molt,
    view::{Screen, View},
};
use crate::{
    components::{
        card::{Look, Motion, RAISED_GAP, RAISED_PAD},
        code::Scroll,
    },
    features::PanelCx,
    panel::{
        Block, Cell, Chip, Code, CodeLine, CodeMotion, Column, FrameView, HeaderLead,
        HeaderStatus, Item, Mark, Table, TableRow, Text, TextStyle, Tone, Trail, Views, Width,
    },
    theme::{CRITICAL, GREEN, MAUVE, OVERLAY2, SUBTEXT1, YELLOW},
    ui::{HEADING_SIZE, TITLE_SIZE},
};

const LEAD: f32 = 36.0;
const STEP_TIME_W: f32 = 64.0;

fn header(title: String, subtitle: String, target: Target, p: &PanelCx<'_>) -> Views {
    views(
        Some(HeaderLead {
            mark: p
                .glyph("molt-notes")
                .map_or(Mark::Letter("N".into(), MAUVE), |path| Mark::Glyph(path, MAUVE)),
            size: LEAD,
            overline: subtitle.into(),
            title: title.into(),
            title_color: None,
            on_click: None,
        }),
        Some(p.on_local::<Molt>(move |_, view: &mut View| view.open(Screen::Build(target)))),
        p,
    )
}

/// Markup as text with its styles and links.
fn rich(markup: &str, tone: Tone) -> Text {
    let (text, styled, links) = spans(markup);
    let mut spans: Vec<_> = styled
        .into_iter()
        .map(|(range, inline)| {
            (
                range,
                TextStyle {
                    bold: inline.bold,
                    italic: inline.italic,
                    strike: inline.strike,
                    underline: inline.underline,
                    code: inline.code,
                    ..TextStyle::default()
                },
            )
        })
        .collect();
    spans.extend(links.iter().map(|(range, _)| {
        (
            range.clone(),
            TextStyle {
                link: true,
                ..TextStyle::default()
            },
        )
    }));
    Text {
        spans,
        links,
        ..Text::new(tone, text)
    }
}

fn steps(steps: &[(String, String)]) -> Block {
    Block::Table(Table {
        columns: vec![
            Column {
                title: "".into(),
                width: Width::Share(1.0),
                end: false,
            },
            Column {
                title: "".into(),
                width: Width::Fixed(STEP_TIME_W),
                end: true,
            },
        ],
        rows: steps
            .iter()
            .map(|(step, at)| {
                TableRow::cells(
                    vec![
                        Cell::mono(format!("· {step}"), OVERLAY2),
                        Cell::text(at.clone(), OVERLAY2),
                    ],
                    Some(step.clone().into()),
                )
            })
            .collect(),
    })
}

fn release_blocks(release: &notes::ReleaseView) -> Vec<Block> {
    let mut blocks = Vec::new();
    if !release.tag.is_empty() {
        blocks.push(Block::Chip(Chip::new(release.tag.clone(), MAUVE)));
    }
    for block in &release.blocks {
        let tint = block.hue.map(hue);
        match block.kind {
            MarkdownBlockKind::Rule => blocks.push(Block::Rule),
            MarkdownBlockKind::Heading(level) => {
                let (text, _, _) = spans(&block.markup);
                blocks.push(Block::Item(
                    Item {
                        id: format!("molt-notes-heading-{text}").into(),
                        title: text.into(),
                        title_size: Some(match level {
                            1 => HEADING_SIZE,
                            2 => TITLE_SIZE,
                            _ => crate::ui::TEXT_SIZE,
                        }),
                        title_color: tint,
                        wrap: true,
                        ..Default::default()
                    },
                    Vec::new(),
                ));
            }
            MarkdownBlockKind::Item(depth) => {
                let indent = "    ".repeat(depth.saturating_sub(1));
                blocks.push(Block::Text(rich(
                    &format!("{indent}• {}", block.markup),
                    Tone::Body,
                )));
            }
            MarkdownBlockKind::Paragraph => blocks.push(Block::Text(rich(&block.markup, Tone::Body))),
            MarkdownBlockKind::Quote => {
                blocks.push(Block::Text(rich(&format!("<i>{}</i>", block.markup), Tone::Body)));
            }
            MarkdownBlockKind::Code => {
                let (text, _, _) = spans(&block.markup);
                blocks.push(Block::Code(Code {
                    heading: None,
                    lines: text.lines().map(CodeLine::plain).collect(),
                    wrap: true,
                    copy: None,
                    motion: CodeMotion::Still,
                    scroll: None,
                }));
            }
        }
    }
    blocks
}

pub(super) fn notes(notes: &Notes, target: Target, p: &PanelCx<'_>) -> (Views, Vec<Block>) {
    let shown = notes::view(notes, &IslandUnits);
    let versions: String = shown.versions.iter().map(|(text, _)| text.as_str()).collect();
    let mut head = header(
        shown.title.clone(),
        format!("{versions} · {}", shown.configuration),
        target,
        p,
    );
    if let Some((_, elapsed)) = &shown.loading {
        head.status = Some(HeaderStatus {
            text: elapsed.clone().into(),
            color: SUBTEXT1,
            busy: true,
        });
    }
    let body = body_h();
    let room = body - 2.0 * RAISED_PAD - 24.0 - RAISED_GAP;
    let mut held = Vec::new();
    if let Some((step, _)) = &shown.loading {
        held.push(Block::Item(
            Item {
                id: "molt-notes-step".into(),
                title: step.clone().into(),
                ..Default::default()
            },
            Vec::new(),
        ));
        held.push(steps(&shown.steps));
    }
    if let Some((title, error)) = &shown.error {
        held.push(Block::Frame(
            FrameView {
                id: "molt-notes-error".into(),
                look: Look::Washed(YELLOW),
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
                    id: "molt-notes-error-title".into(),
                    title: (*title).into(),
                    title_color: Some(YELLOW),
                    ..Default::default()
                },
                vec![Block::caption(error.clone(), YELLOW)],
            )],
        ));
        if let Some(title) = &shown.steps_title {
            held.push(Block::Item(
                Item {
                    id: "molt-notes-steps-title".into(),
                    title: title.clone().into(),
                    title_color: Some(OVERLAY2),
                    ..Default::default()
                },
                Vec::new(),
            ));
            held.push(steps(&shown.steps));
        }
    }
    if let Some(description) = &shown.description {
        held.push(Block::caption(description.clone(), SUBTEXT1));
    }
    if let Some(empty) = shown.empty {
        held.push(Block::note(empty));
    }
    for (index, release) in shown.releases.iter().enumerate() {
        if index > 0 {
            held.push(Block::Rule);
        }
        held.extend(release_blocks(release));
    }
    let title = Block::Item(
        Item {
            id: "molt-notes-heading".into(),
            title: shown.heading.clone().into(),
            title_size: Some(TITLE_SIZE),
            trail: shown
                .source
                .iter()
                .map(|source| Trail::Faint(source.clone().into()))
                .collect(),
            ..Default::default()
        },
        Vec::new(),
    );
    (
        head,
        vec![Block::Frame(
            frame("molt-notes".to_owned(), Some(body)),
            vec![
                title,
                Block::Scrolled("molt-notes-body".into(), room.max(0.0), held),
            ],
        )],
    )
}

pub(super) fn diff(review: &Review, index: usize, p: &PanelCx<'_>) -> (Views, Vec<Block>) {
    let Some(change) = review
        .report
        .config
        .as_ref()
        .ok()
        .and_then(|config| config.changes.get(index))
    else {
        return (
            header(String::new(), String::new(), review.report.target, p),
            Vec::new(),
        );
    };
    let shown = notes::diff(change, &review.report);
    let body = body_h();
    let room = body - 2.0 * RAISED_PAD - 24.0 - RAISED_GAP;
    let content = match shown.binary {
        Some(binary) => Block::note(binary),
        None => Block::Code(Code {
            heading: None,
            lines: shown
                .lines
                .iter()
                .map(|(text, kind)| CodeLine {
                    number: None,
                    text: text.clone().into(),
                    color: Some(match kind {
                        DiffLineKind::Hunk => (MAUVE, false),
                        DiffLineKind::Context => (SUBTEXT1, false),
                        DiffLineKind::Removed => (CRITICAL, true),
                        DiffLineKind::Added => (GREEN, true),
                    }),
                })
                .collect(),
            wrap: false,
            copy: None,
            motion: CodeMotion::Still,
            scroll: Some(Scroll {
                key: "molt-diff-lines".into(),
                height: room.max(0.0),
                follows: false,
            }),
        }),
    };
    (
        header(
            shown.title.clone(),
            shown.subtitle.clone(),
            review.report.target,
            p,
        ),
        vec![Block::Frame(
            frame("molt-diff".to_owned(), None),
            vec![
                Block::Item(
                    Item {
                        id: "molt-diff-heading".into(),
                        title: shown.heading.into(),
                        title_size: Some(TITLE_SIZE),
                        trail: vec![Trail::Faint(shown.note.into())],
                        ..Default::default()
                    },
                    Vec::new(),
                ),
                content,
            ],
        )],
    )
}
