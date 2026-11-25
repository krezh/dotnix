//! Molt's sizes and times, written as island writes them.

use std::time::Duration;

use chrono::{DateTime, Local, TimeZone};
use molt_core::units::Units;

use crate::units;

pub(super) struct IslandUnits;

impl Units for IslandUnits {
    fn size(&self, bytes: u64) -> String {
        units::bytes(bytes)
    }

    fn span(&self, seconds: u64) -> String {
        units::span(seconds)
    }

    /// The largest unit only, as `4d`, since it sits in a line of others.
    fn age(&self, seconds: u64) -> String {
        units::span(seconds)
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_owned()
    }

    fn old(&self, seconds: u64) -> String {
        self.age(seconds)
    }

    fn moment(&self, unix: i64) -> String {
        Local
            .timestamp_opt(unix, 0)
            .single()
            .map(|at| at.format("%a %H:%M").to_string())
            .unwrap_or_default()
    }

    fn date(&self, unix: i64) -> String {
        Local
            .timestamp_opt(unix, 0)
            .single()
            .map(|at| at.format("%Y-%m-%d").to_string())
            .unwrap_or_default()
    }

    fn parse_moment(&self, iso: &str) -> Option<i64> {
        DateTime::parse_from_rfc3339(iso)
            .ok()
            .map(|at| at.timestamp())
    }

    fn amount(&self, bytes: u64) -> String {
        units::bytes(bytes)
    }

    fn elapsed(&self, duration: Duration) -> String {
        units::running(duration.as_secs())
    }

    fn timing(&self, duration: Duration, precise: bool) -> String {
        let seconds = duration.as_secs_f64();
        if precise && seconds < 10.0 {
            format!("{seconds:.1}s")
        } else if seconds < 1.0 {
            "<1s".to_owned()
        } else {
            units::running(duration.as_secs())
        }
    }

    fn transfer(&self, done: u64, expected: u64) -> String {
        if expected == 0 {
            return units::bytes(done);
        }
        format!("{} / {}", units::bytes(done), units::bytes(expected))
    }

    fn rate(&self, bytes_per_second: f64) -> String {
        units::rate(bytes_per_second)
    }
}

#[cfg(test)]
#[path = "format_test.rs"]
mod tests;
