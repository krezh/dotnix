//! Picks one of several values, showing the current one while closed, such as which device an app
//! plays on. Opening grows the chip itself into the list, down and equally to both sides, as the
//! island grows into a panel: one shape, whose size, corners and color move from the chip's to the
//! list's, the choices fading in where they stay as it grows. Closing folds it back into the chip. The list keeps its order, the current choice marked in its place.

use std::{path::PathBuf, rc::Rc};

use gpui::{
    AnyElement, Bounds, Context, FontWeight, Hsla, MouseButton, MouseDownEvent, Pixels,
    SharedString, div, prelude::*, px, rgb,
};

use crate::{
    components::{
        button, icon,
        measure::{self, Measured},
        pointer,
        popover::{self, Growth, Popovers},
    },
    island::Island,
    panel::Handler,
    panel::{IslandUi, Ui},
    shape,
    theme::{ACCENT, OVERLAY1, POPOVER_OPACITY, SUBTEXT1, TEXT},
    ui::{self, Radius, SMALL_SIZE},
};

const ROW_H: f32 = 30.0;
/// Between rows, so the selected one's highlight and a hovered one's never touch.
const ROW_SPACING: f32 = 2.0;
const ROW_PAD: f32 = 8.0;
const ROW_GAP: f32 = 8.0;
const GLYPH: f32 = 16.0;
/// The list's inset from the surface's edges.
const INSET: f32 = 4.0;
/// A chip's label size.
const CHIP_TEXT: f32 = 12.0;
const SHADOW_SPREAD: f32 = 6.0;
/// How far inside the island's edges the open list keeps.
const EDGE_MARGIN: f32 = 8.0;

#[derive(Clone)]
pub struct Choice {
    pub label: SharedString,
    pub glyph: Option<PathBuf>,
    /// At the row's end in the open list, such as how many it holds; the chip shows only the label.
    pub detail: Option<SharedString>,
}

const DETAIL_GAP: f32 = 16.0;

pub type Select = Rc<dyn Fn(usize, &mut dyn Ui)>;

pub struct Dropdown {
    pub id: SharedString,
    pub choices: Vec<Choice>,
    pub selected: Option<usize>,
    pub open: bool,
    /// Opens or closes it; also called to close it when a click lands elsewhere.
    pub on_toggle: Handler,
    pub on_select: Select,
    /// As tall as this and drawn as a strip, such as beside a strip of tabs; otherwise a chip.
    pub height: Option<f32>,
}

/// A dropdown drawn as a strip: its label's inset from the sides.
const STRIP_LABEL_PAD: f32 = 14.0;

fn mix(from: f32, to: f32, at: f32) -> f32 {
    from + (to - from) * at
}

/// Each color counts by how opaque it is, so a faint light tint turning into a dark surface does
/// not pass through a light one on the way.
fn mix_color(from: Hsla, to: Hsla, at: f32) -> Hsla {
    let (from, to) = (from.to_rgb(), to.to_rgb());
    let a = mix(from.a, to.a, at);
    let channel = |from_c: f32, to_c: f32| {
        if a <= f32::EPSILON {
            mix(from_c, to_c, at)
        } else {
            mix(from_c * from.a, to_c * to.a, at) / a
        }
    };
    gpui::Rgba {
        r: channel(from.r, to.r),
        g: channel(from.g, to.g),
        b: channel(from.b, to.b),
        a,
    }
    .into()
}

/// One color with another drawn over it, as a single color.
fn over(top: Hsla, under: Hsla) -> Hsla {
    let (top, under) = (top.to_rgb(), under.to_rgb());
    let a = top.a + under.a * (1.0 - top.a);
    let channel = |top_c: f32, under_c: f32| {
        if a <= f32::EPSILON {
            0.0
        } else {
            (top_c * top.a + under_c * under.a * (1.0 - top.a)) / a
        }
    };
    gpui::Rgba {
        r: channel(top.r, under.r),
        g: channel(top.g, under.g),
        b: channel(top.b, under.b),
        a,
    }
    .into()
}

