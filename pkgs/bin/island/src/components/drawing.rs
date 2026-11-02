//! Paints a [`Drawing`] in its square.

use gpui::{
    AnyElement, Bounds, Hsla, PathBuilder, PathStyle, Pixels, Point, Rgba, StrokeOptions, Window,
    canvas, div, point, prelude::*, px, rgb,
};

use crate::drawing::{Color, Drawing, Shape};

fn hsla([r, g, b, a]: Color) -> Hsla {
    Rgba { r, g, b, a }.into()
}

fn placed(bounds: &Bounds<Pixels>, points: &[(f32, f32)]) -> Vec<Point<Pixels>> {
    let (x0, y0) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    points
        .iter()
        .map(|(x, y)| point(px(x0 + x), px(y0 + y)))
        .collect()
}

fn fill(window: &mut Window, points: &[Point<Pixels>], color: Hsla) {
    let Some((first, rest)) = points.split_first() else {
        return;
    };
    let mut path = PathBuilder::fill();
    path.move_to(*first);
    for at in rest {
        path.line_to(*at);
    }
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

fn stroke(window: &mut Window, points: &[Point<Pixels>], width: f32, color: Hsla) {
    let Some((first, rest)) = points.split_first() else {
        return;
    };
    let width = width.max(1.0);
    let style = StrokeOptions::default()
        .with_line_width(width)
        .with_miter_limit(1.0);
    let mut path = PathBuilder::stroke(px(width)).with_style(PathStyle::Stroke(style));
    path.move_to(*first);
    for at in rest {
        path.line_to(*at);
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

pub fn drawing(drawing: Drawing) -> AnyElement {
    let Drawing { side, shapes } = drawing;
    let (circles, paths): (Vec<_>, Vec<_>) = shapes
        .into_iter()
        .partition(|shape| matches!(shape, Shape::Circle { .. }));
    let circles = circles.into_iter().filter_map(|shape| match shape {
        Shape::Circle {
            left,
            top,
            size,
            color,
        } => Some(
            div()
                .absolute()
                .left(px(left))
                .top(px(top))
                .size(px(size))
                .rounded_full()
                .bg(rgb(color)),
        ),
        _ => None,
    });
    div()
        .relative()
        .flex_none()
        .size(px(side))
        .when(!paths.is_empty(), |square| {
            square.child(
                canvas(
                    |_, _, _| {},
                    move |bounds, (), window, _| {
                        for shape in &paths {
                            match shape {
                                Shape::Fill { points, color } => {
                                    fill(window, &placed(&bounds, points), hsla(*color));
                                }
                                Shape::Stroke {
                                    points,
                                    width,
                                    color,
                                } => {
                                    stroke(window, &placed(&bounds, points), *width, hsla(*color));
                                }
                                Shape::Circle { .. } => {}
                            }
                        }
                    },
                )
                .size_full(),
            )
        })
        .children(circles)
        .into_any_element()
}
