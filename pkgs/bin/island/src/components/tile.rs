//! Boards of stat tiles, laid out in columns that end level.

use std::path::PathBuf;

use gpui::{
    AnimationExt, AnyElement, Context, Div, FontWeight, SharedString, SpringAnimation, div,
    prelude::*, px, relative, rgb, rgba,
};

use crate::{
    components::{
        card::{self, Stat},
        chart::{Series, chart},
        icon, motion,
        ring::{self, Ring, Rings},
        tooltip::Tooltips,
    },
    island::{CONTENT_IN, Island},
    panel::Handler,
    theme::{OVERLAY2, SUBTEXT1, SURFACE1, TEXT, TINT, TRACK_ALPHA, with_alpha},
    ui::{self, Radius, SMALL_SIZE},
};

/// Height of one row unit. A tile spans whole units, so every column of a board ends level.
pub const UNIT: f32 = 196.0;
const PAD: f32 = 14.0;
const PART_GAP: f32 = 9.0;
const CHART_H: f32 = 40.0;
const TALL_CHART_H: f32 = 56.0;
const CAPTION: f32 = 12.0;
const MARK_W: f32 = 2.0;
/// How far a meter's mark reaches past its bar, above and below.
const MARK_OVERHANG: f32 = 3.0;
const LABEL_W: f32 = 70.0;
const RINGS: f32 = 92.0;
const RINGS_GAP: f32 = 14.0;
const DOT: f32 = 6.0;
const DETAIL: f32 = 10.5;
const VALUE_W: f32 = 64.0;
const CORE_COLUMNS: usize = 16;
const CORE_GAP: f32 = 3.0;
/// Line heights tighter than gpui's default (≈1.6), so the host tile's hero and facts fit one unit and
/// partition rows stay short.
const HERO_LEADING: f32 = 1.4;
const FACT_LEADING: f32 = 1.3;
/// How far an entry's row reaches into the tile's padding on each side, so its text lines up with
/// the rest of the tile and its highlight still has room round it.
const ENTRY_BLEED: f32 = 8.0;
const ENTRY_GLYPH: f32 = 16.0;
const CHOSEN_ALPHA: u8 = 0x26;

pub struct Tile {
    pub id: SharedString,
    /// Row units tall: 1 or 2.
    pub span: usize,
    pub title: Option<(SharedString, Option<Stat>)>,
    pub on_click: Option<Handler>,
    pub parts: Vec<Part>,
}

pub struct Meter {
    pub label: SharedString,
    pub fraction: f32,
    pub value: SharedString,
    pub color: u32,
    /// A tick across the bar at this fraction, such as how much of a quota's window has passed.
    pub mark: Option<f32>,
}

/// A line beside rings: a dot in its ring's color, what it is and its value, and a fainter detail
/// under them.
pub struct Row {
    pub color: Option<u32>,
    pub label: SharedString,
    pub value: SharedString,
    pub detail: SharedString,
}

pub struct Volume {
    pub id: SharedString,
    pub mount: SharedString,
    pub usage: SharedString,
    pub fraction: f32,
    pub color: u32,
}

pub struct Segment {
    pub weight: f32,
    pub color: Option<u32>,
}

pub struct Drive {
    pub name: SharedString,
    pub size: SharedString,
    pub segments: Vec<Segment>,
}

/// What leads an entry: a glyph in a color, or a dot, such as whether something is up.
pub enum EntryLead {
    Glyph(Option<PathBuf>, u32),
    Dot(u32),
}

/// One row of a list in a tile, such as a network to join; clickable when it has a handler.
pub struct Entry {
    /// Stable across frames, for its morphing text.
    pub id: SharedString,
    pub lead: Option<EntryLead>,
    pub title: SharedString,
    pub title_color: Option<u32>,
    pub detail: SharedString,
    pub value: SharedString,
    pub value_color: u32,
    /// The row tinted in this color, such as the one in use.
    pub chosen: Option<u32>,
    pub on_click: Option<Handler>,
}

