//! A short state in a pill, such as "Online", tinted by what it means; the energy beam runs round it
//! while work it stands for is on its way, such as a scan.

use gpui::{AnyElement, FontWeight, SharedString, div, prelude::*, px, rgb, rgba};

use crate::{
    components::{energy, motion},
    theme::{SUBTEXT1, TINT, TRACK_ALPHA, with_alpha},
    ui::{self, Radius},
};

const H: f32 = 22.0;
const PAD: f32 = 9.0;
const DOT: f32 = 6.0;
const TEXT: f32 = 12.0;
/// The fill under a tinted pill's text.
const TINT_ALPHA: u8 = 0x22;

/// With a color, a dot and text in it on a faint fill of it; without, quiet text on the track.
pub fn pill(
    id: SharedString,
    text: SharedString,
    color: Option<u32>,
    busy: bool,
) -> AnyElement {
    let fill = match color {
        Some(color) => rgba(with_alpha(color, TINT_ALPHA)),
        None => rgba(with_alpha(TINT, TRACK_ALPHA)),
    };
    div()
        .id(id.clone())
        .relative()
        .flex_none()
        .h(px(H))
        .px(px(PAD))
        .flex()
        .items_center()
        .gap(px(6.0))
        .text_size(px(TEXT))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(color.unwrap_or(SUBTEXT1)))
        .child(ui::shape(Radius::Pill, Some(fill.into())))
        .when_some(color, |pill, color| {
            pill.child(div().size(px(DOT)).rounded_full().bg(rgb(color)))
        })
        .child(motion::text(SharedString::from(format!("{id}-text")), text))
        .child(energy::border("energy", Radius::Pill, 0.0, busy))
        .into_any_element()
}
