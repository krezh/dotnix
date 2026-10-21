//! A history drawn as smooth lines, such as CPU use over the last minute; or one stretch of time
//! filling as it passes, such as a quota's window, against a dashed guide.

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
const GUIDE_LINE: f32 = 1.5;
const DASH: f32 = 4.0;
const DASH_GAP: f32 = 4.0;
const HEAD_DOT: f32 = 7.0;

pub struct Series {
    pub values: Vec<f32>,
    pub max: f32,
    pub color: u32,
    pub label: SharedString,
    pub format: fn(f32) -> String,
}

/// A dashed straight line across a window chart, such as an even pace from empty to full.
pub struct Guide {
    pub from: f32,
    pub to: f32,
    pub max: f32,
    pub color: u32,
}

/// How values are spread across the chart.
enum Layout {
    /// The newest at the right edge, scrolling left as `phase` goes from 0 to 1.
    History { phase: f32 },
    /// From the left edge, each sample one of `slots` steps into the window, the newest marked.
    Window { guide: Option<Guide> },
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

/// The value at `fraction` across a window chart of `slots` steps, if it has been reached.
fn window_sample(values: &[f32], slots: usize, fraction: f32) -> Option<f32> {
    if !(0.0..=1.0).contains(&fraction) {
        return None;
    }
    let index = (fraction * slots.saturating_sub(1) as f32).round() as usize;
    values.get(index).copied()
}

fn window_text(series: &[Series], slots: usize, fraction: f32) -> String {
    let mut text = String::new();
    for series in series {
        if let Some(value) = window_sample(&series.values, slots, fraction) {
            if !text.is_empty() {
                text.push('\n');
            }
            let _ = write!(text, "{}: {}", series.label, (series.format)(value));
        }
    }
    text
}

fn paint_dashes(window: &mut Window, from: (f32, f32), to: (f32, f32), color: Hsla) {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length = dx.hypot(dy);
    if length <= 0.0 {
        return;
    }
    let (ux, uy) = (dx / length, dy / length);
    let mut path = PathBuilder::stroke(px(GUIDE_LINE));
    let mut start = 0.0;
    while start < length {
        let end = (start + DASH).min(length);
        path.move_to(at(from.0 + ux * start, from.1 + uy * start));
        path.line_to(at(from.0 + ux * end, from.1 + uy * end));
        start = end + DASH_GAP;
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

pub fn chart(
    id: SharedString,
    series: Vec<Series>,
    slots: usize,
    phase: f32,
    tips: &Tooltips,
) -> impl IntoElement {
    plot(
        id,
        series,
        slots,
        Layout::History {
            phase: phase.clamp(0.0, 1.0),
        },
        tips,
    )
}

/// `slots` is how many samples fill the window; the values so far start at its left edge.
pub fn window(
    id: SharedString,
    series: Vec<Series>,
    slots: usize,
    guide: Option<Guide>,
    tips: &Tooltips,
) -> impl IntoElement {
    plot(id, series, slots, Layout::Window { guide }, tips)
}

fn plot(
    id: SharedString,
    series: Vec<Series>,
    slots: usize,
    layout: Layout,
    tips: &Tooltips,
) -> impl IntoElement {
    let (chart_id, chart_tips) = (id.clone(), tips.clone());
    let plot = canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            let pixel = 1.0 / window.scale_factor();
            let x0 = f32::from(bounds.origin.x);
            let y0 = f32::from(bounds.origin.y);
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            let right = x0 + w;
            let bottom = y0 + h;
            if let Some(at) = chart_tips.position(&chart_id) {
                let text = if bounds.contains(&at) && w > 0.0 {
                    let fraction = (f32::from(at.x) - x0) / w;
                    match layout {
                        Layout::History { phase } => text(&series, slots, phase, fraction),
                        Layout::Window { .. } => window_text(&series, slots, fraction),
                    }
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
                if let Layout::Window {
                    guide: Some(guide), ..
                } = &layout
                {
                    paint_dashes(
                        window,
                        (x0, y(guide.from, guide.max)),
                        (right, y(guide.to, guide.max)),
                        rgb(guide.color).into(),
                    );
                }
                for (index, series) in series.iter().enumerate() {
                    let values = &series.values;
                    if values.len() < 2 {
                        continue;
                    }
                    let (points, history) = match layout {
                        Layout::History { phase } => {
                            history_points(values, series.max, slots, phase, x0, w, &y)
                        }
                        Layout::Window { .. } => {
                            let step = w / slots.saturating_sub(1).max(1) as f32;
                            let points: Vec<(f32, f32)> = values
                                .iter()
                                .enumerate()
                                .map(|(at, value)| (x0 + at as f32 * step, y(*value, series.max)))
                                .collect();
                            let len = points.len();
                            (points, len)
                        }
                    };
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
                    if let (Layout::Window { .. }, Some(&(x, y))) = (&layout, points.last()) {
                        window.paint_quad(
                            fill(
                                Bounds {
                                    origin: at(x - HEAD_DOT / 2.0, y - HEAD_DOT / 2.0),
                                    size: size(px(HEAD_DOT), px(HEAD_DOT)),
                                },
                                rgb(series.color),
                            )
                            .corner_radii(px(HEAD_DOT / 2.0)),
                        );
                    }
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
