use std::sync::Arc;

use super::state::State;

/// How much of one limit's current window was used, as each refresh saw it.
#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    pub(super) provider: String,
    pub(super) limit: String,
    pub(super) start: i64,
    pub(super) duration: i64,
    /// When, and how much was used then, oldest first.
    pub(super) samples: Vec<(i64, f64)>,
}

/// Adds what the latest refresh saw to each limit's window; a limit that started a new window
/// starts its samples over. Gives the windows it changed, to keep.
pub(super) fn record(state: &mut State) -> Vec<Window> {
    let mut windows = state.windows.as_ref().clone();
    let mut changed = Vec::new();
    for provider in state
        .providers
        .iter()
        .filter(|provider| provider.freshness.is_fresh())
    {
        for limit in &provider.limits {
            let (Some(reset), Some(duration)) = (limit.resets_at, limit.duration_ms) else {
                continue;
            };
            if duration <= 0 {
                continue;
            }
            let start = reset - duration;
            let sample = (provider.fetched_at, limit.used);
            match windows
                .iter_mut()
                .find(|window| window.provider == provider.id && window.limit == limit.id)
            {
                Some(window) if window.start == start => {
                    if window.samples.last().is_none_or(|last| last.0 < sample.0) {
                        window.samples.push(sample);
                        changed.push(window.clone());
                    }
                }
                Some(window) => {
                    window.start = start;
                    window.duration = duration;
                    window.samples = vec![sample];
                    changed.push(window.clone());
                }
                None => {
                    let window = Window {
                        provider: provider.id.clone(),
                        limit: limit.id.clone(),
                        start,
                        duration,
                        samples: vec![sample],
                    };
                    changed.push(window.clone());
                    windows.push(window);
                }
            }
        }
    }
    if !changed.is_empty() {
        state.windows = Arc::new(windows);
    }
    changed
}
