use std::{
    any::Any,
    io,
    panic::{self, AssertUnwindSafe},
    thread,
    time::{Duration, Instant},
};

use tracing::{debug, error, warn};

const RETRY_MIN: Duration = Duration::from_secs(2);
const RETRY_MAX: Duration = Duration::from_secs(60);
const RECONNECT_MIN: Duration = Duration::from_millis(500);
const RECONNECT_MAX: Duration = Duration::from_secs(10);
/// A connection that lasted this long was working, so losing it is worth reconnecting at once.
const RECONNECT_HELD: Duration = Duration::from_secs(10);

/// Paces a worker reconnecting to a source that went away, such as a restarting sound server:
/// straight away after a connection that held, and a little longer after each that failed at once,
/// so a source that refuses connections is not hammered.
#[derive(Default)]
pub struct Reconnect {
    delay: Duration,
}

impl Reconnect {
    pub fn wait(&mut self, started: Instant) {
        self.delay = if started.elapsed() >= RECONNECT_HELD {
            Duration::ZERO
        } else {
            (self.delay * 2).clamp(RECONNECT_MIN, RECONNECT_MAX)
        };
        thread::sleep(self.delay);
    }
}

pub enum Ended {
    /// Done for good.
    Unwanted,
    /// Runs again after a pause.
    Failed(String),
}

fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("unknown cause")
}

/// Everything `run` captures outlives a restart; state it builds inside is rebuilt, so each run
/// should publish afresh.
pub fn supervise(
    name: &'static str,
    mut run: impl FnMut() -> Ended + Send + 'static,
) -> io::Result<()> {
    thread::Builder::new()
        .name(format!("island-{name}"))
        .spawn(move || {
            let mut delay = RETRY_MIN;
            loop {
                let started = Instant::now();
                debug!(worker = name, "worker started");
                match panic::catch_unwind(AssertUnwindSafe(&mut run)) {
                    Ok(Ended::Unwanted) => {
                        debug!(worker = name, "worker finished");
                        return;
                    }
                    Ok(Ended::Failed(reason)) => {
                        if started.elapsed() > RETRY_MAX {
                            delay = RETRY_MIN;
                        }
                        warn!(worker = name, cause = reason, retry_in = ?delay, "worker stopped, restarting");
                    }
                    Err(payload) => {
                        if started.elapsed() > RETRY_MAX {
                            delay = RETRY_MIN;
                        }
                        error!(
                            worker = name,
                            cause = panic_message(&*payload),
                            retry_in = ?delay,
                            "worker crashed, restarting"
                        );
                    }
                }
                thread::sleep(delay);
                delay = (delay * 2).min(RETRY_MAX);
            }
        })
        .map(drop)
}
