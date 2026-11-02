//! When an island may draw next: once the compositor says so, or once a frame that could not be
//! shown has been waited out.

use std::time::{Duration, Instant};

const RETRY: Duration = Duration::from_millis(100);

pub struct Turn {
    ready: bool,
    /// The compositor has been asked, and the answer only comes after a frame is shown.
    asked: bool,
    again: Option<Instant>,
}

impl Turn {
    pub fn new() -> Self {
        Self {
            ready: true,
            asked: false,
            again: None,
        }
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    /// When a frame that was not shown is tried again.
    pub fn again(&self) -> Option<Instant> {
        self.again
    }

    /// A frame begins. Whether the compositor has to be asked to say when the next may.
    pub fn start(&mut self) -> bool {
        self.ready = false;
        !std::mem::replace(&mut self.asked, true)
    }

    pub fn answered(&mut self) {
        self.ready = true;
        self.asked = false;
        self.again = None;
    }

    /// The frame could not be shown, so no answer is coming.
    pub fn missed(&mut self, now: Instant) {
        self.again = Some(now + RETRY);
    }

    /// Whether the wait after a missed frame has just ended.
    pub fn elapse(&mut self, now: Instant) -> bool {
        let over = self.again.is_some_and(|at| at <= now);
        if over {
            self.again = None;
            self.ready = true;
        }
        over
    }
}

#[cfg(test)]
#[path = "turn_test.rs"]
mod tests;
