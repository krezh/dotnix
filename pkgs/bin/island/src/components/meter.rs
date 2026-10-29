//! A level shown with what it is of, in one of three shapes: a bar, such as the volume pop-up, with
//! a tinted glyph, a caption and label, and the amount; a ring round its glyph with the amount and
//! label under it, such as a battery; or a number with its label over it and a line of its recent
//! history under it, such as CPU use. Several share rows.

use std::path::PathBuf;

use gpui::{
    AnyElement, Context, FontWeight, SharedString, div, prelude::*, px, relative, rgb, rgba,
};

use crate::{
    components::{
        card,
        chart::{self, Series},
        icon, motion,
        ring::{self, Ring, Rings},
        slider::slider,
        tooltip::Tooltips,
    },
    island::Island,
    theme::{ACCENT_TINT_ALPHA, OVERLAY2, SUBTEXT1, TEXT, with_alpha},
    ui::{self, Radius, SMALL_SIZE},
};

const WIDTH: f32 = 360.0;
const GLYPH: f32 = 22.0;
const TILE: f32 = 40.0;
const RING: f32 = 54.0;
const RING_GLYPH: f32 = 22.0;
const RING_W: f32 = 84.0;
const HISTORY_NAME: f32 = 11.5;
const HISTORY_GLYPH: f32 = 14.0;
const HISTORY_VALUE: f32 = 20.0;
const HISTORY_UNIT: f32 = 12.0;
const HISTORY_H: f32 = 24.0;

pub enum Shape {
    Bar,
    /// The level as a ring round the glyph, as a share of 100.
    Ring,
    /// The amount, in its unit, in place of the level, and the line of what it was before.
    History {
        amount: SharedString,
        unit: SharedString,
        history: Option<Series>,
    },
}

pub struct Meter {
    /// Keeps its morphing text and moving parts apart from any other meter's.
    pub id: SharedString,
    pub glyph: Option<PathBuf>,
    pub caption: SharedString,
    pub label: SharedString,
    /// Level and muted; `None` shows only what the meter is of, such as a device just switched to.
    pub level: Option<(f32, bool)>,
    pub color: u32,
    pub shape: Shape,
}

/// Meters this many to a row; rings take as many as fit.
pub fn meters(
    columns: usize,
    meters: Vec<Meter>,
    phase: f32,
    tips: &Tooltips,
    cx: &mut Context<Island>,
) -> AnyElement {
    if meters.iter().all(|meter| matches!(meter.shape, Shape::Ring)) {
        return div()
            .flex()
            .flex_wrap()
            .gap_y(px(10.0))
            .children(meters.into_iter().map(ring_meter))
            .into_any_element();
    }
    let columns = columns.max(1);
    let mut cells = meters.into_iter().peekable();
    let mut rows = Vec::new();
    while cells.peek().is_some() {
        let row: Vec<AnyElement> = (0..columns)
            .map(|_| match cells.next() {
                Some(next) => div()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(meter(next, phase, tips, cx))
                    .into_any_element(),
                None => div().flex_1().into_any_element(),
            })
            .collect();
        rows.push(div().flex().gap(px(18.0)).children(row));
    }
    div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .children(rows)
        .into_any_element()
}

pub fn meter(meter: Meter, phase: f32, tips: &Tooltips, cx: &mut Context<Island>) -> AnyElement {
    match meter.shape {
        Shape::Bar => bar_meter(meter, cx),
        Shape::Ring => ring_meter(meter),
        Shape::History { .. } => history_meter(meter, phase, tips),
    }
}

