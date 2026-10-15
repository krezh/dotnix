use std::sync::Arc;

use gpui::{Canvas, Div, FontFeatures, FontWeight, Hsla, Rgba, div, hsla, prelude::*, px, rgb, rgba};

use crate::{
    squircle,
    theme::{
        FONT, HOVER_ALPHA, SELECTED_ALPHA, SHADOW_ALPHA, SURFACE, SURFACE_BORDER_ALPHA, SURFACE1, TEXT, TINT,
        with_alpha,
    },
};

pub const INSET: f32 = 24.0;
pub const TIGHT_INSET: f32 = 10.0;
pub const HEADER_H: f32 = 32.0;
pub const HEADER_GAP: f32 = 16.0;
pub const CONTROL_H: f32 = 30.0;
pub const BLOCK_GAP: f32 = 12.0;
pub const CARD_PAD: f32 = 16.0;
pub const TEXT_SIZE: f32 = 14.0;
pub const SMALL_SIZE: f32 = 13.0;
pub const TITLE_SIZE: f32 = 16.0;
pub const HEADING_SIZE: f32 = 18.0;

#[derive(Clone, Copy)]
pub enum Radius {
    Panel,
    Tooltip,
    Control,
    Row,
    Pill,
}

pub const PANEL_RADIUS: f32 = 16.0;
pub const CONTROL_RADIUS: f32 = 10.0;
const CIRCULAR: f32 = 2.0;

impl Radius {
    pub fn geometry(self) -> (f32, f32) {
        let radius = match self {
            Radius::Panel => PANEL_RADIUS,
            Radius::Tooltip => 10.0,
            Radius::Control => CONTROL_RADIUS,
            Radius::Row => 8.0,
            Radius::Pill => 1.0e4,
        };
        (radius, CIRCULAR)
    }
}

pub fn shape(radius: Radius, rest: Option<Hsla>, hover: Option<Hsla>) -> Canvas<()> {
    let (radius, power) = radius.geometry();
    squircle::fill(radius, power, rest, hover)
}

pub fn inset_shape(radius: Radius, inset: f32, hover: Option<Hsla>) -> Canvas<()> {
    let (radius, power) = radius.geometry();
    squircle::inset_fill(radius, power, inset, None, hover)
}

pub fn outlined(radius: Radius, fill: Hsla, border: Hsla) -> Canvas<()> {
    let (radius, power) = radius.geometry();
    squircle::background(radius, power, fill, Some(border))
}

pub fn surface_fill(opacity: f32) -> Hsla {
    Hsla::from(rgb(SURFACE)).opacity(opacity)
}

pub fn surface_border() -> Hsla {
    rgba(with_alpha(SURFACE1, SURFACE_BORDER_ALPHA)).into()
}

pub fn shadow(radius: f32, power: f32, spread: f32) -> Canvas<()> {
    squircle::outer_shadow(radius, power, hsla(0.0, 0.0, 0.0, SHADOW_ALPHA), spread)
}

pub fn hover_fill() -> Rgba {
    rgba(with_alpha(TINT, HOVER_ALPHA))
}

pub fn selected_fill() -> Rgba {
    rgba(with_alpha(TINT, SELECTED_ALPHA))
}

pub fn base() -> Div {
    div()
        .font_family(FONT)
        .font_weight(FontWeight::MEDIUM)
        .font_features(FontFeatures(Arc::new(vec![("tnum".to_owned(), 1)])))
        .text_size(px(TEXT_SIZE))
        .text_color(rgb(TEXT))
}
