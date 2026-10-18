//! Everything clickable: plain and toggle buttons, icon buttons, action and pick tiles, chips.

use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
};

use gpui::{
    AnimationExt, Div, FontWeight, Hsla, SharedString, SpringAnimation, SpringConfig, Stateful, div, prelude::*, px, rgb,
    rgba,
};

use crate::{
    components::{
        energy, icon,
        marquee::{self, TextLayouts},
        measure::{self, Measured},
        motion, pointer,
    },
    island::{CONTENT_IN, SHAPE_SPRING},
    shape,
    theme::{
        ACCENT, ACCENT_TINT_ALPHA, BASE, CONTROL_ALPHA, ON_ACCENT, SUBTEXT1, SURFACE1, TEXT, TINT,
        TRACK_ALPHA, with_alpha,
    },
    ui::{self, CONTROL_H, HEADER_H, Radius, SMALL_SIZE},
};

const KNOB_SPRING: SpringConfig = SpringConfig::new(420.0, 34.0, 1.0);
const TRACK_W: f32 = 32.0;
const TRACK_H: f32 = 18.0;
const KNOB_INSET: f32 = 2.0;
const KNOB: f32 = TRACK_H - 2.0 * KNOB_INSET;

/// Buttons say only whether they react, never how, so they all react the same way; pass false for
/// one that is already selected.
pub fn fill(id: impl Into<SharedString>, radius: Radius, rest: Option<Hsla>, hover: bool) -> Stateful<Div> {
    div()
        .id(id.into())
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .when(hover, |control| control.hover(|style| style))
        .child(ui::control(radius, rest, hover))
}

pub fn ghost(id: impl Into<SharedString>, hover: bool) -> Stateful<Div> {
    fill(id, Radius::Control, None, hover)
}

fn surface_fill(active: bool) -> Hsla {
    rgba(if active { with_alpha(ACCENT, ACCENT_TINT_ALPHA) } else { with_alpha(SURFACE1, CONTROL_ALPHA) }).into()
}

pub fn surface(id: impl Into<SharedString>, active: bool, hover: bool) -> Stateful<Div> {
    fill(id, Radius::Control, Some(surface_fill(active)), hover)
        .h(px(CONTROL_H))
        .px(px(12.0))
        .gap(px(8.0))
        .text_size(px(SMALL_SIZE))
        .font_weight(FontWeight::SEMIBOLD)
        .when(active, |button| button.text_color(rgb(ACCENT)))
}

pub fn button(id: impl Into<SharedString>, label: impl Into<SharedString>, active: bool, hover: bool) -> Stateful<Div> {
    surface(id, active, hover).child(motion::text("label", label))
}

/// Keeps the width of the widest of `fits`, so changing label never resizes it.
pub fn busy(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    fits: Vec<SharedString>,
    active: bool,
    busy: bool,
    hover: bool,
) -> Stateful<Div> {
    surface(id, active, hover)
        .child(motion::steady("label", label, fits))
        .child(energy::border("energy", Radius::Control, 0.0, busy))
}

const PICK_H: f32 = 72.0;
const PICK_GLYPH: f32 = 22.0;
const PICK_PAD: f32 = 8.0;

thread_local! {
    /// Each pick tile's drawn size, which its name has to fit.
    static PICK_SIZES: RefCell<HashMap<SharedString, Measured>> = RefCell::default();
}