/// One building block of a tile. Parts stack top to bottom; `Spacer` pushes the rest down.
pub enum Part {
    /// A history chart; `grow` lets it take the tile's spare height.
    Chart {
        series: Vec<Series>,
        grow: bool,
    },
    Cores {
        loads: Vec<f32>,
        color: u32,
    },
    /// Label, bar and value rows; `scroll` lets a long list scroll inside the tile.
    Meters {
        rows: Vec<Meter>,
        scroll: bool,
    },
    /// Concentric rings around a value, outermost first, beside rows naming what they show.
    Rings {
        rings: Vec<Ring>,
        value: SharedString,
        value_color: u32,
        label: SharedString,
        rows: Vec<Row>,
    },
    /// Each optionally led by a coloured dot.
    Legend(Vec<(Option<u32>, SharedString)>),
    Facts(Vec<(SharedString, SharedString)>),
    Hero {
        name: SharedString,
        lines: Vec<SharedString>,
    },
    /// Scrolls past what fits.
    Volumes(Vec<Volume>),
    Drives(Vec<Drive>),
    /// Rows of a list; `scroll` lets a long one scroll inside the tile.
    Entries {
        entries: Vec<Entry>,
        scroll: bool,
    },
    Spacer,
}

pub fn height(span: usize) -> f32 {
    let span = span.max(1) as f32;
    UNIT * span + card::GAP * (span - 1.0)
}

fn track() -> gpui::Rgba {
    rgba(with_alpha(TINT, TRACK_ALPHA / 2))
}

/// `id` keeps its fill and mark gliding to new values rather than jumping.
fn bar(id: &str, fraction: f32, color: u32, thickness: f32, mark: Option<f32>) -> Div {
    let fill = div()
        .size_full()
        .rounded_full()
        .bg(track())
        .overflow_hidden()
        .child(
            div()
                .h_full()
                .rounded_full()
                .bg(rgb(color))
                .with_spring(
                    SharedString::from(format!("{id}-fill")),
                    SpringAnimation::new(CONTENT_IN).to(fraction.clamp(0.0, 1.0)),
                    |fill, at| fill.w(relative(at.clamp(0.0, 1.0))),
                ),
        );
    let mark_id = SharedString::from(format!("{id}-mark"));
    div()
        .relative()
        .flex_1()
        .min_w(px(0.0))
        .h(px(thickness))
        .child(fill)
        .when_some(mark, |bar, mark| {
            bar.child(
                div()
                    .absolute()
                    .top(px(-MARK_OVERHANG))
                    .ml(px(-MARK_W / 2.0))
                    .w(px(MARK_W))
                    .h(px(thickness + 2.0 * MARK_OVERHANG))
                    .rounded_full()
                    .bg(rgb(TEXT))
                    .with_spring(
                        mark_id,
                        SpringAnimation::new(CONTENT_IN).to(mark.clamp(0.0, 1.0)),
                        |tick, at| tick.left(relative(at.clamp(0.0, 1.0))),
                    ),
            )
        })
}

fn hairline() -> gpui::Rgba {
    rgba(with_alpha(SURFACE1, 0x59))
}

/// `key` names the rows for their morphing values.
fn meters(rows: Vec<Meter>, key: &str) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(7.0))
        .children(rows.into_iter().enumerate().map(|(index, row)| {
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .text_size(px(CAPTION))
                .child(
                    div()
                        .w(px(LABEL_W))
                        .flex_none()
                        .truncate()
                        .text_color(rgb(SUBTEXT1))
                        .child(row.label),
                )
                .child(bar(
                    &format!("{key}-bar-{index}"),
                    row.fraction,
                    row.color,
                    5.0,
                    row.mark,
                ))
                .child(
                    div()
                        .min_w(px(VALUE_W))
                        .flex_none()
                        .flex()
                        .justify_end()
                        .whitespace_nowrap()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(motion::text(
                            SharedString::from(format!("{key}-{index}")),
                            row.value,
                        )),
                )
        }))
}

/// `key` names the rows for their morphing values.
fn rows(rows: Vec<Row>, key: &str) -> Div {
    div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .gap(px(5.0))
        .children(rows.into_iter().enumerate().map(|(index, row)| {
            div()
                .flex()
                .flex_col()
                .line_height(relative(FACT_LEADING))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .text_size(px(CAPTION))
                        .when_some(row.color, |line, color| {
                            line.child(
                                div()
                                    .size(px(DOT))
                                    .flex_none()
                                    .rounded_full()
                                    .bg(rgb(color)),
                            )
                        })
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .truncate()
                                .text_color(rgb(SUBTEXT1))
                                .child(row.label),
                        )
                        .child(
                            div()
                                .flex_none()
                                .whitespace_nowrap()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(motion::text(
                                    SharedString::from(format!("{key}-{index}")),
                                    row.value,
                                )),
                        ),
                )
                .when(!row.detail.is_empty(), |entry| {
                    entry.child(
                        div()
                            .when(row.color.is_some(), |detail| detail.pl(px(DOT + 6.0)))
                            .truncate()
                            .text_size(px(DETAIL))
                            .text_color(rgb(OVERLAY2))
                            .child(motion::text(
                                SharedString::from(format!("{key}-{index}-detail")),
                                row.detail,
                            )),
                    )
                })
        }))
}

