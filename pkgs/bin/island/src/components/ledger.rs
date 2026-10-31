//! A list of named entries divided by rules, each beside one or more aligned bars with an optional
//! mark, continuation, value and note; entries may fold to one summary line, and long lists scroll
//! under soft edges.

use std::{f32::consts::FRAC_PI_2, path::PathBuf};

use gpui::{
    AnimationExt, AnyElement, Context, Div, FontWeight, Hsla, Rgba, ScrollHandle, SharedString,
    SpringAnimation, SpringConfig, Stateful, Transformation, div, prelude::*, px, radians,
    relative, rgb, rgba, svg,
};

use crate::{
    components::{
        badge::{self, Mark},
        button, edges, motion, reveal,
    },
    island::{CONTENT_IN, Island},
    panel::Handler,
    theme::{
        ACCENT_TINT_ALPHA, BASE, CARD_ALPHA, CONTROL_ALPHA, OVERLAY1, OVERLAY2, RED, SUBTEXT1,
        SURFACE1, TEXT, TINT, TRACK_ALPHA, with_alpha,
    },
    ui::{self, Radius, SMALL_SIZE},
};

const ENTRY_W: f32 = 228.0;
const LABEL_W: f32 = 126.0;
const VALUE_W: f32 = 46.0;
const NOTE_W: f32 = 124.0;
const COL_GAP: f32 = 10.0;
const SECTION_GAP: f32 = 14.0;
const MAX_ROWS_H: f32 = 320.0;

const HEAD_SIZE: f32 = 11.0;
const DETAIL_SIZE: f32 = 12.0;
const SUB_SIZE: f32 = 11.5;
const CHIP_SIZE: f32 = 10.5;

const MARK_BOX: f32 = 28.0;
const BAR_H: f32 = 6.0;
const TICK_W: f32 = 2.0;
const TICK_H: f32 = 12.0;
const HALO_W: f32 = 4.0;
const HALO_H: f32 = 14.0;
const CHEVRON: f32 = 13.0;
const AHEAD_ALPHA: u8 = 0x47;
const ROW_TINT_ALPHA: u8 = 0x10;

const TURN_SPRING: SpringConfig = SpringConfig::new(420.0, 41.0, 1.0);

pub struct Bar {
    pub key: SharedString,
    pub label: SharedString,
    pub label_color: u32,
    pub fraction: f32,
    pub ahead: Option<f32>,
    pub mark: Option<f32>,
    pub color: u32,
    pub value: SharedString,
    pub value_color: u32,
    pub note: SharedString,
    pub note_color: u32,
    pub appear: bool,
}

pub struct Folded {
    pub label: SharedString,
    pub value: SharedString,
    pub note: SharedString,
}

pub struct Entry {
    pub key: SharedString,
    pub mark: Mark,
    pub tint: Option<u32>,
    /// Left-edge highlight pill; `RED` also softly tints the entry's background.
    pub accent: Option<u32>,
    pub faded: bool,
    pub title: SharedString,
    pub subtitle: SharedString,
    pub chip: Option<(SharedString, u32)>,
    pub bars: Vec<Bar>,
    pub folded: Option<Folded>,
    pub expanded: bool,
    pub on_toggle: Option<Handler>,
}

pub struct Columns {
    pub entry: SharedString,
    pub label: SharedString,
    pub bar: SharedString,
    pub mark_key: Option<SharedString>,
    pub ahead_key: Option<SharedString>,
    pub value: SharedString,
    pub note: SharedString,
}

pub struct Ledger {
    pub columns: Option<Columns>,
    pub chevron: Option<PathBuf>,
    pub entries: Vec<Entry>,
}

impl Entry {
    fn line_count(&self) -> usize {
        (self.bars.len() + usize::from(self.folded.is_some())).max(1)
    }

    fn estimated_height(&self) -> f32 {
        16.0 + self.line_count() as f32 * 22.0
    }
}

impl Ledger {
    pub fn estimated_rows_height(&self) -> f32 {
        self.entries
            .iter()
            .map(Entry::estimated_height)
            .sum::<f32>()
    }

    pub fn overflows(&self) -> bool {
        self.estimated_rows_height() > MAX_ROWS_H
    }
}

fn card_glass(glass: Hsla) -> Hsla {
    let base = Rgba::from(glass);
    let card = Rgba::from(rgba(with_alpha(SURFACE1, CARD_ALPHA)));
    let a = card.a;
    Rgba {
        r: base.r * (1.0 - a) + card.r * a,
        g: base.g * (1.0 - a) + card.g * a,
        b: base.b * (1.0 - a) + card.b * a,
        a: (base.a + a * (1.0 - base.a)).max(0.9),
    }
    .into()
}

