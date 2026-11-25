//! Fixtures shared by the test suites of both binaries.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::units::Units;

/// Plain sizes and seconds, so tests read what was formatted without a front end's wording.
pub struct PlainUnits;

impl Units for PlainUnits {
    fn size(&self, bytes: u64) -> String {
        format!("{bytes} B")
    }

    fn span(&self, seconds: u64) -> String {
        format!("{seconds}s")
    }

    fn age(&self, seconds: u64) -> String {
        format!("{seconds}s")
    }

    fn old(&self, seconds: u64) -> String {
        format!("{seconds}s")
    }

    fn moment(&self, unix: i64) -> String {
        unix.to_string()
    }

    fn date(&self, unix: i64) -> String {
        unix.to_string()
    }

    fn parse_moment(&self, _: &str) -> Option<i64> {
        None
    }

    fn amount(&self, bytes: u64) -> String {
        format!("{bytes} B")
    }

    fn elapsed(&self, duration: Duration) -> String {
        format!("{}s", duration.as_secs())
    }

    fn timing(&self, duration: Duration, _: bool) -> String {
        format!("{}s", duration.as_secs())
    }

    fn transfer(&self, done: u64, expected: u64) -> String {
        format!("{done} / {expected} B")
    }

    fn rate(&self, bytes_per_second: f64) -> String {
        format!("{bytes_per_second:.0} B/s")
    }
}

/// Writes an executable `/bin/sh` script through a child `sh`, so the calling process never holds
/// a write descriptor to it. Otherwise a parallel test that forks in that window inherits the
/// descriptor, and executing the script fails with ETXTBSY ("Text file busy").
pub fn write_executable_script(path: &Path, body: &str) {
    let mut writer = Command::new("sh")
        .args(["-c", "cat > \"$1\" && chmod 755 \"$1\"", "sh"])
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .expect("failed to start the script writer");
    writer
        .stdin
        .take()
        .expect("script writer stdin")
        .write_all(format!("#!/bin/sh\n{body}\n").as_bytes())
        .expect("failed to write the script");
    assert!(
        writer
            .wait()
            .expect("script writer did not finish")
            .success(),
        "failed to write {}",
        path.display()
    );
}
