//! How a front end writes sizes and times. Molt and island word everything else the same, but each
//! keeps its own way of writing a unit.

use std::time::Duration;

/// The palette colour something is drawn in; each front end has its own shade of each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hue {
    Green,
    Red,
    Blue,
    Yellow,
    Sapphire,
    Mauve,
    Peach,
    Pink,
    Sky,
    Teal,
    Lavender,
    Maroon,
    Overlay,
}

/// `4 days`: molt's own wording for how long, which messages from the engine use.
pub fn duration_text(seconds: u64) -> String {
    let plural =
        |count: u64, unit: &str| format!("{count} {unit}{}", if count == 1 { "" } else { "s" });
    match seconds {
        0..60 => "under a minute".to_owned(),
        60..3_600 => plural(seconds / 60, "minute"),
        3_600..86_400 => plural(seconds / 3_600, "hour"),
        86_400..5_184_000 => plural(seconds / 86_400, "day"),
        _ => plural(seconds / 2_592_000, "month"),
    }
}

pub trait Units {
    /// A size in bytes, such as `22.0 GiB`.
    fn size(&self, bytes: u64) -> String;
    /// A stretch of time, such as `34 minutes`, as in `34 minutes ago`.
    fn span(&self, seconds: u64) -> String;
    /// A short age, such as `4d`, as in `nixpkgs 4d`.
    fn age(&self, seconds: u64) -> String;
    /// How old something is, such as `4 days`, as in `4 days old`.
    fn old(&self, seconds: u64) -> String;
    /// A moment as weekday and time, such as `Wed 18:39`.
    fn moment(&self, unix: i64) -> String;
    /// A day, such as `2026-11-21`.
    fn date(&self, unix: i64) -> String;
    /// Unix seconds of an ISO 8601 date, such as Git reports.
    fn parse_moment(&self, iso: &str) -> Option<i64>;
    /// A size in whole units once large, such as `312 MiB`, as a running total shows it.
    fn amount(&self, bytes: u64) -> String;
    /// How long something has been going, such as `3:18`, as in `3:18 elapsed`.
    fn elapsed(&self, duration: Duration) -> String;
    /// How long one piece of work took or has taken, such as `42 s`; to a tenth of a second
    /// under ten when `precise`.
    fn timing(&self, duration: Duration, precise: bool) -> String;
    /// Bytes moved of those expected, such as `38 / 71 MiB`.
    fn transfer(&self, done: u64, expected: u64) -> String;
    /// Bytes a second, such as `4.2 MiB/s`.
    fn rate(&self, bytes_per_second: f64) -> String;
}
