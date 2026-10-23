//! A button that opens a short list of things to do with what it sits on, such as trusting or
//! forgetting a device. Opening grows the button into the list below it, its right edge kept, as
//! the island grows into a panel; closing folds it back.

use std::{cell::RefCell, collections::HashMap, path::PathBuf, time::Instant};

use gpui::{
    AnyElement, Bounds, Context, FontWeight, MouseDownEvent, Pixels, SharedString, div, prelude::*,
    px, rgb,
};

use crate::{
    components::{
        button, icon,
        measure::{self, Measured},
        popover::Popovers,
        tooltip::{Tip, Tooltips},
    },
    island::{CONTENT_DRIFT, CONTENT_IN, CONTENT_OUT, Island, REVEAL_DELAY, SHAPE_SPRING},
    panel::Handler,
    shape,
    spring::Spring,
    theme::{POPOVER_OPACITY, RED, SUBTEXT1, TEXT},
    ui::{self, Radius, SMALL_SIZE},
};

const ROW_H: f32 = 30.0;
const ROW_SPACING: f32 = 2.0;
const ROW_PAD: f32 = 8.0;
const ROW_GAP: f32 = 12.0;
const GLYPH: f32 = 16.0;
const INSET: f32 = 4.0;
/// Between the button and the list under it.
const DROP: f32 = 6.0;
const SHADOW_SPREAD: f32 = 6.0;
const EDGE_MARGIN: f32 = 8.0;

pub struct Action {
    pub id: SharedString,
    pub label: SharedString,
    pub glyph: Option<PathBuf>,
    /// A switch at the row's end, on or off, for something that stays set, such as trust.
    pub switch: Option<bool>,
    /// Drawn in red, for something that cannot be taken back.
    pub danger: bool,
    pub on_click: Handler,
}

pub struct Actions {
    pub id: SharedString,
    pub glyph: Option<PathBuf>,
    /// The button's tooltip.
    pub label: SharedString,
    pub actions: Vec<Action>,
    pub open: bool,
    /// Opens or closes it; also called to close it when a click lands elsewhere.
    pub on_toggle: Handler,
}

struct Morph {
    grown: Spring,
    revealed: Spring,
    open: bool,
    at: Instant,
    natural: Measured,
}

impl Morph {
    fn new() -> Self {
        Self {
            grown: Spring::new(0.0, SHAPE_SPRING, 0.001),
            revealed: Spring::new(0.0, CONTENT_IN, 0.002),
            open: false,
            at: Instant::now(),
            natural: measure::measured(),
        }
    }

    fn advance(&mut self, open: bool) {
        let now = Instant::now();
        let dt = now.duration_since(self.at).as_secs_f32().min(0.05);
        self.at = now;
        if open != self.open {
            self.open = open;
            if open {
                self.grown.retarget(1.0, SHAPE_SPRING);
                self.revealed
                    .retarget_at(1.0, CONTENT_IN, now + REVEAL_DELAY);
            } else {
                self.grown.retarget(0.0, SHAPE_SPRING);
                self.revealed.retarget(0.0, CONTENT_OUT);
            }
        }
        self.grown.advance(dt, now);
        self.revealed.advance(dt, now);
    }

    fn moving(&self) -> bool {
        !(self.grown.is_settled() && self.revealed.is_settled())
    }

    fn shown(&self) -> bool {
        self.open || self.grown.value().abs() > 0.001
    }
}

thread_local! {
    static MORPHS: RefCell<HashMap<SharedString, Morph>> = RefCell::default();
}

fn mix(from: f32, to: f32, at: f32) -> f32 {
    from + (to - from) * at
}

fn row(action: &Action, hover: bool) -> gpui::Stateful<gpui::Div> {
    let color = if action.danger { RED } else { TEXT };
    button::ghost(action.id.clone(), hover)
        .h(px(ROW_H))
        .px(px(ROW_PAD))
        .justify_between()
        .gap(px(ROW_GAP))
        .text_size(px(SMALL_SIZE))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(color))
        .whitespace_nowrap()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .when_some(action.glyph.clone(), |label, path| {
                    label.child(icon::glyph(&path, GLYPH, if action.danger { RED } else { SUBTEXT1 }))
                })
                .child(action.label.clone()),
        )
        .when_some(action.switch, |row, on| {
            row.child(button::switch(SharedString::from(format!("{}-switch", action.id)), on))
        })
}

