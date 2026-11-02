//! A value from 0 to 1 that can be dragged, optionally with a live level over its fill. Hovering
//! one that can be dragged grows a handle out of its fill's end; holding it grows a little more.
//! A large one carries its glyph, name and amount inside it instead, light over the empty track and
//! dark over the fill.

use std::{cell::Cell, path::PathBuf, rc::Rc};

use gpui::{
    AnimationExt, AnyElement, Bounds, Context, DispatchPhase, Div, FontWeight, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, SharedString, SpringAnimation,
    SpringPlayback, Window, canvas, div, prelude::*, px, relative, rgb, rgba,
};

use crate::{
    components::{
        icon, motion,
        pointer::{self, Pointer},
    },
    island::{Island, SHAPE_SPRING},
    panel::{IslandUi, Ui},
    spring::SpringConfig,
    spring::rubber,
    theme::{ON_ACCENT, TEXT, TINT, TRACK_ALPHA, with_alpha},
    ui::{self, Radius, SMALL_SIZE},
};

pub type OnChange = Rc<dyn Fn(f32, &mut dyn Ui)>;

pub const FILL_SPRING: SpringConfig = SpringConfig::new(420.0, 41.0, 1.0);
pub const TRACK_H: f32 = 10.0;
pub const HIT_H: f32 = 16.0;
/// The fill of a metered slider, under the live signal.
const DIM_ALPHA: u8 = 0x8c;
/// The handle sticks out 4 px past each side of the track while hovered, 5 px while held.
pub const HANDLE_W: f32 = 4.0;
pub const HOVER_H: f32 = TRACK_H + 8.0;
pub const HELD_H: f32 = TRACK_H + 10.0;
const LARGE_H: f32 = 42.0;
const LARGE_GLYPH: f32 = 20.0;
const LARGE_PAD: f32 = 12.0;
/// The furthest a pull past either end stretches the track.
pub const STRETCH_REACH: f32 = 24.0;

/// What a large slider shows inside itself.
#[derive(Clone)]
pub struct SliderLabel {
    pub glyph: Option<PathBuf>,
    pub label: SharedString,
    pub amount: SharedString,
}

/// The glyph and name at the start and the amount at the end, across the whole track; `id` names
/// this copy of it, light or dark, so its text morphs as it changes.
fn label_row(id: &str, inside: &SliderLabel, color: u32) -> Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .h_full()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(LARGE_PAD))
        .text_size(px(SMALL_SIZE))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(color))
        .whitespace_nowrap()
        .children(
            inside
                .glyph
                .as_deref()
                .map(|path| icon::glyph(path, LARGE_GLYPH, color)),
        )
        .child(div().flex_1().min_w(px(0.0)).truncate().child(motion::text(
            SharedString::from(format!("{id}-label")),
            inside.label.clone(),
        )))
        .child(div().flex_none().child(motion::text(
            SharedString::from(format!("{id}-amount")),
            inside.amount.clone(),
        )))
}

