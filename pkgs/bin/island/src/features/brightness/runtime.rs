use std::sync::mpsc::{self, Sender};

use tracing::warn;

use super::{device, state::Level};
use crate::{
    control::{self, Change, Report, Row, Shown},
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

    pub(super) fn command(&self, level: &Level, args: &[&str]) -> Report {
        if args.len() == 1 {
            return Report::shows(vec![row(level, None)]);
        }
        let Ok(change) = control::change(&args[1..], false) else {
            return Report::default();
        };
        let _ = self.changes.send(change);
        Report::did(row(level, Some(change)))
    }
}

/// Known once island has changed it, since screens are only read as they are changed.
pub(super) fn row(level: &Level, change: Option<Change>) -> Row {
    let value = match change {
        Some(Change::Set(value)) => Some(value),
        Some(Change::Step(delta)) if level.changes > 0 => Some(level.value + delta),
        None if level.changes > 0 => Some(level.value),
        _ => None,
    };
    Row::new(
        "Brightness",
        value.map_or(Shown::Unknown, |value| Shown::Level {
            value: value.clamp(0.0, 1.0),
            muted: false,
        }),
    )
}
