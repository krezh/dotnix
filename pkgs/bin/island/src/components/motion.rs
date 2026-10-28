//! Text that morphs into its new value instead of jumping, and state kept across frames.

use std::{
    cell::Cell,
    marker::PhantomData,
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, LayoutId,
    Pixels, SharedString, Window, div, prelude::*, px,
};

const MORPH: Duration = Duration::from_millis(320);
/// How much of a morph each changing character takes to turn into its new one, when several do;
/// they start one after another so the change runs along the text.
const SPAN: f32 = 0.55;

pub enum Next {
    /// It is still; nothing more until something else redraws it.
    Rest,
    /// At the display's next frame, for short motion that should be as fluid as the display.
    Frame,
    /// After this long, for longer-running motion that does not need the display's full rate
    /// (often 144–240 Hz), since every frame rebuilds the view it is in.
    After(Duration),
}

/// What a [`keyed`] element keeps between frames: its own state, and whether a delayed redraw is
/// already on its way, so overlapping renders do not stack more.
struct Slot<S> {
    state: S,
    pending: Rc<Cell<bool>>,
}

/// An element drawn from state kept across frames under its id: `build` updates the state and
/// returns what to draw and when to draw it again.
pub struct Keyed<S, F> {
    id: ElementId,
    build: Option<F>,
    state: PhantomData<fn() -> S>,
}

pub fn keyed<S, F>(id: impl Into<ElementId>, build: F) -> Keyed<S, F>
where
    S: Default + 'static,
    F: FnOnce(&mut S, &mut Window, &App) -> (AnyElement, Next) + 'static,
{
    Keyed {
        id: id.into(),
        build: Some(build),
        state: PhantomData,
    }
}

