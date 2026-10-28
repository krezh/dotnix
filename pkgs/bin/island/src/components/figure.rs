//! Values that matter at a glance, each with a smaller unit and a caption, such as how long the
//! internet takes to answer: large ones lead, small ones gather at the far end.

use gpui::{AnyElement, Div, FontWeight, SharedString, div, prelude::*, px, rgb};

use crate::{
    components::motion,
    theme::{OVERLAY2, TEXT},
};

const LARGE: f32 = 30.0;
const LARGE_UNIT: f32 = 15.0;
const SMALL: f32 = 15.0;
const CAPTION: f32 = 12.0;
const DOT: f32 = 8.0;
const LARGE_GAP: f32 = 28.0;
/// The large value's descent below its baseline, so its smaller unit sits on that baseline.
const UNIT_DROP: f32 = 2.0;
/// Room for the widest value and its unit, such as "999.9 MB/s" and "99.9 ms".
const LARGE_SLOT: f32 = 100.0;
const SMALL_SLOT: f32 = 58.0;
const SMALL_GAP: f32 = 18.0;

pub struct Figure {
    pub id: SharedString,
    pub value: SharedString,
    pub unit: SharedString,
    pub caption: SharedString,
    /// A dot before the caption, such as the color the value is drawn in on a chart.
    pub dot: Option<u32>,
    /// The value itself in a color, such as a state.
    pub color: Option<u32>,
    pub large: bool,
    /// Its caption follows it on its line, and such figures stack at the far end, such as what
    /// today and this week add up to.
    pub inline: bool,
}

/// A value with its caption after it on one line.
fn inline(figure: Figure) -> AnyElement {
    div()
        .flex()
        .items_baseline()
        .justify_end()
        .gap(px(5.0))
        .whitespace_nowrap()
        .child(
            div()
                .text_size(px(SMALL))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(figure.color.unwrap_or(TEXT)))
                .child(motion::text(
                    SharedString::from(format!("{}-value", figure.id)),
                    figure.value,
                )),
        )
        .child(
            div()
                .text_size(px(CAPTION))
                .text_color(rgb(OVERLAY2))
                .child(figure.caption),
        )
        .into_any_element()
}

fn caption(figure_id: &SharedString, text: SharedString, dot: Option<u32>) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .text_size(px(CAPTION))
        .text_color(rgb(OVERLAY2))
        .whitespace_nowrap()
        .when_some(dot, |line, color| {
            line.child(div().size(px(DOT)).flex_none().rounded_full().bg(rgb(color)))
        })
        .child(motion::text(
            SharedString::from(format!("{figure_id}-caption")),
            text,
        ))
}

fn figure(figure: Figure) -> AnyElement {
    let (size, unit_size) = if figure.large {
        (LARGE, LARGE_UNIT)
    } else {
        (SMALL, SMALL)
    };
    // Both sit on one line box at the value's height with their text at its foot, so the unit
    // lines up with the value's baseline rather than floating at its top.
    let value = div()
        .flex()
        .items_end()
        .gap(px(3.0))
        .whitespace_nowrap()
        .child(
            div()
                .text_size(px(size))
                .font_weight(FontWeight::SEMIBOLD)
                .line_height(px(size))
                .text_color(rgb(figure.color.unwrap_or(TEXT)))
                .child(motion::text(
                    SharedString::from(format!("{}-value", figure.id)),
                    figure.value,
                )),
        )
        .when(!figure.unit.is_empty(), |value| {
            value.child(
                div()
                    .text_size(px(unit_size))
                    .line_height(px(unit_size))
                    .pb(px(if figure.large { UNIT_DROP } else { 0.0 }))
                    .when(figure.large, |unit| unit.text_color(rgb(OVERLAY2)))
                    .when(!figure.large, |unit| unit.font_weight(FontWeight::SEMIBOLD))
                    .child(figure.unit),
            )
        });
    // Each sits in a slot of its own width, so a value growing a digit moves nothing beside it.
    div()
        .flex()
        .flex_col()
        .gap(px(if figure.large { 6.0 } else { 2.0 }))
        .min_w(px(if figure.large { LARGE_SLOT } else { SMALL_SLOT }))
        .when(!figure.large, |column| column.items_end())
        .child(value)
        .child(caption(&figure.id, figure.caption, figure.dot))
        .into_any_element()
}

/// `beside` sits at the far end before the small figures, such as what the values add up to.
pub fn figures(figures: Vec<Figure>, beside: Option<AnyElement>) -> AnyElement {
    let (large, small): (Vec<Figure>, Vec<Figure>) =
        figures.into_iter().partition(|figure| figure.large);
    let (stacked, small): (Vec<Figure>, Vec<Figure>) =
        small.into_iter().partition(|figure| figure.inline);
    let stack = (!stacked.is_empty()).then(|| {
        div()
            .flex()
            .flex_col()
            .items_end()
            .gap(px(4.0))
            .children(stacked.into_iter().map(inline))
    });
    div()
        .flex()
        .items_end()
        .gap(px(SMALL_GAP))
        .child(
            div()
                .flex()
                .items_end()
                .gap(px(LARGE_GAP))
                .children(large.into_iter().map(figure)),
        )
        .child(div().flex_1())
        .children(beside)
        .children(stack)
        .child(
            div()
                .flex()
                .items_end()
                .gap(px(SMALL_GAP))
                .children(small.into_iter().map(figure)),
        )
        .into_any_element()
}
