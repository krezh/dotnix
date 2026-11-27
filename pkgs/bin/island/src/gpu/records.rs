//! Blocks that keep a record: code, a conversation, a timeline, a ledger, and the card asking for
//! one thing over a panel.

use std::{f32::consts::FRAC_PI_2, rc::Rc};

use super::{
    blocks::{BADGE_RADIUS, ROW_RADIUS, card_fill, control, input, mark, surface_fill, text_block},
    node::{self, MEDIUM, Node, SEMIBOLD, div, glyph, moving, text},
    panel::{Easing, PulseKind},
    widget::tinted,
};
use crate::{
    components::{
        badge::Mark,
        code::{Code, CodeLine, Motion},
        ledger::{self, Bar, Columns, Entry, Folded, Ledger},
        prompt::{Change, PromptView},
        table::{Cell, Column, Line, Row, Table, Width},
        text::{Style as TextStyle, Text},
        timeline::{Pin, Side, Span, Timeline},
        transcript::{Detail, Entry as ListEntry, Piece, Source, Transcript, Turn},
    },
    math::color,
    panel::{Handler, SharedString},
    stage::CONTENT_IN,
    theme::{
        ACCENT, ACCENT_TINT_ALPHA, BASE, CARD_BORDER_ALPHA, CONTROL_ALPHA, CRUST,
        NOTIFICATION_ALPHA, OVERLAY0, OVERLAY1, OVERLAY2, RED, SUBTEXT0, SUBTEXT1, SURFACE1, TEXT,
        TINT, TRACK_ALPHA,
    },
    ui::{CONTROL_H, CONTROL_RADIUS, PANEL_RADIUS, SMALL_SIZE, TEXT_SIZE, TITLE_SIZE},
};

pub(super) const CODE_SIZE: f32 = 12.0;
const CODE_LINE: f32 = 19.0;
pub(super) const CODE_PAD_X: f32 = 10.0;
pub(super) const CODE_PAD_Y: f32 = 8.0;
const NUMBER_W: f32 = 34.0;
pub(super) const GROUND_ALPHA: u8 = 0x80;

fn code_line(line: CodeLine, numbered: bool, wrap: bool) -> Node {
    let (tint, washed) = line.color.unwrap_or((TEXT, false));
    div()
        .flex()
        .px(if numbered { 0.0 } else { CODE_PAD_X })
        .color(tint)
        .when(washed, |row| row.bg(tinted(tint, 0x1a), 0.0))
        .when(numbered, |row| {
            row.child(
                div()
                    .w(NUMBER_W)
                    .flex_none()
                    .pr(CODE_PAD_X)
                    .flex()
                    .justify_end()
                    .color(OVERLAY1)
                    .children(line.number.map(|number| text(number.to_string()))),
            )
            .child(div().flex_none().w(CODE_SIZE))
        })
        .child(
            div()
                .min_w(0.0)
                .pr(CODE_PAD_X)
                .when(!wrap, |words| words.nowrap().clip())
                .child(text(line.text)),
        )
}

fn chip(label: impl Into<SharedString>, handler: Handler) -> Node {
    control(Some(surface_fill(false)), BADGE_RADIUS)
        .flex_none()
        .h(24.0)
        .px(8.0)
        .gap(4.0)
        .text_size(12.0)
        .weight(SEMIBOLD)
        .color(SUBTEXT1)
        .child(text(label).nowrap())
        .on_click(handler)
}

pub fn code(view: Code) -> Node {
    let numbered = view.lines.iter().any(|line| line.number.is_some());
    let wrap = view.wrap;
    let copy = view.copy.map(|handler| chip("Copy", handler));
    let heading = match (view.heading, copy.is_some()) {
        (Some(heading), _) => Some(heading),
        (None, true) => Some((SharedString::default(), SharedString::default())),
        (None, false) => None,
    };
    let rising = view.motion == Motion::Appear;
    let rows: Vec<Node> = view
        .lines
        .into_iter()
        .map(|line| code_line(line, numbered, wrap))
        .collect();
    let rule = tinted(SURFACE1, CARD_BORDER_ALPHA);
    div()
        .min_w(0.0)
        .clip()
        .bg(tinted(CRUST, GROUND_ALPHA), CONTROL_RADIUS)
        .when_some(heading, |block, (name, detail)| {
            block
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(12.0)
                        .px(CODE_PAD_X)
                        .py(6.0)
                        .text_size(CODE_SIZE)
                        .weight(SEMIBOLD)
                        .color(SUBTEXT1)
                        .child(div().min_w(0.0).truncate().child(text(name)))
                        .child(
                            div()
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap(8.0)
                                .child(div().mono().child(text(detail)))
                                .children(copy),
                        ),
                )
                .child(div().flex_none().h(1.0).bg(rule, 0.0))
        })
        .when(!rows.is_empty(), |block| {
            let lines = div()
                .py(CODE_PAD_Y)
                .col()
                .mono()
                .weight(400)
                .text_size(CODE_SIZE)
                .line(CODE_LINE)
                .children(rows)
                .when(rising, Node::rises);
            block.child(match view.scroll {
                Some(scroll) => div()
                    .max_h(scroll.height)
                    .scrolls(scroll.key)
                    .soft_edges()
                    .when(scroll.follows, Node::follows_end)
                    .child(lines),
                None => lines,
            })
        })
}

const MARK: f32 = 28.0;
const MARK_GLYPH: f32 = 15.0;
const TURN_GAP: f32 = 16.0;
const PIECE_GAP: f32 = 12.0;
const BUBBLE_PAD_X: f32 = 14.0;
const BUBBLE_PAD_Y: f32 = 10.0;
const REPLY_PAD: f32 = 16.0;
const OVERLINE: f32 = 12.0;
const BUBBLE_SHARE: f32 = 0.7;
const DOT: f32 = 6.0;

