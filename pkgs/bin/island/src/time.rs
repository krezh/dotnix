use std::time::{SystemTime, UNIX_EPOCH};

pub fn seconds(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |elapsed| {
        i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
    })
}

pub fn now() -> i64 {
    seconds(SystemTime::now())
}
