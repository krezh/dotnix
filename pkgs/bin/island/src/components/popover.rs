//! Surfaces drawn over everything else on the island, such as tooltips and an open dropdown, and
//! how one grows out of what opens it. A surface belongs to the panel it was opened from: it fades
//! with it and closes when that panel stops being the one showing. While open it sits over a
//! [`scrim`], so a click anywhere else only closes it. While closing, or before its rows show, it
//! takes no clicks at all.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    time::Instant,
};

use gpui::{
    AnyElement, Bounds, Context, Div, MouseDownEvent, Pixels, SharedString, canvas, div,
    prelude::*, px, size,
};

use crate::{
    components::measure::{self, Measured},
    island::{CONTENT_IN, CONTENT_OUT, Island, SHAPE_SPRING},
    panel::Handler,
    panel::IslandUi,
    shape,
    spring::Spring,
    theme::POPOVER_OPACITY,
    ui::{self, Radius},
};

/// Room around the surface for its shadow; popovers are laid out as roots, so it is kept as margin.
pub const SHADOW_SPREAD: f32 = 6.0;
/// How far a surface has grown when what opened it has faded and its rows start to show.
pub const ROWS_FROM: f32 = 0.2;
/// How far it has grown when its rows show in full and it has its own color, and it starts taking
/// clicks.
pub const ROWS_AT: f32 = 0.6;

/// A surface's growth out of what opens it, kept across frames.
struct Morph {
    /// 0 is what opens it, 1 the surface.
    grown: Spring,
    revealed: Spring,
    open: bool,
    at: Instant,
    /// Where what opens it is, followed smoothly, as it can change size when picked from.
    left: Spring,
    width: Spring,
    /// The surface's own size, measured while it is drawn.
    natural: Measured,
    /// What is chosen on it, as of when it last stood open.
    chosen: Option<usize>,
}

impl Morph {
    fn new(now: Instant) -> Self {
        Self {
            grown: Spring::new(0.0, SHAPE_SPRING, 0.001),
            revealed: Spring::new(0.0, CONTENT_IN, 0.002),
            open: false,
            at: now,
            left: Spring::new(0.0, SHAPE_SPRING, 0.1),
            width: Spring::new(0.0, SHAPE_SPRING, 0.1),
            natural: measure::measured(),
            chosen: None,
        }
    }

    /// What to mark on the surface: picking closes it, and it leaves as it stood rather than
    /// changing as it goes, so the new choice does not flash on its way out.
    fn hold(&mut self, chosen: Option<usize>) -> Option<usize> {
        if self.open {
            self.chosen = chosen;
        }
        self.chosen
    }

    fn advance(&mut self, open: bool, anchor: Option<Bounds<Pixels>>, now: Instant) {
        // At rest its last frame may be long past; a step that long would start it partway.
        let dt = if self.moving() {
            now.duration_since(self.at).as_secs_f32().min(0.05)
        } else {
            0.0
        };
        self.at = now;
        if let Some(anchor) = anchor {
            let (left, width) = (f32::from(anchor.origin.x), f32::from(anchor.size.width));
            if self.shown() {
                self.left.retarget(left, SHAPE_SPRING);
                self.width.retarget(width, SHAPE_SPRING);
            } else {
                self.left.reset(left);
                self.width.reset(width);
            }
        }
        if open != self.open {
            self.open = open;
            if open {
                self.grown.retarget(1.0, SHAPE_SPRING);
                self.revealed.retarget(1.0, CONTENT_IN);
            } else {
                self.grown.retarget(0.0, SHAPE_SPRING);
                self.revealed.retarget(0.0, CONTENT_OUT);
            }
        }
        for spring in [
            &mut self.grown,
            &mut self.revealed,
            &mut self.left,
            &mut self.width,
        ] {
            spring.advance(dt, now);
        }
    }

    fn moving(&self) -> bool {
        !(self.grown.is_settled()
            && self.revealed.is_settled()
            && self.left.is_settled()
            && self.width.is_settled())
    }

    /// Within a pixel of what opens it, it gives way to that rather than settling for longer.
    fn shown(&self) -> bool {
        self.open || self.grown.value().abs() > 0.01
    }

    fn growth(&self, anchor: Option<Bounds<Pixels>>) -> Growth {
        let look = self.grown.value().clamp(0.0, 1.0);
        Growth {
            look,
            grown: self.grown.value(),
            rows: rows_shown(look).min(self.revealed.value().clamp(0.0, 1.0)),
            cover: cover(look),
            opener: opener_shown(look),
            open: self.open,
            live: self.open && look >= ROWS_AT,
            shown: self.shown(),
            anchor: anchor.map(|anchor| Bounds {
                origin: gpui::point(px(self.left.value()), anchor.origin.y),
                size: size(px(self.width.value()), anchor.size.height),
            }),
            natural: self.natural.clone(),
        }
    }
}

/// How much of a surface's rows show at `look`: none until what opened it has faded, so the same
/// words never show twice.
pub fn rows_shown(look: f32) -> f32 {
    ((look - ROWS_FROM) / (ROWS_AT - ROWS_FROM)).clamp(0.0, 1.0)
}

/// How much of what opened it still shows on the surface at `look`.
pub fn opener_shown(look: f32) -> f32 {
    (1.0 - look / ROWS_FROM).clamp(0.0, 1.0)
}