fn bar_meter(meter: Meter, cx: &mut Context<Island>) -> AnyElement {
    let at = meter.id.clone();
    let muted = meter.level.is_some_and(|(_, muted)| muted);
    let tint = if muted { OVERLAY2 } else { meter.color };
    let tile = div()
        .relative()
        .size(px(TILE))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(ui::shape(
            Radius::Control,
            Some(rgba(with_alpha(tint, ACCENT_TINT_ALPHA)).into()),
        ))
        .when_some(meter.glyph.as_deref(), |slot, path| {
            slot.child(icon::glyph(path, GLYPH, tint))
        });
    let id = |part: &str| SharedString::from(format!("{at}-{part}"));
    let caption = motion::text(id("caption"), meter.caption.clone());
    let label = motion::text(id("label"), meter.label.clone());
    let row = div()
        .w(px(WIDTH))
        .px(px(4.0))
        .flex()
        .items_center()
        .gap(px(12.0))
        .child(tile);
    let Some((value, muted)) = meter.level else {
        return row
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .when(!meter.caption.is_empty(), |text| {
                        text.child(
                            div()
                                .text_size(px(SMALL_SIZE))
                                .text_color(rgb(SUBTEXT1))
                                .child(caption),
                        )
                    })
                    .child(
                        div()
                            .min_w(px(0.0))
                            .truncate()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(meter.color))
                            .child(label),
                    ),
            )
            .into_any_element();
    };
    let percent = (value * 100.0).round() as i32;
    let heading = div()
        .flex()
        .items_baseline()
        .gap(px(8.0))
        .min_w(px(0.0))
        .when(!meter.caption.is_empty(), |line| {
            line.child(
                div()
                    .flex_none()
                    .text_size(px(SMALL_SIZE))
                    .text_color(rgb(SUBTEXT1))
                    .child(caption),
            )
        })
        .child(
            div()
                .min_w(px(0.0))
                .truncate()
                .font_weight(FontWeight::SEMIBOLD)
                .child(label),
        );
    let amount = div()
        .flex_none()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(if muted { OVERLAY2 } else { TEXT }))
        .child(motion::text(
            id("amount"),
            if muted {
                "Muted".to_owned()
            } else {
                format!("{percent}%")
            },
        ));
    row.child(
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .justify_between()
                    .gap(px(12.0))
                    .child(heading)
                    .child(amount),
            )
            .child(slider(
                id("slider"),
                if muted { 0.0 } else { value },
                tint,
                None,
                cx,
            )),
    )
    .into_any_element()
}

fn ring_meter(meter: Meter) -> AnyElement {
    let level = meter.level.map_or(0.0, |(level, _)| level.clamp(0.0, 1.0));
    let ring = ring::rings(Rings {
        id: meter.id.clone(),
        size: RING,
        rings: vec![Ring {
            fraction: level,
            color: meter.color,
            mark: None,
            ahead: None,
        }],
        value: SharedString::default(),
        value_color: TEXT,
        label: SharedString::default(),
        glyph: meter.glyph.map(|path| (path, SUBTEXT1, RING_GLYPH)),
    });
    div()
        .w(px(RING_W))
        .flex()
        .flex_col()
        .items_center()
        .gap(px(4.0))
        .child(ring)
        .child(
            div()
                .pt(px(2.0))
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child(motion::text(
                    SharedString::from(format!("{}-value", meter.id)),
                    format!("{}%", (level * 100.0).round() as i32),
                )),
        )
        .child(
            div()
                .max_w_full()
                .truncate()
                .text_size(px(11.0))
                .text_color(rgb(OVERLAY2))
                .child(motion::text(
                    SharedString::from(format!("{}-label", meter.id)),
                    meter.label,
                )),
        )
        .into_any_element()
}

fn history_meter(meter: Meter, phase: f32, tips: &Tooltips) -> AnyElement {
    let Shape::History {
        amount,
        unit,
        history,
    } = meter.shape
    else {
        return div().into_any_element();
    };
    let history = history.map(|series| {
                div().h(px(HISTORY_H)).mt(px(4.0)).child(chart::chart(
            SharedString::from(format!("{}-history", meter.id)),
            vec![series],
            card::SLOTS,
            phase,
            tips,
        ))
    });
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(5.0))
                .text_size(px(HISTORY_NAME))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(meter.color))
                .children(
                    meter
                        .glyph
                        .as_deref()
                        .map(|path| icon::glyph(path, HISTORY_GLYPH, meter.color)),
                )
                .child(div().min_w(px(0.0)).truncate().child(motion::text(
                    SharedString::from(format!("{}-label", meter.id)),
                    meter.label,
                ))),
        )
        .child(
            div()
                .flex()
                .items_baseline()
                .gap(px(3.0))
                .line_height(relative(1.2))
                .child(
                    div()
                        .text_size(px(HISTORY_VALUE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(motion::text(
                            SharedString::from(format!("{}-value", meter.id)),
                            amount,
                        )),
                )
                .child(
                    div()
                        .text_size(px(HISTORY_UNIT))
                        .text_color(rgb(SUBTEXT1))
                        .child(motion::text(
                            SharedString::from(format!("{}-unit", meter.id)),
                            unit,
                        )),
                ),
        )
        .children(history)
        .into_any_element()
}
