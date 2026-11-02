//! Spans and pins along one stretch of time, packed onto lanes over dashed gridlines and an axis.

use gpui::{
    AnimationExt, AnyElement, Bounds, ContentMask, Div, FontWeight, Hsla, PathBuilder, Pixels,
    SharedString, SpringAnimation, Window, canvas, div, point, prelude::*, px, relative, rgb, rgba,
};

use crate::{
    components::{
        motion,
        tooltip::{Tip, Tooltips},
    },
    island::CONTENT_IN,
    theme::{BASE, OVERLAY1, OVERLAY2, RED, SUBTEXT1, SURFACE1, TEXT, with_alpha},
};

const SIDE_W: f32 = 148.0;
const SECTION_GAP: f32 = 16.0;
const TRACK_RIGHT_PAD: f32 = 14.0;
const LANE_H: f32 = 18.0;
const LANE_GAP: f32 = 5.0;
const PIN_H: f32 = 16.0;
const AXIS_H: f32 = 14.0;

const TITLE_SIZE: f32 = 13.5;
const SUB_SIZE: f32 = 11.5;
const KEY_SIZE: f32 = 10.5;
const CHIP_SIZE: f32 = 10.5;
const AXIS_SIZE: f32 = 10.8;

const STRIPE_W: f32 = 1.9;
const STRIPE_EVERY: f32 = 5.5;
const STRIPE_ALPHA: u8 = 0x99;

pub struct Span {
    pub key: SharedString,
    pub label: SharedString,
    pub tooltip: SharedString,
    pub from: f32,
    pub to: f32,
    pub color: u32,
    pub start_tick: bool,
    pub end_dot: bool,
}

pub struct Pin {
    pub key: SharedString,
    pub at: f32,
    pub label: SharedString,
    pub tooltip: SharedString,
    pub color: u32,
    pub width: f32,
}

pub struct Side {
    pub title: SharedString,
    pub detail: SharedString,
    pub striped_key: Option<SharedString>,
    pub dot_key: Option<SharedString>,
}

pub struct Timeline {
    pub side: Option<Side>,
    pub lanes: Vec<Vec<Span>>,
    pub pins: Vec<Pin>,
    pub axis: Vec<SharedString>,
}

fn stripes(bounds: Bounds<Pixels>, window: &mut Window, color: Hsla) {
    let (x0, y0) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        let mut x = x0 - h;
        while x < x0 + w {
            let mut path = PathBuilder::stroke(px(STRIPE_W));
            path.move_to(point(px(x), px(y0 + h)));
            path.line_to(point(px(x + h), px(y0)));
            if let Ok(path) = path.build() {
                window.paint_path(path, color);
            }
            x += STRIPE_EVERY;
        }
    });
}

fn gridlines(bounds: Bounds<Pixels>, window: &mut Window, count: usize) {
    if count < 2 {
        return;
    }
    let (x0, y0) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    let color: Hsla = rgba(with_alpha(SURFACE1, 0x66)).into();
    let segments = (count - 1) as f32;
    for idx in 0..count {
        let x = x0 + w * (idx as f32 / segments);
        let mut y = y0;
        while y < y0 + h {
            let y_end = (y + 2.5).min(y0 + h);
            let mut path = PathBuilder::stroke(px(1.0));
            path.move_to(point(px(x), px(y)));
            path.line_to(point(px(x), px(y_end)));
            if let Ok(path) = path.build() {
                window.paint_path(path, color);
            }
            y += 5.5;
        }
    }
}

fn span_element(at: &str, span: Span, tips: &Tooltips) -> AnyElement {
    let from = span.from.clamp(0.0, 1.0);
    let to = span.to.clamp(from, 1.0);
    let width = (to - from).max(0.025);
    let tint = span.color;
    let hatch_color: Hsla = rgba(with_alpha(tint, STRIPE_ALPHA)).into();
    let id = SharedString::from(format!("{at}-span-{}", span.key));
    let pos_id = SharedString::from(format!("{id}-pos"));
    let w_id = SharedString::from(format!("{id}-w"));
    let show_label = width >= 0.16 && !span.label.is_empty();

    let inner = div()
        .id(id.clone())
        .relative()
        .h_full()
        .child(
            div()
                .relative()
                .size_full()
                .rounded(px(5.0))
                .bg(rgba(with_alpha(tint, 0x22)))
                .border_1()
                .border_color(rgba(with_alpha(tint, 0x99)))
                .overflow_hidden()
                .child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, (), window, _| stripes(bounds, window, hatch_color),
                    )
                    .size_full(),
                )
                .when(show_label, |bar| {
                    bar.child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(px(5.0))
                            .right(px(8.0))
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .max_w_full()
                                    .px(px(5.5))
                                    .py(px(0.5))
                                    .rounded(px(3.5))
                                    .bg(rgba(with_alpha(BASE, 0xe0)))
                                    .truncate()
                                    .text_size(px(CHIP_SIZE))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(TEXT))
                                    .child(span.label),
                            ),
                    )
                }),
        )
        .when(span.start_tick, |outer| {
            outer.child(
                div()
                    .absolute()
                    .left(px(-1.0))
                    .top(px(-2.0))
                    .w(px(2.2))
                    .h(px(LANE_H + 4.0))
                    .rounded_full()
                    .bg(rgb(tint)),
            )
        })
        .when(span.end_dot, |outer| {
            outer.child(
                div()
                    .absolute()
                    .right(px(-4.5))
                    .top(px((LANE_H - 9.0) / 2.0))
                    .size(px(9.0))
                    .rounded_full()
                    .bg(rgb(BASE))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().size(px(6.0)).rounded_full().bg(rgb(TEXT))),
            )
        })
        .tip(tips, id, span.tooltip)
        .with_spring(
            w_id,
            SpringAnimation::new(CONTENT_IN.into()).to(width),
            |el, w| el.w(relative(w.clamp(0.02, 1.0))),
        );

    div()
        .absolute()
        .top_0()
        .w_full()
        .h(px(LANE_H))
        .child(inner)
        .with_spring(
            pos_id,
            SpringAnimation::new(CONTENT_IN.into()).to(from),
            |el, left| el.left(relative(left.clamp(0.0, 1.0))),
        )
        .into_any_element()
}