/// One of several to choose between, sharing its row equally with the others; the chosen one is
/// tinted with `color`. A name too long for the tile dissolves into its edge, and scrolls through
/// while the tile is hovered.
pub fn pick(
    id: impl Into<SharedString>,
    glyph: Option<&Path>,
    label: impl Into<SharedString>,
    color: u32,
    chosen: bool,
    hover: bool,
    layouts: &TextLayouts,
) -> Stateful<Div> {
    let id = id.into();
    let label = label.into();
    let (rest, text) = if chosen {
        (rgba(with_alpha(color, ACCENT_TINT_ALPHA)).into(), color)
    } else {
        (surface_fill(false), TEXT)
    };
    let size = PICK_SIZES.with_borrow_mut(|sizes| sizes.entry(id.clone()).or_insert_with(measure::measured).clone());
    let slot = size.get().map_or(0.0, |size| f32::from(size.width) - 2.0 * PICK_PAD);
    let name = if slot > 0.0 {
        let hovered = hover && pointer::get(&id).hovered;
        marquee::marquee_on_hover(layouts, &id, label, SMALL_SIZE, FontWeight::SEMIBOLD, slot, hovered).into_any_element()
    } else {
        div().max_w_full().truncate().child(label).into_any_element()
    };
    let hover_id = id.clone();
    fill(id, Radius::Control, Some(rest), hover && !chosen)
        .flex_1()
        .min_w(px(0.0))
        .h(px(PICK_H))
        .px(px(PICK_PAD))
        .flex_col()
        .gap(px(8.0))
        .text_size(px(SMALL_SIZE))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(text))
        .when_some(glyph, |tile, path| tile.child(icon::glyph(path, PICK_GLYPH, text)))
        .child(name)
        .child(measure::probe(&size))
        .on_hover(move |hovered, window, _| pointer::set(&hover_id, |pointer| pointer.hovered = *hovered, window))
}

const CHIP_H: f32 = 24.0;

/// A small button showing a current choice, which opens the others.
pub fn chip(id: impl Into<SharedString>, label: impl Into<SharedString>, open: bool, hover: bool) -> Stateful<Div> {
    fill(id, Radius::Badge, Some(surface_fill(open)), hover)
        .flex_none()
        .h(px(CHIP_H))
        .px(px(8.0))
        .gap(px(4.0))
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(if open { ACCENT } else { SUBTEXT1 }))
        .child(label.into())
}

pub const ACTION_W: f32 = 136.0;
pub const ACTION_H: f32 = 96.0;
const ACTION_GLYPH: f32 = 26.0;

/// `color` marks only the glyph, and fills the tile while `armed`, for an action waiting to be
/// confirmed.
pub fn action(
    id: impl Into<SharedString>,
    glyph: Option<&Path>,
    label: impl Into<SharedString>,
    color: u32,
    armed: bool,
    hover: bool,
) -> Stateful<Div> {
    let (rest, text, mark) = if armed {
        (rgb(color).into(), ON_ACCENT, ON_ACCENT)
    } else {
        (surface_fill(false), TEXT, color)
    };
    fill(id, Radius::Control, Some(rest), hover)
        .w(px(ACTION_W))
        .h(px(ACTION_H))
        .flex_none()
        .flex_col()
        .gap(px(10.0))
        .text_size(px(SMALL_SIZE))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(text))
        .when_some(glyph, |tile, path| tile.child(icon::glyph(path, ACTION_GLYPH, mark)))
        .child(motion::text("label", label))
}

pub const ICON_BUTTON: f32 = 36.0;
const ICON_GLYPH: f32 = 18.0;

/// Past the top-right corner of what the badge sits on.
pub const BADGE_OVERHANG: f32 = 6.0;
const BADGE_H: f32 = 17.0;
/// The base-coloured ring that cuts a badge out of what it sits on.
const BADGE_RING: f32 = 2.0;

/// The overhang is reserved whether or not there is a badge, so the badge is never clipped by a
/// scrolling parent and its coming and going moves nothing.
pub fn badged(id: impl Into<SharedString>, target: impl IntoElement, badge: Option<SharedString>) -> Stateful<Div> {
    div()
        .id(id.into())
        .relative()
        .flex_none()
        .pt(px(BADGE_OVERHANG))
        .pr(px(BADGE_OVERHANG))
        .child(target)
        .when_some(badge, |wrapper, count| {
            // The fill's corners are the ring's less its width, so the two stay concentric.
            let (radius, power) = Radius::Badge.geometry();
            wrapper.child(
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .flex()
                    .min_w(px(BADGE_H))
                    .h(px(BADGE_H))
                    .p(px(BADGE_RING))
                    .child(shape::fill(radius, power, Some(rgb(BASE).into()), None))
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .px(px(4.0))
                            .text_size(px(10.5))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(ON_ACCENT))
                            .child(shape::fill(radius - BADGE_RING, power, Some(rgb(ACCENT).into()), None))
                            .child(motion::text("badge", count)),
                    ),
            )
        })
}

