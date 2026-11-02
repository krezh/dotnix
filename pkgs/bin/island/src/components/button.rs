//! Everything clickable: plain and toggle buttons, icon buttons, action and pick tiles, chips.

use std::{cell::RefCell, collections::HashMap, path::Path};

use gpui::{
    AnimationExt, Div, FontWeight, Hsla, SharedString, SpringAnimation, Stateful, Transformation,
    div, prelude::*, px, rgb, rgba, size, svg,
};

use crate::{
    components::{
        energy, icon,
        marquee::{self, TextLayouts},
        measure::{self, Measured},
        motion, pointer,
    },
    shape,
    spring::SpringConfig,
    theme::{
        ACCENT, ACCENT_TINT_ALPHA, BASE, CARD_ALPHA, CONTROL_ALPHA, ON_ACCENT, OVERLAY2, SUBTEXT1,
        SURFACE1, TEXT, TINT, TRACK_ALPHA, with_alpha,
    },
    ui::{self, CONTROL_H, HEADER_H, Radius, SMALL_SIZE},
};

pub const KNOB_SPRING: SpringConfig = SpringConfig::new(420.0, 41.0, 1.0);
const TRACK_W: f32 = 32.0;
const TRACK_H: f32 = 18.0;
const KNOB_INSET: f32 = 2.0;
const KNOB: f32 = TRACK_H - 2.0 * KNOB_INSET;

/// Buttons say only whether they react, never how, so they all react the same way; pass false for
/// one that is already selected.
pub fn fill(
    id: impl Into<SharedString>,
    radius: Radius,
    rest: Option<Hsla>,
    hover: bool,
) -> Stateful<Div> {
    div()
        .id(id.into())
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .when(hover, |control| {
            control
                .hover(|style| style)
                .active(|style| style.opacity(0.85))
        })
        .child(ui::control(radius, rest, hover))
}

pub fn ghost(id: impl Into<SharedString>, hover: bool) -> Stateful<Div> {
    fill(id, Radius::Control, None, hover)
}

fn surface_fill(active: bool) -> Hsla {
    rgba(if active {
        with_alpha(ACCENT, ACCENT_TINT_ALPHA)
    } else {
        with_alpha(SURFACE1, CONTROL_ALPHA)
    })
    .into()
}

pub struct Button<'a> {
    id: SharedString,
    label: Option<SharedString>,
    glyph: Option<&'a Path>,
    color: Option<u32>,
    bare: bool,
    compact: bool,
    active: bool,
    toggle: bool,
    fits: Vec<SharedString>,
    busy: bool,
}

impl<'a> Button<'a> {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: None,
            glyph: None,
            color: None,
            bare: false,
            compact: false,
            active: false,
            toggle: false,
            fits: Vec::new(),
            busy: false,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn glyph(mut self, glyph: Option<&'a Path>) -> Self {
        self.glyph = glyph;
        self
    }

    pub fn color(mut self, color: Option<u32>) -> Self {
        self.color = color;
        self
    }

    pub fn bare(mut self, bare: bool) -> Self {
        self.bare = bare;
        self
    }

    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn toggle(mut self, toggle: bool) -> Self {
        self.toggle = toggle;
        self
    }

    pub fn fits(mut self, fits: Vec<SharedString>) -> Self {
        self.fits = fits;
        self
    }

    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }
}