fn cores(loads: Vec<f32>, color: u32) -> Div {
    let inner = card::Size::Small.width() - 2.0 * PAD;
    let cell = (inner - CORE_GAP * (CORE_COLUMNS - 1) as f32) / CORE_COLUMNS as f32;
    div()
        .flex()
        .flex_wrap()
        .gap(px(CORE_GAP))
        .children(loads.into_iter().map(move |load| {
            div()
                .w(px(cell))
                .h(px(8.0))
                .rounded(px(2.5))
                .bg(rgb(color))
                .opacity(0.15 + load.clamp(0.0, 1.0) * 0.85)
        }))
}

fn legend(items: Vec<(Option<u32>, SharedString)>) -> Div {
    div()
        .flex()
        .gap(px(12.0))
        .text_size(px(CAPTION))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(SUBTEXT1))
        .children(items.into_iter().map(|(dot, text)| {
            div()
                .flex()
                .items_center()
                .gap(px(5.0))
                .whitespace_nowrap()
                .when_some(dot, |item, color| {
                    item.text_color(rgb(TEXT)).child(
                        div()
                            .size(px(7.0))
                            .flex_none()
                            .rounded_full()
                            .bg(rgb(color)),
                    )
                })
                .child(text)
        }))
}

/// `key` names the values for their morphing.
fn facts(items: Vec<(SharedString, SharedString)>, key: &str) -> Div {
    colored_facts(
        items
            .into_iter()
            .map(|(label, value)| (label, value, None))
            .collect(),
        key,
    )
}

/// Also a panel block of its own, outside a tile; a value with a color shows in it, such as a
/// state.
pub fn colored_facts(items: Vec<(SharedString, SharedString, Option<u32>)>, key: &str) -> Div {
    let mut rows = Vec::new();
    let mut items = items.into_iter().enumerate().peekable();
    while items.peek().is_some() {
        let row: Vec<_> = items.by_ref().take(3).collect();
        rows.push(div().flex().gap(px(6.0)).children(row.into_iter().map(
            |(index, (label, value, color))| {
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .px(px(8.0))
                    .py(px(4.0))
                    .rounded(px(9.0))
                    .bg(rgba(with_alpha(TINT, 0x0d)))
                    .flex()
                    .flex_col()
                    .line_height(relative(FACT_LEADING))
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(rgb(OVERLAY2))
                            .child(label),
                    )
                    .child(
                        div()
                            .text_size(px(SMALL_SIZE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .when_some(color, |value, color| value.text_color(rgb(color)))
                            .truncate()
                            .child(motion::text(
                                SharedString::from(format!("{key}-{index}")),
                                value,
                            )),
                    )
            },
        )));
    }
    div().flex().flex_col().gap(px(6.0)).children(rows)
}

/// Also a panel block of its own, outside a tile.
pub fn hero(name: SharedString, lines: Vec<SharedString>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .text_size(px(20.0))
                .font_weight(FontWeight::BOLD)
                .child(name),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .text_size(px(CAPTION))
                .line_height(relative(HERO_LEADING))
                .text_color(rgb(SUBTEXT1))
                .children(lines.into_iter().map(|line| div().truncate().child(line))),
        )
}

fn volumes(id: SharedString, items: Vec<Volume>) -> AnyElement {
    div()
        .id(id.clone())
        .flex_1()
        .min_h(px(0.0))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .children(items.into_iter().enumerate().map(move |(index, volume)| {
            div()
                .flex_none()
                .when(index > 0, |row| row.border_t_1().border_color(hairline()))
                .flex()
                .flex_col()
                .gap(px(4.0))
                .py(px(6.0))
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .gap(px(10.0))
                        .text_size(px(CAPTION))
                        .line_height(relative(FACT_LEADING))
                        .child(
                            div()
                                .min_w(px(0.0))
                                .truncate()
                                .text_color(rgb(SUBTEXT1))
                                .child(motion::text(
                                    SharedString::from(format!("{id}-{}-label", volume.id)),
                                    volume.mount,
                                )),
                        )
                        .child(
                            div()
                                .flex_none()
                                .whitespace_nowrap()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(motion::text(
                                    SharedString::from(format!("{id}-{}-usage", volume.id)),
                                    volume.usage,
                                )),
                        ),
                )
                .child(div().flex().child(bar(
                    &format!("{id}-{}-bar", volume.id),
                    volume.fraction,
                    volume.color,
                    5.0,
                    None,
                )))
        }))
        .into_any_element()
}

