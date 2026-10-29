//! The icons audio draws, and which one stands for what.

use super::state::{Device, Form};
use crate::features::GlyphSpec;

pub const GLYPHS: &[GlyphSpec] = &[
    ("tab-audio", &["volume-2"]),
    ("volume-high", &["volume-2"]),
    ("volume-medium", &["volume-1"]),
    ("volume-low", &["volume"]),
    ("volume-muted", &["volume-x"]),
    ("mic", &["mic"]),
    ("mic-muted", &["mic-off"]),
    ("device-headset", &["headset"]),
    ("device-speakers", &["speaker"]),
    ("device-card", &["audio-lines"]),
];

pub fn speaker(volume: f32, muted: bool) -> &'static str {
    match volume {
        _ if muted || volume <= 0.0 => "volume-muted",
        volume if volume < 0.34 => "volume-low",
        volume if volume < 0.67 => "volume-medium",
        _ => "volume-high",
    }
}

pub fn microphone(muted: bool) -> &'static str {
    if muted { "mic-muted" } else { "mic" }
}

/// Whether a device names a word of the headset's name, such as "A50 Chat" of "Logitech ASTRO A50
/// Gen 5": headsets often report no form of their own.
fn of_headset(description: &str, headset: &str) -> bool {
    let words: Vec<&str> = headset.split_whitespace().collect();
    description
        .split_whitespace()
        .any(|word| word.len() >= 3 && words.iter().any(|name| name.eq_ignore_ascii_case(word)))
}

pub fn device(device: &Device, headset: Option<&str>) -> &'static str {
    let form = match device.form {
        Form::Card if headset.is_some_and(|name| of_headset(&device.description, name)) => {
            Form::Headset
        }
        form => form,
    };
    match form {
        Form::Headset => "device-headset",
        Form::Speakers => "device-speakers",
        Form::Microphone => "mic",
        Form::Card => "device-card",
    }
}

#[cfg(test)]
#[path = "glyphs_test.rs"]
mod tests;
