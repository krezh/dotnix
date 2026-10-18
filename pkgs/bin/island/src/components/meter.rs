//! A level shown with what it is of, such as the volume pop-up: a tinted glyph, a caption and
//! label, the amount, and a bar.

use std::path::PathBuf;

use gpui::{AnyElement, Context, FontWeight, SharedString, div, prelude::*, px, rgb, rgba};

use crate::{
    components::{icon, motion, slider::slider},
    island::Island,
    theme::{ACCENT_TINT_ALPHA, OVERLAY2, SUBTEXT1, TEXT, with_alpha},
    ui::{self, Radius, SMALL_SIZE},
};

const WIDTH: f32 = 360.0;
const GLYPH: f32 = 22.0;
const TILE: f32 = 40.0;

pub struct Meter {
    pub glyph: Option<PathBuf>,
    pub caption: SharedString,
    pub label: SharedString,
    /// Level and muted; `None` shows only what the meter is of, such as a device just switched to.
    pub level: Option<(f32, bool)>,
    pub color: u32,
}

/// `at` keeps its morphing text apart from any other meter's.
pub fn meter(meter: Meter, at: &str, cx: &mut Context<Island>) -> AnyElement {
    let muted = meter.level.is_some_and(|(_, muted)| muted);
    let tint = if muted { OVERLAY2 } else { meter.color };
    let tile = div()
        .relative()
        .size(px(TILE))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(ui::shape(Radius::Control, Some(rgba(with_alpha(tint, ACCENT_TINT_ALPHA)).into())))
        .when_some(meter.glyph.as_deref(), |slot, path| slot.child(icon::glyph(path, GLYPH, tint)));
    let id = |part: &str| SharedString::from(format!("{at}-{part}"));
    let caption = motion::text(id("caption"), meter.caption.clone());
    let label = motion::text(id("label"), meter.label.clone());
    let row = div().w(px(WIDTH)).px(px(4.0)).flex().items_center().gap(px(12.0)).child(tile);
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
                        text.child(div().text_size(px(SMALL_SIZE)).text_color(rgb(SUBTEXT1)).child(caption))
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
        .child(div().min_w(px(0.0)).truncate().font_weight(FontWeight::SEMIBOLD).child(label));
    let amount = div()
        .flex_none()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(if muted { OVERLAY2 } else { TEXT }))
        .child(motion::text(id("amount"), if muted { "Muted".to_owned() } else { format!("{percent}%") }));
    row.child(
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(div().flex().items_baseline().justify_between().gap(px(12.0)).child(heading).child(amount))
            .child(slider(id("slider"), if muted { 0.0 } else { value }, tint, None, cx)),
    )
    .into_any_element()
}
