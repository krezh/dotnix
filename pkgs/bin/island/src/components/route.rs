//! Something on its way through a series of stops, each reached, on its way, or not yet, such as a
//! pull request through the branches it lands in, or a task through its steps, one to a line.

use gpui::{AnyElement, Div, FontWeight, SharedString, div, prelude::*, px, rgb, rgba};

use crate::{
    components::reveal,
    theme::{CONTROL_ALPHA, GREEN, OVERLAY2, SUBTEXT1, SURFACE1, TEXT, YELLOW, with_alpha},
    ui::SMALL_SIZE,
};

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
