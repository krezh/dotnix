//! Something on its way through a series of stops, each reached, on its way, or not yet, such as a
//! pull request through the branches it lands in, or a task through its steps, one to a line; and
//! a path drawn across, such as the hops traffic takes out to the internet.

use std::{path::PathBuf, time::Instant};

use gpui::{
    AnimationExt, AnyElement, BoxShadow, Div, FontWeight, Hsla, SharedString, SpringAnimation,
    SpringConfig, div, linear_color_stop, linear_gradient, point, prelude::*, px, relative, rgb,
    rgba,
};

use crate::{
    components::{icon, motion, reveal},
    theme::{CONTROL_ALPHA, GREEN, OVERLAY2, SUBTEXT1, SURFACE1, TEXT, YELLOW, with_alpha},
    ui::{self, CONTROL_RADIUS, Radius, SMALL_SIZE},
};

/// A path's links, faint enough to read as the way between its stops.
const LINK_ALPHA: u8 = 0xb3;
/// A stop's square: the card's own tone, made solid.
const NODE_FILL: u32 = 0x2a2a3d;
const NODE_RING_ALPHA: u8 = 0x59;

const DETAIL_SIZE: f32 = 12.0;
const DOT: f32 = 7.0;
const LINK: f32 = 14.0;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StopState {
    Reached,
    /// On its way, with how far it has to go, such as "+63".
    Pending(Option<SharedString>),
    NotYet,
}

pub struct Stop {
    pub label: SharedString,
    pub state: StopState,
}

pub struct Route {
    /// With the detail, left out when empty.
    pub title: SharedString,
    pub detail: SharedString,
    pub done: Option<SharedString>,
    /// One stop to a line, joined down the left, rather than wrapping across.
    pub vertical: bool,
    pub stops: Vec<Stop>,
}

const STEP_H: f32 = 22.0;

fn stop(stop: Stop) -> Div {
    let (mark, text) = match stop.state {
        StopState::Reached => (GREEN, TEXT),
        StopState::Pending(_) => (YELLOW, TEXT),
        StopState::NotYet => (OVERLAY2, OVERLAY2),
    };
    let dot = div().size(px(DOT)).flex_none().rounded_full();
    let dot = match stop.state {
        StopState::NotYet => dot.border_1().border_color(rgb(mark)).into_any_element(),
        _ => dot.bg(rgb(mark)).into_any_element(),
    };
    let mut element = div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.0))
        .child(dot)
        .child(div().text_color(rgb(text)).child(stop.label));
    if let StopState::Pending(Some(left)) = stop.state {
        element = element.child(div().text_color(rgb(SUBTEXT1)).child(left));
    }
    element
}

pub fn routes(routes: Vec<Route>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .children(routes.into_iter().enumerate().map(|(index, route)| {
            let stops = if route.vertical {
                steps(route.stops)
            } else {
                across(route.stops)
            };
            let titled = !route.title.is_empty();
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .py(px(8.0))
                .when(index > 0, |row| {
                    row.border_t_1()
                        .border_color(rgba(with_alpha(SURFACE1, CONTROL_ALPHA)))
                })
                .when(titled, |route_row| {
                    route_row.child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(12.0))
                            .child(
                                div()
                                    .min_w(px(0.0))
                                    .truncate()
                                    .text_size(px(SMALL_SIZE))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(route.title),
                            )
                            .when_some(route.done, |line, done| {
                                line.child(
                                    div()
                                        .flex_none()
                                        .text_size(px(DETAIL_SIZE))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(GREEN))
                                        .child(done),
                                )
                            }),
                    )
                })
                .when(titled, |route_row| {
                    route_row.child(
                        div()
                            .min_w(px(0.0))
                            .truncate()
                            .text_size(px(DETAIL_SIZE))
                            .text_color(rgb(SUBTEXT1))
                            .child(route.detail),
                    )
                })
                .child(stops)
        }))
        .into_any_element()
}

fn link() -> gpui::Rgba {
    rgba(with_alpha(SURFACE1, CONTROL_ALPHA))
}

fn across(stops: Vec<Stop>) -> Div {
    let mut row = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_y(px(4.0))
        .text_size(px(DETAIL_SIZE));
    for (position, next) in stops.into_iter().enumerate() {
        if position > 0 {
            row = row.child(
                div()
                    .w(px(LINK))
                    .mx(px(6.0))
                    .h(px(1.0))
                    .flex_none()
                    .bg(link()),
            );
        }
        row = row.child(stop(next));
    }
    row
}

