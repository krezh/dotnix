//! A battery a feature knows of, such as a mouse's: the one reading its bar gauge and the
//! dashboard's Batteries card are both drawn from, with one color scale and one idea of low.

use gpui::SharedString;

use crate::theme::{CRITICAL, GREEN, PEACH, RED, TEXT, YELLOW};

/// At or below this, a battery not charging is worth a warning.
pub const LOW: u8 = 20;

#[derive(Clone, Debug, PartialEq)]
pub struct Battery {
    /// Names it across frames, such as a device's address.
    pub id: SharedString,
    /// The key of the glyph for what it powers.
    pub glyph: &'static str,
    pub name: SharedString,
    /// 0 to 100.
    pub level: u8,
    pub charging: bool,
}

/// Red when low, then peach, yellow and green as it fills.
pub fn level_color(level: u8) -> u32 {
    match level {
        level if level <= LOW => RED,
        ..=40 => PEACH,
        ..=60 => YELLOW,
        _ => GREEN,
    }
}

impl Battery {
    pub fn fraction(&self) -> f32 {
        f32::from(self.level.min(100)) / 100.0
    }

    /// Green while charging, otherwise by how full it is.
    pub fn color(&self) -> u32 {
        if self.charging {
            GREEN
        } else {
            level_color(self.level)
        }
    }

    pub fn low(&self) -> bool {
        !self.charging && self.level <= LOW
    }

    /// What the glyph beside its gauge is drawn in: green while charging, red when low.
    pub fn glyph_color(&self) -> u32 {
        match self {
            battery if battery.charging => GREEN,
            battery if battery.low() => CRITICAL,
            _ => TEXT,
        }
    }

    pub fn percent(&self) -> String {
        format!("{}%", self.level)
    }
}
