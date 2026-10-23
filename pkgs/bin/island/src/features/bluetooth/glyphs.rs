//! Which glyph stands for Bluetooth and for each kind of device.

use crate::features::GlyphSpec;

pub const GLYPHS: &[GlyphSpec] = &[
    ("tab-bluetooth", &["bluetooth-active-symbolic", "bluetooth-symbolic"]),
    ("bt-on", &["bluetooth-active-symbolic", "bluetooth-symbolic"]),
    ("bt-off", &["bluetooth-disabled-symbolic", "bluetooth-symbolic"]),
    ("bt-scan", &["view-refresh-symbolic"]),
    ("bt-visible", &["view-reveal-symbolic", "view-visible-symbolic"]),
    ("bt-more", &["view-more-horizontal-symbolic", "view-more-symbolic"]),
    ("bt-trust", &["security-high-symbolic"]),
    ("bt-forget", &["edit-delete-symbolic", "user-trash-symbolic"]),
    ("bt-headphones", &["audio-headphones-symbolic", "audio-headset-symbolic"]),
    ("bt-speaker", &["audio-speakers-symbolic", "audio-card-symbolic"]),
    ("bt-gamepad", &["input-gaming-symbolic"]),
    ("bt-keyboard", &["input-keyboard-symbolic"]),
    ("bt-mouse", &["input-mouse-symbolic"]),
    ("bt-tablet", &["input-tablet-symbolic"]),
    ("bt-phone", &["phone-symbolic", "smartphone-symbolic"]),
    ("bt-computer", &["computer-symbolic"]),
    ("bt-display", &["video-display-symbolic"]),
    ("bt-printer", &["printer-symbolic"]),
    ("bt-player", &["multimedia-player-symbolic"]),
    ("bt-device", &["bluetooth-active-symbolic", "bluetooth-symbolic"]),
];

/// The glyph for a device, from BlueZ's icon name.
pub fn device(icon: &str) -> &'static str {
    match icon {
        "audio-headset" | "audio-headphones" => "bt-headphones",
        "audio-card" => "bt-speaker",
        "input-gaming" => "bt-gamepad",
        "input-keyboard" => "bt-keyboard",
        "input-mouse" => "bt-mouse",
        "input-tablet" => "bt-tablet",
        "phone" => "bt-phone",
        "computer" => "bt-computer",
        "video-display" => "bt-display",
        "printer" => "bt-printer",
        "multimedia-player" => "bt-player",
        _ => "bt-device",
    }
}

/// A battery's color: green while there is plenty, then yellow, then peach.
pub fn battery(level: Option<u8>) -> u32 {
    use crate::theme::{GREEN, OVERLAY2, PEACH, YELLOW};
    match level {
        Some(level) if level < 25 => PEACH,
        Some(level) if level < 50 => YELLOW,
        Some(_) => GREEN,
        None => OVERLAY2,
    }
}

#[cfg(test)]
#[path = "glyphs_test.rs"]
mod tests;
