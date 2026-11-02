//! A button that opens a short list of things to do with what it sits on, such as trusting or
//! forgetting a device. Opening grows the button into the list below it, its right edge kept, as
//! the island grows into a panel; closing folds it back.

use std::path::PathBuf;

use gpui::{
    AnyElement, Bounds, Context, FontWeight, MouseButton, MouseDownEvent, Pixels, SharedString,
    div, prelude::*, px, rgb,
};

use crate::{
    components::{
        button, icon,
        measure::{self, Measured},
        popover::{self, Growth, Popovers},
        tooltip::{Tip, Tooltips},
    },
    island::Island,
    panel::Handler,
    panel::IslandUi,
    shape,
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

fn mix(from: f32, to: f32, at: f32) -> f32 {
    from + (to - from) * at
}

fn row(action: &Action, hover: bool) -> gpui::Stateful<gpui::Div> {
    let color = if action.danger { RED } else { TEXT };
    button::ghost(action.id.clone(), hover)
        .active(|style| style)
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
                    label.child(icon::glyph(
                        &path,
                        GLYPH,
                        if action.danger { RED } else { SUBTEXT1 },
                    ))
                })
                .child(action.label.clone()),
        )
        .when_some(action.switch, |row, on| {
            row.child(button::switch(
                SharedString::from(format!("{}-switch", action.id)),
                on,
            ))
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
    growth: &Growth,
    popovers: &Popovers,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let (button_w, button_h) = (f32::from(anchor.size.width), f32::from(anchor.size.height));
    let (open_w, open_h) = growth.natural.get().map_or((button_w, button_h), |size| {
        (f32::from(size.width), f32::from(size.height))
    });
    let live = growth.live;
    // On whole pixels, so text, which is drawn on them, keeps still against the edges.
    let w = mix(button_w, open_w, growth.grown).max(0.0).round();
    let h = mix(button_h, open_h, growth.grown).max(0.0).round();
    let (radius, power) = Radius::Control.geometry();
    let right = f32::from(anchor.origin.x + anchor.size.width);
    let left = popovers.keep_within(right - w, w, EDGE_MARGIN).round();
    let top = (f32::from(anchor.origin.y) + mix(0.0, button_h + DROP, growth.look)).round();
    let rows: Vec<AnyElement> = view
        .actions
        .into_iter()
        .map(|action| {
            let row = row(&action, hover);
            if live {
                let handler = action.on_click.clone();
                row.on_click(
                    cx.listener(move |this, _, _, cx| handler(&mut IslandUi::new(this, cx))),
                )
                .into_any_element()
            } else {
                row.into_any_element()
            }
        })
        .collect();
    let list = div()
        .absolute()
        .top(px(INSET))
        .right(px(INSET))
        .w(px((open_w - 2.0 * INSET).max(0.0)))
        .opacity(growth.rows)
        .flex()
        .flex_col()
        .gap(px(ROW_SPACING))
        .children(rows);
    ui::base()
        .absolute()
        .left(px(left))
        .top(px(top))
        .w(px(w))
        .h(px(h))
        .child(
            div()
                .absolute()
                .size_full()
                .opacity(growth.cover)
                .child(ui::shadow(radius, power, SHADOW_SPREAD)),
        )
        .child(shape::background(
            radius,
            power,
            ui::surface_fill(POPOVER_OPACITY).opacity(growth.cover),
            Some(ui::surface_border().opacity(growth.cover)),
        ))
        .child(popovers.reach())
        .child(div().absolute().size_full().overflow_hidden().child(list))
        .when(live, |surface| surface.occlude())
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
    let growth = popovers.grow(&view.id, view.open);
    let toggle = view.on_toggle.clone();
    let button = button::button(
        button::Button::new(view.id.clone())
            .glyph(view.glyph.as_deref())
            .compact(true)
            .active(view.open),
        hover,
    )
    .child(popovers.anchor(view.id.clone()))
    .tip(tips, view.id.clone(), view.label.clone())
    // Opens on the press, as a menu does: the morph is the feedback, with no dimming before it.
    .active(|style| style)
    .on_mouse_down(
        MouseButton::Left,
        cx.listener(move |this, _: &MouseDownEvent, _, cx| {
            cx.stop_propagation();
            toggle(&mut IslandUi::new(this, cx));
        }),
    );
    if growth.shown
        && let Some(bounds) = growth.anchor
    {
        if growth.open {
            popovers.show(popover::scrim(bounds, view.on_toggle.clone(), cx));
        }
        popovers.show(measuring(&view.actions, &growth.natural));
        popovers.show(surface(view, bounds, &growth, popovers, hover, cx));
    }
    button.into_any_element()
}