/// An unseen copy of the list at its own size, so the surface knows how big to grow.
fn measuring(actions: &[Action], natural: &Measured) -> AnyElement {
    ui::base()
        .absolute()
        .top_0()
        .left_0()
        .invisible()
        .child(
            div()
                .relative()
                .p(px(INSET))
                .flex()
                .flex_col()
                .gap(px(ROW_SPACING))
                .children(actions.iter().map(|action| row(action, false)))
                .child(measure::probe(natural)),
        )
        .into_any_element()
}

fn surface(
    view: Actions,
    anchor: Bounds<Pixels>,
    grown: f32,
    revealed: f32,
    natural: Option<gpui::Size<Pixels>>,
    popovers: &Popovers,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let (button_w, button_h) = (f32::from(anchor.size.width), f32::from(anchor.size.height));
    let (open_w, open_h) = natural.map_or((button_w, button_h), |size| {
        (f32::from(size.width), f32::from(size.height))
    });
    let w = mix(button_w, open_w, grown).max(0.0);
    let h = mix(button_h, open_h, grown).max(0.0);
    let look = grown.clamp(0.0, 1.0);
    let shown = revealed.clamp(0.0, 1.0);
    let (radius, power) = Radius::Control.geometry();
    let right = f32::from(anchor.origin.x + anchor.size.width);
    let left = popovers.keep_within(right - w, w, EDGE_MARGIN);
    let top = f32::from(anchor.origin.y) + mix(0.0, button_h + DROP, look);
    let rows: Vec<AnyElement> = view
        .actions
        .into_iter()
        .map(|action| {
            let handler = action.on_click.clone();
            row(&action, hover)
                .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
                .into_any_element()
        })
        .collect();
    let list = div()
        .absolute()
        .top(px(INSET - (1.0 - shown) * CONTENT_DRIFT))
        .right(px(INSET))
        .w(px((open_w - 2.0 * INSET).max(0.0)))
        .opacity(shown)
        .flex()
        .flex_col()
        .gap(px(ROW_SPACING))
        .children(rows);
    let dismiss = view.on_toggle;
    ui::base()
        .absolute()
        .left(px(left))
        .top(px(top))
        .w(px(w))
        .h(px(h))
        .child(div().absolute().size_full().opacity(look).child(ui::shadow(
            radius,
            power,
            SHADOW_SPREAD,
        )))
        .child(shape::background(
            radius,
            power,
            ui::surface_fill(POPOVER_OPACITY),
            Some(ui::surface_border()),
        ))
        .child(popovers.reach(radius, power))
        .child(
            div()
                .absolute()
                .size_full()
                .overflow_hidden()
                .child(list),
        )
        .on_mouse_down_out(cx.listener(move |this, event: &MouseDownEvent, _, cx| {
            if !anchor.contains(&event.position) {
                dismiss(this, cx);
            }
        }))
        .into_any_element()
}

/// The button, in place; while open or moving, the list it grows into is drawn over everything.
pub fn actions(
    view: Actions,
    popovers: &Popovers,
    tips: &Tooltips,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let (grown, revealed, shown, natural) = MORPHS.with_borrow_mut(|morphs| {
        let morph = morphs.entry(view.id.clone()).or_insert_with(Morph::new);
        morph.advance(view.open);
        if morph.moving() {
            popovers.animate();
        }
        (
            morph.grown.value(),
            morph.revealed.value(),
            morph.shown(),
            morph.natural.clone(),
        )
    });
    let toggle = view.on_toggle.clone();
    let button = button::icon(view.id.clone(), view.glyph.as_deref(), None, view.open, hover)
        .size(px(ui::CONTROL_H))
        .child(popovers.anchor(view.id.clone()))
        .tip(tips, view.id.clone(), view.label.clone())
        .on_click(cx.listener(move |this, _, _, cx| {
            cx.stop_propagation();
            toggle(this, cx);
        }));
    if shown && let Some(bounds) = popovers.anchor_bounds(&view.id) {
        popovers.show(measuring(&view.actions, &natural));
        popovers.show(surface(
            view,
            bounds,
            grown,
            revealed,
            natural.get(),
            popovers,
            hover,
            cx,
        ));
    }
    button.into_any_element()
}