fn turn_mark(path: Option<&std::path::Path>, tint: u32, alive: bool) -> Node {
    div()
        .flex_none()
        .size(MARK)
        .flex()
        .items_center()
        .justify_center()
        .bg(tinted(tint, ACCENT_TINT_ALPHA), ROW_RADIUS)
        .when(alive, |lead| {
            lead.child(
                node::pulse(PulseKind::Glow, color(tint), ROW_RADIUS)
                    .absolute()
                    .inset(0.0),
            )
        })
        .children(path.map(|path| glyph(path, MARK_GLYPH, tint)))
}

fn dots() -> Node {
    div().flex().gap(4.0).py(6.0).children((0..3).map(|index| {
        node::pulse(PulseKind::Dot(index), color(OVERLAY2), DOT / 2.0)
            .size(DOT)
            .flex_none()
    }))
}

fn reply_surface() -> Node {
    div()
        .bg(tinted(SURFACE1, NOTIFICATION_ALPHA), PANEL_RADIUS)
        .outline(tinted(SURFACE1, CARD_BORDER_ALPHA), PANEL_RADIUS)
}

fn overline(words: SharedString, tint: u32) -> Node {
    div()
        .text_size(OVERLINE)
        .weight(SEMIBOLD)
        .color(tint)
        .child(text(words))
}

const DETAIL_GAP: f32 = 4.0;
const DETAIL_LINE_SIZE: f32 = 12.0;
const DETAIL_GLYPH: f32 = 13.0;
const READING_LINE: f32 = 22.0;
const ENTRY_GAP: f32 = 5.0;
const BULLET_W: f32 = 16.0;
const NUMBER_MARK_W: f32 = 20.0;
const MARK_GAP: f32 = 4.0;
const BULLET: f32 = 5.0;
const QUOTE_RULE: f32 = 2.0;
const QUOTE_GAP: f32 = 10.0;
const SOURCE_NUMBER: f32 = 15.0;
const SOURCE_NUMBER_SIZE: f32 = 10.0;
const SOURCE_NUMBER_ALPHA: u8 = 0x38;

fn detail_line(detail: Detail) -> Node {
    div()
        .flex()
        .items_center()
        .gap(6.0)
        .text_size(DETAIL_LINE_SIZE)
        .children(
            detail
                .glyph
                .map(|path| glyph(&path, DETAIL_GLYPH, detail.tint)),
        )
        .child({
            let mut line = String::new();
            let styles = detail
                .parts
                .into_iter()
                .map(|(words, tint)| {
                    let start = line.len();
                    line.push_str(&words);
                    let style = TextStyle {
                        color: Some(tint),
                        ..TextStyle::default()
                    };
                    (start..line.len(), style)
                })
                .collect();
            div()
                .flex_1()
                .min_w(0.0)
                .child(node::styled(line, styles, Vec::new(), Some(1)))
        })
}

/// Each entry's mark in a column of its own, so wrapped lines stay under its words, and entries
/// further in starting under the words of the one out from them.
fn list(entries: Vec<ListEntry>) -> Node {
    div()
        .col()
        .gap(ENTRY_GAP)
        .children(entries.into_iter().map(|entry| {
            let numbered = entry.number.is_some();
            let mark_w = if numbered { NUMBER_MARK_W } else { BULLET_W };
            let mark = match entry.number {
                Some(number) => div()
                    .text_size(TEXT_SIZE)
                    .line(READING_LINE)
                    .weight(MEDIUM)
                    .color(OVERLAY2)
                    .child(text(format!("{number}.")).nowrap()),
                None => {
                    let dot = div().size(BULLET).ml(3.0).mt((READING_LINE - BULLET) / 2.0);
                    if entry.depth == 0 {
                        dot.bg(color(OVERLAY2), BULLET / 2.0)
                    } else {
                        dot.outline(color(OVERLAY1), BULLET / 2.0)
                    }
                }
            };
            div()
                .flex()
                .items_start()
                .gap(MARK_GAP)
                .pl(entry.depth as f32 * (BULLET_W + MARK_GAP))
                .child(div().w(mark_w).flex_none().child(mark))
                .child(div().flex_1().min_w(0.0).child(text_block(&entry.text)))
        }))
}

fn quote(content: &Text) -> Node {
    div()
        .flex()
        .gap(QUOTE_GAP)
        .child(
            div()
                .w(QUOTE_RULE)
                .flex_none()
                .bg(color(OVERLAY0), QUOTE_RULE / 2.0),
        )
        .child(
            div()
                .flex_1()
                .min_w(0.0)
                .color(SUBTEXT0)
                .child(text_block(content)),
        )
}

fn chip_rule() -> Node {
    div()
        .w(1.0)
        .h(16.0)
        .mx(2.0)
        .flex_none()
        .bg(tinted(OVERLAY0, GROUND_ALPHA), 0.0)
}

fn source_chip(number: usize, source: Source, tint: u32) -> Node {
    control(Some(surface_fill(false)), BADGE_RADIUS)
        .flex_none()
        .h(24.0)
        .pl(5.0)
        .pr(9.0)
        .gap(6.0)
        .items_center()
        .text_size(12.0)
        .weight(SEMIBOLD)
        .color(SUBTEXT1)
        .child(
            div()
                .size(SOURCE_NUMBER)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .bg(tinted(tint, SOURCE_NUMBER_ALPHA), 4.0)
                .text_size(SOURCE_NUMBER_SIZE)
                .color(tint)
                .child(text(number.to_string())),
        )
        .child(text(source.label).nowrap())
        .on_click(source.on_click)
}

fn mine(content: &Text) -> Node {
    div().flex().justify_end().child(
        div()
            .max_w_share(BUBBLE_SHARE)
            .px(BUBBLE_PAD_X)
            .py(BUBBLE_PAD_Y)
            .bg(tinted(ACCENT, ACCENT_TINT_ALPHA), CONTROL_RADIUS)
            .child(text_block(content)),
    )
}

