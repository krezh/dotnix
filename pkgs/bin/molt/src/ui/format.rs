//! Sizes, ages, and dates as Molt shows them.

use std::time::{SystemTime, UNIX_EPOCH};

use gtk::glib;

const BYTE_UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

/// The binary unit to show `bytes` in, and that unit's size in bytes: the smallest unit whose
/// value, rounded to `decimals` places, stays below 1024.
pub(crate) fn byte_unit(bytes: f64, decimals: i32) -> (f64, &'static str) {
    let limit = 1024.0 - 0.5 / 10_f64.powi(decimals);
    let mut scale = 1.0;
    let mut unit = 0;
    while unit + 1 < BYTE_UNITS.len() && bytes / scale >= limit {
        scale *= 1024.0;
        unit += 1;
    }
    (scale, BYTE_UNITS[unit])
}

/// `512.0 B`, `1.5 KiB`, `28.7 GiB`.
pub(crate) fn format_size(bytes: u64) -> String {
    let (scale, unit) = byte_unit(bytes as f64, 1);
    format!("{:.1} {unit}", bytes as f64 / scale)
}

/// `+1.5 KiB`, `-28.7 GiB`.
pub(crate) fn signed_size(bytes: i64) -> String {
    let sign = if bytes >= 0 { "+" } else { "-" };
    format!("{sign}{}", format_size(bytes.unsigned_abs()))
}

/// `312 MiB`, `1.2 GiB`: whole units from ten up, one decimal below.
pub(crate) fn compact_size(bytes: f64) -> String {
    let (scale, unit) = byte_unit(bytes, 0);
    let value = bytes / scale;
    if value < 9.95 && unit != "B" {
        format!("{value:.1} {unit}")
    } else {
        format!("{value:.0} {unit}")
    }
}

/// Seconds from a Unix timestamp to `now`; future or invalid timestamps count as zero.
pub(crate) fn seconds_since(now: SystemTime, timestamp: i64) -> u64 {
    let now = now
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    now.saturating_sub(u64::try_from(timestamp).unwrap_or(0))
}

pub(crate) fn format_timestamp(timestamp: i64, format: &str) -> Option<String> {
    let date = glib::DateTime::from_unix_local(timestamp).ok()?;
    date.format(format).ok().map(|date| date.to_string())
}

/// `3 hours`, `12 days`: an elapsed time for sentences such as "… ago" or "… old".
pub(crate) fn duration_text(seconds: u64) -> String {
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

/// `40m`, `5h`, `12d`: a compact elapsed time for dense rows.
pub(crate) fn short_age(seconds: u64) -> String {
    match seconds {
        0..3_600 => format!("{}m", seconds / 60),
        3_600..86_400 => format!("{}h", seconds / 3_600),
        _ => format!("{}d", seconds / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_pick_the_unit_their_rounded_value_fits() {
        assert_eq!(format_size(1023), "1023.0 B");
        assert_eq!(format_size(1536), "1.5 KiB");
        assert_eq!(format_size((1 << 20) - 1), "1.0 MiB");
        assert_eq!(format_size((1 << 30) - 1), "1.0 GiB");
        assert_eq!(format_size(2 * 1024 * 1024 * 1024), "2.0 GiB");
        assert_eq!(format_size(3 << 40), "3.0 TiB");
        assert_eq!(signed_size(-((1 << 30) - 1)), "-1.0 GiB");
        assert_eq!(signed_size(512), "+512.0 B");
        assert_eq!(compact_size(1023.7), "1.0 KiB");
        assert_eq!(compact_size(312.0 * 1024.0 * 1024.0), "312 MiB");
        assert_eq!(compact_size(9.97 * 1024.0 * 1024.0), "10 MiB");
        assert_eq!(compact_size(1.2 * 1024.0 * 1024.0 * 1024.0), "1.2 GiB");
        assert_eq!(compact_size(((1 << 30) - 1) as f64), "1.0 GiB");
    }

    #[test]
    fn ages_switch_units_at_their_boundaries() {
        assert_eq!(duration_text(59), "under a minute");
        assert_eq!(duration_text(60), "1 minute");
        assert_eq!(duration_text(3_599), "59 minutes");
        assert_eq!(duration_text(3_600), "1 hour");
        assert_eq!(duration_text(86_399), "23 hours");
        assert_eq!(duration_text(86_400), "1 day");
        assert_eq!(duration_text(5_183_999), "59 days");
        assert_eq!(duration_text(5_184_000), "2 months");
        assert_eq!(short_age(3_599), "59m");
        assert_eq!(short_age(3_600), "1h");
        assert_eq!(short_age(86_400), "1d");
    }
}
