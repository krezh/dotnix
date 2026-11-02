use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, RecvTimeoutError, Sender},
    },
    time::{Duration, Instant},
};

use super::{sampler::Sampler, stats::SysStats};
use crate::{
    components::chart,
    supervisor::{self, Ended},
};

struct Latest {
    stats: SysStats,
    /// When the histories last advanced, which the graphs slide from.
    at: Instant,
    /// Bumped on every change, including the in-between refresh when the panel opens.
    revision: u64,
}

#[derive(Clone)]
pub struct Samples {
    latest: Arc<Mutex<Latest>>,
    watchers: Arc<AtomicUsize>,
    wake: Sender<()>,
}

impl Samples {
    /// `None` while nothing changed since `revision`.
    pub fn since(&self, revision: u64) -> Option<(SysStats, Instant, u64)> {
        let latest = self.latest.lock().ok()?;
        (latest.revision != revision).then(|| (latest.stats.clone(), latest.at, latest.revision))
    }

    /// The costly readings (processes, drives, sensors) are taken only while one is held.
    pub fn watch(&self) -> Watch {
        if self.watchers.fetch_add(1, Ordering::SeqCst) == 0 {
            let _ = self.wake.send(());
        }
        Watch(self.watchers.clone())
    }
}

pub struct Watch(Arc<AtomicUsize>);

impl Drop for Watch {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Also how often the System panel and the dashboard update.
pub const INTERVAL: Duration = chart::SAMPLE_EVERY;
/// Between the two process scans a CPU share needs, when someone starts watching.
const FIRST_LOOK: Duration = Duration::from_millis(500);

pub fn watch() -> Samples {
    let mut sampler = Sampler::default();
    sampler.sample(true);
    let watchers = Arc::new(AtomicUsize::new(0));
    let latest = Arc::new(Mutex::new(Latest {
        stats: sampler.stats.clone(),
        at: Instant::now(),
        revision: 1,
    }));
    let (wake, woken) = mpsc::channel();
    let writer = latest.clone();
    let watched = watchers.clone();
    let spawned = supervisor::supervise("sys", move || {
        let mut next = Instant::now() + INTERVAL;
        loop {
            match woken.recv_timeout(next.saturating_duration_since(Instant::now())) {
                Ok(()) => {
                    sampler.read_instant();
                    match writer.lock() {
                        Ok(mut latest) => {
                            latest.stats = sampler.stats.clone();
                            latest.revision += 1;
                        }
                        Err(_) => return Ended::Unwanted,
                    }
                    // Fresh costly readings now, rather than up to an interval of stale ones.
                    sampler.scan_processes();
                    std::thread::sleep(FIRST_LOOK);
                    sampler.look();
                    match writer.lock() {
                        Ok(mut latest) => {
                            latest.stats = sampler.stats.clone();
                            latest.revision += 1;
                        }
                        Err(_) => return Ended::Unwanted,
                    }
                    continue;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return Ended::Unwanted,
            }
            let now = Instant::now();
            next = now + INTERVAL;
            let watching = watched.load(Ordering::SeqCst) > 0;
            sampler.sample(watching);
            match writer.lock() {
                Ok(mut latest) => {
                    latest.at = now;
                    if watching {
                        latest.stats = sampler.stats.clone();
                        latest.revision += 1;
                    }
                }
                Err(_) => return Ended::Unwanted,
            }
        }
    });
    if let Err(error) = spawned {
        tracing::error!("system sampler failed to start: {error}");
    }
    Samples {
        latest,
        watchers,
        wake,
    }
}