pub fn transcript(view: Transcript) -> Node {
    let Transcript {
        turns,
        glyph: path,
        color: tint,
        appear_from,
        ..
    } = view;
    let rows: Vec<Node> = turns
        .into_iter()
        .enumerate()
        .map(|(index, turn)| (index >= appear_from, turn))
        .map(|(rising, turn)| {
            (
                rising,
                match turn {
                    Turn::Mine(content) => mine(&content),
                    Turn::Theirs {
                        overline: line,
                        detail,
                        pieces,
                        streaming,
                        actions,
                        sources,
                        source_tint,
                    } => {
                        let count = pieces.len();
                        let body: Vec<Node> = pieces
                            .into_iter()
                            .enumerate()
                            .map(|(part, piece)| {
                                let shown = match piece {
                                    Piece::Text(content) => text_block(&content),
                                    Piece::List(entries) => list(entries),
                                    Piece::Quote(content) => quote(&content),
                                    Piece::Table(view) => table(view),
                                    Piece::Code(block) => code(block),
                                };
                                if streaming && part + 1 == count {
                                    div()
                                        .flex()
                                        .items_end()
                                        .gap(2.0)
                                        .child(div().min_w(0.0).child(shown))
                                        .child(node::caret(14.0))
                                } else {
                                    shown
                                }
                            })
                            .collect();
                        let mut chips: Vec<Node> = actions
                            .into_iter()
                            .map(|(label, handler)| chip(label, handler))
                            .collect();
                        if !sources.is_empty() {
                            if !chips.is_empty() {
                                chips.push(chip_rule());
                            }
                            chips.extend(sources.into_iter().enumerate().map(|(index, source)| {
                                source_chip(index + 1, source, source_tint)
                            }));
                        }
                        div()
                            .flex()
                            .items_start()
                            .gap(10.0)
                            .child(turn_mark(path.as_deref(), tint, streaming))
                            .child(
                                reply_surface().flex_1().min_w(0.0).child(
                                    div()
                                        .p(REPLY_PAD)
                                        .col()
                                        .gap(PIECE_GAP)
                                        .text_size(SMALL_SIZE)
                                        .child(
                                            div()
                                                .col()
                                                .gap(DETAIL_GAP)
                                                .child(overline(line, tint))
                                                .children(detail.map(detail_line)),
                                        )
                                        .children(body)
                                        .when(!chips.is_empty(), |reply| {
                                            reply.child(
                                                div()
                                                    .flex()
                                                    .wrap()
                                                    .items_center()
                                                    .gap(6.0)
                                                    .children(chips),
                                            )
                                        }),
                                ),
                            )
                    }
                    Turn::Waiting { overline: line } => div()
                        .flex()
                        .items_start()
                        .gap(10.0)
                        .child(turn_mark(path.as_deref(), tint, true))
                        .child(
                            reply_surface().child(
                                div()
                                    .px(REPLY_PAD)
                                    .py(REPLY_PAD - 2.0)
                                    .col()
                                    .gap(2.0)
                                    .child(overline(line, tint))
                                    .child(dots()),
                            ),
                        ),
                },
            )
        })
        .map(|(rising, row)| row.when(rising, Node::rises))
        .collect();
    div().col().gap(TURN_GAP).children(rows)
}

const SIDE_W: f32 = 148.0;
const SECTION_GAP: f32 = 16.0;
const TRACK_RIGHT_PAD: f32 = 14.0;
const LANE_H: f32 = 18.0;
const LANE_GAP: f32 = 5.0;
const PIN_H: f32 = 16.0;
const AXIS_H: f32 = 14.0;
const SIDE_TITLE: f32 = 13.5;
const SIDE_SUB: f32 = 11.5;
const KEY_SIZE: f32 = 10.5;
const CHIP_SIZE: f32 = 10.5;
const AXIS_SIZE: f32 = 10.8;
const STRIPE_ALPHA: u8 = 0x99;

fn span_node(span: Span) -> Node {
    let from = span.from.clamp(0.0, 1.0);
    let to = span.to.clamp(from, 1.0);
    let width = (to - from).max(0.025);
    let tint = span.color;
    let labelled = width >= 0.16 && !span.label.is_empty();
    let bar = div()
        .size_full()
        .bg(tinted(tint, 0x22), 5.0)
        .outline(tinted(tint, 0x99), 5.0)
        .clip()
        .child(
            node::stripes(tinted(tint, STRIPE_ALPHA))
                .absolute()
                .inset(1.0),
        )
        .when(labelled, |bar| {
            bar.child(
                div()
                    .absolute()
                    .top(1.0)
                    .bottom(1.0)
                    .left(6.0)
                    .right(9.0)
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .px(5.5)
                            .py(0.5)
                            .bg(tinted(BASE, 0xe0), 3.5)
                            .truncate()
                            .text_size(CHIP_SIZE)
                            .weight(SEMIBOLD)
                            .color(TEXT)
                            .child(text(span.label)),
                    ),
            )
        });
    div()
        .absolute()
        .top(0.0)
        .left_share(from)
        .w_share(width)
        .eases(Easing::Stretch { left: 0.0 }, width, CONTENT_IN)
        .eases(Easing::Along, from, CONTENT_IN)
        .h(LANE_H)
        .child(bar)
        .when(span.start_tick, |outer| {
            outer.child(
                div()
                    .absolute()
                    .left(-1.0)
                    .top(-2.0)
                    .w(2.2)
                    .h(LANE_H + 4.0)
                    .bg(color(tint), 1.1),
            )
        })
        .when(span.end_dot, |outer| {
            outer.child(
                div()
                    .absolute()
                    .right(-4.5)
                    .top((LANE_H - 9.0) / 2.0)
                    .size(9.0)
                    .bg(color(BASE), 4.5)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().size(6.0).bg(color(TEXT), 3.0)),
            )
        })
        .tip(span.tooltip)
}

