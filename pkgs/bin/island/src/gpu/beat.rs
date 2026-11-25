//! When the frame being drawn will be shown, from when the compositor last showed one, so what
//! moves is placed for the moment it is seen rather than the moment it was drawn.

use std::time::Duration;

/// A frame shown longer ago than this says nothing of when the next will be.
const STALE: Duration = Duration::from_secs(1);

/// A frame the compositor showed: when, on its clock, and how long until its next refresh.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shown {
    pub at: Duration,
    pub refresh: Duration,
}

#[derive(Default)]
pub struct Beat {
    /// The compositor's clock, as `clock_gettime` names it.
    pub clock: Option<u32>,
    pub shown: Option<Shown>,
}

impl Beat {
    /// How long from now until the frame drawn now is shown; nothing if that cannot be told.
    pub fn ahead(&self) -> Duration {
        let (Some(clock), Some(shown)) = (self.clock, self.shown) else {
            return Duration::ZERO;
        };
        read_clock(clock).map_or(Duration::ZERO, |now| ahead_of(shown, now))
    }
}

/// How long from `now` until the first refresh after it, counting from `shown`.
pub fn ahead_of(shown: Shown, now: Duration) -> Duration {
    if shown.refresh.is_zero() {
        return Duration::ZERO;
    }
    let Some(since) = now.checked_sub(shown.at) else {
        return Duration::ZERO;
    };
    if since > STALE {
        return Duration::ZERO;
    }
    let frames = since.as_nanos().div_ceil(shown.refresh.as_nanos()).max(1);
    let next = shown.at + Duration::from_nanos((shown.refresh.as_nanos() * frames) as u64);
    next - now
}

fn read_clock(clock: u32) -> Option<Duration> {
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // `now` lives past the call, and the clock is only read.
    if unsafe { libc::clock_gettime(clock as libc::clockid_t, &mut now) } != 0 {
        return None;
    }
    Some(Duration::new(
        u64::try_from(now.tv_sec).ok()?,
        u32::try_from(now.tv_nsec).ok()?,
    ))
}

#[cfg(test)]
#[path = "beat_test.rs"]
mod tests;
