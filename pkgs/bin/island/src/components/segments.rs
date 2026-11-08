//! A live level as a row of segments, such as what a speaker or microphone hears right now.

use crate::theme::{GREEN, PEACH, RED, YELLOW};

/// Green, yellow, peach, then red as a level nears full.
pub fn color(level: f32) -> u32 {
    match level {
        level if level < 0.34 => GREEN,
        level if level < 0.67 => YELLOW,
        level if level <= 0.9 => PEACH,
        _ => RED,
    }
}
