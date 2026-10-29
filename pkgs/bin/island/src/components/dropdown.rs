//! Picks one of several values, showing the current one while closed, such as which device an app
//! plays on. Opening grows the chip itself into the list, spreading equally to both sides and
//! down, as the island grows into a panel; closing folds it back into the chip.

use std::{cell::RefCell, collections::HashMap, path::PathBuf, rc::Rc, time::Instant};

use gpui::{
    AnyElement, Bounds, Context, FontWeight, Hsla, MouseDownEvent, Pixels, SharedString, Size, div,
    prelude::*, px, rgb, rgba,
};

use crate::{
    components::{
        button, icon,
        measure::{self, Measured},
        popover::Popovers,
    },
    island::{CONTENT_DRIFT, CONTENT_IN, CONTENT_OUT, Island, REVEAL_DELAY, SHAPE_SPRING},
    panel::Handler,
    shape,
    spring::Spring,
    theme::{
        ACCENT, CONTROL_ALPHA, OVERLAY1, POPOVER_OPACITY, SUBTEXT1, SURFACE1, TEXT, with_alpha,
    },
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
/// Where a row's label starts, past its glyph.
const LABEL_X: f32 = INSET + ROW_PAD + GLYPH + ROW_GAP;
/// The chip's label size, which the current choice grows from.
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

pub type Select = Rc<dyn Fn(usize, &mut Island, &mut Context<Island>)>;

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

/// One dropdown's motion, kept across frames.
struct Morph {
    /// 0 is the chip, 1 the list; it overshoots a little on the way.
    grown: Spring,
    revealed: Spring,
    open: bool,
    at: Instant,
    /// The list's own size, measured while it is drawn.
    natural: Measured,
    /// The current choice's label at the rows' size.
    label: Measured,
}

impl Morph {
    fn new() -> Self {
        Self {
            grown: Spring::new(0.0, SHAPE_SPRING, 0.001),
            revealed: Spring::new(0.0, CONTENT_IN, 0.002),
            open: false,
            at: Instant::now(),
            natural: measure::measured(),
            label: measure::measured(),
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
                if self.revealed.value() < 0.05 {
                    self.revealed
                        .retarget_at(1.0, CONTENT_IN, now + REVEAL_DELAY);
                } else {
                    self.revealed.retarget(1.0, CONTENT_IN);
                }
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

fn choice_row(id: SharedString, choice: Choice, hover: bool) -> gpui::Stateful<gpui::Div> {
    button::ghost(id, hover)
        .h(px(ROW_H))
        .px(px(ROW_PAD))
        .justify_start()
        .gap(px(ROW_GAP))
        .text_size(px(SMALL_SIZE))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(TEXT))
        .whitespace_nowrap()
        .when_some(choice.glyph, |row, path| {
            row.child(icon::glyph(&path, GLYPH, SUBTEXT1))
        })
        .child(choice.label)
        .when_some(choice.detail, |row, detail| {
            row.child(
                div()
                    .ml_auto()
                    .pl(px(DETAIL_GAP))
                    .text_color(rgb(OVERLAY1))
                    .child(detail),
            )
        })
}

/// An unseen copy of the open list at its own size, laid out where nothing squeezes it, so the
/// surface knows how big to grow and how wide the current choice's label is.
fn measuring(
    id: &SharedString,
    current: Choice,
    others: &[(usize, Choice)],
    natural: &Measured,
    label: &Measured,
) -> AnyElement {
    let selected = div()
        .h(px(ROW_H))
        .px(px(ROW_PAD))
        .flex()
        .items_center()
        .gap(px(ROW_GAP))
        .text_size(px(SMALL_SIZE))
        .font_weight(FontWeight::SEMIBOLD)
        .whitespace_nowrap()
        .child(div().size(px(GLYPH)).flex_none())
        .child(
            div()
                .relative()
                .child(current.label)
                .child(measure::probe(label)),
        )
        .when_some(current.detail, |row, detail| {
            row.child(div().pl(px(DETAIL_GAP)).child(detail))
        });
    let rows = others.iter().map(|(index, choice)| {
        choice_row(
            SharedString::from(format!("{id}-measure-{index}")),
            choice.clone(),
            false,
        )
        .into_any_element()
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
                .child(selected)
                .children(rows)
                .child(measure::probe(natural)),
        )
        .into_any_element()
}

/// Where the surface is in its motion, and the sizes it moves between.
struct Motion {
    grown: f32,
    revealed: f32,
    natural: Option<Size<Pixels>>,
    /// The current choice's label at the rows' size.
    label: Option<Size<Pixels>>,
}

/// The surface while it is not the chip, placed over where the chip was drawn. The chip's label
/// becomes the first row, selected: it slides from the chip's centre to its place in the row and
/// grows to the rows' size, while its glyph, its highlight and the other choices fade in.
fn surface(
    dropdown: Dropdown,
    current: Choice,
    chip: Bounds<Pixels>,
    motion: Motion,
    popovers: &Popovers,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let Motion {
        grown,
        revealed,
        natural,
        label,
    } = motion;
    let (chip_w, chip_h) = (f32::from(chip.size.width), f32::from(chip.size.height));
    let (open_w, open_h) = natural.map_or((chip_w, chip_h), |size| {
        (f32::from(size.width).max(chip_w), f32::from(size.height))
    });
    let w = mix(chip_w, open_w, grown).max(0.0);
    let h = mix(chip_h, open_h, grown).max(0.0);
    let look = grown.clamp(0.0, 1.0);
    let strip = dropdown.height.is_some();
    // Grows from the chip as it rests, a strip or a chip.
    let ((rest_r, power), rest, rest_border) = if strip {
        (ui::strip_geometry(), ui::strip_fill(), ui::strip_border())
    } else {
        let rest: Hsla = rgba(with_alpha(SURFACE1, CONTROL_ALPHA)).into();
        (Radius::Badge.geometry(), rest, rest)
    };
    let (open_r, _) = Radius::Control.geometry();
    let radius = mix(rest_r, open_r, look);
    // Nearly opaque, as tooltips are: whatever of the island lies under it cannot be blurred.
    let fill = mix_color(rest, ui::surface_fill(POPOVER_OPACITY), look);
    let border = mix_color(rest_border, ui::surface_border(), look);
    let chip_text = if strip { SMALL_SIZE } else { CHIP_TEXT };
    // Spreads equally from the chip, unless that would take it past the island's edge.
    let centred = f32::from(chip.center().x) - w / 2.0;
    let left = popovers.keep_within(centred, w, EDGE_MARGIN);
    let shift = left - centred;
    let top = f32::from(chip.origin.y);

    let rows: Vec<AnyElement> = dropdown
        .choices
        .into_iter()
        .enumerate()
        .filter(|(index, _)| dropdown.selected != Some(*index))
        .map(|(index, choice)| {
            let select = dropdown.on_select.clone();
            let id = SharedString::from(format!("{}-choice-{index}", dropdown.id));
            choice_row(id, choice, hover)
                .on_click(cx.listener(move |this, _, _, cx| select(index, this, cx)))
                .into_any_element()
        })
        .collect();
    let shown = revealed.clamp(0.0, 1.0);
    // Laid out at the open size and centred, so the rows sit where they will settle as the surface
    // grows round them.
    let offset = (w - open_w) / 2.0;
    let row_top = mix(0.0, INSET, look);
    let row_h = mix(chip_h, ROW_H, look);
    let size = mix(chip_text, SMALL_SIZE, look);
    let label_w = label.map_or(0.0, |label| f32::from(label.width)) * size / SMALL_SIZE;
    // Starts centred on the chip, wherever the surface had to sit.
    let label_left = mix((w - label_w) / 2.0 - shift, offset + LABEL_X, look);
    let selected = div()
        .absolute()
        .left(px(INSET))
        .top(px(row_top))
        .w(px((w - 2.0 * INSET).max(0.0)))
        .h(px(row_h))
        .opacity(look)
        .child(ui::shape(Radius::Row, Some(ui::selected_fill().into())));
    let glyph = current.glyph.map(|path| {
        div()
            .absolute()
            .left(px(offset + INSET + ROW_PAD))
            .top(px(row_top + (row_h - GLYPH) / 2.0))
            .opacity(shown)
            .child(icon::glyph(&path, GLYPH, ACCENT))
    });
    let detail = current.detail.clone().map(|detail| {
        div()
            .absolute()
            .right(px(offset + INSET + ROW_PAD))
            .top(px(row_top))
            .h(px(row_h))
            .flex()
            .items_center()
            .opacity(shown)
            .text_size(px(SMALL_SIZE))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(ACCENT))
            .child(detail)
    });
    let title = div()
        .absolute()
        .left(px(label_left))
        .top(px(row_top))
        .h(px(row_h))
        .flex()
        .items_center()
        .text_size(px(size))
        .font_weight(FontWeight::SEMIBOLD)
        .whitespace_nowrap()
        .text_color(mix_color(rgb(SUBTEXT1).into(), rgb(ACCENT).into(), look))
        .child(current.label);
    let toggle = dropdown.on_toggle.clone();
    let hit = div()
        .id(SharedString::from(format!("{}-selected", dropdown.id)))
        .absolute()
        .top_0()
        .left_0()
        .w_full()
        .h(px(row_top + row_h))
        .cursor_pointer()
        .on_click(cx.listener(move |this, _, _, cx| toggle(this, cx)));
    let others = div()
        .absolute()
        .top(px(
            INSET + ROW_H + ROW_SPACING - (1.0 - shown) * CONTENT_DRIFT
        ))
        .left(px(offset + INSET))
        .w(px(open_w - 2.0 * INSET))
        .opacity(shown)
        .flex()
        .flex_col()
        .gap(px(ROW_SPACING))
        .children(rows);
    let content = div()
        .absolute()
        .size_full()
        .child(selected)
        .children(glyph)
        .children(detail)
        .child(title)
        .child(others)
        .child(hit);
    let anchor = chip;
    let dismiss = dropdown.on_toggle;
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
        .child(shape::background(radius, power, fill, Some(border)))
        .child(popovers.reach(radius, power))
        // Clipped square: a round clip would not follow the squircle drawn above, and the list's
        // inset keeps its rows clear of the corners.
        .child(
            div()
                .absolute()
                .size_full()
                .overflow_hidden()
                .child(content),
        )
        .on_mouse_down_out(cx.listener(move |this, event: &MouseDownEvent, _, cx| {
            if !anchor.contains(&event.position) {
                dismiss(this, cx);
            }
        }))
        .into_any_element()
}

/// The chip, in place. While open or moving, the surface it grows into is drawn over everything,
/// and the chip stays behind it unseen, keeping its place in the row.
pub fn dropdown(
    dropdown: Dropdown,
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
    let (motion, shown, natural, label) = MORPHS.with_borrow_mut(|morphs| {
        let morph = morphs.entry(dropdown.id.clone()).or_insert_with(Morph::new);
        morph.advance(dropdown.open);
        if morph.moving() {
            popovers.animate();
        }
        let motion = Motion {
            grown: morph.grown.value(),
            revealed: morph.revealed.value(),
            natural: morph.natural.get(),
            label: morph.label.get(),
        };
        (
            motion,
            morph.shown(),
            morph.natural.clone(),
            morph.label.clone(),
        )
    });
    let toggle = dropdown.on_toggle.clone();
    let chip = match dropdown.height {
        Some(height) => button::fill(dropdown.id.clone(), Radius::Row, None, hover && !shown)
            .relative()
            .flex_none()
            .h(px(height))
            .px(px(STRIP_LABEL_PAD))
            .text_size(px(SMALL_SIZE))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(SUBTEXT1))
            .child(div().absolute().top_0().left_0().size_full().child(ui::strip()))
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
    .on_click(cx.listener(move |this, _, _, cx| toggle(this, cx)));
    if shown && let Some(bounds) = popovers.anchor_bounds(&dropdown.id) {
        let others: Vec<(usize, Choice)> = dropdown
            .choices
            .iter()
            .cloned()
            .enumerate()
            .filter(|(index, _)| dropdown.selected != Some(*index))
            .collect();
        popovers.show(measuring(
            &dropdown.id,
            current.clone(),
            &others,
            &natural,
            &label,
        ));
        popovers.show(surface(
            dropdown, current, bounds, motion, popovers, hover, cx,
        ));
    }
    chip.into_any_element()
}
