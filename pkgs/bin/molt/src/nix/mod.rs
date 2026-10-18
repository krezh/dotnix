mod errors;
mod progress;

use std::cell::RefCell;
use std::process::{Command, Output};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use molt::command::{self, OutputLimits};

pub(crate) use errors::failed_derivations;
use errors::{nix_error_message, update_best_error};
/// Only tests build activity records by hand; the app reads them through `NixBuildProgress`.
#[cfg(test)]
pub(crate) use progress::NixActivityProgress;
use progress::NixProgressTracker;
pub(crate) use progress::{FlakeFetch, NixBuildProgress, NixProgressMetric};

pub(crate) fn run_nix_command(
    command: &mut Command,
    name: &str,
    cancellation: &AtomicBool,
    timeout: Duration,
    progress: impl Fn(NixBuildProgress),
) -> Result<Output, String> {
    let mut tracker = NixProgressTracker::default();
    let mut dirty = false;
    let mut last_progress = Instant::now() - Duration::from_millis(100);
    let best_error = RefCell::new(None::<(u8, String)>);
    let output = command::run(
        command,
        name,
        cancellation,
        timeout,
        OutputLimits {
            stdout: 64 * 1024,
            stderr: 4 * 1024 * 1024,
        },
        |line| {
            update_best_error(&best_error, line);
            if tracker.update(line).is_some() {
                dirty = true;
                // Nix may go quiet once the last download ends, so show that without waiting.
                let transfers_finished = tracker.take_transfers_finished();
                if transfers_finished || last_progress.elapsed() >= Duration::from_millis(100) {
                    progress(tracker.snapshot());
                    dirty = false;
                    last_progress = Instant::now();
                }
            }
        },
        |stderr| {
            best_error
                .borrow()
                .as_ref()
                .map(|(_, message)| message.clone())
                .or_else(|| nix_error_message(stderr))
                .unwrap_or_else(|| String::from_utf8_lossy(stderr).trim().to_owned())
        },
    );
    if dirty {
        progress(tracker.snapshot());
    }
    output
}

pub(crate) fn strip_ansi(text: &str) -> String {
    let mut output = String::new();
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' && chars.next_if_eq(&'[').is_some() {
            for code in chars.by_ref() {
                if ('@'..='~').contains(&code) {
                    break;
                }
            }
        } else {
            output.push(character);
        }
    }
    output
}

pub(crate) fn run(
    command: &mut Command,
    name: &str,
    cancellation: &AtomicBool,
    timeout: Duration,
    stdout_limit: usize,
) -> Result<Output, String> {
    command::run(
        command,
        name,
        cancellation,
        timeout,
        OutputLimits {
            stdout: stdout_limit,
            stderr: 1024 * 1024,
        },
        |_| {},
        |stderr| {
            strip_ansi(String::from_utf8_lossy(stderr).trim())
                .trim()
                .to_owned()
        },
    )
}
