//! Something on its way through a series of stops, each reached, on its way, or not yet, such as a
//! pull request through the branches it lands in.

use gpui::{AnyElement, Div, FontWeight, SharedString, div, prelude::*, px, rgb, rgba};

use crate::{
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
    pub title: SharedString,
    pub detail: SharedString,
    pub done: Option<SharedString>,
    pub stops: Vec<Stop>,
}

fn stop(stop: Stop) -> Div {
    let (mark, text) = match stop.state {
        StopState::Reached => (GREEN, TEXT),
        StopState::Pending(_) => (YELLOW, TEXT),
        StopState::NotYet => (OVERLAY2, OVERLAY2),
    };
    let dot = div().size(px(DOT)).flex_none().rounded_full();
    let dot = match stop.state {
        StopState::NotYet => dot.border_1().border_color(rgb(mark)),
        _ => dot.bg(rgb(mark)),
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
            let mut stops = div().flex().flex_wrap().items_center().gap_y(px(4.0)).text_size(px(DETAIL_SIZE));
            for (position, next) in route.stops.into_iter().enumerate() {
                if position > 0 {
                    stops = stops.child(
                        div()
                            .w(px(LINK))
                            .mx(px(6.0))
                            .h(px(1.0))
                            .flex_none()
                            .bg(rgba(with_alpha(SURFACE1, CONTROL_ALPHA))),
                    );
                }
                stops = stops.child(stop(next));
            }
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .py(px(8.0))
                .when(index > 0, |row| row.border_t_1().border_color(rgba(with_alpha(SURFACE1, CONTROL_ALPHA))))
                .child(
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
                .child(
                    div()
                        .min_w(px(0.0))
                        .truncate()
                        .text_size(px(DETAIL_SIZE))
                        .text_color(rgb(SUBTEXT1))
                        .child(route.detail),
                )
                .child(stops)
        }))
        .into_any_element()
}
