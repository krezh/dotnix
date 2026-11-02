//! Fetches every 15 minutes or when asked; in between, checks every minute against what was last
//! fetched, without the network, so the counts follow local changes such as a pull.

use std::{
    sync::mpsc::{self, RecvTimeoutError, Sender},
    time::Duration,
};

use tracing::{debug, info, warn};

use super::{
    cache::{self, now},
    state::{Report, Updates},
};
use crate::{features::Feed, run, supervisor::Ended};

const INTERVAL: Duration = Duration::from_secs(15 * 60);
const TICK: Duration = Duration::from_secs(60);
const TIMEOUT: Duration = Duration::from_secs(120);

/// Only with `fetch` does it touch the network, fetching the upstream and refreshing the watchlist
/// from GitHub.
fn check(fetch: bool) -> Result<Report, String> {
    let args: &[&str] = if fetch {
        &["check", "--json"]
    } else {
        &["check", "--json", "--no-fetch"]
    };
    let output = run::output("molt", args, TIMEOUT)?;
    if !output.success {
        return Err(output.failure("molt check failed"));
    }
    serde_json::from_str(&output.stdout).map_err(|_| "invalid JSON from molt check".to_owned())
}

enum Request {
    Fetch,
    /// Without the network.
    Look,
}

fn work(feed: &Feed<Updates>, requests: &mpsc::Receiver<Request>) -> Ended {
    let mut cache = cache::load();
    let mut state = Updates {
        report: cache.report.clone(),
        checked_at: (cache.attempted_at > 0).then_some(cache.attempted_at),
        ..Updates::default()
    };
    if !feed.publish(state.clone()) {
        return Ended::Unwanted;
    }
    let mut forced = false;
    loop {
        let fetch = forced || now().saturating_sub(cache.attempted_at) >= INTERVAL.as_secs();
        if fetch {
            state.checking = true;
            state.refreshing = forced;
            if !feed.publish(state.clone()) {
                return Ended::Unwanted;
            }
            cache.attempted_at = now();
            state.checked_at = Some(cache.attempted_at);
        }
        debug!(forced, fetch, "checking for updates");
        let checked = check(fetch);
        let changed = match checked {
            Ok(report) => {
                if fetch {
                    info!(incoming = report.incoming, "update check finished");
                }
                let changed = state.report.as_ref() != Some(&report) || state.error.is_some();
                cache.report = Some(report.clone());
                state.report = Some(report);
                state.error = None;
                changed
            }
            Err(error) => {
                warn!(fetch, "update check failed: {error}");
                let changed = state.error.as_ref() != Some(&error);
                state.error = Some(error);
                changed
            }
        };
        if fetch || changed {
            state.checking = false;
            state.refreshing = false;
            cache::save(&cache);
            if !feed.publish(state.clone()) {
                return Ended::Unwanted;
            }
        }
        forced = match requests.recv_timeout(TICK) {
            Ok(Request::Fetch) => true,
            Ok(Request::Look) | Err(RecvTimeoutError::Timeout) => false,
            Err(RecvTimeoutError::Disconnected) => return Ended::Unwanted,
        };
    }
}

pub struct Molt {
    requests: Sender<Request>,
}

impl Molt {
    pub(super) fn start(feed: Feed<Updates>) -> Self {
        let (requests, incoming) = mpsc::channel();
        feed.supervise(move |feed| work(feed, &incoming));
        Self { requests }
    }

    pub(super) fn refresh(&self) {
        let _ = self.requests.send(Request::Fetch);
    }

    pub(super) fn look(&self) {
        let _ = self.requests.send(Request::Look);
    }
}
