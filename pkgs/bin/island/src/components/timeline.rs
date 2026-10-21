//! Lanes along one stretch of time from now, such as the coming week: each lane a bar up to where
//! it ends, a tick where something comes sooner with the rest of the bar striped, and a note at
//! its end, over a shared axis.

use gpui::{
    AnimationExt, AnyElement, Bounds, ContentMask, Div, FontWeight, Hsla, PathBuilder, Pixels,
    SharedString, SpringAnimation, canvas, div, point, prelude::*, px, relative, rgb, rgba,
};

use crate::{
    island::CONTENT_IN,
    theme::{OVERLAY1, SUBTEXT1, TINT, TRACK_ALPHA, YELLOW, with_alpha},
};

const LABEL_W: f32 = 190.0;
const NOTE_W: f32 = 104.0;
const GAP: f32 = 10.0;
const LANE_H: f32 = 18.0;
const CAPTION: f32 = 12.0;
const AXIS: f32 = 10.5;
const LINE: f32 = 2.0;
const BAR: f32 = 6.0;
const TICK_W: f32 = 2.0;
const TICK_H: f32 = 14.0;
const STRIPE: f32 = 2.0;
const STRIPE_EVERY: f32 = 5.0;
const STRIPE_ALPHA: u8 = 0x99;

pub struct Lane {
    /// Names the lane's parts for their motion; stable while it shows.
    pub key: SharedString,
    pub label: SharedString,
    /// Where the bar ends, from 0 (now) to 1 (the end of the axis).
    pub to: f32,
    pub color: u32,
    /// Where something comes first, such as running out, which ends the solid bar there and
    /// stripes the rest of it.
    pub cut: Option<f32>,
    pub note: SharedString,
    pub note_color: u32,
}

pub struct Timeline {
    pub lanes: Vec<Lane>,
    /// Spread evenly from now to the end.
    pub axis: Vec<SharedString>,
}

/// Diagonal stripes across `bounds`, as for time spent without something.
fn stripes(bounds: Bounds<Pixels>, window: &mut gpui::Window, color: Hsla) {
    let (x0, y0) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        let mut x = x0 - h;
        while x < x0 + w {
            let mut path = PathBuilder::stroke(px(STRIPE));
            path.move_to(point(px(x), px(y0 + h)));
            path.line_to(point(px(x + h), px(y0)));
            if let Ok(path) = path.build() {
                window.paint_path(path, color);
            }
            x += STRIPE_EVERY;
        }
    });
}

/// From now to `to` of the track, gliding there.
fn bar(id: String, element: Div, to: f32) -> AnyElement {
    element
        .absolute()
        .left_0()
        .top(px((LANE_H - BAR) / 2.0))
        .h(px(BAR))
        .rounded_full()
        .overflow_hidden()
        .with_spring(
            SharedString::from(id),
            SpringAnimation::new(CONTENT_IN).to(to.clamp(0.0, 1.0)),
            |bar, at| bar.w(relative(at.clamp(0.0, 1.0))),
        )
        .into_any_element()
}

fn track(at: &str, lane: &Lane) -> Div {
    let to = lane.to.clamp(0.0, 1.0);
    let cut = lane.cut.filter(|cut| *cut < to);
    let striped: Hsla = rgba(with_alpha(YELLOW, STRIPE_ALPHA)).into();
    div()
        .relative()
        .flex_1()
        .min_w(px(0.0))
        .h(px(LANE_H))
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(px((LANE_H - LINE) / 2.0))
                .h(px(LINE))
                .rounded_full()
                .bg(rgba(with_alpha(TINT, TRACK_ALPHA / 2))),
        )
        // The stripes run the whole bar; the solid part covers them up to the cut.
        .when(cut.is_some(), |track| {
            track.child(bar(
                format!("{at}-{}-striped", lane.key),
                div().child(
                    canvas(|_, _, _| {}, move |bounds, (), window, _| {
                        stripes(bounds, window, striped)
                    })
                    .size_full(),
                ),
                to,
            ))
        })
        .child(bar(
            format!("{at}-{}-bar", lane.key),
            div().bg(rgb(lane.color)),
            cut.unwrap_or(to),
        ))
        .children(cut.map(|cut| {
            div()
                .absolute()
                .top(px((LANE_H - TICK_H) / 2.0))
                .ml(px(-TICK_W / 2.0))
                .w(px(TICK_W))
                .h(px(TICK_H))
                .rounded_full()
                .bg(rgb(YELLOW))
                .with_spring(
                    SharedString::from(format!("{at}-{}-cut", lane.key)),
                    SpringAnimation::new(CONTENT_IN).to(cut.clamp(0.0, 1.0)),
                    |tick, at| tick.left(relative(at.clamp(0.0, 1.0))),
                )
        }))
}

/// The label, track and note columns every row shares, so the axis lines up with the lanes.
fn row() -> Div {
    div().flex().items_center().gap(px(GAP))
}

fn lane(at: &str, lane: Lane) -> Div {
    let track = track(at, &lane);
    row()
        .text_size(px(CAPTION))
        .child(
            div()
                .w(px(LABEL_W))
                .flex_none()
                .truncate()
                .text_color(rgb(SUBTEXT1))
                .child(lane.label),
        )
        .child(track)
        .child(
            div()
                .w(px(NOTE_W))
                .flex_none()
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(lane.note_color))
                .child(lane.note),
        )
}

pub fn timeline(timeline: Timeline, at: &str) -> AnyElement {
    let Timeline { lanes, axis } = timeline;
    div()
        .flex()
        .flex_col()
        .gap(px(GAP))
        .children(lanes.into_iter().map(|piece| lane(at, piece)))
        .child(
            row()
                .text_size(px(AXIS))
                .text_color(rgb(OVERLAY1))
                .child(div().w(px(LABEL_W)).flex_none())
                .child(div().flex_1().flex().justify_between().children(axis))
                .child(div().w(px(NOTE_W)).flex_none()),
        )
        .into_any_element()
}