fn pin_node(pin: Pin, track_w: f32) -> Node {
    let x = pin.at.clamp(0.0, 1.0);
    let least = -x * track_w;
    let most = ((1.0 - x) * track_w - pin.width).max(least);
    div()
        .absolute()
        .top(0.0)
        .left_share(x)
        .eases(Easing::Along, x, CONTENT_IN)
        .ml((-pin.width / 2.0).clamp(least, most))
        .h(PIN_H)
        .px(8.0)
        .bg(tinted(BASE, 0xd8), PIN_H / 2.0)
        .outline(tinted(SURFACE1, 0x99), PIN_H / 2.0)
        .flex()
        .items_center()
        .gap(5.0)
        .nowrap()
        .text_size(CHIP_SIZE)
        .weight(MEDIUM)
        .color(SUBTEXT1)
        .child(div().size(5.5).flex_none().bg(color(pin.color), 2.75))
        .child(text(pin.label))
        .tip(pin.tooltip)
}

fn side_column(side: Side) -> Node {
    div()
        .w(SIDE_W)
        .flex_none()
        .col()
        .justify_between()
        .gap(4.0)
        .child(
            div()
                .col()
                .gap(2.0)
                .child(
                    div()
                        .text_size(SIDE_TITLE)
                        .weight(SEMIBOLD)
                        .color(TEXT)
                        .child(text(side.title)),
                )
                .child(
                    div()
                        .text_size(SIDE_SUB)
                        .color(OVERLAY2)
                        .child(moving(side.detail)),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(10.0)
                .text_size(KEY_SIZE)
                .color(OVERLAY2)
                .when_some(side.striped_key, |key, label| {
                    key.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(4.5)
                            .child(
                                div()
                                    .w(13.0)
                                    .h(8.0)
                                    .bg(tinted(RED, 0x22), 2.5)
                                    .outline(tinted(RED, 0x88), 2.5)
                                    .clip()
                                    .child(
                                        node::stripes(tinted(RED, STRIPE_ALPHA))
                                            .absolute()
                                            .inset(0.0),
                                    ),
                            )
                            .child(text(label)),
                    )
                })
                .when_some(side.dot_key, |key, label| {
                    key.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(4.0)
                            .child(div().size(5.5).bg(color(TEXT), 2.75))
                            .child(text(label)),
                    )
                }),
        )
}

pub fn timeline(view: Timeline) -> Node {
    let Timeline {
        side,
        lanes,
        pins,
        axis,
    } = view;
    let ticks = axis.len();
    let segments = ticks.saturating_sub(1).max(1) as f32;
    let track_w = if side.is_some() { 624.0 } else { 774.0 };
    let swimlanes = div()
        .w_full()
        .py(2.0)
        .col()
        .gap(LANE_GAP)
        .child(
            node::gridlines(ticks, tinted(SURFACE1, 0x66))
                .absolute()
                .inset(0.0),
        )
        .children(lanes.into_iter().map(|lane| {
            div()
                .w_full()
                .h(LANE_H)
                .children(lane.into_iter().map(span_node))
        }))
        .when(!pins.is_empty(), |column| {
            column.child(
                div()
                    .w_full()
                    .h(PIN_H)
                    .children(pins.into_iter().map(|pin| pin_node(pin, track_w))),
            )
        });
    let axis_row = div()
        .w_full()
        .h(AXIS_H)
        .text_size(AXIS_SIZE)
        .color(OVERLAY1)
        .children(axis.into_iter().enumerate().map(|(index, label)| {
            div()
                .absolute()
                .top(0.0)
                .left_share(index as f32 / segments)
                .ml(-18.0)
                .w(36.0)
                .flex()
                .justify_center()
                .nowrap()
                .child(text(label))
        }));
    let has_side = side.is_some();
    div()
        .flex()
        .gap(SECTION_GAP)
        .children(side.map(side_column))
        .child(
            div()
                .flex_1()
                .min_w(0.0)
                .when(!has_side, |track| track.pl(TRACK_RIGHT_PAD))
                .pr(TRACK_RIGHT_PAD)
                .col()
                .justify_between()
                .gap(6.0)
                .child(swimlanes)
                .child(axis_row),
        )
}

const ENTRY_W: f32 = 228.0;
/// The name column of a compact ledger.
const COMPACT_ENTRY_W: f32 = 150.0;
const LABEL_W: f32 = 126.0;
const VALUE_W: f32 = 46.0;
const NOTE_W: f32 = 124.0;
const COL_GAP: f32 = 10.0;
const LEDGER_GAP: f32 = 14.0;
const MAX_ROWS_H: f32 = 320.0;
const HEAD_SIZE: f32 = 11.0;
const DETAIL_SIZE: f32 = 12.0;
const SUB_SIZE: f32 = 11.5;
const MARK_BOX: f32 = 28.0;
/// The tick before a row that can be left out, and its box.
const PICK_BOX: f32 = 24.0;
const PICK_MARK: f32 = 20.0;
const BAR_H: f32 = 6.0;
const TICK_W: f32 = 2.0;
const TICK_H: f32 = 12.0;
const HALO_W: f32 = 4.0;
const HALO_H: f32 = 14.0;
const CHEVRON: f32 = 13.0;
const AHEAD_ALPHA: u8 = 0x47;
const ROW_TINT_ALPHA: u8 = 0x10;

fn ledger_track(dimmed: bool) -> [f32; 4] {
    tinted(
        TINT,
        if dimmed {
            TRACK_ALPHA / 3
        } else {
            TRACK_ALPHA / 2
        },
    )
}

fn chevron(path: Option<&std::path::PathBuf>, expanded: bool) -> Option<Node> {
    let turn = if expanded { FRAC_PI_2 } else { 0.0 };
    Some(node::glyph(path?, CHEVRON, OVERLAY2).flex_none().eases(
        Easing::Turn,
        turn,
        ledger::TURN_SPRING,
    ))
}

