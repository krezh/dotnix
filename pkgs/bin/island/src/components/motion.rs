//! Text that morphs into its new value instead of jumping, and state kept across frames.

use std::time::Duration;

pub const MORPH: Duration = Duration::from_millis(320);
/// How much of a morph each changing character takes to turn into its new one, when several do;
/// they start one after another so the change runs along the text.
const SPAN: f32 = 0.55;

/// Lengths of the shared start and shared end, in bytes.
pub fn shared(old: &str, new: &str) -> (usize, usize) {
    let start: usize = old
        .chars()
        .zip(new.chars())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum();
    let end: usize = old[start..]
        .chars()
        .rev()
        .zip(new[start..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum();
    (start, end)
}

/// How far character `index` of `count` changing ones is through its own change, `progress` of the
/// way through the morph: each takes [`SPAN`] of it, the first starting at once and the last
/// finishing with the morph.
pub fn stagger(progress: f32, index: usize, count: usize) -> f32 {
    if count <= 1 {
        return progress.clamp(0.0, 1.0);
    }
    let start = index as f32 / (count - 1) as f32 * (1.0 - SPAN);
    ((progress - start) / SPAN).clamp(0.0, 1.0)
}

#[cfg(test)]
#[path = "motion_test.rs"]
mod tests;
