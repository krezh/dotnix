use std::{cell::Cell, rc::Rc};

use gpui::{
    AnimationExt, AnyElement, Bounds, Context, MouseButton, MouseDownEvent, MouseMoveEvent, Pixels, SharedString,
    SpringAnimation, SpringConfig, canvas, div, prelude::*, px, relative, rgb, rgba,
};

use crate::{
    island::Island,
    theme::{TINT, TRACK_ALPHA, with_alpha},
    ui::{self, Radius},
};

pub type OnChange = Rc<dyn Fn(f32, &mut Island, &mut Context<Island>)>;

const FILL_SPRING: SpringConfig = SpringConfig::new(420.0, 34.0, 1.0);
const TRACK_H: f32 = 10.0;
const HIT_H: f32 = 16.0;

fn fraction(bounds: &Cell<Option<Bounds<Pixels>>>, x: Pixels) -> Option<f32> {
    let bounds = bounds.get()?;
    let width = f32::from(bounds.size.width);
    (width > 0.0).then(|| (f32::from(x - bounds.origin.x) / width).clamp(0.0, 1.0))
}

pub fn slider(
    id: impl Into<SharedString>,
    value: f32,
    color: u32,
    on_change: Option<OnChange>,
    cx: &mut Context<Island>,
) -> AnyElement {
    let id = id.into();
    let value = value.clamp(0.0, 1.0);
    let bounds: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
    let fill = div()
        .absolute()
        .top_0()
        .left_0()
        .h_full()
        .with_spring(
            SharedString::from(format!("{id}-fill")),
            SpringAnimation::new(FILL_SPRING).to(value),
            move |fill, level| {
                fill.w(relative(level.clamp(0.0, 1.0)))
                    .child(ui::shape(Radius::Pill, Some(rgb(color).into()), None))
            },
        );
    let track = div()
        .relative()
        .w_full()
        .h(px(TRACK_H))
        .child(ui::shape(Radius::Pill, Some(rgba(with_alpha(TINT, TRACK_ALPHA)).into()), None))
        .child(fill)
        .child({
            let bounds = bounds.clone();
            canvas(move |area, _, _| bounds.set(Some(area)), |_, _, _, _| {})
                .absolute()
                .size_full()
        });
    let mut element = div()
        .id(id)
        .relative()
        .w_full()
        .py(px((HIT_H - TRACK_H) / 2.0))
        .child(track);
    if let Some(on_change) = on_change {
        let down = on_change.clone();
        let down_bounds = bounds.clone();
        element = element
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    if let Some(value) = fraction(&down_bounds, event.position.x) {
                        down(value, this, cx);
                    }
                }),
            )
            .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
                if event.pressed_button == Some(MouseButton::Left)
                    && let Some(value) = fraction(&bounds, event.position.x)
                {
                    on_change(value, this, cx);
                }
            }));
    }
    element.into_any_element()
}
