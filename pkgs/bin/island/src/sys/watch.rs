use std::{
    sync::mpsc::{self, RecvTimeoutError, Sender},
    time::{Duration, Instant},
};

use super::{sampler::Sampler, stats::SysStats};
use crate::{
    components::chart,
    supervisor::{self, Ended},
    sync::{Watchers, Watching},
};

#[derive(Clone)]
pub struct Samples {
    watchers: Watchers,
    wake: Sender<()>,
}

impl Samples {
    /// The costly readings (processes, drives, sensors) are taken only while one is held.
    pub fn watch(&self) -> Watching {
        self.watchers.watch(|| {
            let _ = self.wake.send(());
        })
    }
}

/// How often the system is read while something shows it.
pub const INTERVAL: Duration = chart::SAMPLE_EVERY;
/// Between the two process scans a CPU share needs, when someone starts watching.
const FIRST_LOOK: Duration = Duration::from_millis(500);

/// Reads the system on a thread of its own, giving `publish` each reading with when the histories
/// last grew; it stops once `publish` says nothing wants them.
pub fn watch(mut publish: impl FnMut(&SysStats, Instant) -> bool + Send + 'static) -> Samples {
    let mut sampler = Sampler::default();
    sampler.sample(true);
    let watchers = Watchers::default();
    let (wake, woken) = mpsc::channel();
    let watched = watchers.clone();
    let spawned = supervisor::supervise("sys", move || {
        let mut sampled = Instant::now();
        if !publish(&sampler.stats, sampled) {
            return Ended::Unwanted;
        }
        let mut next = sampled + INTERVAL;
        loop {
            match woken.recv_timeout(next.saturating_duration_since(Instant::now())) {
                Ok(()) => {
                    sampler.read_instant();
                    if !publish(&sampler.stats, sampled) {
                        return Ended::Unwanted;
                    }
                    // Fresh costly readings now, rather than up to an interval of stale ones.
                    sampler.scan_processes();
                    std::thread::sleep(FIRST_LOOK);
                    sampler.look();
                    if !publish(&sampler.stats, sampled) {
                        return Ended::Unwanted;
                    }
                    continue;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return Ended::Unwanted,
            }
            sampled = Instant::now();
            next = sampled + INTERVAL;
            let watching = watched.watching();
            sampler.sample(watching);
            if watching && !publish(&sampler.stats, sampled) {
                return Ended::Unwanted;
            }
        }
    });
    if let Err(error) = spawned {
        tracing::error!("system sampler failed to start: {error}");
    }
    Samples { watchers, wake }
}