impl<S, F> IntoElement for Keyed<S, F>
where
    S: Default + 'static,
    F: FnOnce(&mut S, &mut Window, &App) -> (AnyElement, Next) + 'static,
{
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<S, F> Element for Keyed<S, F>
where
    S: Default + 'static,
    F: FnOnce(&mut S, &mut Window, &App) -> (AnyElement, Next) + 'static,
{
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, AnyElement) {
        let build = self.build.take().expect("an element is laid out once");
        window.with_element_state(
            id.expect("keyed elements have an id"),
            |slot: Option<Slot<S>>, window| {
                let mut slot = slot.unwrap_or_else(|| Slot {
                    state: S::default(),
                    pending: Rc::new(Cell::new(false)),
                });
                let (mut element, next) = build(&mut slot.state, window, cx);
                match next {
                    Next::Rest => {}
                    Next::Frame => window.request_animation_frame(),
                    Next::After(delay) => {
                        if !slot.pending.replace(true) {
                            let pending = slot.pending.clone();
                            let view = window.current_view();
                            window
                                .spawn(cx, async move |cx| {
                                    cx.background_executor().timer(delay).await;
                                    pending.set(false);
                                    cx.update(move |_, cx| cx.notify(view)).ok();
                                })
                                .detach();
                        }
                    }
                }
                ((element.request_layout(window, cx), element), slot)
            },
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        element: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        element: &mut AnyElement,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        element.paint(window, cx);
    }
}

#[derive(Default)]
struct Morph {
    text: Option<SharedString>,
    from: Option<(SharedString, Instant)>,
}

/// Lengths of the shared start and shared end, in bytes.
fn shared(old: &str, new: &str) -> (usize, usize) {
    let start: usize = old
        .chars()
        .zip(new.chars())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum();
    let end: usize = old[start..]
        .chars()
        .rev()
        .zip(new[start..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum();
    (start, end)
}

/// How far character `index` of `count` changing ones is through its own change, `progress` of the
/// way through the morph: each takes [`SPAN`] of it, the first starting at once and the last
/// finishing with the morph.
fn stagger(progress: f32, index: usize, count: usize) -> f32 {
    if count <= 1 {
        return progress.clamp(0.0, 1.0);
    }
    let start = index as f32 / (count - 1) as f32 * (1.0 - SPAN);
    ((progress - start) / SPAN).clamp(0.0, 1.0)
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// How far, in lines, a rolling digit travels as it leaves or arrives.
const ROLL: f32 = 0.6;

/// The first number in `text`, such as 2.5 in "2.5 kB/s".
fn number(text: &str) -> Option<f32> {
    let start = text.find(|c: char| c.is_ascii_digit())?;
    let digits: String = text[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    digits.parse().ok()
}

/// Whether changing `old` to `new` is a number rolling, and if so whether it rolls up.
fn roll(old: &str, new: &str) -> Option<bool> {
    let (old, new) = (number(old)?, number(new)?);
    Some(new >= old)
}

/// What the old and new text share stays; each other character turns into its new one in place,
/// one after another along the text. `id` must be unique among its siblings.
pub fn text(id: impl Into<ElementId>, text: impl Into<SharedString>) -> impl IntoElement {
    morph(id, text.into(), Vec::new(), Hold::Centre)
}

/// Keeps the width of the widest of `fits`, so what it sits in never resizes as it changes.
pub fn steady(
    id: impl Into<ElementId>,
    text: impl Into<SharedString>,
    fits: Vec<SharedString>,
) -> impl IntoElement {
    morph(id, text.into(), fits, Hold::Centre)
}

/// Where steady text sits in the width it keeps: at its start, such as a value its unit
/// follows, or its end, such as an amount ending a row.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Hold {
    Start,
    Centre,
    End,
}

/// Keeps the width of the widest of `fits`, the text held at `hold` within it.
pub fn steady_at(
    id: impl Into<ElementId>,
    text: impl Into<SharedString>,
    fits: Vec<SharedString>,
    hold: Hold,
) -> impl IntoElement {
    morph(id, text.into(), fits, hold)
}

fn morph(
    id: impl Into<ElementId>,
    text: SharedString,
    fits: Vec<SharedString>,
    hold_at: Hold,
) -> impl IntoElement {
    keyed(id, move |state: &mut Morph, window, cx| {
        let style = window.text_style();
        let size = style.font_size.to_pixels(window.rem_size());
        let line = style.line_height_in_pixels(window.rem_size());
        let width = |part: &SharedString| {
            if part.is_empty() {
                0.0
            } else {
                f32::from(
                    window
                        .text_system()
                        .layout_line(part, size, &[style.to_run(part.len())], None)
                        .width,
                )
            }
        };
        let held =
            (!fits.is_empty()).then(|| fits.iter().chain([&text]).map(&width).fold(0.0, f32::max));
        let hold = |element: AnyElement| match held {
            Some(held) => div()
                .flex()
                .flex_none()
                .map(|line| match hold_at {
                    Hold::Start => line.justify_start(),
                    Hold::Centre => line.justify_center(),
                    Hold::End => line.justify_end(),
                })
                .whitespace_nowrap()
                .w(px(held.ceil()))
                .child(element)
                .into_any_element(),
            None => element,
        };
        let now = Instant::now();
        match &state.text {
            Some(shown) if *shown != text => {
                state.from = Some((shown.clone(), now));
                state.text = Some(text.clone());
            }
            Some(_) => {}
            None => state.text = Some(text.clone()),
        }
        let progress = state
            .from
            .as_ref()
            .map(|(_, since)| now.duration_since(*since).as_secs_f32() / MORPH.as_secs_f32())
            .filter(|progress| *progress < 1.0 && !cx.reduce_motion());
        let (Some(progress), Some((old, _))) = (progress, &state.from) else {
            state.from = None;
            return (hold(text.into_any_element()), Next::Rest);
        };
        let (start, end) = shared(old, &text);
        let characters = |part: &str| {
            part.chars()
                .map(|char| SharedString::from(char.to_string()))
                .collect::<Vec<_>>()
        };
        let gone = characters(&old[start..old.len() - end]);
        let come = characters(&text[start..text.len() - end]);
        let count = gone.len().max(come.len());
        // A number rolls, each changed digit sliding out as the next slides in, up as it grows
        // and down as it falls; other text crossfades in place.
        let roll = roll(old, &text);
        let rise = f32::from(line) * ROLL;
        let changing = div().flex().flex_none().children((0..count).map(|index| {
            let turned = smoothstep(stagger(progress, index, count));
            let (from, to) = (gone.get(index), come.get(index));
            let (from_w, to_w) = (from.map_or(0.0, &width), to.map_or(0.0, &width));
            let (out_y, in_y) = match roll {
                Some(up) => {
                    let way = if up { -1.0 } else { 1.0 };
                    (way * rise * turned, -way * rise * (1.0 - turned))
                }
                None => (0.0, 0.0),
            };
            div()
                .relative()
                .flex_none()
                .overflow_hidden()
                .h(line)
                .w(px(from_w + (to_w - from_w) * turned))
                .when_some(from.cloned(), |slot, char| {
                    slot.child(
                        div()
                            .absolute()
                            .left_0()
                            .top(px(out_y))
                            .opacity(1.0 - turned)
                            .child(char),
                    )
                })
                .when_some(to.cloned(), |slot, char| {
                    slot.child(
                        div()
                            .absolute()
                            .left_0()
                            .top(px(in_y))
                            .opacity(turned)
                            .child(char),
                    )
                })
        }));
        let element = div()
            .flex()
            .flex_none()
            .whitespace_nowrap()
            .when(start > 0, |row| {
                row.child(SharedString::from(text[..start].to_owned()))
            })
            .child(changing)
            .when(end > 0, |row| {
                row.child(SharedString::from(text[text.len() - end..].to_owned()))
            })
            .into_any_element();
        (hold(element), Next::Frame)
    })
}

#[cfg(test)]
#[path = "motion_test.rs"]
mod tests;