/// Each stop on its own line, a short line joining its dot to the one above.
fn steps(stops: Vec<Stop>) -> Div {
    div()
        .flex()
        .flex_col()
        .text_size(px(DETAIL_SIZE))
        .children(stops.into_iter().enumerate().map(|(position, next)| {
            let entered = SharedString::from(format!("route-step-{}", next.label));
            let row = div()
                .relative()
                .h(px(STEP_H))
                .flex()
                .items_center()
                .when(position > 0, |row| {
                    row.child(
                        div()
                            .absolute()
                            .left(px(DOT / 2.0 - 0.5))
                            .top(px(-(STEP_H - DOT) / 2.0))
                            .w(px(1.0))
                            .h(px(STEP_H - DOT))
                            .bg(link()),
                    )
                })
                .child(stop(next).min_w(px(0.0)));
            reveal::appear(entered, row)
        }))
}

/// One stop on a path drawn across, such as a router traffic passes on its way out.
pub struct Hop {
    pub id: SharedString,
    pub glyph: Option<PathBuf>,
    pub name: SharedString,
    /// Fainter, under the name, such as its address.
    pub address: SharedString,
    /// Under the address, such as how long it takes to answer.
    pub value: SharedString,
    pub value_color: u32,
    /// The glyph's color, and the link's leading into it.
    pub color: u32,
    /// Off, it folds away and its neighbours close up.
    pub shown: bool,
}

/// Stops side by side, joined by a line in each link's color; `down` fades all past the first,
/// such as everything beyond a machine whose link is down.
pub struct Path {
    pub id: SharedString,
    pub hops: Vec<Hop>,
    pub down: bool,
}

const NODE: f32 = 34.0;
const NAME_LINE: f32 = 17.0;
/// Room either side of a stop's lines, so neighbours' text never touches.
const HOP_PAD: f32 = 4.0;
const DETAIL_LINE: f32 = 15.0;
const NODE_GLYPH: f32 = 16.0;
const WIRE: f32 = 2.0;
const DOWN_OPACITY: f32 = 0.35;
const FOLD: SpringConfig = crate::spring::SOFT_FOLD;

/// The least share of the width a stop takes, in characters, so a short one keeps its node
/// clear of its neighbours'.
const MIN_WEIGHT: usize = 7;
/// A pulse of light running the path's length, and how many follow each other along it.
const PULSE_LAP: f32 = 2.6;
const PULSES: usize = 3;
const PULSE_W: f32 = 22.0;
const PULSE_ALPHA: u8 = 0xd9;
/// Round each node, in the card's own tone, so the link stops short of it.
const NODE_GAP: f32 = 4.0;
const NODE_GAP_FILL: u32 = 0x242436;

thread_local! {
    static PULSE_START: Instant = Instant::now();
}

/// Where along the path, 0 to 1, each pulse is now.
fn pulses() -> Vec<f32> {
    let lap = PULSE_START.with(|start| start.elapsed().as_secs_f32() / PULSE_LAP);
    (0..PULSES)
        .map(|index| (lap + index as f32 / PULSES as f32).fract())
        .collect()
}

/// Half a link, from a stop's centre to its edge; `along` is where it runs on the whole path,
/// so pulses passing through it show.
fn half(color: Option<u32>, right: bool, along: Option<(f32, f32)>, pulses: &[f32]) -> Div {
    let line = div()
        .absolute()
        .top(px((NODE - WIRE) / 2.0))
        .h(px(WIRE))
        .w_1_2();
    let line = if right { line.right_0() } else { line.left_0() };
    let Some(color) = color else {
        return line;
    };
    let glow: Hsla = rgba(with_alpha(0xffffff, PULSE_ALPHA)).into();
    let clear: Hsla = rgba(with_alpha(0xffffff, 0)).into();
    line.overflow_hidden()
        .bg(rgba(with_alpha(color, LINK_ALPHA)))
        .children(along.into_iter().flat_map(|(from, to)| {
            pulses
                .iter()
                .filter(move |at| **at >= from && **at < to)
                .map(move |at| {
                    let local = (at - from) / (to - from);
                    div()
                        .absolute()
                        .top(px(-1.0))
                        .left(relative(local))
                        .ml(px(-PULSE_W / 2.0))
                        .w(px(PULSE_W))
                        .h(px(WIRE + 2.0))
                        .flex()
                        .child(div().flex_1().h_full().bg(linear_gradient(
                            90.0,
                            linear_color_stop(clear, 0.0),
                            linear_color_stop(glow, 1.0),
                        )))
                        .child(div().flex_1().h_full().bg(linear_gradient(
                            90.0,
                            linear_color_stop(glow, 0.0),
                            linear_color_stop(clear, 1.0),
                        )))
                })
        }))
}