/// Reaches the surface's own color only as its rows arrive, so it is never dark and empty.
pub fn cover(look: f32) -> f32 {
    (look / ROWS_AT).clamp(0.0, 1.0)
}

/// Where a surface is in growing, this frame.
pub struct Growth {
    /// 0 to 1.
    pub look: f32,
    /// As [`Self::look`], but free to pass its ends.
    pub grown: f32,
    /// How much of its rows show.
    pub rows: f32,
    /// How far it has turned from looking like what opens it to looking like itself.
    pub cover: f32,
    /// How much of what opened it, such as its label, shows on it.
    pub opener: f32,
    /// Open or opening, and so over a [`scrim`].
    pub open: bool,
    /// Its rows show, so it takes the pointer. Otherwise everything passes through it.
    pub live: bool,
    /// Drawn at all, opening, open or closing.
    pub shown: bool,
    /// Where what opens it is.
    pub anchor: Option<Bounds<Pixels>>,
    pub natural: Measured,
}

/// Lies under an open surface and over everything else but `opener`, which stays itself, hovered
/// and clickable: nothing else under it is hovered, and a click anywhere on it closes the surface
/// and does nothing more. `dismiss` is called at most once.
pub fn scrim(opener: Bounds<Pixels>, dismiss: Handler, cx: &mut Context<Island>) -> AnyElement {
    let spent = Rc::new(Cell::new(false));
    let pane = |pane: Div| {
        let (spent, dismiss) = (spent.clone(), dismiss.clone());
        pane.absolute().occlude().on_any_mouse_down(cx.listener(
            move |this, _: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
                if !spent.replace(true) {
                    dismiss(&mut IslandUi::new(this, cx));
                }
            },
        ))
    };
    let (top, bottom) = (opener.origin.y, opener.origin.y + opener.size.height);
    let (left, right) = (opener.origin.x, opener.origin.x + opener.size.width);
    let above = pane(div().top_0().left_0().right_0().h(top));
    let below = pane(div().top(bottom).left_0().right_0().bottom_0());
    let before = pane(div().top(top).left_0().w(left).h(opener.size.height));
    let after = pane(div().top(top).left(right).right_0().h(opener.size.height));
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .child(above)
        .child(below)
        .child(before)
        .child(after)
        .into_any_element()
}

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

#[derive(Default)]
struct State {
    /// Where each anchor was last drawn, in window coordinates.
    anchors: HashMap<SharedString, Bounds<Pixels>>,
    /// Shown this frame, each placed in window coordinates by whatever shows it.
    shown: Vec<AnyElement>,
    /// What last frame's surfaces covered, so the island takes clicks there.
    reached: Vec<Bounds<Pixels>>,
    reaching: Vec<Bounds<Pixels>>,
    animating: bool,
    /// The island's own outline, which surfaces keep inside.
    within: Option<Bounds<Pixels>>,
    morphs: HashMap<SharedString, Morph>,
    /// The panel being drawn, which what is shown belongs to.
    panel: Option<Layer>,
}

#[derive(Clone, Copy)]
struct Layer {
    opacity: f32,
    /// The panel showing, rather than one fading out or drawn just to be measured.
    active: bool,
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

    /// What follows is drawn for a panel this faded, and shown only if `active`.
    pub fn enter_panel(&self, opacity: f32, active: bool) {
        self.0.borrow_mut().panel = Some(Layer { opacity, active });
    }

    /// Moves the surface opened from `id` on a frame. It closes when its panel is not the one
    /// showing, whatever `open` says.
    pub fn grow(&self, id: &SharedString, open: bool) -> Growth {
        let now = Instant::now();
        let mut state = self.0.borrow_mut();
        let state = &mut *state;
        let open = open && state.panel.is_none_or(|panel| panel.active);
        let anchor = state.anchors.get(id).copied();
        let morph = state
            .morphs
            .entry(id.clone())
            .or_insert_with(|| Morph::new(now));
        morph.advance(open, anchor, now);
        state.animating |= morph.moving();
        morph.growth(anchor)
    }

    /// What the surface opened from `id` shows as chosen, given what is: see [`Morph::hold`].
    pub fn hold(&self, id: &SharedString, chosen: Option<usize>) -> Option<usize> {
        self.0
            .borrow_mut()
            .morphs
            .get_mut(id)
            .map_or(chosen, |morph| morph.hold(chosen))
    }

    /// Put inside a shown surface, filling it, so the island knows where it reaches.
    pub fn reach(&self) -> impl IntoElement {
        let state = self.0.clone();
        canvas(
            move |bounds, _, _| state.borrow_mut().reaching.push(bounds),
            |_, _, _, _| {},
        )
        .absolute()
        .size_full()
    }

    /// Faded with the panel it belongs to; left out once that has faded away.
    pub fn show(&self, element: AnyElement) {
        let mut state = self.0.borrow_mut();
        let opacity = state.panel.map_or(1.0, |panel| panel.opacity);
        if opacity <= 0.001 {
            return;
        }
        state.shown.push(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .opacity(opacity)
                .child(element)
                .into_any_element(),
        );
    }

    /// Where the island takes clicks besides its own body. Not blurred: surfaces are opaque, and a
    /// blur region changing on every frame of their motion makes the compositor redo the blur.
    pub fn reached(&self) -> Vec<Bounds<Pixels>> {
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

#[cfg(test)]
#[path = "popover_test.rs"]
mod tests;
