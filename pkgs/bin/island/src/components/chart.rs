use gpui::{Bounds, Hsla, IntoElement, Window, canvas, fill, point, prelude::*, px, rgb, rgba, size};

use crate::theme::{TINT, with_alpha};

const LINE: f32 = 3.0;
const FILL_ALPHA: u8 = 0x26;
const BASELINE_ALPHA: u8 = 0x1a;

pub struct Series {
    pub values: Vec<f32>,
    pub max: f32,
    pub color: u32,
}

fn column(window: &mut Window, x: f32, width: f32, top: f32, bottom: f32, color: Hsla) {
    if bottom > top {
        window.paint_quad(fill(
            Bounds {
                origin: point(px(x), px(top)),
                size: size(px(width), px(bottom - top)),
            },
            color,
        ));
    }
}

pub fn chart(series: Vec<Series>, slots: usize, phase: f32) -> impl IntoElement {
    let phase = phase.clamp(0.0, 1.0);
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            let scale = window.scale_factor();
            let pixel = 1.0 / scale;
            let x0 = f32::from(bounds.origin.x);
            let y0 = f32::from(bounds.origin.y);
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            let right = x0 + w;
            let floor = y0 + h - LINE / 2.0;
            let step = w / slots.saturating_sub(2).max(1) as f32;
            let y = |value: f32, max: f32| {
                floor - (value / max.max(f32::EPSILON)).clamp(0.0, 1.0) * (h - LINE)
            };
            column(window, x0, w, y0 + h - pixel, y0 + h, rgba(with_alpha(TINT, BASELINE_ALPHA)).into());
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
                let history_start = points[0].0;
                if history_start > x0 {
                    let floor = points.iter().map(|(_, level)| *level).fold(f32::MIN, f32::max);
                    points.insert(0, (history_start, floor));
                    points.insert(0, (x0, floor));
                }
                if points.len() < 2 {
                    continue;
                }
                let color: Hsla = rgb(series.color).into();
                let area: Hsla = rgba(with_alpha(series.color, FILL_ALPHA)).into();
                let at = |x: f32, segment: usize| {
                    let (ax, ay) = points[segment];
                    let (bx, by) = points[segment + 1];
                    let t = if bx > ax { ((x - ax) / (bx - ax)).clamp(0.0, 1.0) } else { 1.0 };
                    ay + (by - ay) * t
                };
                let start = ((points[0].0.max(x0)) * scale).floor() as i64;
                let end = (right * scale).ceil() as i64;
                let mut segment = 0;
                for device in start..end {
                    let left = (device as f32 * pixel).max(points[0].0);
                    let edge = ((device + 1) as f32 * pixel).min(right);
                    if edge <= left {
                        continue;
                    }
                    while segment + 2 < points.len() && points[segment + 1].0 <= left {
                        segment += 1;
                    }
                    let first = at(left, segment);
                    let (mut high, mut low) = (first, first);
                    let mut area_sum = 0.0;
                    let mut cursor = (left, first);
                    let mut walk = segment;
                    loop {
                        let stop = points[walk + 1].0.min(edge);
                        let value = at(stop, walk);
                        area_sum += (cursor.1 + value) / 2.0 * (stop - cursor.0);
                        high = high.min(value);
                        low = low.max(value);
                        cursor = (stop, value);
                        if stop >= edge || walk + 2 >= points.len() {
                            break;
                        }
                        walk += 1;
                    }
                    let width = edge - left;
                    if index == 0 && left >= history_start {
                        column(window, left, width, area_sum / (cursor.0 - left).max(f32::EPSILON), y0 + h, area);
                    }
                    column(window, left, width, high - LINE / 2.0, low + LINE / 2.0, color);
                }
            }
        },
    )
    .size_full()
}
