//! A history drawn as smooth lines, such as CPU use over the last minute.

use gpui::{
    Bounds, ContentMask, Hsla, IntoElement, PathBuilder, PathStyle, Pixels, Point, StrokeOptions, Window, canvas, fill,
    point, prelude::*, px, rgb, rgba, size,
};

use crate::theme::{TINT, with_alpha};

const LINE: f32 = 3.0;
const FILL_ALPHA: u8 = 0x26;
const BASELINE_ALPHA: u8 = 0x1a;

pub struct Series {
    pub values: Vec<f32>,
    pub max: f32,
    pub color: u32,
}

fn at(x: f32, y: f32) -> Point<Pixels> {
    point(px(x), px(y))
}

fn paint_area(window: &mut Window, line: &[(f32, f32)], bottom: f32, color: Hsla) {
    let (Some(first), Some(last)) = (line.first(), line.last()) else {
        return;
    };
    let mut path = PathBuilder::fill();
    path.move_to(at(first.0, bottom));
    for (x, y) in line {
        path.line_to(at(*x, *y));
    }
    path.line_to(at(last.0, bottom));
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

fn paint_line(window: &mut Window, line: &[(f32, f32)], color: Hsla) {
    let Some(first) = line.first() else {
        return;
    };
    // A miter limit of 1 bevels every corner, so a sharp spike doesn't grow a long point.
    let style = StrokeOptions::default().with_line_width(LINE).with_miter_limit(1.0);
    let mut path = PathBuilder::stroke(px(LINE)).with_style(PathStyle::Stroke(style));
    path.move_to(at(first.0, first.1));
    for (x, y) in &line[1..] {
        path.line_to(at(*x, *y));
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

pub fn chart(series: Vec<Series>, slots: usize, phase: f32) -> impl IntoElement {
    let phase = phase.clamp(0.0, 1.0);
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            let pixel = 1.0 / window.scale_factor();
            let x0 = f32::from(bounds.origin.x);
            let y0 = f32::from(bounds.origin.y);
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            let right = x0 + w;
            let bottom = y0 + h;
            let floor = bottom - LINE / 2.0;
            let step = w / slots.saturating_sub(2).max(1) as f32;
            let y = |value: f32, max: f32| {
                floor - (value / max.max(f32::EPSILON)).clamp(0.0, 1.0) * (h - LINE)
            };
            window.paint_quad(fill(
                Bounds {
                    origin: at(x0, bottom - pixel),
                    size: size(px(w), px(pixel)),
                },
                rgba(with_alpha(TINT, BASELINE_ALPHA)),
            ));
            window.with_content_mask(Some(ContentMask { bounds }), |window| {
                for (index, series) in series.iter().enumerate() {
                    let values = &series.values;
                    if values.len() < 2 {
                        continue;
                    }
                    let newest = values.len() - 1;
                    let mut points: Vec<(f32, f32)> = values[..newest]
                        .iter()
                        .enumerate()
                        .map(|(at, value)| {
                            let back = (newest - 1 - at) as f32 + phase;
                            (right - back * step, y(*value, series.max))
                        })
                        .filter(|(x, _)| *x >= x0 - step)
                        .collect();
                    let head = values[newest - 1] + (values[newest] - values[newest - 1]) * phase;
                    points.push((right, y(head, series.max)));
                    let history = points.len();
                    let history_start = points[0].0;
                    if history_start > x0 {
                        let floor = points.iter().map(|(_, level)| *level).fold(f32::MIN, f32::max);
                        points.insert(0, (history_start, floor));
                        points.insert(0, (x0, floor));
                    }
                    if points.len() < 2 {
                        continue;
                    }
                    if index == 0 {
                        paint_area(
                            window,
                            &points[points.len() - history..],
                            bottom,
                            rgba(with_alpha(series.color, FILL_ALPHA)).into(),
                        );
                    }
                    paint_line(window, &points, rgb(series.color).into());
                }
            });
        },
    )
    .size_full()
}
