//! Times the stages of a frame, only while `frames=debug` is logged.

use std::{
    fmt,
    time::{Duration, Instant},
};

pub struct Laps {
    last: Option<Instant>,
    laps: Vec<(&'static str, Duration)>,
}

impl Laps {
    pub fn start() -> Self {
        Self {
            last: tracing::enabled!(target: "island::frames", tracing::Level::DEBUG)
                .then(Instant::now),
            laps: Vec::new(),
        }
    }

    pub fn lap(&mut self, name: &'static str) {
        if let Some(last) = &mut self.last {
            let now = Instant::now();
            self.laps.push((name, now - *last));
            *last = now;
        }
    }

    /// The whole time, once it is over `limit`.
    pub fn over(&self, limit: Duration) -> Option<Duration> {
        let total: Duration = self.laps.iter().map(|&(_, took)| took).sum();
        (total > limit).then_some(total)
    }
}

impl fmt::Display for Laps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sep = "";
        for (name, took) in &self.laps {
            write!(f, "{sep}{name}={took:?}")?;
            sep = " ";
        }
        Ok(())
    }
}
