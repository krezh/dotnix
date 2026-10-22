//! A history drawn as smooth lines, such as CPU use over the last minute.

use std::fmt::Write;

use gpui::{
    Bounds, ContentMask, Hsla, IntoElement, PathBuilder, PathStyle, Pixels, Point, SharedString,
    StrokeOptions, Window, canvas, div, fill, point, prelude::*, px, rgb, rgba, size,
};

use crate::{
    components::tooltip::Tooltips,
    theme::{TINT, with_alpha},
};

const LINE: f32 = 3.0;
const FILL_ALPHA: u8 = 0x26;
const BASELINE_ALPHA: u8 = 0x1a;

pub struct Series {
    pub values: Vec<f32>,
    pub max: f32,
    pub color: u32,
    pub label: SharedString,
    pub format: fn(f32) -> String,
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
    let style = StrokeOptions::default()
        .with_line_width(LINE)
        .with_miter_limit(1.0);
    let mut path = PathBuilder::stroke(px(LINE)).with_style(PathStyle::Stroke(style));
    path.move_to(at(first.0, first.1));
    for (x, y) in &line[1..] {
        path.line_to(at(*x, *y));
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

fn sample(values: &[f32], slots: usize, phase: f32, fraction: f32) -> Option<f32> {
    if values.len() < 2 || !(0.0..=1.0).contains(&fraction) {
        return None;
    }
    let newest = values.len() - 1;
    // Invert the scrolling vertices below, including the interpolated head at the right edge.
    let back = (1.0 - fraction) * slots.saturating_sub(2).max(1) as f32;
    if back > (newest - 1) as f32 + phase {
        return None;
    }
    if back <= phase / 2.0 {
        return Some(values[newest - 1] + (values[newest] - values[newest - 1]) * phase);
    }
    let index = newest - 1 - (back - phase).round().max(0.0) as usize;
    values.get(index).copied()
}

fn text(series: &[Series], slots: usize, phase: f32, fraction: f32) -> String {
    let mut text = String::new();
    for series in series {
        if let Some(value) = sample(&series.values, slots, phase, fraction) {
            if !text.is_empty() {
                text.push('\n');
            }
            let _ = write!(text, "{}: {}", series.label, (series.format)(value));
        }
    }
    text
}

/// The newest value at the right edge, scrolling left as `phase` goes from 0 to 1.
pub fn chart(
    id: SharedString,
    series: Vec<Series>,
    slots: usize,
    phase: f32,
    tips: &Tooltips,
) -> impl IntoElement {
    let phase = phase.clamp(0.0, 1.0);
    let (chart_id, chart_tips) = (id.clone(), tips.clone());
    let plot = canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            let pixel = 1.0 / window.scale_factor();
            let x0 = f32::from(bounds.origin.x);
            let y0 = f32::from(bounds.origin.y);
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            let bottom = y0 + h;
            if let Some(at) = chart_tips.position(&chart_id) {
                let text = if bounds.contains(&at) && w > 0.0 {
                    text(&series, slots, phase, (f32::from(at.x) - x0) / w)
                } else {
                    String::new()
                };
                chart_tips.update(&chart_id, text, window);
            }
            let floor = bottom - LINE / 2.0;
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
                    let (points, history) =
                        history_points(values, series.max, slots, phase, x0, w, &y);
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
    .size_full();
    tips.track(div().id(id.clone()).size_full().child(plot), id)
}

/// Points scrolling in from the right, and how many of them are real history rather than the
/// flat lead-in drawn where there is none yet.
fn history_points(
    values: &[f32],
    max: f32,
    slots: usize,
    phase: f32,
    x0: f32,
    w: f32,
    y: &impl Fn(f32, f32) -> f32,
) -> (Vec<(f32, f32)>, usize) {
    let right = x0 + w;
    let step = w / slots.saturating_sub(2).max(1) as f32;
    let newest = values.len() - 1;
    let mut points: Vec<(f32, f32)> = values[..newest]
        .iter()
        .enumerate()
        .map(|(at, value)| {
            let back = (newest - 1 - at) as f32 + phase;
            (right - back * step, y(*value, max))
        })
        .filter(|(x, _)| *x >= x0 - step)
        .collect();
    let head = values[newest - 1] + (values[newest] - values[newest - 1]) * phase;
    points.push((right, y(head, max)));
    let history = points.len();
    let history_start = points[0].0;
    if history_start > x0 {
        let floor = points
            .iter()
            .map(|(_, level)| *level)
            .fold(f32::MIN, f32::max);
        points.insert(0, (history_start, floor));
        points.insert(0, (x0, floor));
    }
    (points, history)
}

#[cfg(test)]
#[path = "chart_test.rs"]
mod tests;