/// A slim pill on the fill's end, growing out of it.
fn handle(id: &SharedString, grip: Pointer) -> impl IntoElement {
    let target = match grip {
        Pointer { held: true, .. } => HELD_H,
        Pointer { hovered: true, .. } => HOVER_H,
        _ => 0.0,
    };
    div().absolute().with_spring(
        SharedString::from(format!("{id}-handle")),
        SpringAnimation::new(SHAPE_SPRING.into()).to(target),
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

fn release(id: &SharedString, window: &mut Window) {
    pointer::set(
        id,
        |grip| {
            grip.held = false;
            grip.grab_offset = 0.0;
            grip.stretch = 0.0;
        },
        window,
    );
}

pub fn slider(
    id: impl Into<SharedString>,
    value: f32,
    color: u32,
    on_change: Option<OnChange>,
    cx: &mut Context<Island>,
) -> AnyElement {
    build(id.into(), value, color, None, on_change, None, true, cx)
}

pub struct Slider {
    pub id: SharedString,
    pub value: f32,
    pub color: u32,
    /// Dims the fill and draws a live level over it, as a share of the value: 1 reaches the end
    /// of the fill.
    pub signal: Option<f32>,
    pub on_change: Option<OnChange>,
    /// Makes it large, carrying these inside it.
    pub label: Option<SliderLabel>,
}

/// `live` is whether the pointer is over the island at all: GPUI's hover goes stale once it leaves.
pub fn view(slider: Slider, live: bool, cx: &mut Context<Island>) -> AnyElement {
    let signal = slider.signal.map(|signal| signal.clamp(0.0, 1.0));
    build(
        slider.id,
        slider.value,
        slider.color,
        signal,
        slider.on_change,
        slider.label,
        live,
        cx,
    )
}

#[allow(clippy::too_many_arguments)]
fn build(
    id: SharedString,
    value: f32,
    color: u32,
    signal: Option<f32>,
    on_change: Option<OnChange>,
    inside: Option<SliderLabel>,
    live: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let value = value.clamp(0.0, 1.0);
    let bounds: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::new(Cell::new(None));
    let fill_color = if signal.is_some() {
        rgba(with_alpha(color, DIM_ALPHA))
    } else {
        rgb(color)
    };
    let (track_h, radius) = match inside {
        Some(_) => (LARGE_H, Radius::Control),
        None => (TRACK_H, Radius::Pill),
    };
    let pointer = pointer::get(&id);
    // A hold outlasts the pointer leaving the island; hovering does not.
    let grip = Pointer {
        hovered: live && pointer.hovered,
        ..pointer
    };
    // Held, it is wherever the pointer puts it; let go, it springs on from there.
    let playback = if grip.held {
        SpringPlayback::Completed
    } else {
        SpringPlayback::Running
    };
    let dark = inside.clone();
    let dark_id = format!("{id}-dark");
    let fill = div().absolute().top_0().left_0().h_full().with_spring(
        SharedString::from(format!("{id}-fill")),
        SpringAnimation::new(FILL_SPRING.into())
            .to(value)
            .playback(playback),
        move |fill, level| {
            let level = level.clamp(0.0, 1.0);
            // The dark copy spans the whole track, cut off where the fill ends.
            let row = dark
                .as_ref()
                .filter(|_| level > 0.0)
                .map(|inside| label_row(&dark_id, inside, ON_ACCENT).w(relative(1.0 / level)));
            fill.w(relative(level))
                .overflow_hidden()
                .child(ui::shape(radius, Some(fill_color.into())))
                .children(row)
        },
    );
    let signal = signal.filter(|signal| *signal > 0.0).map(|signal| {
        div()
            .absolute()
            .top_0()
            .left_0()
            .h_full()
            .w(relative(signal * value))
            .child(ui::shape(radius, Some(rgb(color).into())))
    });
    let light = inside
        .as_ref()
        .map(|inside| label_row(&format!("{id}-light"), inside, TEXT).w_full());
    // The handle rides the fill's spring, on an element of its own so it is drawn over the signal.
    let rider = (on_change.is_some() && inside.is_none()).then(|| {
        let handle_id = id.clone();
        div().absolute().top_0().left_0().h_full().with_spring(
            SharedString::from(format!("{id}-rider")),
            SpringAnimation::new(FILL_SPRING.into())
                .to(value)
                .playback(playback),
            move |rider, level| {
                rider
                    .w(relative(level.clamp(0.0, 1.0)))
                    .child(handle(&handle_id, grip))
            },
        )
    });
    let body = div()
        .absolute()
        .top_0()
        .bottom_0()
        .child(ui::shape(
            radius,
            Some(rgba(with_alpha(TINT, TRACK_ALPHA)).into()),
        ))
        .children(light)
        .child(fill)
        .children(signal)
        .children(rider);
    // Stretches past an end while pulled there.
    let track = div().relative().w_full().h(px(track_h)).with_spring(
        SharedString::from(format!("{id}-stretch")),
        SpringAnimation::new(SHAPE_SPRING.into())
            .to(grip.stretch)
            .playback(playback),
        move |track, stretch| {
            track.child(body.left(px(stretch.min(0.0))).right(px(-stretch.max(0.0))))
        },
    );
    // Measured outside the stretch, so a pull past an end does not move where values fall.
    let reach = {
        let bounds = bounds.clone();
        // While held, the pointer is followed anywhere, not only over the bar: past either end
        // is 0 or 1, and it need not stay level with the bar.
        let follow = on_change.clone().filter(|_| grip.held);
        let (island, held_id) = (cx.weak_entity(), id.clone());
        canvas(
            move |area, _, _| bounds.set(Some(area)),
            move |area, _, window, _| {
                let Some(on_change) = follow else { return };
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                    if phase != DispatchPhase::Bubble {
                        return;
                    }
                    // Released where no release reached the slider, such as outside the island.
                    if event.pressed_button != Some(MouseButton::Left) {
                        release(&held_id, window);
                        return;
                    }
                    let width = f32::from(area.size.width);
                    if width > 0.0 {
                        let grip = pointer::get(&held_id);
                        let x = f32::from(event.position.x - area.origin.x) - grip.grab_offset;
                        let over = if x < 0.0 { x } else { (x - width).max(0.0) };
                        let stretch = rubber(over, STRETCH_REACH);
                        if stretch != grip.stretch {
                            pointer::set(&held_id, |grip| grip.stretch = stretch, window);
                        }
                        let value = (x / width).clamp(0.0, 1.0);
                        let _ = island.update(cx, |this, cx| {
                            on_change(value, &mut IslandUi::new(this, cx))
                        });
                    }
                });
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    };
    let mut element = div()
        .id(id.clone())
        .relative()
        .w_full()
        .when(inside.is_none(), |hit| hit.py(px((HIT_H - TRACK_H) / 2.0)))
        .child(track)
        .child(reach);
    if let Some(down) = on_change {
        let down_bounds = bounds.clone();
        let (hover_id, down_id, up_id, out_id) = (id.clone(), id.clone(), id.clone(), id);
        element = element
            .map(|element| {
                if grip.held {
                    element.cursor_grabbing()
                } else {
                    element.cursor_grab()
                }
            })
            .on_hover(move |hovered, window, _| {
                pointer::set(&hover_id, |grip| grip.hovered = *hovered, window)
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    let (offset, seek_value) = if let Some(bounds) = down_bounds.get() {
                        let width = f32::from(bounds.size.width);
                        let x_h = bounds.origin.x + px(width * value);
                        let dist = f32::from(event.position.x - x_h).abs();
                        if dist <= HANDLE_W {
                            (f32::from(event.position.x - x_h), None)
                        } else {
                            (0.0, fraction(&down_bounds, event.position.x))
                        }
                    } else {
                        (0.0, fraction(&down_bounds, event.position.x))
                    };
                    pointer::set(
                        &down_id,
                        |grip| {
                            grip.held = true;
                            grip.grab_offset = offset;
                            grip.stretch = 0.0;
                        },
                        window,
                    );
                    if let Some(val) = seek_value {
                        down(val, &mut IslandUi::new(this, cx));
                    }
                }),
            )
            .on_mouse_up(MouseButton::Left, move |_: &MouseUpEvent, window, _| {
                release(&up_id, window)
            })
            .on_mouse_up_out(MouseButton::Left, move |_: &MouseUpEvent, window, _| {
                release(&out_id, window)
            });
    }
    element.into_any_element()
}
