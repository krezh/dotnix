//! Text for build progress: clock readings, row timings, sizes, and readable store-path names.

use std::time::Duration;

use crate::ui::{byte_unit, compact_size};

/// `0:42`, `3:18`, `1:02:09`.
pub(crate) fn clock(duration: Duration) -> String {
    let seconds = duration.as_secs();
    if seconds >= 3_600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3_600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

/// `42 s` or `3:18` for row timings; `0.8 s` under ten seconds when `precise`, otherwise `<1 s`
/// for work that finished within a second.
pub(crate) fn short_duration(duration: Duration, precise: bool) -> String {
    let seconds = duration.as_secs_f64();
    if precise && seconds < 10.0 {
        format!("{seconds:.1} s")
    } else if seconds < 1.0 {
        "<1 s".to_owned()
    } else if seconds < 60.0 {
        format!("{} s", duration.as_secs())
    } else {
        clock(duration)
    }
}

/// `38 / 71 MiB`, both sides in the unit of the total.
pub(super) fn transfer_text(done: u64, expected: u64) -> String {
    if expected == 0 {
        return compact_size(done as f64);
    }
    let (scale, unit) = byte_unit(expected as f64, 0);
    format!(
        "{:.0} / {:.0} {unit}",
        done as f64 / scale,
        expected as f64 / scale
    )
}

/// Store-path or message text to a readable name: `/nix/store/<hash>-swix-0.1.0.drv` → `swix-0.1.0`.
pub(crate) fn activity_name(text: &str) -> String {
    let Some(store_name) = text
        .split("/nix/store/")
        .nth(1)
        .and_then(|value| value.split(['\'', ' ']).next())
    else {
        return text.to_owned();
    };
    store_name
        .trim_end_matches(".drv")
        .split_once('-')
        .map_or(store_name, |(_, name)| name)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_and_sizes_read_compactly() {
        assert_eq!(clock(Duration::from_secs(222)), "3:42");
        assert_eq!(clock(Duration::from_secs(3_729)), "1:02:09");
        assert_eq!(short_duration(Duration::from_millis(800), true), "0.8 s");
        assert_eq!(short_duration(Duration::from_secs(42), false), "42 s");
        assert_eq!(short_duration(Duration::from_secs(198), false), "3:18");
        assert_eq!(transfer_text(38 << 20, 71 << 20), "38 / 71 MiB");
        assert_eq!(activity_name("/nix/store/hash-swix-0.1.0.drv"), "swix-0.1.0");
    }
}
