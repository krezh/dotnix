//! A history drawn as smooth lines, such as CPU use over the last minute.

use std::{cell::RefCell, collections::HashMap, fmt::Write, time::Instant};

use gpui::{
    Bounds, ContentMask, Hsla, IntoElement, PathBuilder, PathStyle, Pixels, Point, SharedString,
    StrokeOptions, Window, canvas, div, fill, linear_color_stop, linear_gradient, point,
    prelude::*, px, rgb, rgba, size,
};

use crate::{
    components::tooltip::Tooltips,
    theme::{BASE, TINT, with_alpha},
};

const LINE: f32 = 1.75;
/// A wider, faint stroke under the line, so it glows.
const GLOW: f32 = 5.0;
const GLOW_ALPHA: u8 = 0x2e;
/// The fill under a line at its top, fading to nothing at the foot.
const FILL_ALPHA: u8 = 0x40;
const GRID_ALPHA: u8 = 0x0f;
const GRID_LINES: usize = 3;
/// Room above the highest value and below the lowest, so the line and its dot are never cut.
const PAD_TOP: f32 = 6.0;
const PAD_BOTTOM: f32 = 3.0;
const DOT: f32 = 3.0;
const DOT_RING: f32 = 1.5;
/// How far the newest value's pulse grows past its dot, and how strong it starts.
const PULSE: f32 = 7.0;
const PULSE_ALPHA: f32 = 0.24;

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

/// Through each point, leaving and arriving level halfway between them, so the line curves
/// smoothly and never overshoots a value.
fn smooth(path: &mut PathBuilder, line: &[(f32, f32)]) {
    for pair in line.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        let mid = (x0 + x1) / 2.0;
        path.cubic_bezier_to(at(x1, y1), at(mid, y0), at(mid, y1));
    }
}

fn paint_area(window: &mut Window, line: &[(f32, f32)], bottom: f32, color: u32) {
    let (Some(first), Some(last)) = (line.first(), line.last()) else {
        return;
    };
    let mut path = PathBuilder::fill();
    path.move_to(at(first.0, bottom));
    path.line_to(at(first.0, first.1));
    smooth(&mut path, line);
    path.line_to(at(last.0, bottom));
    path.close();
    if let Ok(path) = path.build() {
        let top: Hsla = rgba(with_alpha(color, FILL_ALPHA)).into();
        let foot: Hsla = rgba(with_alpha(color, 0)).into();
        window.paint_path(
            path,
            linear_gradient(180.0, linear_color_stop(top, 0.0), linear_color_stop(foot, 1.0)),
        );
    }
}

fn paint_line(window: &mut Window, line: &[(f32, f32)], width: f32, color: Hsla) {
    let Some(first) = line.first() else {
        return;
    };
    // A miter limit of 1 bevels every corner, so a sharp spike doesn't grow a long point.
    let style = StrokeOptions::default()
        .with_line_width(width)
        .with_miter_limit(1.0);
    let mut path = PathBuilder::stroke(px(width)).with_style(PathStyle::Stroke(style));
    path.move_to(at(first.0, first.1));
    smooth(&mut path, line);
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

fn circle(window: &mut Window, (x, y): (f32, f32), radius: f32, color: Hsla) {
    window.paint_quad(
        fill(
            Bounds {
                origin: at(x - radius, y - radius),
                size: size(px(2.0 * radius), px(2.0 * radius)),
            },
            color,
        )
        .corner_radii(px(radius)),
    );
}

/// The newest value: a dot ringed in the panel's tone, with a pulse growing out of it and
/// fading as the next value scrolls in.
fn paint_head(window: &mut Window, head: (f32, f32), color: u32, phase: f32) {
    let pulse = rgba(with_alpha(color, (PULSE_ALPHA * (1.0 - phase) * 255.0) as u8));
    circle(window, head, DOT + PULSE * phase, pulse.into());
    circle(window, head, DOT + DOT_RING, rgb(BASE).into());
    circle(window, head, DOT, rgb(color).into());
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

/// How long a chart's scale takes to settle most of the way to a new peak.
const SCALE_EASE: f32 = 0.35;

thread_local! {
    /// Each chart's scale as drawn, by chart and series, and when it was last moved.
    static SCALES: RefCell<HashMap<(SharedString, usize), (f32, Instant)>> = RefCell::default();
}

/// The scale to draw `series` of chart `id` at: easing towards `target`, so a new peak grows or
/// shrinks the chart smoothly rather than jumping it.
fn eased_scale(id: &SharedString, series: usize, target: f32) -> f32 {
    SCALES.with_borrow_mut(|scales| {
        let now = Instant::now();
        let (scale, at) = scales
            .entry((id.clone(), series))
            .or_insert((target, now));
        let dt = now.duration_since(*at).as_secs_f32();
        *scale += (target - *scale) * (1.0 - (-dt / SCALE_EASE).exp());
        *at = now;
        *scale
    })
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
            let floor = bottom - PAD_BOTTOM;
            let y = |value: f32, max: f32| {
                floor
                    - (value / max.max(f32::EPSILON)).clamp(0.0, 1.0)
                        * (h - PAD_TOP - PAD_BOTTOM)
            };
            for line in 1..=GRID_LINES {
                let at_y = y0 + (h * line as f32 / (GRID_LINES + 1) as f32).round();
                window.paint_quad(fill(
                    Bounds {
                        origin: at(x0, at_y),
                        size: size(px(w), px(pixel)),
                    },
                    rgba(with_alpha(TINT, GRID_ALPHA)),
                ));
            }
            // The first series is drawn last, so it sits over the others.
            let mut heads = Vec::new();
            window.with_content_mask(Some(ContentMask { bounds }), |window| {
                for (index, series) in series.iter().enumerate().rev() {
                    let values = &series.values;
                    if values.len() < 2 {
                        continue;
                    }
                    let max = eased_scale(&chart_id, index, series.max);
                    let (points, history) = history_points(values, max, slots, phase, x0, w, &y);
                    if points.len() < 2 {
                        continue;
                    }
                    paint_area(window, &points[points.len() - history..], bottom, series.color);
                    paint_line(
                        window,
                        &points,
                        GLOW,
                        rgba(with_alpha(series.color, GLOW_ALPHA)).into(),
                    );
                    paint_line(window, &points, LINE, rgb(series.color).into());
                    if let Some(head) = points.last() {
                        heads.push((*head, series.color));
                    }
                }
            });
            // Drawn past the mask, so the dot at the edge stays round.
            for (head, color) in heads {
                paint_head(window, (head.0 - DOT - DOT_RING, head.1), color, phase);
            }
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
