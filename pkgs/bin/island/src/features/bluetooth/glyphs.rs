//! Which glyph stands for Bluetooth and for each kind of device.

use crate::features::GlyphSpec;

pub const GLYPHS: &[GlyphSpec] = &[
    ("tab-bluetooth", &["bluetooth"]),
    ("bt-on", &["bluetooth"]),
    ("bt-off", &["bluetooth-off"]),
    ("bt-scan", &["refresh-cw"]),
    ("bt-visible", &["eye"]),
    ("bt-more", &["ellipsis"]),
    ("bt-trust", &["shield-check"]),
    ("bt-forget", &["trash-2"]),
    ("bt-headphones", &["headphones"]),
    ("bt-speaker", &["speaker"]),
    ("bt-gamepad", &["gamepad-2"]),
    ("bt-keyboard", &["keyboard"]),
    ("bt-mouse", &["mouse"]),
    ("bt-tablet", &["tablet"]),
    ("bt-phone", &["smartphone"]),
    ("bt-computer", &["laptop"]),
    ("bt-display", &["monitor"]),
    ("bt-printer", &["printer"]),
    ("bt-player", &["disc-3"]),
    ("bt-device", &["bluetooth"]),
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

/// A battery's color, on the scale every battery uses.
pub fn battery(level: Option<u8>) -> u32 {
    level.map_or(crate::theme::OVERLAY2, crate::features::battery::level_color)
}
