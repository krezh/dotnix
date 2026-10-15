use std::path::Path;

use gpui::{
    AnimationExt, Div, FontWeight, Hsla, SharedString, SpringAnimation, SpringConfig, Stateful, div, prelude::*, px, rgb,
    rgba,
};

use crate::{
    components::icon,
    theme::{
        ACCENT, ACCENT_HOVER_ALPHA, ACCENT_TINT_ALPHA, BASE, CONTROL_ALPHA, ON_ACCENT, SUBTEXT1, SURFACE1, TEXT, TINT,
        TRACK_ALPHA, with_alpha,
    },
    ui::{self, CONTROL_H, HEADER_H, Radius, SMALL_SIZE, hover_fill},
};

const KNOB_SPRING: SpringConfig = SpringConfig::new(420.0, 34.0, 1.0);
const TRACK_W: f32 = 32.0;
const TRACK_H: f32 = 18.0;
const KNOB_INSET: f32 = 2.0;
const KNOB: f32 = TRACK_H - 2.0 * KNOB_INSET;

pub fn fill(id: impl Into<SharedString>, radius: Radius, rest: Option<Hsla>, hover: Option<Hsla>) -> Stateful<Div> {
    div()
        .id(id.into())
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .when(hover.is_some(), |control| control.hover(|style| style))
        .child(ui::shape(radius, rest, hover))
}

pub fn ghost(id: impl Into<SharedString>, hover: bool) -> Stateful<Div> {
    fill(id, Radius::Control, None, hover.then(|| hover_fill().into()))
}

pub fn surface(id: impl Into<SharedString>, active: bool, hover: bool) -> Stateful<Div> {
    let (rest, over) = if active {
        (with_alpha(ACCENT, ACCENT_TINT_ALPHA), rgba(with_alpha(ACCENT, ACCENT_HOVER_ALPHA)))
    } else {
        (with_alpha(SURFACE1, CONTROL_ALPHA), hover_fill())
    };
    fill(id, Radius::Control, Some(rgba(rest).into()), hover.then(|| over.into()))
        .h(px(CONTROL_H))
        .px(px(12.0))
        .justify_start()
        .gap(px(8.0))
        .text_size(px(SMALL_SIZE))
        .font_weight(FontWeight::SEMIBOLD)
        .when(active, |button| button.text_color(rgb(ACCENT)))
}

pub fn button(id: impl Into<SharedString>, label: impl Into<SharedString>, active: bool, hover: bool) -> Stateful<Div> {
    surface(id, active, hover).child(label.into())
}

pub const ICON_BUTTON: f32 = 36.0;
const ICON_GLYPH: f32 = 18.0;

/// A square button showing a glyph, tinted when `active`, with an optional count badge in its corner.
pub fn icon(
    id: impl Into<SharedString>,
    glyph: Option<&Path>,
    active: bool,
    badge: Option<SharedString>,
    hover: bool,
) -> Stateful<Div> {
    let (rest, over) = if active {
        (with_alpha(ACCENT, ACCENT_TINT_ALPHA), rgba(with_alpha(ACCENT, ACCENT_HOVER_ALPHA)))
    } else {
        (with_alpha(SURFACE1, CONTROL_ALPHA), hover_fill())
    };
    let tint = if active { ACCENT } else { SUBTEXT1 };
    fill(id, Radius::Control, Some(rgba(rest).into()), hover.then(|| over.into()))
        .flex_none()
        .size(px(ICON_BUTTON))
        .when_some(glyph, |button, path| button.child(icon::glyph(path, ICON_GLYPH, tint)))
        .when_some(badge, |button, count| {
            button.child(
                div()
                    .absolute()
                    .top(px(-5.0))
                    .right(px(-6.0))
                    .min_w(px(17.0))
                    .h(px(17.0))
                    .px(px(4.0))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(rgb(ACCENT))
                    .border_2()
                    .border_color(rgb(BASE))
                    .text_size(px(10.5))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(ON_ACCENT))
                    .child(count),
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
                .child(ui::shape(Radius::Pill, Some(off), None))
                .child(
                    div()
                        .absolute()
                        .size_full()
                        .opacity(phase.0.clamp(0.0, 1.0))
                        .child(ui::shape(Radius::Pill, Some(accent), None)),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(KNOB_INSET))
                        .left(px(phase.interpolate(KNOB_INSET, TRACK_W - KNOB_INSET - KNOB)))
                        .size(px(KNOB))
                        .child(ui::shape(Radius::Pill, Some(rgb(TEXT).into()), None)),
                )
        })
}

pub fn toggle(id: impl Into<SharedString>, label: impl Into<SharedString>, on: bool, hover: bool) -> Stateful<Div> {
    let id = id.into();
    let knob = SharedString::from(format!("{id}-switch"));
    surface(id, false, hover).child(switch(knob, on)).child(label.into())
}

pub fn close(id: impl Into<SharedString>, hover: bool) -> Stateful<Div> {
    div()
        .id(id.into())
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .size(px(HEADER_H))
        .text_color(rgb(SUBTEXT1))
        .when(hover, |close| close.hover(|style| style.text_color(rgb(TEXT))))
        .child(ui::shape(Radius::Control, None, hover.then(|| hover_fill().into())))
        .child("✕")
}