/// How far below its place a rising part starts, as a share of the glyph.
const RISE: f32 = 3.0 / 16.0;

/// Parts drawn over an icon button's glyph only while it is active, each the glyph's full size.
/// Turning on, they rise into place one after another out of the line at `from` (a share of the
/// glyph down from its top), as steam out of a cup; turning off, they fade where they are.
#[derive(Clone)]
pub struct Rising {
    pub parts: Vec<PathBuf>,
    pub from: f32,
}

fn rising(id: SharedString, rising: &Rising, active: bool, tint: u32) -> impl IntoElement {
    let parts = rising.parts.clone();
    let count = parts.len();
    // Off settles more gently than on, with nothing to overshoot.
    let spring = if active { SHAPE_SPRING } else { CONTENT_IN };
    div()
        .absolute()
        .top_0()
        .left_0()
        .w(px(ICON_GLYPH))
        .h(px(ICON_GLYPH * rising.from))
        .overflow_hidden()
        .with_spring(id, SpringAnimation::new(spring).to(active), move |clip, strength| {
            clip.children(parts.iter().enumerate().map(|(index, part)| {
                // The last to come is the first to go.
                let order = if active { index } else { count - 1 - index };
                let share = (strength.0 * 1.3 - order as f32 * 0.15).clamp(0.0, 1.0);
                let lift = if active { (1.0 - share) * RISE * ICON_GLYPH } else { 0.0 };
                div()
                    .absolute()
                    .left_0()
                    .top(px(lift))
                    .opacity(share)
                    .child(icon::glyph(part, ICON_GLYPH, tint))
            }))
        })
}

pub fn icon(
    id: impl Into<SharedString>,
    glyph: Option<&Path>,
    parts: Option<&Rising>,
    active: bool,
    hover: bool,
) -> Stateful<Div> {
    let id = id.into();
    let tint = if active { ACCENT } else { SUBTEXT1 };
    let rise_id = SharedString::from(format!("{id}-rising"));
    fill(id, Radius::Control, Some(surface_fill(active)), hover)
        .flex_none()
        .size(px(ICON_BUTTON))
        .when_some(glyph, |button, path| {
            button.child(
                div()
                    .relative()
                    .size(px(ICON_GLYPH))
                    .child(icon::glyph(path, ICON_GLYPH, tint))
                    .when_some(parts, |glyph, parts| glyph.child(rising(rise_id, parts, active, tint))),
            )
        })
}

pub fn switch(id: impl Into<SharedString>, on: bool) -> impl IntoElement {
    let off: Hsla = rgba(with_alpha(TINT, TRACK_ALPHA)).into();
    let accent: Hsla = rgb(ACCENT).into();
    div()
        .relative()
        .flex_none()
        .w(px(TRACK_W))
        .h(px(TRACK_H))
        .with_spring(id.into(), SpringAnimation::new(KNOB_SPRING).to(on), move |track, phase| {
            track
                .child(ui::shape(Radius::Pill, Some(off)))
                .child(
                    div()
                        .absolute()
                        .size_full()
                        .opacity(phase.0.clamp(0.0, 1.0))
                        .child(ui::shape(Radius::Pill, Some(accent))),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(KNOB_INSET))
                        .left(px(phase.interpolate(KNOB_INSET, TRACK_W - KNOB_INSET - KNOB)))
                        .size(px(KNOB))
                        .child(ui::shape(Radius::Pill, Some(rgb(TEXT).into()))),
                )
        })
}

pub fn toggle(id: impl Into<SharedString>, label: impl Into<SharedString>, on: bool, hover: bool) -> Stateful<Div> {
    let id = id.into();
    let knob = SharedString::from(format!("{id}-switch"));
    surface(id, false, hover).child(switch(knob, on)).child(motion::text("label", label))
}

pub fn close(id: impl Into<SharedString>, hover: bool) -> Stateful<Div> {
    ghost(id, hover)
        .size(px(HEADER_H))
        .text_color(rgb(SUBTEXT1))
        .when(hover, |close| close.hover(|style| style.text_color(rgb(TEXT))))
        .child("✕")
}
