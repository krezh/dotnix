//! Surfaces drawn over everything else on the island, such as tooltips and an open dropdown.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gpui::{AnyElement, Bounds, Div, Pixels, SharedString, canvas, div, prelude::*, px};

use crate::{
    shape,
    theme::POPOVER_OPACITY,
    ui::{self, Radius},
};

/// Room around the surface for its shadow; popovers are laid out as roots, so it is kept as margin.
pub const SHADOW_SPREAD: f32 = 6.0;

pub fn surface(radius: Radius) -> Div {
    let (radius, power) = radius.geometry();
    div()
        .relative()
        .m(px(SHADOW_SPREAD))
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .child(ui::shadow(radius, power, SHADOW_SPREAD)),
        )
        .child(shape::background(
            radius,
            power,
            ui::surface_fill(POPOVER_OPACITY),
            Some(ui::surface_border()),
        ))
}

/// Where a floating surface was drawn and how round its corners are, in window coordinates.
#[derive(Clone, Copy, PartialEq)]
pub struct Reach {
    pub bounds: Bounds<Pixels>,
    pub radius: f32,
    pub power: f32,
}

#[derive(Default)]
struct State {
    /// Where each anchor was last drawn, in window coordinates.
    anchors: HashMap<SharedString, Bounds<Pixels>>,
    /// Shown this frame, each placed in window coordinates by whatever shows it.
    shown: Vec<AnyElement>,
    /// What last frame's surfaces covered, so the island takes clicks and blurs behind them.
    reached: Vec<Reach>,
    reaching: Vec<Reach>,
    animating: bool,
    /// The island's own outline, which surfaces keep inside.
    within: Option<Bounds<Pixels>>,
}

/// What floats over the island this frame. Whatever shows something shows it again every frame it
/// stays; a frame without it removes it.
#[derive(Clone, Default)]
pub struct Popovers(Rc<RefCell<State>>);

impl Popovers {
    pub fn set_within(&self, bounds: Bounds<Pixels>) {
        self.0.borrow_mut().within = Some(bounds);
    }

    /// Where `width` fits at `left`, moved inward only as far as it must to stay `margin` inside
    /// the island's edges.
    pub fn keep_within(&self, left: f32, width: f32, margin: f32) -> f32 {
        let Some(within) = self.0.borrow().within else {
            return left;
        };
        let start = f32::from(within.origin.x) + margin;
        let end = f32::from(within.right()) - margin - width;
        if end < start {
            start
        } else {
            left.clamp(start, end)
        }
    }

    /// Records where `id` is drawn; put it inside the element a popover grows from.
    pub fn anchor(&self, id: SharedString) -> impl IntoElement {
        let state = self.0.clone();
        canvas(
            move |bounds, _, _| {
                state.borrow_mut().anchors.insert(id.clone(), bounds);
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full()
    }

    pub fn anchor_bounds(&self, id: &str) -> Option<Bounds<Pixels>> {
        self.0.borrow().anchors.get(id).copied()
    }

    /// Put inside a shown surface, filling it, so the island knows where it reaches.
    pub fn reach(&self, radius: f32, power: f32) -> impl IntoElement {
        let state = self.0.clone();
        canvas(
            move |bounds, _, _| {
                state.borrow_mut().reaching.push(Reach {
                    bounds,
                    radius,
                    power,
                })
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full()
    }

    pub fn show(&self, element: AnyElement) {
        self.0.borrow_mut().shown.push(element);
    }

    /// Something shown is still moving, so the island keeps drawing frames.
    pub fn animate(&self) {
        self.0.borrow_mut().animating = true;
    }

    /// Where the island takes clicks and blurs besides its own body.
    pub fn reached(&self) -> Vec<Reach> {
        self.0.borrow().reached.clone()
    }

    /// Drawn last, over everything; true in the second value while anything shown still moves.
    pub fn overlay(&self) -> (Option<AnyElement>, bool) {
        let mut state = self.0.borrow_mut();
        state.reached = std::mem::take(&mut state.reaching);
        let animating = std::mem::take(&mut state.animating);
        let shown = std::mem::take(&mut state.shown);
        let overlay = (!shown.is_empty()).then(|| {
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .children(shown)
                .into_any_element()
        });
        (overlay, animating)
    }
}
