use std::{sync::Arc, time::Instant};

use crate::{
    components::chart::Phase,
    sys::{INTERVAL, SysStats},
};

/// What the sampler last read, while something shows it.
#[derive(Clone, Default)]
pub struct State {
    pub stats: Arc<SysStats>,
    /// When the histories last grew, which the charts slide from.
    pub sampled: Option<Instant>,
    /// Bumped on every reading, so a new one is a new state without comparing the stats.
    pub revision: u64,
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.revision == other.revision
    }
}

impl State {
    pub fn phase(&self) -> Phase {
        Phase {
            sampled: self.sampled,
            every: INTERVAL,
        }
    }
}
