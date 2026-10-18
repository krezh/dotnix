use std::sync::mpsc::{self, Sender};

use tracing::warn;

use super::{device, state::Level};
use crate::{
    control::{self, Change},
    features::Feed,
    supervisor::Ended,
};

pub struct Brightness {
    changes: Sender<Change>,
}

impl Brightness {
    pub(super) fn start(feed: Feed<Level>) -> Self {
        let (changes, requests) = mpsc::channel();
        feed.supervise(move |feed| {
            let mut level = Level::default();
            for change in &requests {
                let Some(value) = device::apply(change) else {
                    warn!(
                        ?change,
                        "cannot change the brightness: no backlight or DDC/CI monitor"
                    );
                    continue;
                };
                level = Level {
                    value,
                    changes: level.changes + 1,
                };
                if !feed.publish(level.clone()) {
                    return Ended::Unwanted;
                }
            }
            Ended::Unwanted
        });
        Self { changes }
    }

    pub(super) fn command(&self, args: &[&str]) {
        if let Ok(change) = control::change(&args[1..], false) {
            let _ = self.changes.send(change);
        }
    }
}

pub(super) fn check(args: &[&str]) -> Result<(), String> {
    control::change(&args[1..], false).map(drop)
}