pub fn path(path: Path) -> AnyElement {
    let count = path.hops.len();
    // Each link is drawn as the halves of the two stops it joins, so a stop folding away takes
    // its halves with it and its neighbours' meet.
    let into: Vec<u32> = path.hops.iter().map(|hop| hop.color).collect();
    let shown: Vec<bool> = path.hops.iter().map(|hop| hop.shown).collect();
    let next_shown = |at: usize| (at + 1..count).find(|&index| shown[index]);
    let first_shown = shown.iter().position(|shown| *shown);
    // Each shown stop's place along the path, 0 at the first and 1 at the last, for the pulses.
    let places = shown.iter().filter(|shown| **shown).count().saturating_sub(1).max(1) as f32;
    let mut place = 0.0;
    let along: Vec<f32> = shown
        .iter()
        .map(|shown| {
            let here = place;
            if *shown {
                place += 1.0;
            }
            here / places
        })
        .collect();
    let half_step = 0.5 / places;
    let lit = if path.down { Vec::new() } else { pulses() };
    div()
        .flex()
        .w_full()
        .children(path.hops.into_iter().enumerate().map(|(index, hop)| {
            let leaving = next_shown(index).map(|next| into[next]);
            let entering = (Some(index) != first_shown).then_some(hop.color);
            let faded = path.down && Some(index) != first_shown;
            // Its share of the width follows its longest line, so a long name gets the room it
            // needs from short neighbours rather than being cut.
            let weight = [&hop.name, &hop.address, &hop.value]
                .iter()
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(0)
                .max(MIN_WEIGHT) as f32;
            let at = along[index];
            let left_half = half(
                entering,
                false,
                hop.shown.then_some((at - half_step, at)),
                &lit,
            );
            let right_half = half(leaving, true, hop.shown.then_some((at, at + half_step)), &lit);
            // Solid, and ringed in the card's tone, so the link stops short of it.
            let node = div()
                .relative()
                .size(px(NODE))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(CONTROL_RADIUS))
                .shadow(vec![BoxShadow {
                    color: rgb(NODE_GAP_FILL).into(),
                    offset: point(px(0.0), px(0.0)),
                    blur_radius: px(0.0),
                    spread_radius: px(NODE_GAP),
                    inset: false,
                }])
                .child(ui::outlined(
                    Radius::Control,
                    rgb(NODE_FILL).into(),
                    rgba(with_alpha(hop.color, NODE_RING_ALPHA)).into(),
                ))
                .when_some(hop.glyph, |node, glyph| {
                    node.child(icon::glyph(&glyph, NODE_GLYPH, hop.color))
                });
            let stop = div()
                .relative()
                .min_w(px(0.0))
                .overflow_hidden()
                .flex()
                .flex_col()
                .items_center()
                .whitespace_nowrap()
                .child(left_half)
                .child(right_half)
                .child(node)
                // Longer than its column, a line ends in an ellipsis rather than being cut at both
                // sides of the centre.
                .child(
                    div()
                        .mt(px(6.0))
                        .w_full()
                        .px(px(HOP_PAD))
                        .truncate()
                        .text_center()
                        .text_size(px(SMALL_SIZE))
                        .line_height(px(NAME_LINE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(hop.name),
                )
                .child(
                    div()
                        .w_full()
                        .px(px(HOP_PAD))
                        .truncate()
                        .text_center()
                        .text_size(px(DETAIL_SIZE))
                        .line_height(px(DETAIL_LINE))
                        .text_color(rgb(SUBTEXT1))
                        .child(hop.address),
                )
                .child(
                    div()
                        .text_size(px(DETAIL_SIZE))
                        .line_height(px(DETAIL_LINE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(hop.value_color))
                        .child(motion::text(
                            SharedString::from(format!("{}-value", hop.id)),
                            hop.value,
                        )),
                );
            let fold = SharedString::from(format!("{}-{}-fold", path.id, hop.id));
            div()
                .flex_basis(px(0.0))
                .min_w(px(0.0))
                .child(stop)
                .with_spring(
                    fold,
                    SpringAnimation::new(FOLD).to(if hop.shown { 1.0 } else { 0.0 }),
                    move |mut slot, grown| {
                        let grown = grown.clamp(0.0, 1.0);
                        slot.style().flex_grow = Some((weight * grown).max(0.0001));
                        slot.opacity(if faded { grown * DOWN_OPACITY } else { grown })
                    },
                )
        }))
        .into_any_element()
}