fn bar_meter(bar: &Bar) -> Node {
    let fraction = bar.fraction.clamp(0.0, 1.0);
    let dimmed = fraction == 0.0 && bar.ahead.is_none();
    let round = BAR_H / 2.0;
    let track = div()
        .size_full()
        .bg(ledger_track(dimmed), round)
        .clip()
        .when_some(bar.ahead, |track, ahead| {
            track.child(
                div()
                    .absolute()
                    .top(0.0)
                    .left(0.0)
                    .h_full()
                    .springy(ahead.clamp(fraction, 1.0))
                    .bg(tinted(bar.color, AHEAD_ALPHA), round),
            )
        })
        .child(
            div()
                .absolute()
                .top(0.0)
                .left(0.0)
                .h_full()
                .springy(fraction)
                .bg(color(bar.color), round),
        );
    div()
        .flex_1()
        .min_w(0.0)
        .h(BAR_H)
        .child(track)
        .when_some(bar.mark, |outer, at| {
            outer.child(
                div()
                    .absolute()
                    .top((BAR_H - HALO_H) / 2.0)
                    .left_share(at.clamp(0.0, 1.0))
                    .eases(Easing::Along, at.clamp(0.0, 1.0), CONTENT_IN)
                    .ml(-HALO_W / 2.0)
                    .w(HALO_W)
                    .h(HALO_H)
                    .bg(color(BASE), HALO_W / 2.0)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().w(TICK_W).h(TICK_H).bg(color(TEXT), TICK_W / 2.0)),
            )
        })
}

fn value_cell(value: SharedString, tint: u32, weight: u16) -> Node {
    div()
        .w(VALUE_W)
        .flex_none()
        .flex()
        .justify_end()
        .nowrap()
        .text_size(DETAIL_SIZE)
        .weight(weight)
        .color(tint)
        .child(moving(value))
}

fn note_cell(note: SharedString, tint: u32, trail: Option<Node>) -> Node {
    div()
        .w(NOTE_W)
        .flex_none()
        .flex()
        .items_center()
        .justify_end()
        .gap(3.0)
        .nowrap()
        .text_size(SUB_SIZE)
        .color(tint)
        .child(div().min_w(0.0).truncate().child(moving(note)))
        .children(trail)
}

fn bar_line(bar: Bar, trail: Option<Node>, compact: bool) -> Node {
    let meter = bar_meter(&bar);
    let rising = bar.appear;
    let label = !(compact && bar.label.is_empty());
    let note = !(compact && bar.note.is_empty() && trail.is_none());
    div()
        .h(21.0)
        .flex()
        .items_center()
        .gap(COL_GAP)
        .when(label, |line| {
            line.child(
                div()
                    .w(LABEL_W)
                    .flex_none()
                    .truncate()
                    .text_size(DETAIL_SIZE)
                    .weight(MEDIUM)
                    .color(bar.label_color)
                    .child(text(bar.label)),
            )
        })
        .child(meter)
        .child(value_cell(bar.value, bar.value_color, SEMIBOLD))
        .when(note, |line| {
            line.child(note_cell(bar.note, bar.note_color, trail))
        })
        .when(rising, Node::rises)
}

fn folded_line(folded: Folded, trail: Option<Node>) -> Node {
    div()
        .h(21.0)
        .flex()
        .items_center()
        .gap(COL_GAP)
        .child(
            div()
                .w(LABEL_W)
                .flex_none()
                .truncate()
                .text_size(DETAIL_SIZE)
                .color(OVERLAY2)
                .child(text(folded.label)),
        )
        .child(
            div()
                .flex_1()
                .min_w(0.0)
                .h(BAR_H - 0.5)
                .bg(ledger_track(true), BAR_H / 2.0),
        )
        .child(value_cell(folded.value, OVERLAY2, MEDIUM))
        .child(note_cell(folded.note, OVERLAY2, trail))
}

fn entry_lead(entry: &Entry, key: &str, compact: bool) -> Node {
    let own_shape = matches!(entry.mark, Mark::Letter(..) | Mark::Key(..));
    let fill = (!own_shape).then(|| match (entry.faded, entry.tint) {
        (true, _) => tinted(TINT, 0x10),
        (false, Some(tint)) => tinted(tint, ACCENT_TINT_ALPHA / 2),
        (false, None) => tinted(TINT, 0x14),
    });
    let lead = div()
        .size(MARK_BOX)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .when_some(fill, |lead, fill| lead.bg(fill, ROW_RADIUS))
        .child(
            div()
                .when(entry.faded, |mark| mark.opacity(0.45))
                .child(mark(
                    &entry.mark,
                    MARK_BOX,
                    if own_shape { 1.0 } else { 0.58 },
                    key,
                )),
        );
    let chip = entry.chip.as_ref().map(|(words, tint)| {
        div()
            .flex_none()
            .px(6.0)
            .py(1.0)
            .bg(tinted(*tint, 0x24), 8.0)
            .text_size(CHIP_SIZE)
            .weight(SEMIBOLD)
            .color(*tint)
            .nowrap()
            .child(text(words.clone()))
            .when_some(entry.on_chip.clone(), |chip, handler| {
                chip.hover(8.0).on_click(handler)
            })
    });
    let pick = entry.pick.as_ref().map(|(picked, handler)| {
        div()
            .size(PICK_BOX)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .hover(ROW_RADIUS)
            .on_click(handler.clone())
            .child(mark(
                &Mark::Choice {
                    picked: *picked,
                    many: true,
                },
                PICK_MARK,
                1.0,
                &format!("{key}-pick"),
            ))
    });
    div()
        .w(if compact { COMPACT_ENTRY_W } else { ENTRY_W }
            + pick.as_ref().map_or(0.0, |_| PICK_BOX + 10.0))
        .flex_none()
        .flex()
        .items_center()
        .gap(10.0)
        .children(pick)
        .child(lead)
        .child(
            div()
                .flex_1()
                .min_w(0.0)
                .col()
                .gap(1.0)
                .child(
                    div()
                        .truncate()
                        .text_size(SMALL_SIZE)
                        .weight(SEMIBOLD)
                        .color(if entry.faded { SUBTEXT1 } else { TEXT })
                        .child(text(entry.title.clone())),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(6.0)
                        .min_w(0.0)
                        .when(!entry.subtitle.is_empty(), |sub| {
                            sub.child(
                                div()
                                    .truncate()
                                    .text_size(SUB_SIZE)
                                    .color(OVERLAY2)
                                    .child(text(entry.subtitle.clone())),
                            )
                        })
                        .children(chip),
                ),
        )
}