fn drives(items: Vec<Drive>) -> Div {
    div()
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .pt(px(12.0))
        .border_t_1()
        .border_color(hairline())
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(OVERLAY2))
                .child("DRIVES"),
        )
        .children(items.into_iter().map(|drive| {
            let total: f32 = drive
                .segments
                .iter()
                .map(|segment| segment.weight)
                .sum::<f32>()
                .max(f32::EPSILON);
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .gap(px(10.0))
                        .text_size(px(CAPTION))
                        .text_color(rgb(SUBTEXT1))
                        .child(div().min_w(px(0.0)).truncate().child(drive.name))
                        .child(
                            div()
                                .flex_none()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT))
                                .child(drive.size),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(2.0))
                        .h(px(8.0))
                        .rounded_full()
                        .overflow_hidden()
                        .bg(track())
                        .children(drive.segments.into_iter().map(move |segment| {
                            div()
                                .h_full()
                                .w(px(0.0))
                                .flex_grow(segment.weight / total)
                                .when_some(segment.color, |piece, color| piece.bg(rgb(color)))
                        })),
                )
        }))
}

fn entry_lead(lead: EntryLead) -> AnyElement {
    match lead {
        EntryLead::Glyph(path, color) => div()
            .size(px(ENTRY_GLYPH))
            .flex_none()
            .children(path.map(|path| icon::glyph(&path, ENTRY_GLYPH, color)))
            .into_any_element(),
        EntryLead::Dot(color) => div()
            .size(px(DOT + 1.0))
            .flex_none()
            .rounded_full()
            .bg(rgb(color))
            .into_any_element(),
    }
}

/// `key` names the rows for their morphing text.
fn entries(
    items: Vec<Entry>,
    key: &str,
    scroll: bool,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let rows = items.into_iter().map(|entry| {
        let row = SharedString::from(format!("{key}-{}", entry.id));
        let clickable = entry.on_click.is_some();
        let content = div()
            .relative()
            .flex()
            .items_center()
            .gap(px(10.0))
            .px(px(ENTRY_BLEED))
            .py(px(5.0))
            .children(entry.lead.map(entry_lead))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .line_height(relative(FACT_LEADING))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(SMALL_SIZE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .when_some(entry.title_color, |title, color| {
                                title.text_color(rgb(color))
                            })
                            .child(entry.title),
                    )
                    .when(!entry.detail.is_empty(), |text| {
                        text.child(
                            div()
                                .truncate()
                                .text_size(px(DETAIL + 0.5))
                                .text_color(rgb(OVERLAY2))
                                .child(motion::text(
                                    SharedString::from(format!("{row}-detail")),
                                    entry.detail,
                                )),
                        )
                    }),
            )
            .when(!entry.value.is_empty(), |line| {
                line.child(
                    div()
                        .flex_none()
                        .whitespace_nowrap()
                        .text_size(px(CAPTION))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(entry.value_color))
                        .child(motion::text(
                            SharedString::from(format!("{row}-value")),
                            entry.value,
                        )),
                )
            });
        div()
            .id(row)
            .relative()
            .flex_none()
            .when_some(entry.chosen, |row, color| {
                row.child(ui::shape(
                    Radius::Control,
                    Some(rgba(with_alpha(color, CHOSEN_ALPHA)).into()),
                ))
            })
            .when(clickable, |row| {
                row.cursor_pointer()
                    .when(hover, |row| row.hover(|style| style))
                    .child(ui::control(Radius::Control, None, hover))
            })
            .child(content)
            .when_some(entry.on_click, |row, action| {
                row.on_click(cx.listener(move |island, _, _, cx| {
                    cx.stop_propagation();
                    action(island, cx);
                }))
            })
    });
    let list = div()
        .mx(px(-ENTRY_BLEED))
        .flex()
        .flex_col()
        .gap(px(2.0))
        .children(rows);
    if scroll {
        div()
            .id(SharedString::from(format!("{key}-scroll")))
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .child(list)
            .into_any_element()
    } else {
        list.flex_none().into_any_element()
    }
}

