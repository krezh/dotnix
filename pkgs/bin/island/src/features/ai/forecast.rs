use std::ops::Range;

use super::state::Limit;

const STEP_MS: i64 = 15 * 60 * 1000;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;
/// An hour of the night counts this much of an hour of the day in windows longer than a day,
/// which span nights you are mostly not working through.
const NIGHT: f64 = 0.15;
const NIGHT_HOURS: Range<u32> = 0..8;
/// The recent pace is read over this share of the window, and counts this much against the
/// average pace since the window started.
const RECENT_SHARE: f64 = 0.1;
const RECENT_WEIGHT: f64 = 0.6;
/// Too early in a window, or with too little used, to say anything.
const SETTLED: f64 = 0.1;
const LEAST_USED: f64 = 5.0;

/// Where a limit's window is expected to end, and when it would run out before its reset.
#[derive(Debug, PartialEq)]
pub(super) struct Forecast {
    pub(super) expected: f64,
    pub(super) exhausts_at: Option<i64>,
}

/// `readings` are the window's earlier readings, as when and how much was used; `hour` gives the
/// local hour of a time.
pub(super) fn forecast(
    limit: &Limit,
    readings: &[(i64, f64)],
    now: i64,
    hour: impl Fn(i64) -> u32,
) -> Option<Forecast> {
    let (reset, duration) = (limit.resets_at?, limit.duration_ms?);
    let used = limit.used;
    if duration <= 0 || now >= reset || !(LEAST_USED..100.0).contains(&used) {
        return None;
    }
    let start = reset - duration;
    if ((now - start) as f64) < duration as f64 * SETTLED {
        return None;
    }
    let long = duration > DAY_MS;
    let weight = |at: i64| {
        if long && NIGHT_HOURS.contains(&hour(at)) {
            NIGHT
        } else {
            1.0
        }
    };
    // Time between two moments, with nights counted for less.
    let active = |from: i64, to: i64| {
        let mut total = 0.0;
        let mut at = from;
        while at < to {
            let step = STEP_MS.min(to - at);
            total += weight(at) * step as f64;
            at += step;
        }
        total
    };
    let done = active(start, now);
    if done <= 0.0 {
        return None;
    }
    let average = used / done;
    let since = now - (duration as f64 * RECENT_SHARE) as i64;
    let recent = readings
        .iter()
        .filter(|(at, _)| *at >= start.max(since) && *at < now)
        .min_by_key(|(at, _)| *at)
        .and_then(|(at, then)| {
            let span = active(*at, now);
            (span > 0.0).then(|| (used - then).max(0.0) / span)
        });
    let rate = recent.map_or(average, |recent| {
        RECENT_WEIGHT * recent + (1.0 - RECENT_WEIGHT) * average
    });
    let expected = used + rate * active(now, reset);
    let exhausts_at = (expected >= 100.0).then(|| {
        let mut level = used;
        let mut at = now;
        while at < reset {
            let step = STEP_MS.min(reset - at);
            let pace = rate * weight(at);
            if pace > 0.0 && level + pace * step as f64 >= 100.0 {
                return Some(at + ((100.0 - level) / pace) as i64);
            }
            level += pace * step as f64;
            at += step;
        }
        None
    });
    Some(Forecast {
        expected,
        exhausts_at: exhausts_at.flatten(),
    })
}

#[cfg(test)]
#[path = "forecast_test.rs"]
mod tests;