fn entry_row(
    at: &str,
    index: usize,
    entry: Entry,
    chevron_path: Option<&std::path::PathBuf>,
    compact: bool,
) -> Node {
    let key = format!("{at}-{}", entry.key);
    let lines_in = (entry.bars.len() + usize::from(entry.folded.is_some())).max(1);
    let single = lines_in == 1;
    let left = entry_lead(&entry, &key, compact);
    let accent = entry.accent;
    let whole = entry.on_toggle.is_some() && (entry.bars.is_empty() || entry.folded.is_none());
    let sub = entry.on_toggle.is_some() && !whole;
    let expanded = entry.expanded;
    let on_toggle = entry.on_toggle;
    let count = entry.bars.len();
    let mut lines: Vec<Node> = entry
        .bars
        .into_iter()
        .enumerate()
        .map(|(number, bar)| {
            let trail = (whole && number + 1 == count)
                .then(|| chevron(chevron_path, expanded))
                .flatten();
            bar_line(bar, trail, compact)
        })
        .collect();
    if let Some(folded) = entry.folded {
        let line = folded_line(folded, chevron(chevron_path, expanded));
        match on_toggle.clone().filter(|_| sub) {
            Some(handler) => lines.push(
                control(None, ROW_RADIUS)
                    .w_full()
                    .px(4.0)
                    .mx(-4.0)
                    .justify_start()
                    .child(line.w_full())
                    .on_click(handler),
            ),
            None => lines.push(line),
        }
    }
    let inner = div()
        .w_full()
        .py(8.0)
        .pl(10.0)
        .pr(4.0)
        .flex()
        .when(single, Node::items_center)
        .when(!single, Node::items_start)
        .gap(LEDGER_GAP)
        .when_some(accent, |row, tint| {
            row.child(
                div()
                    .absolute()
                    .left(0.0)
                    .top(9.0)
                    .bottom(9.0)
                    .w(3.0)
                    .bg(color(tint), 1.5),
            )
        })
        .child(left.when(!single, |cell| cell.pt(2.0)))
        .child(div().flex_1().min_w(0.0).col().gap(3.0).children(lines));
    let rest = (accent == Some(RED)).then(|| tinted(RED, ROW_TINT_ALPHA));
    let body = match on_toggle.filter(|_| whole) {
        Some(handler) => control(rest, ROW_RADIUS)
            .w_full()
            .justify_start()
            .child(inner)
            .on_click(handler),
        None => div()
            .w_full()
            .when_some(rest, |row, fill| row.bg(fill, 10.0))
            .child(inner),
    };
    div()
        .w_full()
        .when(index > 0, |wrapper| {
            wrapper
                .pt(2.0)
                .mt(2.0)
                .rule(tinted(SURFACE1, CONTROL_ALPHA))
        })
        .child(body)
}

fn ledger_header(columns: Columns) -> Node {
    let keyed = columns.mark_key.is_some() || columns.ahead_key.is_some();
    let legend = keyed.then(|| {
        div()
            .flex()
            .items_center()
            .gap(10.0)
            .text_size(HEAD_SIZE)
            .weight(400)
            .color(OVERLAY2)
            .when_some(columns.mark_key, |legend, label| {
                legend.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(4.0)
                        .child(div().w(2.0).h(9.0).bg(color(TEXT), 1.0))
                        .child(text(label)),
                )
            })
            .when_some(columns.ahead_key, |legend, label| {
                legend.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(4.0)
                        .child(div().w(13.0).h(5.5).bg(tinted(TEXT, AHEAD_ALPHA), 2.75))
                        .child(text(label)),
                )
            })
    });
    div()
        .col()
        .mb(2.0)
        .child(
            div()
                .pl(10.0)
                .pr(4.0)
                .pb(8.0)
                .flex()
                .items_center()
                .gap(LEDGER_GAP)
                .text_size(HEAD_SIZE)
                .weight(SEMIBOLD)
                .color(OVERLAY1)
                .child(
                    div()
                        .w(ENTRY_W)
                        .flex_none()
                        .truncate()
                        .child(text(columns.entry)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(0.0)
                        .flex()
                        .items_center()
                        .gap(COL_GAP)
                        .child(div().w(LABEL_W).flex_none().child(text(columns.label)))
                        .child(
                            div()
                                .flex_1()
                                .min_w(0.0)
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(text(columns.bar))
                                .children(legend),
                        )
                        .child(
                            div()
                                .w(VALUE_W)
                                .flex_none()
                                .flex()
                                .justify_end()
                                .child(text(columns.value)),
                        )
                        .child(
                            div()
                                .w(NOTE_W)
                                .flex_none()
                                .flex()
                                .justify_end()
                                .child(text(columns.note)),
                        ),
                ),
        )
        .child(
            div()
                .flex_none()
                .h(1.0)
                .bg(tinted(SURFACE1, CONTROL_ALPHA), 0.0),
        )
}

