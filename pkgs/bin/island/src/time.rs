use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The inverse of `seconds()`; anything before the epoch is the epoch.
pub fn at(seconds: i64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(u64::try_from(seconds).unwrap_or(0))
}

pub fn seconds(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |elapsed| {
        i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
    })
}

pub fn millis(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |elapsed| {
        i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
    })
}

pub fn now() -> i64 {
    seconds(SystemTime::now())
}

pub fn now_ms() -> i64 {
    millis(SystemTime::now())
}
