//! Concentric rings, each filled part way round from the top with an optional tick across it and
//! a faint continuation past its fill, around a value or a glyph in the middle, such as how much
//! of each window of a quota is used and where it is expected to end.

use std::{
    f32::consts::{FRAC_PI_2, TAU},
    path::PathBuf,
};

use gpui::{
    AnimationExt, AnyElement, Bounds, FontWeight, Hsla, PathBuilder, Pixels, Point, SharedString,
    SpringAnimation, Window, canvas, div, fill, point, prelude::*, px, relative, rgb, rgba, size,
};

use crate::{
    components::{icon, motion},
    island::CONTENT_IN,
    theme::{SUBTEXT1, TEXT, TINT, TRACK_ALPHA, with_alpha},
};

const STROKE: f32 = 7.0;
const RING_GAP: f32 = 3.0;
const TICK_W: f32 = 2.0;
/// How far a tick reaches past its ring, inside and out.
const TICK_OVERHANG: f32 = 2.5;
/// Straight pieces a whole ring is drawn with.
const STEPS: f32 = 120.0;
/// How strongly the continuation past a fill shows.
const AHEAD_ALPHA: u8 = 0x52;
const VALUE: f32 = 18.0;
const LABEL: f32 = 10.5;

pub struct Ring {
    pub fraction: f32,
    pub color: u32,
    /// A tick across the ring at this fraction, such as how much of a window has passed.
    pub mark: Option<f32>,
    /// A faint continuation of the fill up to here, such as where it is expected to end.
    pub ahead: Option<f32>,
}

/// Outermost first.
pub struct Rings {
    pub id: SharedString,
    pub size: f32,
    pub rings: Vec<Ring>,
    pub value: SharedString,
    pub value_color: u32,
    pub label: SharedString,
    /// Drawn in the middle, in its color and at its size, in place of the value and label.
    pub glyph: Option<(PathBuf, u32, f32)>,
}

fn at(x: f32, y: f32) -> Point<Pixels> {
    point(px(x), px(y))
}

/// The point `fraction` of the way round from the top, clockwise.
fn round(center: (f32, f32), radius: f32, fraction: f32) -> (f32, f32) {
    let angle = fraction * TAU - FRAC_PI_2;
    (
        center.0 + radius * angle.cos(),
        center.1 + radius * angle.sin(),
    )
}

fn cap(window: &mut Window, (x, y): (f32, f32), color: Hsla) {
    window.paint_quad(
        fill(
            Bounds {
                origin: at(x - STROKE / 2.0, y - STROKE / 2.0),
                size: size(px(STROKE), px(STROKE)),
            },
            color,
        )
        .corner_radii(px(STROKE / 2.0)),
    );
}

/// How the ends of an arc are drawn.
#[derive(Clone, Copy, PartialEq)]
enum Ends {
    Round,
    Square,
    /// Closed into a whole ring.
    Closed,
}

