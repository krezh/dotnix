//! Things coming into view and going out of it: rising a little as they fade in, and sliding
//! aside as they fade out, the way a pop-up card does.

use std::time::Duration;

use gpui::{AnimationExt, AnyElement, ElementId, SpringAnimation, div, prelude::*, px};

use crate::island::{CONTENT_DRIFT, CONTENT_IN, CONTENT_OUT};

/// How long [`leave`] takes, for whatever keeps the element around until it is gone.
pub const LEAVE: Duration = Duration::from_millis(220);

/// Fades `element` in and lets it settle into place the first time `id` is drawn; drawn again
/// under the same id, it stays put.
pub fn appear(id: impl Into<ElementId>, element: impl IntoElement) -> AnyElement {
    div()
        .relative()
        .child(element)
        .with_spring(
            id,
            SpringAnimation::new(CONTENT_IN).to(1.0_f32).from(0.0),
            |shown, value| {
                let value = value.clamp(0.0, 1.0);
                shown.opacity(value).top(px((1.0 - value) * CONTENT_DRIFT))
            },
        )
        .into_any_element()
}

/// Fades `element` out as it slides aside, once, from the first time `id` is drawn.
pub fn leave(id: impl Into<ElementId>, element: impl IntoElement) -> AnyElement {
    div()
        .relative()
        .child(element)
        .with_spring(
            id,
            SpringAnimation::new(CONTENT_OUT).to(0.0_f32).from(1.0),
            |gone, value| {
                let value = value.clamp(0.0, 1.0);
                gone.opacity(value).top(px((1.0 - value) * CONTENT_DRIFT))
            },
        )
        .into_any_element()
}
