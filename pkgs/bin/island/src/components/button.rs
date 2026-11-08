//! Everything clickable: plain and toggle buttons, icon buttons, action and pick tiles, chips.

use crate::spring::SpringConfig;

pub const KNOB_SPRING: SpringConfig = SpringConfig::new(420.0, 41.0, 1.0);

pub const ACTION_W: f32 = 136.0;
pub const ACTION_H: f32 = 96.0;

pub const ICON_BUTTON: f32 = 36.0;

/// Past the top-right corner of what the badge sits on.
pub const BADGE_OVERHANG: f32 = 6.0;

pub const FILL_SPRING: SpringConfig = SpringConfig::new(300.0, 34.6, 1.0);
pub const STRIP_GLYPH: f32 = 18.0;

/// The least a strip icon's glyph shrinks to on its way between off and on.
pub const POP_DIP: f32 = 0.8;
