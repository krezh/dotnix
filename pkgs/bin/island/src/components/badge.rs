//! A small picture standing for something in a row: a glyph, an app's icon, a picture, or a
//! letter.

use std::path::PathBuf;

use gpui::{AnyElement, FontWeight, ObjectFit, SharedString, div, prelude::*, px, rgb};

use crate::{
    components::icon::{self, Picture},
    theme::ON_ACCENT,
    ui::{self, CONTROL_RADIUS, Radius},
};

#[derive(Clone)]
pub enum Mark {
    /// A symbolic icon in the color.
    Glyph(PathBuf, u32),
    /// A full-color icon, such as an app's.
    Image(PathBuf),
    /// A picture that may be in memory rather than a file: cropped to the square with rounded
    /// corners, such as a photo, or fitted whole, such as an icon.
    Picture(Picture, bool),
    /// The first letter of the text, on the color, for something without an icon.
    Letter(SharedString, u32),
}

/// In a square of `size`, filling `fill` of it; glyphs sit smaller, as they are drawn edge to
/// edge.
pub fn mark(mark: &Mark, size: f32, fill: f32) -> AnyElement {
    let picture = (size * fill).round();
    match mark {
        Mark::Glyph(path, color) => icon::glyph(path, (size * 0.56).round(), *color),
        Mark::Image(path) => icon::image(path, picture),
        Mark::Picture(source, true) => icon::rounded(source, picture, ObjectFit::Cover, CONTROL_RADIUS),
        Mark::Picture(source, false) => icon::picture(source, picture, ObjectFit::Contain),
        Mark::Letter(text, color) => div()
            .relative()
            .size(px(picture))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(ui::shape(Radius::Control, Some(rgb(*color).into())))
            .text_size(px((picture * 0.45).round()))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(ON_ACCENT))
            .child(text.chars().next().map(|first| first.to_uppercase().to_string()).unwrap_or_default())
            .into_any_element(),
    }
}