fn part(
    part: Part,
    id: &SharedString,
    index: usize,
    span: usize,
    phase: f32,
    tips: &Tooltips,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    match part {
        Part::Chart { series, grow } => div()
            .relative()
            .when(grow, |area| area.flex_1().min_h(px(CHART_H)))
            .when(!grow, |area| {
                area.flex_none()
                    .h(px(if span > 1 { TALL_CHART_H } else { CHART_H }))
            })
            .child(div().absolute().size_full().flex().child(chart(
                format!("{id}-chart-{index}").into(),
                series,
                card::SLOTS,
                phase,
                tips,
            )))
            .into_any_element(),
        Part::Cores { loads, color } => cores(loads, color).into_any_element(),
        Part::Meters {
            rows,
            scroll: false,
        } => meters(rows, &format!("{id}-meters-{index}"))
            .flex_none()
            .into_any_element(),
        Part::Meters { rows, scroll: true } => div()
            .id(SharedString::from(format!("{id}-meters-{index}")))
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .child(meters(rows, &format!("{id}-meters-{index}")))
            .into_any_element(),
        Part::Rings {
            rings,
            value,
            value_color,
            label,
            rows: lines,
        } => div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(RINGS_GAP))
            .child(ring::rings(Rings {
                id: format!("{id}-rings-{index}").into(),
                size: RINGS,
                rings,
                value,
                value_color,
                label,
            }))
            .child(rows(lines, &format!("{id}-rows-{index}")))
            .into_any_element(),
        Part::Legend(items) => legend(items).flex_none().into_any_element(),
        Part::Facts(items) => facts(items, &format!("{id}-facts-{index}"))
            .flex_none()
            .into_any_element(),
        Part::Hero { name, lines } => hero(name, lines).flex_none().into_any_element(),
        Part::Volumes(items) => volumes(SharedString::from(format!("{id}-volumes")), items),
        Part::Drives(items) => drives(items).into_any_element(),
        Part::Entries {
            entries: items,
            scroll,
        } => entries(items, &format!("{id}-entries-{index}"), scroll, hover, cx),
        Part::Spacer => div().flex_1().into_any_element(),
    }
}

pub fn tile(
    tile: Tile,
    phase: f32,
    hover: bool,
    tips: &Tooltips,
    cx: &mut Context<Island>,
) -> AnyElement {
    let Tile {
        id,
        span,
        title,
        on_click,
        parts,
    } = tile;
    let clickable = on_click.is_some();
    card::surface()
        .id(id.clone())
        .when(clickable, |tile| {
            tile.cursor_pointer()
                .when(hover, |tile| tile.hover(|style| style))
                .child(ui::control(Radius::Panel, None, hover))
        })
        .w(px(card::Size::Small.width()))
        .h(px(height(span)))
        .flex_none()
        .overflow_hidden()
        .p(px(PAD))
        .flex()
        .flex_col()
        .gap(px(PART_GAP))
        .when_some(title, |tile, (title, value)| {
            tile.child(
                div()
                    .flex_none()
                    .flex()
                    .items_baseline()
                    .justify_between()
                    .gap(px(12.0))
                    .child(
                        div()
                            .text_size(px(15.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .children(value.map(|value| {
                        div()
                            .text_size(px(15.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .whitespace_nowrap()
                            .text_color(rgb(value.color))
                            .child(motion::text(
                                SharedString::from(format!("{id}-stat")),
                                value.text,
                            ))
                    })),
            )
        })
        .map(|tile| {
            // Parts after a spacer sit together at the bottom, one gap apart, with no extra gap
            // around the spacer itself.
            let mut parts = parts.into_iter().enumerate();
            let top: Vec<AnyElement> = parts
                .by_ref()
                .take_while(|(_, piece)| !matches!(piece, Part::Spacer))
                .map(|(index, piece)| part(piece, &id, index, span, phase, tips, hover, cx))
                .collect();
            let bottom: Vec<AnyElement> = parts
                .map(|(index, piece)| part(piece, &id, index, span, phase, tips, hover, cx))
                .collect();
            tile.children(top).when(!bottom.is_empty(), |tile| {
                tile.child(
                    div()
                        .flex_1()
                        .min_h(px(0.0))
                        .flex()
                        .flex_col()
                        .justify_end()
                        .gap(px(PART_GAP))
                        .children(bottom),
                )
            })
        })
        .when_some(on_click, |tile, action| {
            tile.on_click(cx.listener(move |island, _, _, cx| action(island, cx)))
        })
        .into_any_element()
}

/// Give every column the same total span so they end level. Columns that fall short of the
/// panel's width sit in its middle, such as the last of several rows.
pub fn board(
    columns: Vec<Vec<Tile>>,
    phase: f32,
    hover: bool,
    tips: &Tooltips,
    cx: &mut Context<Island>,
) -> AnyElement {
    div()
        .flex()
        .justify_center()
        .items_start()
        .gap(px(card::GAP))
        .children(columns.into_iter().map(|column| {
            div().flex().flex_col().gap(px(card::GAP)).children(
                column
                    .into_iter()
                    .map(|piece| tile(piece, phase, hover, tips, cx)),
            )
        }))
        .into_any_element()
}
