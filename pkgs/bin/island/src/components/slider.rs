//! A value from 0 to 1 that can be dragged, optionally with a live level over its fill. Hovering
//! one that can be dragged grows a handle out of its fill's end; holding it grows a little more.

use std::{cell::Cell, rc::Rc};

use gpui::{
    AnimationExt, AnyElement, Bounds, Context, DispatchPhase, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Pixels, SharedString, SpringAnimation, SpringConfig, canvas, div, prelude::*, px, relative, rgb, rgba,
};

use crate::{
    components::pointer::{self, Pointer},
    island::{Island, SHAPE_SPRING},
    theme::{TEXT, TINT, TRACK_ALPHA, with_alpha},
    ui::{self, Radius},
};

pub type OnChange = Rc<dyn Fn(f32, &mut Island, &mut Context<Island>)>;

const FILL_SPRING: SpringConfig = SpringConfig::new(420.0, 34.0, 1.0);
const TRACK_H: f32 = 10.0;
const HIT_H: f32 = 16.0;
/// The fill of a metered slider, under the live signal.
const DIM_ALPHA: u8 = 0x8c;
/// The handle sticks out 4 px past each side of the track while hovered, 5 px while held.
const HANDLE_W: f32 = 4.0;
const HOVER_H: f32 = TRACK_H + 8.0;
const HELD_H: f32 = TRACK_H + 10.0;

/// A slim pill on the fill's end, growing out of it.
fn handle(id: &SharedString, grip: Pointer) -> impl IntoElement {
    let target = match grip {
        Pointer { held: true, .. } => HELD_H,
        Pointer { hovered: true, .. } => HOVER_H,
        _ => 0.0,
    };
    div().absolute().with_spring(
        SharedString::from(format!("{id}-handle")),
        SpringAnimation::new(SHAPE_SPRING).to(target),
        |handle, height| {
            let height = height.max(0.0);
            let width = HANDLE_W + ((height - HOVER_H) / (HELD_H - HOVER_H)).clamp(0.0, 1.0);
            handle
                .w(px(width))
                .h(px(height))
                .right(px(-width / 2.0))
                .top(px((TRACK_H - height) / 2.0))
                .rounded(px(width / 2.0))
                .bg(rgb(TEXT))
                .when(height > 0.5, |handle| handle.shadow_sm())
        },
    )
}

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
    build(id.into(), value, color, None, on_change, true, cx)
}

pub struct Slider {
    pub id: SharedString,
    pub value: f32,
    pub color: u32,
    /// Dims the fill and draws a live level over it, as a share of the value: 1 reaches the end
    /// of the fill.
    pub signal: Option<f32>,
    pub on_change: Option<OnChange>,
}

/// `live` is whether the pointer is over the island at all: GPUI's hover goes stale once it leaves.
pub fn view(slider: Slider, live: bool, cx: &mut Context<Island>) -> AnyElement {
    let signal = slider.signal.map(|signal| signal.clamp(0.0, 1.0));
    build(slider.id, slider.value, slider.color, signal, slider.on_change, live, cx)
}

fn build(
    id: SharedString,
    value: f32,
    color: u32,
    signal: Option<f32>,
    on_change: Option<OnChange>,
    live: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let value = value.clamp(0.0, 1.0);
    let bounds: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
    let fill_color = if signal.is_some() { rgba(with_alpha(color, DIM_ALPHA)) } else { rgb(color) };
    let pointer = pointer::get(&id);
    // A hold outlasts the pointer leaving the island; hovering does not.
    let grip = Pointer {
        hovered: live && pointer.hovered,
        held: pointer.held,
    };
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
                    .child(ui::shape(Radius::Pill, Some(fill_color.into())))
            },
        );
    let signal = signal.filter(|signal| *signal > 0.0).map(|signal| {
        div()
            .absolute()
            .top_0()
            .left_0()
            .h_full()
            .w(relative(signal * value))
            .child(ui::shape(Radius::Pill, Some(rgb(color).into())))
    });
    // The handle rides the fill's spring, on an element of its own so it is drawn over the signal.
    let rider = on_change.is_some().then(|| {
        let handle_id = id.clone();
        div()
            .absolute()
            .top_0()
            .left_0()
            .h_full()
            .with_spring(
                SharedString::from(format!("{id}-rider")),
                SpringAnimation::new(FILL_SPRING).to(value),
                move |rider, level| rider.w(relative(level.clamp(0.0, 1.0))).child(handle(&handle_id, grip)),
            )
    });
    let track = div()
        .relative()
        .w_full()
        .h(px(TRACK_H))
        .child(ui::shape(Radius::Pill, Some(rgba(with_alpha(TINT, TRACK_ALPHA)).into())))
        .child(fill)
        .children(signal)
        .children(rider)
        .child({
            let bounds = bounds.clone();
            // While held, the pointer is followed anywhere, not only over the bar: past either end
            // is 0 or 1, and it need not stay level with the bar.
            let follow = on_change.clone().filter(|_| grip.held);
            let (island, held_id) = (cx.weak_entity(), id.clone());
            canvas(move |area, _, _| bounds.set(Some(area)), move |area, _, window, _| {
                let Some(on_change) = follow else { return };
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                    if phase != DispatchPhase::Bubble {
                        return;
                    }
                    // Released where no release reached the slider, such as outside the island.
                    if event.pressed_button != Some(MouseButton::Left) {
                        pointer::set(&held_id, |grip| grip.held = false, window);
                        return;
                    }
                    let width = f32::from(area.size.width);
                    if width > 0.0 {
                        let value = (f32::from(event.position.x - area.origin.x) / width).clamp(0.0, 1.0);
                        let _ = island.update(cx, |this, cx| on_change(value, this, cx));
                    }
                });
            })
            .absolute()
            .size_full()
        });
    let mut element = div()
        .id(id.clone())
        .relative()
        .w_full()
        .py(px((HIT_H - TRACK_H) / 2.0))
        .child(track);
    if let Some(down) = on_change {
        let down_bounds = bounds.clone();
        let (hover_id, down_id, up_id, out_id) = (id.clone(), id.clone(), id.clone(), id);
        element = element
            .map(|element| if grip.held { element.cursor_grabbing() } else { element.cursor_grab() })
            .on_hover(move |hovered, window, _| pointer::set(&hover_id, |grip| grip.hovered = *hovered, window))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    pointer::set(&down_id, |grip| grip.held = true, window);
                    if let Some(value) = fraction(&down_bounds, event.position.x) {
                        down(value, this, cx);
                    }
                }),
            )
            .on_mouse_up(MouseButton::Left, move |_: &MouseUpEvent, window, _| {
                pointer::set(&up_id, |grip| grip.held = false, window);
            })
            .on_mouse_up_out(MouseButton::Left, move |_: &MouseUpEvent, window, _| {
                pointer::set(&out_id, |grip| grip.held = false, window);
            });
    }
    element.into_any_element()
}
