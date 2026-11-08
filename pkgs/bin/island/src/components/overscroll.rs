//! Something that scrolls giving a little past its ends: a touchpad pulling on at the top or bottom
//! stretches what it holds that way, ever less the further it goes, and lets it spring back once
//! the pull stops. A wheel stops at the ends.

use std::time::Duration;

/// The furthest a pull stretches it.
pub const REACH: f32 = 48.0;
/// How long after the last scroll a pull counts as let go: a touchpad never says when it lifts.
pub const LET_GO: Duration = Duration::from_millis(80);

/// After a scroll by `delta` has moved what scrolls to `offset`, which may be past either end of
/// `0` to `-most`: the pull it leaves, and where what scrolls stays. A pull gives way before what
/// it holds scrolls again.
pub fn settle(pull: f32, offset: f32, delta: f32, most: f32) -> (f32, f32) {
    if pull != 0.0 {
        let edge = if pull > 0.0 { 0.0 } else { -most };
        let next = pull + delta;
        return if next * pull > 0.0 {
            (next, edge)
        } else {
            (0.0, (edge + next).clamp(-most, 0.0))
        };
    }
    if offset > 0.0 {
        (offset, 0.0)
    } else if offset < -most {
        (offset + most, -most)
    } else {
        (0.0, offset)
    }
}

#[cfg(test)]
#[path = "overscroll_test.rs"]
mod tests;
