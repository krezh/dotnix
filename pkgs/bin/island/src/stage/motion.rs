//! How things move in an island.

use crate::spring::SpringConfig;

pub const SHAPE_SPRING: SpringConfig = SpringConfig::new(420.0, 41.0, 1.0);
pub const CONTENT_IN: SpringConfig = SpringConfig::new(170.0, 26.0, 1.0);
pub const CONTENT_OUT: SpringConfig = SpringConfig::new(1100.0, 66.0, 1.0);
/// Critically damped and unhurried, so folding widgets glide without overshoot.
pub const FOLD_SPRING: SpringConfig = SpringConfig::new(150.0, 24.5, 1.0);
pub const CONTENT_DRIFT: f32 = 8.0;
/// A card growing into the view it opens, or shrinking back into its place: unhurried enough to
/// follow, settling with the slightest give.
pub const GROW_SPRING: SpringConfig = SpringConfig::new(230.0, 27.5, 1.0);
