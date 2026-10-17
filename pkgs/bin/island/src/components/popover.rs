use gpui::{Div, div, prelude::*, px};

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
        .child(div().absolute().top_0().left_0().size_full().child(ui::shadow(radius, power, SHADOW_SPREAD)))
        .child(shape::background(
            radius,
            power,
            ui::surface_fill(POPOVER_OPACITY),
            Some(ui::surface_border()),
        ))
}