fn pin_element(at: &str, pin: Pin, track_w: f32, tips: &Tooltips) -> AnyElement {
    let frac = pin.at.clamp(0.0, 1.0);
    let pin_w = pin.width;
    let id = SharedString::from(format!("{at}-pin-{}", pin.key));
    let pos_id = SharedString::from(format!("{id}-pos"));
    div()
        .id(id.clone())
        .absolute()
        .top_0()
        .h(px(PIN_H))
        .px(px(7.0))
        .rounded_full()
        .bg(rgba(with_alpha(BASE, 0xd8)))
        .border_1()
        .border_color(rgba(with_alpha(SURFACE1, 0x99)))
        .flex()
        .items_center()
        .gap(px(5.0))
        .whitespace_nowrap()
        .text_size(px(CHIP_SIZE))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(SUBTEXT1))
        .child(
            div()
                .size(px(5.5))
                .flex_none()
                .rounded_full()
                .bg(rgb(pin.color)),
        )
        .child(pin.label)
        .tip(tips, id, pin.tooltip)
        .with_spring(
            pos_id,
            SpringAnimation::new(CONTENT_IN.into()).to(frac),
            move |el, x| {
                let x = x.clamp(0.0, 1.0);
                let min_ml = -x * track_w;
                let max_ml = ((1.0 - x) * track_w - pin_w).max(min_ml);
                let offset = (-pin_w / 2.0).clamp(min_ml, max_ml);
                el.left(relative(x)).ml(px(offset))
            },
        )
        .into_any_element()
}

fn side_column(at: &str, side: Side) -> Div {
    let swatch_color: Hsla = rgba(with_alpha(RED, STRIPE_ALPHA)).into();
    let detail_id = SharedString::from(format!("{at}-timeline-detail"));

    div()
        .w(px(SIDE_W))
        .flex_none()
        .flex()
        .flex_col()
        .justify_between()
        .gap(px(4.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(TITLE_SIZE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT))
                        .child(side.title),
                )
                .child(
                    div()
                        .text_size(px(SUB_SIZE))
                        .text_color(rgb(OVERLAY2))
                        .child(motion::text(detail_id, side.detail)),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .text_size(px(KEY_SIZE))
                .text_color(rgb(OVERLAY2))
                .when_some(side.striped_key, |key, label| {
                    key.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.5))
                            .child(
                                div()
                                    .w(px(13.0))
                                    .h(px(8.0))
                                    .rounded(px(2.5))
                                    .bg(rgba(with_alpha(RED, 0x22)))
                                    .border_1()
                                    .border_color(rgba(with_alpha(RED, 0x88)))
                                    .overflow_hidden()
                                    .child(
                                        canvas(
                                            |_, _, _| {},
                                            move |bounds, (), window, _| {
                                                stripes(bounds, window, swatch_color)
                                            },
                                        )
                                        .size_full(),
                                    ),
                            )
                            .child(label),
                    )
                })
                .when_some(side.dot_key, |key, label| {
                    key.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .child(div().size(px(5.5)).rounded_full().bg(rgb(TEXT)))
                            .child(label),
                    )
                }),
        )
}

pub fn timeline(timeline: Timeline, at: &str, tips: &Tooltips) -> AnyElement {
    let Timeline {
        side,
        lanes,
        pins,
        axis,
    } = timeline;
    let tick_count = axis.len();
    let segments = tick_count.saturating_sub(1).max(1) as f32;
    let track_w = if side.is_some() { 624.0 } else { 774.0 };

    let swimlanes = div()
        .relative()
        .w_full()
        .py(px(2.0))
        .flex()
        .flex_col()
        .gap(px(LANE_GAP))
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left_0()
                .right_0()
                .child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, (), window, _| gridlines(bounds, window, tick_count),
                    )
                    .size_full(),
                ),
        )
        .children(lanes.into_iter().map(|lane| {
            div()
                .relative()
                .w_full()
                .h(px(LANE_H))
                .children(lane.into_iter().map(|span| span_element(at, span, tips)))
        }))
        .when(!pins.is_empty(), |col| {
            col.child(
                div().relative().w_full().h(px(PIN_H)).children(
                    pins.into_iter()
                        .map(|pin| pin_element(at, pin, track_w, tips)),
                ),
            )
        });

    let axis_row = div()
        .relative()
        .w_full()
        .h(px(AXIS_H))
        .text_size(px(AXIS_SIZE))
        .text_color(rgb(OVERLAY1))
        .children(axis.into_iter().enumerate().map(|(idx, label)| {
            let frac = idx as f32 / segments;
            div()
                .absolute()
                .top_0()
                .left(relative(frac))
                .ml(px(-18.0))
                .w(px(36.0))
                .flex()
                .justify_center()
                .whitespace_nowrap()
                .child(label)
        }));

    let has_side = side.is_some();
    div()
        .flex()
        .items_stretch()
        .gap(px(SECTION_GAP))
        .when_some(side, |el, side| el.child(side_column(at, side)))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .when(!has_side, |el| el.pl(px(TRACK_RIGHT_PAD)))
                .pr(px(TRACK_RIGHT_PAD))
                .flex()
                .flex_col()
                .justify_between()
                .gap(px(6.0))
                .child(swimlanes)
                .child(axis_row),
        )
        .into_any_element()
}