fn track(dimmed: bool) -> gpui::Rgba {
    rgba(with_alpha(
        TINT,
        if dimmed {
            TRACK_ALPHA / 3
        } else {
            TRACK_ALPHA / 2
        },
    ))
}

fn chevron_glyph(
    id: &str,
    path: Option<&PathBuf>,
    expanded: bool,
    color: u32,
) -> Option<AnyElement> {
    let path = path?;
    Some(
        svg()
            .path(SharedString::from(path.to_string_lossy().into_owned()))
            .size(px(CHEVRON))
            .flex_none()
            .text_color(rgb(color))
            .with_spring(
                SharedString::from(format!("{id}-chevron")),
                SpringAnimation::new(TURN_SPRING).to(if expanded { FRAC_PI_2 } else { 0.0 }),
                |glyph, angle| glyph.with_transformation(Transformation::rotate(radians(angle))),
            )
            .into_any_element(),
    )
}

fn bar_meter(id: &str, bar: &Bar) -> Div {
    let fraction = bar.fraction.clamp(0.0, 1.0);
    let dimmed = fraction == 0.0 && bar.ahead.is_none();
    let ahead_id = SharedString::from(format!("{id}-ahead"));
    let fill_id = SharedString::from(format!("{id}-fill"));
    let tick_id = SharedString::from(format!("{id}-tick"));

    let bar_track = div()
        .relative()
        .size_full()
        .rounded_full()
        .bg(track(dimmed))
        .overflow_hidden()
        .when_some(bar.ahead, |track_el, ahead| {
            let target = ahead.clamp(fraction, 1.0);
            track_el.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h_full()
                    .rounded_full()
                    .bg(rgba(with_alpha(bar.color, AHEAD_ALPHA)))
                    .with_spring(
                        ahead_id,
                        SpringAnimation::new(CONTENT_IN).to(target),
                        |fill, at| fill.w(relative(at.clamp(0.0, 1.0))),
                    ),
            )
        })
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .h_full()
                .rounded_full()
                .bg(rgb(bar.color))
                .with_spring(
                    fill_id,
                    SpringAnimation::new(CONTENT_IN).to(fraction),
                    |fill, at| fill.w(relative(at.clamp(0.0, 1.0))),
                ),
        );

    div()
        .relative()
        .flex_1()
        .min_w(px(0.0))
        .h(px(BAR_H))
        .child(bar_track)
        .when_some(bar.mark, |outer, mark_at| {
            let target = mark_at.clamp(0.0, 1.0);
            outer.child(
                div()
                    .absolute()
                    .top(px((BAR_H - HALO_H) / 2.0))
                    .ml(px(-HALO_W / 2.0))
                    .w(px(HALO_W))
                    .h(px(HALO_H))
                    .rounded_full()
                    .bg(rgb(BASE))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .w(px(TICK_W))
                            .h(px(TICK_H))
                            .rounded_full()
                            .bg(rgb(TEXT)),
                    )
                    .with_spring(
                        tick_id,
                        SpringAnimation::new(CONTENT_IN).to(target),
                        |tick, at| tick.left(relative(at.clamp(0.0, 1.0))),
                    ),
            )
        })
}

fn bar_line(at: &str, bar: Bar, trailing_chevron: Option<AnyElement>) -> AnyElement {
    let key = format!("{at}-{}", bar.key);
    let value_id = SharedString::from(format!("{key}-val"));
    let note_id = SharedString::from(format!("{key}-note"));
    let appear_id = SharedString::from(format!("{key}-appear"));
    let appear = bar.appear;
    let meter = bar_meter(&key, &bar);

    let line = div()
        .h(px(21.0))
        .flex()
        .items_center()
        .gap(px(COL_GAP))
        .child(
            div()
                .w(px(LABEL_W))
                .flex_none()
                .truncate()
                .text_size(px(DETAIL_SIZE))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(bar.label_color))
                .child(bar.label),
        )
        .child(meter)
        .child(
            div()
                .w(px(VALUE_W))
                .flex_none()
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .text_size(px(DETAIL_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(bar.value_color))
                .child(motion::text(value_id, bar.value)),
        )
        .child(
            div()
                .w(px(NOTE_W))
                .flex_none()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(3.0))
                .whitespace_nowrap()
                .text_size(px(SUB_SIZE))
                .text_color(rgb(bar.note_color))
                .child(motion::text(note_id, bar.note))
                .children(trailing_chevron),
        );

    if appear {
        reveal::appear(appear_id, line)
    } else {
        line.into_any_element()
    }
}

