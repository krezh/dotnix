//! The soft edges of something that scrolls: what passes under its top or bottom fades into the
//! island's glass, as if going behind it, each edge showing only while there is more that way.

use gpui::{
    AnyElement, Div, Hsla, ScrollHandle, Stateful, div, linear_color_stop, linear_gradient,
    prelude::*, px,
};

/// How far in from each edge content fades.
const EDGE_H: f32 = 44.0;
/// How far it scrolls before an edge shows fully, so it fades in rather than appearing.
const RAMP: f32 = 24.0;

/// `scrolling` with its edges over it, fading into `glass`, the color of the surface it is on.
pub fn edges(scrolling: Stateful<Div>, scroll: &ScrollHandle, glass: Hsla) -> AnyElement {
    // Kept to how far it can scroll: past its ends, as when scrolled further than there is, each
    // edge stays as it is at that end rather than jumping.
    let most = f32::from(scroll.max_offset().y).max(0.0);
    let above = (-f32::from(scroll.offset().y)).clamp(0.0, most);
    let below = most - above;
    let clear = Hsla { a: 0.0, ..glass };
    // Always drawn, only its strength changing, so it never blinks in and out at an end.
    let edge = |shown: f32, from_top: bool| {
        let (start, end) = if from_top {
            (glass, clear)
        } else {
            (clear, glass)
        };
        div()
            .absolute()
            .left_0()
            .right_0()
            .h(px(EDGE_H))
            .when(from_top, |edge| edge.top_0())
            .when(!from_top, |edge| edge.bottom_0())
            .opacity((shown / RAMP).clamp(0.0, 1.0))
            .bg(linear_gradient(
                180.0,
                linear_color_stop(start, 0.0),
                linear_color_stop(end, 1.0),
            ))
    };
    div()
        .relative()
        .flex()
        .flex_col()
        .min_h(px(0.0))
        .child(scrolling)
        .child(edge(above, true))
        .child(edge(below, false))
        .into_any_element()
}
