//! A small picture standing for something in a row: a glyph, an app's icon, a picture, a letter,
//! the mark a choice shows (a check box or a radio dot), or a mascot.

use std::path::PathBuf;

use gpui::{
    AnimationExt, AnyElement, FontWeight, Hsla, ObjectFit, PathBuilder, PathStyle, Rgba,
    SharedString, SpringAnimation, StrokeOptions, canvas, div, point, prelude::*, px, rgb, rgba,
};

use crate::{
    components::{
        icon::{self, Picture},
        mascot::{self, Pose},
    },
    island::CONTENT_IN,
    theme::{ACCENT, ON_ACCENT, OVERLAY1, with_alpha},
    ui::{self, CONTROL_RADIUS, Radius},
};

/// A choice's outline while it is not picked.
const CHOICE_EDGE_ALPHA: u8 = 0xb3;
/// The tick's stroke, as a part of the box's side.
const TICK_WIDTH: f32 = 0.14;

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
    /// Whether an option is picked: a check box when `many` may be, a radio dot when only one.
    /// `key` names its animation, so it fills and empties in place.
    Choice {
        key: SharedString,
        picked: bool,
        many: bool,
    },
    /// A character as it stands now.
    Mascot(Pose),
}

fn blend(from: Rgba, to: Rgba, t: f32) -> Hsla {
    let at = |a: f32, b: f32| a + (b - a) * t;
    Rgba {
        r: at(from.r, to.r),
        g: at(from.g, to.g),
        b: at(from.b, to.b),
        a: at(from.a, to.a),
    }
    .into()
}

/// A tick across a box of `side`, popping in as `t` goes from 0 to 1.
fn tick(side: f32, t: f32) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            if t < 0.05 {
                return;
            }
            let (x0, y0) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
            let scale = 0.6 + 0.4 * t;
            let at = |x: f32, y: f32| {
                let (cx, cy) = (0.5 + (x - 0.5) * scale, 0.5 + (y - 0.5) * scale);
                point(px(x0 + cx * side), px(y0 + cy * side))
            };
            let width = (side * TICK_WIDTH).max(1.5);
            let style = StrokeOptions::default()
                .with_line_width(width)
                .with_miter_limit(1.0);
            let mut path = PathBuilder::stroke(px(width)).with_style(PathStyle::Stroke(style));
            path.move_to(at(0.27, 0.52));
            path.line_to(at(0.43, 0.68));
            path.line_to(at(0.74, 0.35));
            if let Ok(path) = path.build() {
                window.paint_path(path, Hsla::from(rgb(ON_ACCENT)).opacity(t));
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

fn choice(key: SharedString, picked: bool, many: bool, side: f32) -> AnyElement {
    let radius = if many { Radius::Badge } else { Radius::Pill };
    let accent = rgb(ACCENT);
    let empty = rgba(with_alpha(ACCENT, 0));
    let edge = rgba(with_alpha(OVERLAY1, CHOICE_EDGE_ALPHA));
    div()
        .relative()
        .size(px(side))
        .flex_none()
        .with_spring(
            key,
            SpringAnimation::new(CONTENT_IN).to(picked),
            move |mark, phase| {
                let t = phase.0.clamp(0.0, 1.0);
                let outline = div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .child(ui::outlined(radius, blend(empty, accent, t), blend(edge, accent, t)));
                let inside = if many {
                    tick(side, t).into_any_element()
                } else {
                    let dot = (side * 0.4 * t).max(0.0);
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .relative()
                                .size(px(dot))
                                .child(ui::shape(Radius::Pill, Some(rgb(ON_ACCENT).into()))),
                        )
                        .into_any_element()
                };
                mark.child(outline).child(inside)
            },
        )
        .into_any_element()
}

/// In a square of `size`, filling `fill` of it; glyphs sit smaller, as they are drawn edge to
/// edge.
pub fn mark(mark: &Mark, size: f32, fill: f32) -> AnyElement {
    let picture = (size * fill).round();
    match mark {
        Mark::Glyph(path, color) => icon::glyph(path, (size * 0.56).round(), *color),
        Mark::Image(path) => icon::image(path, picture),
        Mark::Picture(source, true) => {
            icon::rounded(source, picture, ObjectFit::Cover, CONTROL_RADIUS)
        }
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
            .child(
                text.chars()
                    .next()
                    .map(|first| first.to_uppercase().to_string())
                    .unwrap_or_default(),
            )
            .into_any_element(),
        Mark::Choice { key, picked, many } => {
            choice(key.clone(), *picked, *many, (picture * 0.9).round())
        }
        Mark::Mascot(pose) => mascot::mascot(pose.clone(), size).into_any_element(),
    }
}
