//! A history drawn as smooth lines, such as CPU use over the last minute.

use std::{
    fmt::Write,
    time::{Duration, Instant},
};

use crate::panel::SharedString;

/// How often a history charted here gets a new sample. Whatever keeps a history samples at this
/// pace unless it has reason for its own, and then charts it with its own phase.
pub const SAMPLE_EVERY: Duration = Duration::from_secs(2);

/// The clock of a history sampled on its own: when it last grew, and how often it does.
#[derive(Clone, Copy)]
pub struct Phase {
    pub sampled: Option<Instant>,
    pub every: Duration,
}

impl Phase {
    /// How far it has scrolled since it last grew, 0 to 1.
    pub fn now(self) -> f32 {
        self.sampled.map_or(1.0, |sampled| {
            (sampled.elapsed().as_secs_f32() / self.every.as_secs_f32()).min(1.0)
        })
    }
}

pub const LINE: f32 = 3.0;
pub const FILL_ALPHA: u8 = 0x26;
pub const BASELINE_ALPHA: u8 = 0x1a;

#[derive(Clone)]
pub struct Series {
    pub values: Vec<f32>,
    pub max: f32,
    pub color: u32,
    pub label: SharedString,
    pub format: fn(f32) -> String,
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

pub fn text(series: &[Series], slots: usize, phase: f32, fraction: f32) -> String {
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

/// Points scrolling in from the right, and how many of them are real history rather than the
/// flat lead-in drawn where there is none yet.
pub fn history_points(
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