/// How the chip looks at rest: its fill, with the hover tint if `hovered`, and its border. The tint
/// lies under a strip's own fill, and over a chip's.
fn chip_look(strip: bool, hovered: bool) -> (Hsla, Hsla) {
    let tint: Hsla = ui::hover_fill().into();
    if strip {
        let fill = ui::strip_fill();
        let fill = if hovered { over(fill, tint) } else { fill };
        (fill, ui::strip_border())
    } else {
        let fill = button::chip_fill();
        let border = Hsla { a: 0.0, ..fill };
        (if hovered { over(tint, fill) } else { fill }, border)
    }
}

/// `current` marks it as the one chosen.
fn choice_row(
    id: SharedString,
    choice: Choice,
    current: bool,
    hover: bool,
) -> gpui::Stateful<gpui::Div> {
    let (glyph, label, detail) = if current {
        (ACCENT, ACCENT, ACCENT)
    } else {
        (SUBTEXT1, TEXT, OVERLAY1)
    };
    button::ghost(id, hover)
        // Picks on the press: the list closing is the feedback, with no dimming before it.
        .active(|style| style)
        .relative()
        .h(px(ROW_H))
        .px(px(ROW_PAD))
        .justify_start()
        .gap(px(ROW_GAP))
        .text_size(px(SMALL_SIZE))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(label))
        .whitespace_nowrap()
        .when(current, |row| {
            row.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .child(ui::shape(Radius::Row, Some(ui::selected_fill().into()))),
            )
        })
        .when_some(choice.glyph, |row, path| {
            row.child(icon::glyph(&path, GLYPH, glyph))
        })
        .child(choice.label)
        .when_some(choice.detail, |row, value| {
            row.child(
                div()
                    .ml_auto()
                    .pl(px(DETAIL_GAP))
                    .text_color(rgb(detail))
                    .child(value),
            )
        })
}

/// An unseen copy of the open list at its own size, laid out where nothing squeezes it, so the
/// surface knows how big to grow.
fn measuring(
    id: &SharedString,
    choices: &[Choice],
    selected: Option<usize>,
    natural: &Measured,
) -> AnyElement {
    let rows = choices.iter().enumerate().map(|(index, choice)| {
        choice_row(
            SharedString::from(format!("{id}-measure-{index}")),
            choice.clone(),
            selected == Some(index),
            false,
        )
    });
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
                .children(rows)
                .child(measure::probe(natural)),
        )
        .into_any_element()
}

/// The chip and the list as one shape: while drawn it is the chip, which starts exactly as the chip
/// looks, and its size, corners and color all move together to the list's.
fn surface(
    dropdown: Dropdown,
    label: SharedString,
    chip: Bounds<Pixels>,
    growth: &Growth,
    popovers: &Popovers,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let (chip_w, chip_h) = (f32::from(chip.size.width), f32::from(chip.size.height));
    let (open_w, open_h) = growth.natural.get().map_or((chip_w, chip_h), |size| {
        (f32::from(size.width).max(chip_w), f32::from(size.height))
    });
    let grown = growth.grown;
    let look = growth.look;
    let live = growth.live;
    let w = mix(chip_w, open_w, grown).max(0.0);
    let h = mix(chip_h, open_h, grown).max(0.0).round();
    let strip = dropdown.height.is_some();
    let (rest_r, power) = if strip {
        ui::strip_geometry()
    } else {
        Radius::Badge.geometry()
    };
    let (open_r, _) = Radius::Control.geometry();
    let radius = mix(rest_r, open_r, look);
    let (rest_fill, rest_border) = chip_look(strip, hover && pointer::get(&dropdown.id).hovered);
    // Opaque, as tooltips are: whatever of the island lies under it cannot be blurred.
    let fill = mix_color(rest_fill, ui::surface_fill(POPOVER_OPACITY), growth.cover);
    let border = mix_color(rest_border, ui::surface_border(), growth.cover);
    let label_size = if strip { SMALL_SIZE } else { CHIP_TEXT };
    // Grows down and equally to both sides of the chip, as the island grows into a panel; the open
    // list moves in only as far as it must to stay inside the island, eased in as it grows.
    let chip_x = f32::from(chip.origin.x);
    let centre = f32::from(chip.center().x);
    let open_left = popovers.keep_within(centre - open_w / 2.0, open_w, EDGE_MARGIN);
    let pushed = open_left - (centre - open_w / 2.0);
    // Each edge on a whole pixel, so text, which is drawn on them, keeps still against it, and
    // neither edge steps back as the other rounds.
    let left = (centre - w / 2.0 + pushed * look).round();
    let w = (centre + w / 2.0 + pushed * look).round() - left;
    let top = f32::from(chip.origin.y).round();
    // Where the list stays on screen, whatever the surface's size.
    let list_x = (open_left - left).round();

    let label = div()
        .absolute()
        .top_0()
        .left(px(chip_x - left))
        .w(px(chip_w))
        .h(px(chip_h))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(label_size))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(SUBTEXT1))
        .whitespace_nowrap()
        .opacity(growth.opener)
        .child(label);
    let rows: Vec<AnyElement> = dropdown
        .choices
        .into_iter()
        .enumerate()
        .map(|(index, choice)| {
            let id = SharedString::from(format!("{}-choice-{index}", dropdown.id));
            let current = dropdown.selected == Some(index);
            let row = choice_row(id, choice, current, hover);
            if !live {
                row.into_any_element()
            } else if current {
                let toggle = dropdown.on_toggle.clone();
                row.on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                        toggle(&mut IslandUi::new(this, cx))
                    }),
                )
                .into_any_element()
            } else {
                let select = dropdown.on_select.clone();
                row.on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                        select(index, &mut IslandUi::new(this, cx))
                    }),
                )
                .into_any_element()
            }
        })
        .collect();
    // Laid out where they stay, uncovered as the surface grows over them.
    let list = div()
        .absolute()
        .top(px(INSET))
        .left(px(list_x + INSET))
        .w(px(open_w - 2.0 * INSET))
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
        .child(shape::background(radius, power, fill, Some(border)))
        .child(popovers.reach())
        // Clipped square: a round clip would not follow the squircle drawn above, and the list's
        // inset keeps its rows clear of the corners.
        .child(
            div()
                .absolute()
                .size_full()
                .overflow_hidden()
                .child(list)
                .child(label),
        )
        .when(live, |surface| surface.occlude())
        .into_any_element()
}