/// From `from` to `to` of the way round, clockwise from the top.
fn paint_arc(
    window: &mut Window,
    center: (f32, f32),
    radius: f32,
    (from, to): (f32, f32),
    color: Hsla,
    ends: Ends,
) {
    let (from, to) = (from.clamp(0.0, 1.0), to.clamp(0.0, 1.0));
    if to <= from {
        return;
    }
    let steps = ((to - from) * STEPS).ceil().max(2.0) as usize;
    let mut path = PathBuilder::stroke(px(STROKE));
    let start = round(center, radius, from);
    path.move_to(at(start.0, start.1));
    for step in 1..=steps {
        let (x, y) = round(
            center,
            radius,
            from + (to - from) * step as f32 / steps as f32,
        );
        path.line_to(at(x, y));
    }
    if ends == Ends::Closed {
        path.close();
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
    if ends == Ends::Round {
        cap(window, start, color);
        cap(window, round(center, radius, to), color);
    }
}

fn paint_tick(window: &mut Window, center: (f32, f32), radius: f32, fraction: f32) {
    let reach = STROKE / 2.0 + TICK_OVERHANG;
    let (x0, y0) = round(center, radius - reach, fraction);
    let (x1, y1) = round(center, radius + reach, fraction);
    let mut path = PathBuilder::stroke(px(TICK_W));
    path.move_to(at(x0, y0));
    path.line_to(at(x1, y1));
    if let Ok(path) = path.build() {
        window.paint_path(path, rgb(TEXT));
    }
}

/// The middle of `bounds`, and the radius of each ring's centre line, outermost first.
fn geometry(bounds: Bounds<Pixels>, count: usize) -> ((f32, f32), Vec<f32>) {
    let w = f32::from(bounds.size.width);
    let h = f32::from(bounds.size.height);
    let center = (
        f32::from(bounds.origin.x) + w / 2.0,
        f32::from(bounds.origin.y) + h / 2.0,
    );
    let outer = w.min(h) / 2.0 - STROKE / 2.0;
    let radii = (0..count)
        .map(|index| outer - index as f32 * (STROKE + RING_GAP))
        .collect();
    (center, radii)
}

fn layer() -> gpui::Div {
    div().absolute().top_0().left_0().size_full()
}

pub fn rings(rings: Rings) -> AnyElement {
    let Rings {
        id,
        size: side,
        rings,
        value,
        value_color,
        label,
        glyph,
    } = rings;
    let count = rings.len();
    let tracks = canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            let (center, radii) = geometry(bounds, count);
            let track: Hsla = rgba(with_alpha(TINT, TRACK_ALPHA / 2)).into();
            for radius in radii {
                paint_arc(window, center, radius, (0.0, 1.0), track, Ends::Closed);
            }
        },
    )
    .size_full();
    // Each fill and tick glides to its new place on its own spring.
    let moving = rings.into_iter().enumerate().flat_map(|(index, ring)| {
        let color: Hsla = rgb(ring.color).into();
        let faint: Hsla = rgba(with_alpha(ring.color, AHEAD_ALPHA)).into();
        let start = ring.fraction.clamp(0.0, 1.0);
        let ahead = ring.ahead.map(|ahead| {
            layer().with_spring(
                SharedString::from(format!("{id}-ahead-{index}")),
                SpringAnimation::new(CONTENT_IN).to(ahead.clamp(start, 1.0)),
                move |layer, ahead| {
                    layer.child(
                        canvas(
                            |_, _, _| {},
                            move |bounds, (), window, _| {
                                let (center, radii) = geometry(bounds, count);
                                let span = (start, ahead);
                                paint_arc(window, center, radii[index], span, faint, Ends::Square);
                            },
                        )
                        .size_full(),
                    )
                },
            )
        });
        let fill = layer().with_spring(
            SharedString::from(format!("{id}-ring-{index}")),
            SpringAnimation::new(CONTENT_IN).to(ring.fraction.clamp(0.0, 1.0)),
            move |layer, fraction| {
                layer.child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, (), window, _| {
                            let (center, radii) = geometry(bounds, count);
                            let span = (0.0, fraction);
                            paint_arc(window, center, radii[index], span, color, Ends::Round);
                        },
                    )
                    .size_full(),
                )
            },
        );
        let tick = ring.mark.map(|mark| {
            layer().with_spring(
                SharedString::from(format!("{id}-tick-{index}")),
                SpringAnimation::new(CONTENT_IN).to(mark.clamp(0.0, 1.0)),
                move |layer, mark| {
                    layer.child(
                        canvas(
                            |_, _, _| {},
                            move |bounds, (), window, _| {
                                let (center, radii) = geometry(bounds, count);
                                paint_tick(window, center, radii[index], mark);
                            },
                        )
                        .size_full(),
                    )
                },
            )
        });
        [
            ahead.map(IntoElement::into_any_element),
            Some(fill.into_any_element()),
            tick.map(IntoElement::into_any_element),
        ]
        .into_iter()
        .flatten()
    });
    let hole = side - 2.0 * count as f32 * (STROKE + RING_GAP);
    let middle = layer().flex().flex_col().items_center().justify_center();
    let middle = match glyph {
        Some((path, color, side)) => middle.child(icon::glyph(&path, side, color)),
        None => middle
            .child(
                div()
                    .text_size(px(VALUE))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(value_color))
                    .line_height(relative(1.15))
                    .child(motion::text(
                        SharedString::from(format!("{id}-value")),
                        value,
                    )),
            )
            .child(
                div()
                    .max_w(px(hole.max(0.0)))
                    .truncate()
                    .text_size(px(LABEL))
                    .text_color(rgb(SUBTEXT1))
                    .child(label),
            ),
    };
    div()
        .relative()
        .flex_none()
        .size(px(side))
        .child(layer().child(tracks))
        .children(moving)
        .child(middle)
        .into_any_element()
}