fn folded_line(at: &str, folded: Folded, chevron: Option<AnyElement>) -> Div {
    let value_id = SharedString::from(format!("{at}-folded-val"));
    let note_id = SharedString::from(format!("{at}-folded-note"));
    div()
        .h(px(21.0))
        .flex()
        .items_center()
        .gap(px(COL_GAP))
        .child(
            div()
                .w(px(LABEL_W))
                .flex_none()
                .truncate()
                .text_size(px(DETAIL_SIZE))
                .text_color(rgb(OVERLAY2))
                .child(folded.label),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .h(px(BAR_H - 0.5))
                .rounded_full()
                .bg(track(true)),
        )
        .child(
            div()
                .w(px(VALUE_W))
                .flex_none()
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .text_size(px(DETAIL_SIZE))
                .text_color(rgb(OVERLAY2))
                .child(motion::text(value_id, folded.value)),
        )
        .child(
            div()
                .w(px(NOTE_W))
                .flex_none()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(3.0))
                .whitespace_nowrap()
                .text_size(px(SUB_SIZE))
                .text_color(rgb(OVERLAY2))
                .child(motion::text(note_id, folded.note))
                .children(chevron),
        )
}

fn entry_lead(entry: &Entry) -> Div {
    let own_shape = matches!(entry.mark, Mark::Letter(..));
    let bg = (!own_shape).then(|| match (entry.faded, entry.tint) {
        (true, _) => rgba(with_alpha(TINT, 0x10)).into(),
        (false, Some(color)) => rgba(with_alpha(color, ACCENT_TINT_ALPHA / 2)).into(),
        (false, None) => rgba(with_alpha(TINT, 0x14)).into(),
    });
    let lead = div()
        .relative()
        .size(px(MARK_BOX))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .when_some(bg, |el, fill| el.child(ui::shape(Radius::Row, Some(fill))))
        .child(
            div()
                .when(entry.faded, |el| el.opacity(0.45))
                .child(badge::mark(
                    &entry.mark,
                    MARK_BOX,
                    if own_shape { 1.0 } else { 0.58 },
                )),
        );

    let chip = entry.chip.as_ref().map(|(text, color)| {
        div()
            .flex_none()
            .px(px(6.0))
            .py(px(1.0))
            .rounded_full()
            .bg(rgba(with_alpha(*color, 0x24)))
            .text_size(px(CHIP_SIZE))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(*color))
            .whitespace_nowrap()
            .child(text.clone())
    });

    div()
        .w(px(ENTRY_W))
        .flex_none()
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(lead)
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(1.0))
                .child(
                    div()
                        .truncate()
                        .text_size(px(SMALL_SIZE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(if entry.faded { SUBTEXT1 } else { TEXT }))
                        .child(entry.title.clone()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .min_w(px(0.0))
                        .when(!entry.subtitle.is_empty(), |sub| {
                            sub.child(
                                div()
                                    .truncate()
                                    .text_size(px(SUB_SIZE))
                                    .text_color(rgb(OVERLAY2))
                                    .child(entry.subtitle.clone()),
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
    chevron_path: Option<&PathBuf>,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let row_key = format!("{at}-{}", entry.key);
    let single_line = entry.line_count() == 1;
    let left_cell = entry_lead(&entry);
    let accent = entry.accent;
    let whole_row_toggle =
        entry.on_toggle.is_some() && (entry.bars.is_empty() || entry.folded.is_none());
    let sub_row_toggle = entry.on_toggle.is_some() && !whole_row_toggle;
    let expanded = entry.expanded;
    let on_toggle = entry.on_toggle;

    let bar_count = entry.bars.len();
    let mut lines: Vec<AnyElement> = entry
        .bars
        .into_iter()
        .enumerate()
        .map(|(idx, bar)| {
            let trail = (whole_row_toggle && idx + 1 == bar_count)
                .then(|| chevron_glyph(&row_key, chevron_path, expanded, OVERLAY2))
                .flatten();
            bar_line(&row_key, bar, trail)
        })
        .collect();

    if let Some(folded) = entry.folded {
        let chevron = chevron_glyph(&row_key, chevron_path, expanded, OVERLAY2);
        let line = folded_line(&row_key, folded, chevron);
        if sub_row_toggle && let Some(handler) = on_toggle.clone() {
            let toggle_id = SharedString::from(format!("{row_key}-sub-toggle"));
            lines.push(
                button::fill(toggle_id, Radius::Row, None, hover)
                    .w_full()
                    .px(px(4.0))
                    .mx(px(-4.0))
                    .justify_start()
                    .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
                    .child(line.w_full())
                    .into_any_element(),
            );
        } else {
            lines.push(line.into_any_element());
        }
    }

    let inner = div()
        .relative()
        .w_full()
        .py(px(8.0))
        .pl(px(10.0))
        .pr(px(4.0))
        .flex()
        .when(single_line, |el| el.items_center())
        .when(!single_line, |el| el.items_start())
        .gap(px(SECTION_GAP))
        .when_some(accent, |el, color| {
            el.child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px(9.0))
                    .bottom(px(9.0))
                    .w(px(3.0))
                    .rounded_full()
                    .bg(rgb(color)),
            )
        })
        .child(left_cell.when(!single_line, |cell| cell.pt(px(2.0))))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(3.0))
                .children(lines),
        );

    let rest = (accent == Some(RED)).then(|| rgba(with_alpha(RED, ROW_TINT_ALPHA)).into());
    let body = if whole_row_toggle && let Some(handler) = on_toggle {
        let toggle_id = SharedString::from(format!("{row_key}-toggle"));
        button::fill(toggle_id, Radius::Row, rest, hover)
            .w_full()
            .justify_start()
            .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
            .child(inner)
            .into_any_element()
    } else {
        div()
            .relative()
            .w_full()
            .when_some(rest, |el, fill: Hsla| el.rounded(px(10.0)).bg(fill))
            .child(inner)
            .into_any_element()
    };

    div()
        .w_full()
        .when(index > 0, |wrapper| {
            wrapper
                .pt(px(2.0))
                .mt(px(2.0))
                .border_t_1()
                .border_color(rgba(with_alpha(SURFACE1, CONTROL_ALPHA)))
        })
        .child(body)
        .into_any_element()
}

fn header_row(columns: Columns) -> Div {
    let has_keys = columns.mark_key.is_some() || columns.ahead_key.is_some();
    let legend = has_keys.then(|| {
        div()
            .flex()
            .items_center()
            .gap(px(10.0))
            .text_size(px(HEAD_SIZE))
            .font_weight(FontWeight::NORMAL)
            .text_color(rgb(OVERLAY2))
            .when_some(columns.mark_key, |el, label| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(div().w(px(2.0)).h(px(9.0)).rounded_full().bg(rgb(TEXT)))
                        .child(label),
                )
            })
            .when_some(columns.ahead_key, |el, label| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            div()
                                .w(px(13.0))
                                .h(px(5.5))
                                .rounded_full()
                                .bg(rgba(with_alpha(TEXT, AHEAD_ALPHA))),
                        )
                        .child(label),
                )
            })
    });

    div()
        .pl(px(10.0))
        .pr(px(4.0))
        .pb(px(8.0))
        .mb(px(2.0))
        .border_b_1()
        .border_color(rgba(with_alpha(SURFACE1, CONTROL_ALPHA)))
        .flex()
        .items_center()
        .gap(px(SECTION_GAP))
        .text_size(px(HEAD_SIZE))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(OVERLAY1))
        .child(
            div()
                .w(px(ENTRY_W))
                .flex_none()
                .truncate()
                .child(columns.entry),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .items_center()
                .gap(px(COL_GAP))
                .child(div().w(px(LABEL_W)).flex_none().child(columns.label))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(columns.bar)
                        .children(legend),
                )
                .child(
                    div()
                        .w(px(VALUE_W))
                        .flex_none()
                        .flex()
                        .justify_end()
                        .child(columns.value),
                )
                .child(
                    div()
                        .w(px(NOTE_W))
                        .flex_none()
                        .flex()
                        .justify_end()
                        .child(columns.note),
                ),
        )
}

pub fn ledger(
    ledger: Ledger,
    at: &str,
    scroll: &ScrollHandle,
    glass: Hsla,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let overflows = ledger.overflows();
    let chevron = ledger.chevron;
    let items: Vec<AnyElement> = ledger
        .entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| entry_row(at, index, entry, chevron.as_ref(), hover, cx))
        .collect();

    let list = div().flex().flex_col().children(items);
    let body = if overflows {
        let scrolling: Stateful<Div> = div()
            .id(SharedString::from(format!("{at}-ledger-scroll")))
            .max_h(px(MAX_ROWS_H))
            .overflow_y_scroll()
            .track_scroll(scroll)
            .child(list);
        edges::edges(scrolling, scroll, card_glass(glass))
    } else {
        list.into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .when_some(ledger.columns, |col, columns| col.child(header_row(columns)))
        .child(body)
        .into_any_element()
}
