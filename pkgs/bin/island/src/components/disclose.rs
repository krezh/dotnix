//! What a row keeps folded away until it opens, such as a network's signal under its name: it grows
//! to its own height on a spring and fades in as it does, and folds back the same way. It sits in a
//! card of fixed height, so opening one never resizes the panel.

use std::time::Instant;

use gpui::{AnyElement, SharedString, div, prelude::*, px};

use crate::{
    components::{
        measure::{self, Measured},
        motion::{Next, keyed},
    },
    island::{CONTENT_DRIFT, CONTENT_IN, CONTENT_OUT, REVEAL_DELAY},
    spring::{SOFT, Spring},
};

/// Between what it holds, and round it, as the rows it sits under are spaced.
const GAP: f32 = 8.0;
const PAD_TOP: f32 = 2.0;
const PAD_BOTTOM: f32 = 10.0;

struct Fold {
    grown: Spring,
    revealed: Spring,
    open: bool,
    /// Drawn before, so the first drawing shows it as it is rather than opening.
    seen: bool,
    at: Instant,
    natural: Measured,
}

impl Default for Fold {
    fn default() -> Self {
        Self {
            grown: Spring::new(0.0, SOFT, 0.001),
            revealed: Spring::new(0.0, CONTENT_IN, 0.002),
            open: false,
            seen: false,
            at: Instant::now(),
            natural: measure::measured(),
        }
    }
}

impl Fold {
    fn advance(&mut self, open: bool, still: bool) {
        let now = Instant::now();
        let dt = now.duration_since(self.at).as_secs_f32().min(0.05);
        self.at = now;
        if !self.seen || still {
            self.seen = true;
            self.open = open;
            let at = if open { 1.0 } else { 0.0 };
            self.grown.reset(at);
            self.revealed.reset(at);
            return;
        }
        if open != self.open {
            self.open = open;
            if open {
                self.grown.retarget(1.0, SOFT);
                self.revealed
                    .retarget_at(1.0, CONTENT_IN, now + REVEAL_DELAY);
            } else {
                self.grown.retarget(0.0, SOFT);
                self.revealed.retarget(0.0, CONTENT_OUT);
            }
        }
        self.grown.advance(dt, now);
        self.revealed.advance(dt, now);
    }

    fn moving(&self) -> bool {
        !(self.grown.is_settled() && self.revealed.is_settled())
    }
}

pub struct Disclosure {
    /// Unique among its siblings.
    pub id: SharedString,
    pub open: bool,
    /// Where what it holds starts, such as past the lead of the row it belongs to.
    pub indent: f32,
}

pub fn disclose(view: Disclosure, children: Vec<AnyElement>) -> impl IntoElement {
    let Disclosure { id, open, indent } = view;
    keyed(id, move |fold: &mut Fold, _, cx| {
        fold.advance(open, cx.reduce_motion());
        let grown = fold.grown.value().max(0.0);
        if grown <= 0.001 && !fold.open {
            // Out of the flow while shut, so the row it belongs to is no taller for it.
            return (div().absolute().into_any_element(), Next::Rest);
        }
        let height = fold
            .natural
            .get()
            .map_or(0.0, |size| f32::from(size.height));
        let shown = fold.revealed.value().clamp(0.0, 1.0);
        let content = div()
            .absolute()
            .top(px(-(1.0 - shown) * CONTENT_DRIFT / 2.0))
            .left(px(indent))
            .right_0()
            .opacity(shown)
            .pt(px(PAD_TOP))
            .pb(px(PAD_BOTTOM))
            .flex()
            .flex_col()
            .gap(px(GAP))
            .children(children)
            .child(measure::probe(&fold.natural));
        let element = div()
            .relative()
            .w_full()
            .h(px(height * grown))
            .overflow_hidden()
            .child(content)
            .into_any_element();
        let next = if fold.moving() { Next::Frame } else { Next::Rest };
        (element, next)
    })
}