pub fn ledger(view: Ledger, at: &str) -> Node {
    let overflows = view.overflows();
    let path = view.chevron;
    let compact = view.compact;
    let rows: Vec<Node> = view
        .entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| entry_row(at, index, entry, path.as_ref(), compact))
        .collect();
    let list = div().col().children(rows);
    let body = if overflows {
        div()
            .max_h(MAX_ROWS_H)
            .scrolls(format!("{at}-ledger-scroll"))
            .soft_edges()
            .child(list)
    } else {
        list
    };
    div()
        .col()
        .children(view.columns.map(ledger_header))
        .child(body)
}

const TABLE_ROW_H: f32 = 28.0;
const TABLE_HEADING_H: f32 = 34.0;
const TABLE_GAP: f32 = 14.0;

fn table_column(column: &Column, cell: Node) -> Node {
    let slot = match column.width {
        Width::Share(share) => div().flex_1().grow(share).min_w(0.0),
        Width::Fixed(width) => div().w(width).flex_none(),
    };
    slot.flex()
        .items_center()
        .when(column.end, Node::justify_end)
        .child(cell)
}

fn table_cell(cell: Cell) -> Node {
    match cell {
        Cell::Text {
            text: words,
            color: tint,
            strong,
            mono,
        } => div()
            .min_w(0.0)
            .color(tint)
            .weight(if strong { SEMIBOLD } else { 400 })
            .when(mono, |cell| cell.mono().text_size(SMALL_SIZE))
            .child(moving(words)),
        Cell::Parts(parts, mono) => div()
            .min_w(0.0)
            .wrap()
            .items_center()
            .when(mono, |cell| cell.mono().text_size(SMALL_SIZE))
            .children(
                parts
                    .into_iter()
                    .map(|(words, tint)| div().min_w(0.0).color(tint).child(moving(words))),
            ),
        Cell::Dot(tint) => div().size(TABLE_DOT).flex_none().bg(color(tint), TABLE_DOT),
        Cell::Bar(parts) if parts.is_empty() => div(),
        Cell::Bar(parts) => parts_bar(parts, BAR_H),
    }
}

/// Shares of the whole laid end to end from the start, each in its color, over a track this tall.
pub(super) fn parts_bar(parts: Vec<(f32, u32)>, height: f32) -> Node {
    let mut at = 0.0;
    let fills: Vec<Node> = parts
        .into_iter()
        .map(|(share, tint)| {
            let share = share.clamp(0.0, 1.0 - at);
            let fill = div()
                .absolute()
                .top(0.0)
                .h_full()
                .left_share(at)
                .springy(share)
                .bg(color(tint), height / 2.0);
            at += share;
            fill
        })
        .collect();
    div()
        .w_full()
        .h(height)
        .bg(ledger_track(false), height / 2.0)
        .clip()
        .children(fills)
}

pub fn table(view: Table) -> Node {
    let headings = view
        .columns
        .iter()
        .any(|column| !column.title.is_empty())
        .then(|| {
            let titles = div()
                .h(TABLE_ROW_H)
                .px(TABLE_INSET)
                .flex()
                .items_center()
                .gap(TABLE_GAP)
                .text_size(SMALL_SIZE)
                .weight(MEDIUM)
                .color(OVERLAY2)
                .children(view.columns.iter().map(|column| {
                    table_column(column, div().truncate().child(text(column.title.clone())))
                }));
            div()
                .flex_none()
                .col()
                .child(titles)
                .child(div().h(1.0).bg(tinted(SURFACE1, CONTROL_ALPHA), 0.0))
        });
    let rows = view.rows.into_iter().map(|row| match row {
        Row::Heading(chip) => div()
            .flex_none()
            .h(TABLE_HEADING_H)
            .flex()
            .items_end()
            .pb(4.0)
            .child(super::blocks::chip(chip)),
        Row::Group(words, tint) => div()
            .flex_none()
            .h(TABLE_HEADING_H)
            .flex()
            .items_end()
            .pb(6.0)
            .text_size(SMALL_SIZE)
            .weight(SEMIBOLD)
            .color(tint)
            .child(moving(words).nowrap()),
        Row::Line(line) => table_line(&view.columns, line),
    });
    div().col().children(headings).children(rows)
}

fn table_line(columns: &[Column], line: Line) -> Node {
    let Line {
        cells,
        tip,
        under,
        wash,
        busy,
        on_click,
    } = line;
    let two = under.is_some();
    let mut under = under;
    let cells = columns
        .iter()
        .zip(cells)
        .enumerate()
        .map(|(index, (column, cell))| {
            let cell = table_cell(cell);
            let cell = match under.take_if(|(at, _)| *at == index) {
                Some((_, words)) => div()
                    .min_w(0.0)
                    .col()
                    .gap(1.0)
                    .child(cell.weight(SEMIBOLD))
                    .child(
                        div()
                            .min_w(0.0)
                            .mono()
                            .text_size(SMALL_SIZE)
                            .color(OVERLAY2)
                            .child(moving(words)),
                    ),
                None => cell,
            };
            table_column(column, cell)
        });
    let row = div()
        .flex()
        .items_center()
        .py(TABLE_ROW_PAD)
        .gap(TABLE_GAP)
        .text_size(TEXT_SIZE)
        .children(cells);
    let height = if two { TABLE_TWO_H } else { TABLE_ROW_H };
    let held = match on_click {
        Some(handler) => control(None, ROW_RADIUS)
            .justify_start()
            .child(row.flex_1().min_w(0.0))
            .on_click(handler),
        None => div().col().justify_center().child(row),
    };
    held.flex_none()
        .min_h(height)
        .px(TABLE_INSET)
        .when_some(wash, |held, tint| {
            held.bg(tinted(tint, TABLE_WASH_ALPHA), ROW_RADIUS)
        })
        .when(busy, |held| {
            held.bg(tinted(SURFACE1, CONTROL_ALPHA), ROW_RADIUS)
                .beams(true)
        })
        .when_some(tip, Node::tip)
}