/// The chip, in place. While open or moving, the surface takes over from it, drawn over everything,
/// and the chip stays behind it unseen, keeping its place in the row and taking clicks.
pub fn dropdown(
    mut dropdown: Dropdown,
    popovers: &Popovers,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let current = dropdown
        .selected
        .and_then(|index| dropdown.choices.get(index))
        .cloned()
        .unwrap_or(Choice {
            label: SharedString::default(),
            glyph: None,
            detail: None,
        });
    let growth = popovers.grow(&dropdown.id, dropdown.open);
    let shown = growth.shown;
    let toggle = dropdown.on_toggle.clone();
    let hover_id = dropdown.id.clone();
    let chip = match dropdown.height {
        Some(height) => button::fill(dropdown.id.clone(), Radius::Row, None, hover && !shown)
            .relative()
            .flex_none()
            .h(px(height))
            .px(px(STRIP_LABEL_PAD))
            .text_size(px(SMALL_SIZE))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(SUBTEXT1))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .child(ui::strip()),
            )
            .child(div().relative().child(current.label.clone())),
        None => button::chip(
            dropdown.id.clone(),
            current.label.clone(),
            false,
            hover && !shown,
        ),
    }
    .child(popovers.anchor(dropdown.id.clone()))
    .when(shown, |chip| chip.opacity(0.0))
    .on_hover(move |hovered, window, _| {
        pointer::set(&hover_id, |pointer| pointer.hovered = *hovered, window)
    })
    // Opens on the press, as a menu does: the morph is the feedback, with no dimming before it.
    .active(|style| style)
    .on_mouse_down(
        MouseButton::Left,
        cx.listener(move |this, _: &MouseDownEvent, _, cx| toggle(&mut IslandUi::new(this, cx))),
    );
    dropdown.selected = popovers.hold(&dropdown.id, dropdown.selected);
    if growth.shown
        && let Some(bounds) = growth.anchor
    {
        if growth.open {
            popovers.show(popover::scrim(bounds, dropdown.on_toggle.clone(), cx));
        }
        popovers.show(measuring(
            &dropdown.id,
            &dropdown.choices,
            dropdown.selected,
            &growth.natural,
        ));
        popovers.show(surface(
            dropdown,
            current.label,
            bounds,
            &growth,
            popovers,
            hover,
            cx,
        ));
    }
    chip.into_any_element()
}
