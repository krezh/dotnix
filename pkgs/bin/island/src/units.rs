use std::fmt::Display;

const STEP: f64 = 1024.0;
const SIZES: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];

fn size(count: f64) -> String {
    let mut value = count.max(0.0);
    let mut unit = 0;
    while value >= STEP && unit + 1 < SIZES.len() {
        value /= STEP;
        unit += 1;
    }
    let name = SIZES[unit];
    if unit == 0 || value >= 10.0 {
        format!("{value:.0} {name}")
    } else {
        format!("{value:.1} {name}")
    }
}

pub fn bytes(count: u64) -> String {
    size(count as f64)
}

pub fn rate(bytes_per_second: f64) -> String {
    format!("{}/s", size(bytes_per_second))
}

pub fn link(mbits: u32) -> String {
    if mbits >= 1000 {
        format!("{} Gb/s", mbits as f32 / 1000.0)
    } else {
        format!("{mbits} Mb/s")
    }
}

pub fn span(seconds: u64) -> String {
    let (days, hours, minutes) = (
        seconds / 86_400,
        seconds % 86_400 / 3600,
        seconds % 3600 / 60,
    );
    match (days, hours, minutes) {
        (0, 0, 0) => "<1m".to_owned(),
        (0, 0, _) => format!("{minutes}m"),
        (0, ..) => format!("{hours}h {minutes}m"),
        _ => format!("{days}d {hours}h"),
    }
}

/// How long something under way has taken, counting the seconds for its first minute.
pub fn running(seconds: u64) -> String {
    match seconds {
        ..60 => format!("{seconds}s"),
        seconds => span(seconds),
    }
}

pub fn ago(seconds: i64) -> String {
    match seconds.max(0) {
        ..60 => "just now".to_owned(),
        seconds @ ..3600 => format!("{}m ago", seconds / 60),
        seconds @ ..86_400 => format!("{}h ago", seconds / 3600),
        seconds => format!("{}d ago", seconds / 86_400),
    }
}

pub fn plural<T: Display + PartialEq + From<u8>>(count: T, one: &str, many: &str) -> String {
    let word = if count == T::from(1) { one } else { many };
    format!("{count} {word}")
}

#[cfg(test)]
#[path = "units_test.rs"]
mod tests;
