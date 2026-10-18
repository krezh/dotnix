//! A live level as a row of segments, such as what a speaker or microphone hears right now.

use gpui::{Div, div, prelude::*, px, rgb, rgba};

use crate::theme::{GREEN, PEACH, RED, TINT, TRACK_ALPHA, YELLOW, with_alpha};

const SEGMENTS: usize = 32;
const HEIGHT: f32 = 4.0;
const GAP: f32 = 2.0;

/// Green, yellow, peach, then red as a level nears full.
pub fn color(level: f32) -> u32 {
    match level {
        level if level < 0.34 => GREEN,
        level if level < 0.67 => YELLOW,
        level if level <= 0.9 => PEACH,
        _ => RED,
    }
}

/// A level as a row of segments, each lit in the color of where it sits, with the recent peak
/// `hold` marked by one lit segment above the level.
pub fn segments(level: f32, hold: f32) -> Div {
    let half = 0.5 / SEGMENTS as f32;
    div()
        .flex()
        .gap(px(GAP))
        .h(px(HEIGHT))
        .children((0..SEGMENTS).map(|index| {
            let at = (index as f32 + 0.5) / SEGMENTS as f32;
            let lit = at <= level || (hold > level && (at - hold).abs() < half);
            let fill = if lit { rgb(color(at)) } else { rgba(with_alpha(TINT, TRACK_ALPHA)) };
            div().flex_1().h_full().rounded(px(1.0)).bg(fill)
        }))
}