const TABLE_DOT: f32 = 8.0;
/// Over and under a row's cells, which grow as long words wrap rather than being cut short.
const TABLE_ROW_PAD: f32 = 4.0;
const TABLE_TWO_H: f32 = 46.0;
/// Rows and headings sit this far in from the table's sides, under their group's chip, so a row's
/// hover or wash has room around its words.
const TABLE_INSET: f32 = 10.0;
const TABLE_WASH_ALPHA: u8 = 0x1a;

const CAPTION_SIZE: f32 = 12.0;
const PROMPT_W: f32 = 340.0;
const PROMPT_PAD: f32 = 16.0;
const SCRIM_ALPHA: u8 = 0x99;
const PAIRING_SIZE: f32 = 30.0;
const PROMPT_ICON: f32 = 36.0;
const PROMPT_GLYPH: f32 = 18.0;

const CHANGE_LABEL_W: f32 = 84.0;
const CHANGE_VALUE_W: f32 = 34.0;

fn prompt_button(label: impl Into<SharedString>, lit: bool, busy: bool, handler: Handler) -> Node {
    let tint = if lit { ACCENT } else { TEXT };
    control(Some(surface_fill(lit)), CONTROL_RADIUS)
        .flex_none()
        .h(CONTROL_H)
        .px(12.0)
        .gap(8.0)
        .text_size(SMALL_SIZE)
        .weight(SEMIBOLD)
        .color(tint)
        .child(moving(label.into()).nowrap())
        .beams(busy)
        .on_click(handler)
}

fn prompt_change(change: Change) -> Node {
    let round = BAR_H / 2.0;
    let fraction = change.fraction.clamp(0.0, 1.0);
    let figure = |words: SharedString, tint: u32| {
        div()
            .w(CHANGE_VALUE_W)
            .flex_none()
            .flex()
            .justify_end()
            .nowrap()
            .weight(SEMIBOLD)
            .color(tint)
            .child(moving(words))
    };
    div()
        .flex()
        .items_center()
        .gap(8.0)
        .text_size(CAPTION_SIZE)
        .child(
            div()
                .w(CHANGE_LABEL_W)
                .flex_none()
                .truncate()
                .color(TEXT)
                .child(text(change.label)),
        )
        .child(
            div().flex_1().min_w(0.0).h(BAR_H).child(
                div()
                    .size_full()
                    .bg(ledger_track(fraction == 0.0), round)
                    .clip()
                    .child(
                        div()
                            .absolute()
                            .top(0.0)
                            .left(0.0)
                            .h_full()
                            .springy(fraction)
                            .bg(color(change.color), round),
                    ),
            ),
        )
        .child(figure(change.from, change.color))
        .child(div().flex_none().color(OVERLAY1).child(text("→")))
        .child(figure(change.to, change.to_color))
}

/// As big as the panel, with the card in the middle.
/// Over a panel `size` big, or taller where the card needs more room.
pub fn prompt(view: PromptView, size: (f32, f32)) -> Node {
    let cancel = view.on_cancel.clone();
    let eye = view.reveal.map(|reveal| {
        let tint = if reveal.shown { ACCENT } else { SUBTEXT1 };
        control(Some(surface_fill(reveal.shown)), CONTROL_RADIUS)
            .flex_none()
            .size(PROMPT_ICON)
            .children(reveal.glyph.map(|path| glyph(path, PROMPT_GLYPH, tint)))
            .on_click(reveal.on_click)
    });
    let card = card_fill(PANEL_RADIUS)
        .casts(PANEL_RADIUS)
        .flex_none()
        .w(PROMPT_W)
        .p(PROMPT_PAD)
        .col()
        .gap(10.0)
        .rises()
        .blocks()
        .on_click(Rc::new(|_| {}))
        .child(
            div()
                .col()
                .gap(2.0)
                .child(
                    div()
                        .truncate()
                        .text_size(TITLE_SIZE)
                        .weight(SEMIBOLD)
                        .child(text(view.title)),
                )
                .when(!view.detail.is_empty(), |words| {
                    words.child(
                        div()
                            .text_size(CAPTION_SIZE)
                            .color(OVERLAY2)
                            .child(text(view.detail)),
                    )
                }),
        )
        .when(!view.changes.is_empty(), |card| {
            card.child(
                div()
                    .col()
                    .gap(6.0)
                    .px(12.0)
                    .py(10.0)
                    .bg(tinted(TINT, 0x0d), CONTROL_RADIUS)
                    .children(view.changes.into_iter().map(prompt_change)),
            )
        })
        .children(view.code.map(|code| {
            div()
                .py(14.0)
                .flex()
                .justify_center()
                .bg(tinted(TINT, 0x0d), CONTROL_RADIUS)
                .child(
                    div()
                        .text_size(PAIRING_SIZE)
                        .weight(node::BOLD)
                        .child(text(code)),
                )
        }))
        .children(view.field.map(|field| {
            div()
                .flex()
                .items_end()
                .gap(8.0)
                .child(div().flex_1().min_w(0.0).child(input(field)))
                .children(eye)
        }))
        .children(
            view.error
                .map(|error| div().text_size(CAPTION_SIZE).color(RED).child(text(error))),
        )
        .child(
            div()
                .flex()
                .justify_end()
                .gap(8.0)
                .child(prompt_button("Cancel", false, false, view.on_cancel))
                .children(
                    view.confirm
                        .map(|(label, handler)| prompt_button(label, true, view.busy, handler)),
                ),
        );
    div()
        .w(size.0)
        .min_h(size.1)
        .py(PROMPT_PAD)
        .flex()
        .items_center()
        .justify_center()
        .bg(tinted(CRUST, SCRIM_ALPHA), 0.0)
        .blocks()
        .on_click(cancel)
        .child(card)
}