pub fn button(spec: Button<'_>, hover: bool) -> Stateful<Div> {
    let lit = spec.active && !spec.toggle;
    let has_label = spec.label.is_some();
    let default_fg = if has_label { TEXT } else { SUBTEXT1 };
    let (rest, tint) = match (spec.bare, spec.color, lit) {
        (_, Some(color), true) => (Some(rgb(color).into()), ON_ACCENT),
        (true, Some(color), false) => (None, color),
        (true, None, true) => (None, ACCENT),
        (true, None, false) => (None, default_fg),
        (false, Some(color), false) => (Some(surface_fill(false)), color),
        (false, None, true) => (Some(surface_fill(true)), ACCENT),
        (false, None, false) => (Some(surface_fill(false)), default_fg),
    };
    let knob_id = SharedString::from(format!("{}-switch", spec.id));
    let side = if spec.bare {
        HEADER_H
    } else if spec.compact {
        CONTROL_H
    } else {
        ICON_BUTTON
    };

    fill(spec.id, Radius::Control, rest, hover)
        .flex_none()
        .when(!has_label, |el| el.size(px(side)))
        .when(has_label, |el| {
            el.h(px(CONTROL_H))
                .px(px(12.0))
                .gap(px(8.0))
                .text_size(px(SMALL_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
        })
        .text_color(rgb(tint))
        .when(spec.bare && hover && !lit && spec.color.is_none(), |el| {
            el.hover(|style| style.text_color(rgb(TEXT)))
        })
        .when(spec.toggle, |el| el.child(switch(knob_id, spec.active)))
        .when_some(spec.glyph, |el, path| {
            el.child(icon::glyph(path, ICON_GLYPH, tint))
        })
        .when_some(spec.label, |el, label| {
            el.child(motion::steady("label", label, spec.fits))
        })
        .when(spec.busy, |el| {
            el.child(energy::border("energy", Radius::Control, 0.0, true))
        })
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
    let size = PICK_SIZES.with_borrow_mut(|sizes| {
        sizes
            .entry(id.clone())
            .or_insert_with(measure::measured)
            .clone()
    });
    let slot = size
        .get()
        .map_or(0.0, |size| f32::from(size.width) - 2.0 * PICK_PAD);
    let name = if slot > 0.0 {
        let hovered = hover && pointer::get(&id).hovered;
        marquee::marquee_on_hover(
            layouts,
            &id,
            label,
            SMALL_SIZE,
            FontWeight::SEMIBOLD,
            slot,
            hovered,
        )
        .into_any_element()
    } else {
        div()
            .max_w_full()
            .truncate()
            .child(label)
            .into_any_element()
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
        .when_some(glyph, |tile, path| {
            tile.child(icon::glyph(path, PICK_GLYPH, text))
        })
        .child(name)
        .child(measure::probe(&size))
        .on_hover(move |hovered, window, _| {
            pointer::set(&hover_id, |pointer| pointer.hovered = *hovered, window)
        })
}

const CHIP_H: f32 = 24.0;

const TINTED_CHIP_H: f32 = 30.0;
const TINTED_CHIP_MARK: f32 = 20.0;
const TINTED_CHIP_GLYPH: f32 = 14.0;

/// A chip tinted with `color`: its glyph on a square of that color, its name, and a value in the
/// color, such as a battery running low.
pub fn tinted_chip(
    id: impl Into<SharedString>,
    glyph: Option<&Path>,
    label: SharedString,
    value: SharedString,
    color: u32,
    hover: bool,
) -> Stateful<Div> {
    let id = id.into();
    let tint = rgba(with_alpha(color, ACCENT_TINT_ALPHA / 2)).into();
    let label_id = SharedString::from(format!("{id}-label"));
    let value_id = SharedString::from(format!("{id}-value"));
    fill(id, Radius::Control, Some(tint), hover)
        .flex_none()
        .h(px(TINTED_CHIP_H))
        .pl(px(6.0))
        .pr(px(12.0))
        .gap(px(7.0))
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .child(
            div()
                .relative()
                .size(px(TINTED_CHIP_MARK))
                .flex()
                .items_center()
                .justify_center()
                .child(ui::shape(Radius::Badge, Some(rgb(color).into())))
                .children(glyph.map(|path| icon::glyph(path, TINTED_CHIP_GLYPH, ON_ACCENT))),
        )
        .child(motion::text(label_id, label))
        .child(
            div()
                .text_color(rgb(color))
                .child(motion::text(value_id, value)),
        )
}

/// What a closed [`chip`] is filled with, for what grows out of one.
pub fn chip_fill() -> Hsla {
    surface_fill(false)
}

/// A small button showing a current choice, which opens the others.
pub fn chip(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    open: bool,
    hover: bool,
) -> Stateful<Div> {
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
/// confirmed. With a `state`, it is a switch instead: a wide tile with its glyph on a square that
/// fills with `color` while armed, its name, and the state under it, such as how many are connected.
pub fn action(
    id: impl Into<SharedString>,
    glyph: Option<&Path>,
    label: impl Into<SharedString>,
    state: Option<SharedString>,
    color: u32,
    armed: bool,
    hover: bool,
) -> Stateful<Div> {
    if let Some(state) = state {
        return switch_tile(id.into(), glyph, label.into(), state, color, armed, hover);
    }
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
        .when_some(glyph, |tile, path| {
            tile.child(icon::glyph(path, ACTION_GLYPH, mark))
        })
        .child(motion::text("label", label))
}

const SWITCH_H: f32 = 58.0;
const SWITCH_MARK: f32 = 36.0;
const SWITCH_GLYPH: f32 = 20.0;
const SWITCH_PAD: f32 = 10.0;

fn switch_tile(
    id: SharedString,
    glyph: Option<&Path>,
    label: SharedString,
    state: SharedString,
    color: u32,
    armed: bool,
    hover: bool,
) -> Stateful<Div> {
    let state_color = if armed { color } else { OVERLAY2 };
    let glyph_color = if armed { ON_ACCENT } else { TEXT };
    let mark_fill: Hsla = rgb(color).into();
    let fill_in = div().absolute().top_0().left_0().size_full().with_spring(
        SharedString::from(format!("{id}-mark")),
        SpringAnimation::new(KNOB_SPRING.into()).to(armed),
        move |layer, phase| {
            layer
                .opacity(phase.0.clamp(0.0, 1.0))
                .child(ui::shape(Radius::Control, Some(mark_fill)))
        },
    );
    let mark = div()
        .relative()
        .flex_none()
        .size(px(SWITCH_MARK))
        .flex()
        .items_center()
        .justify_center()
        .child(ui::shape(Radius::Control, Some(surface_fill(false))))
        .child(fill_in)
        .when_some(glyph, |mark, path| {
            mark.child(icon::glyph(path, SWITCH_GLYPH, glyph_color))
        });
    let state_id = SharedString::from(format!("{id}-state"));
    fill(
        id,
        Radius::Control,
        Some(rgba(with_alpha(SURFACE1, CARD_ALPHA)).into()),
        hover,
    )
    .h(px(SWITCH_H))
    .min_w(px(0.0))
    .justify_start()
    .px(px(SWITCH_PAD))
    .gap(px(11.0))
    .child(mark)
    .child(
        div()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                div()
                    .truncate()
                    .text_size(px(SMALL_SIZE))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(label),
            )
            .child(
                div()
                    .truncate()
                    .text_size(px(11.5))
                    .text_color(rgb(state_color))
                    .child(motion::text(state_id, state)),
            ),
    )
}

pub const ICON_BUTTON: f32 = 36.0;
const ICON_GLYPH: f32 = 18.0;

/// Past the top-right corner of what the badge sits on.
pub const BADGE_OVERHANG: f32 = 6.0;
const BADGE_H: f32 = 17.0;
/// The base-coloured ring that cuts a badge out of what it sits on.
const BADGE_RING: f32 = 2.0;

/// A count on the corner of something, such as how many wait.
pub struct Badge {
    pub text: SharedString,
    pub color: u32,
}

/// With `reserve`, the overhang is kept whether or not there is a badge, so the badge is never
/// clipped by a scrolling parent and its coming and going moves nothing. Without it, as where
/// there is no room to spare, the badge hangs over the corner and makes room only beside it.
pub fn badged(
    id: impl Into<SharedString>,
    target: impl IntoElement,
    badge: Option<Badge>,
    reserve: bool,
) -> Stateful<Div> {
    let overhang = px(BADGE_OVERHANG);
    div()
        .id(id.into())
        .relative()
        .flex_none()
        .when(reserve, |wrapper| wrapper.pt(overhang).pr(overhang))
        .when(!reserve && badge.is_some(), |wrapper| wrapper.mr(overhang))
        .child(target)
        .when_some(badge, |wrapper, badge| {
            // The fill's corners are the ring's less its width, so the two stay concentric.
            let (radius, power) = Radius::Badge.geometry();
            wrapper.child(
                div()
                    .absolute()
                    .when(reserve, |corner| corner.top_0().right_0())
                    .when(!reserve, |corner| corner.top(-overhang).right(-overhang))
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
                            .child(shape::fill(
                                radius - BADGE_RING,
                                power,
                                Some(rgb(badge.color).into()),
                                None,
                            ))
                            .child(motion::text("badge", badge.text)),
                    ),
            )
        })
}

pub const FILL_SPRING: SpringConfig = SpringConfig::new(300.0, 34.6, 1.0);
const STRIP_ICON_W: f32 = 32.0;
const STRIP_ICON_H: f32 = 28.0;
pub const STRIP_GLYPH: f32 = 18.0;

/// The least a strip icon's glyph shrinks to on its way between off and on.
pub const POP_DIP: f32 = 0.8;

/// An icon button inside a strip: bare until hovered, its fill fading in with `color` while active
/// and its glyph popping as it is switched.
pub fn strip_icon(
    id: impl Into<SharedString>,
    glyph: Option<&Path>,
    color: Option<u32>,
    active: bool,
    hover: bool,
) -> Stateful<Div> {
    let id = id.into();
    let (on, tint) = match color {
        Some(color) => (rgb(color).into(), ON_ACCENT),
        None => (surface_fill(true), ACCENT),
    };
    let fill_id = SharedString::from(format!("{id}-fill"));
    let glyph = glyph.map(|path| {
        let path = SharedString::from(path.to_string_lossy().into_owned());
        let layer = move |color: u32, scale: f32| {
            svg()
                .absolute()
                .top_0()
                .left_0()
                .path(path.clone())
                .size(px(STRIP_GLYPH))
                .text_color(rgb(color))
                .with_transformation(Transformation::scale(size(scale, scale)))
        };
        div().relative().size(px(STRIP_GLYPH)).with_spring(
            SharedString::from(format!("{id}-glyph")),
            SpringAnimation::new(FILL_SPRING.into()).to(active),
            move |glyph, phase| {
                let lit_by = phase.0.clamp(0.0, 1.0);
                // Full size at rest, smallest halfway between off and on.
                let scale = 1.0 - (1.0 - POP_DIP) * 4.0 * lit_by * (1.0 - lit_by);
                glyph
                    .child(layer(SUBTEXT1, scale).opacity(1.0 - lit_by))
                    .child(layer(tint, scale).opacity(lit_by))
            },
        )
    });
    fill(id, Radius::Row, None, hover)
        .flex_none()
        .w(px(STRIP_ICON_W))
        .h(px(STRIP_ICON_H))
        .child(div().absolute().top_0().left_0().size_full().with_spring(
            fill_id,
            SpringAnimation::new(FILL_SPRING.into()).to(active),
            move |fill, phase| {
                fill.opacity(phase.0.clamp(0.0, 1.0))
                    .child(ui::shape(Radius::Row, Some(on)))
            },
        ))
        .children(glyph)
}

pub fn switch(id: impl Into<SharedString>, on: bool) -> impl IntoElement {
    let off: Hsla = rgba(with_alpha(TINT, TRACK_ALPHA)).into();
    let accent: Hsla = rgb(ACCENT).into();
    div()
        .relative()
        .flex_none()
        .w(px(TRACK_W))
        .h(px(TRACK_H))
        .with_spring(
            id.into(),
            SpringAnimation::new(KNOB_SPRING.into()).to(on),
            move |track, phase| {
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
                            .left(px(
                                phase.interpolate(KNOB_INSET, TRACK_W - KNOB_INSET - KNOB)
                            ))
                            .size(px(KNOB))
                            .child(ui::shape(Radius::Pill, Some(rgb(TEXT).into()))),
                    )
            },
        )
}

pub fn close(id: impl Into<SharedString>, hover: bool) -> Stateful<Div> {
    ghost(id, hover)
        .size(px(HEADER_H))
        .text_color(rgb(SUBTEXT1))
        .when(hover, |close| {
            close.hover(|style| style.text_color(rgb(TEXT)))
        })
        .child("✕")
}
