use std::sync::mpsc::{self, Sender};

use futures::channel::oneshot;

use super::{
    device::{self, Screen as Device},
    state::{Levels, Screen},
};
use crate::{
    compositor,
    control::{self, Change, LEVEL, Reply, Report, Row, Shown},
    features::Feed,
    supervisor::Ended,
};

pub struct Brightness {
    requests: Sender<Request>,
}

/// Which screens a request is for.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Target {
    All,
    Focused,
    Named(String),
}

pub(super) struct Request {
    pub(super) target: Target,
    /// `None` only reads.
    pub(super) change: Option<Change>,
    pub(super) replies: Vec<oneshot::Sender<Result<Report, String>>>,
}

impl Brightness {
    /// The screens are found on its own thread, since DDC/CI is slow, and before the first press.
    pub(super) fn start(feed: Feed<Levels>) -> Self {
        let (requests, arrived) = mpsc::channel();
        feed.supervise(move |feed| {
            let mut devices = device::find();
            let mut levels = Levels {
                screens: devices.iter_mut().map(read).collect(),
                ..Levels::default()
            };
            if !feed.publish(levels.clone()) {
                return Ended::Unwanted;
            }
            for first in &arrived {
                let queued = std::iter::once(first).chain(arrived.try_iter());
                for request in batch(queued) {
                    let answer = serve(&mut devices, &mut levels, &request);
                    for reply in request.replies {
                        let _ = reply.send(answer.clone());
                    }
                    if !feed.publish(levels.clone()) {
                        return Ended::Unwanted;
                    }
                }
            }
            Ended::Unwanted
        });
        Self { requests }
    }

    /// One screen's level, 0 to 1, without waiting to hear it was set.
    pub(super) fn set(&self, screen: &str, level: f32) {
        let _ = self.requests.send(Request {
            target: Target::Named(screen.to_owned()),
            change: Some(Change::Set(level.clamp(0.0, 1.0))),
            replies: Vec::new(),
        });
    }

    pub(super) fn reply(&self, args: &[&str]) -> Reply {
        let rest = &args[1..];
        let (target, rest) = match rest.split_first() {
            Some((first, after)) if !LEVEL.iter().any(|form| form.word == *first) => {
                (target(first), after)
            }
            _ => (Target::All, rest),
        };
        let change = match rest {
            [] => None,
            rest => match control::change(rest, false) {
                Ok(change) => Some(change),
                Err(err) => return Reply::Now(Report::shows(vec![Row::said(err)])),
            },
        };
        let (reply, answer) = oneshot::channel();
        let request = Request {
            target,
            change,
            replies: vec![reply],
        };
        if self.requests.send(request).is_err() {
            return Report::shows(vec![Row::new(
                "Brightness",
                Shown::Text("stopped".to_owned()),
            )])
            .into();
        }
        Reply::Later(answer)
    }
}

fn target(word: &str) -> Target {
    match word {
        "all" => Target::All,
        "focused" => Target::Focused,
        name => Target::Named(name.to_owned()),
    }
}

fn read(device: &mut Device) -> Screen {
    Screen {
        name: device.name.clone(),
        model: device.model.clone(),
        level: device.read(),
    }
}

/// Requests queued one after another for the same screens, joined into one, since DDC/CI is slow.
pub(super) fn batch(requests: impl IntoIterator<Item = Request>) -> Vec<Request> {
    let mut batched: Vec<Request> = Vec::new();
    for request in requests {
        if let Some(last) = batched
            .last_mut()
            .filter(|last| last.target == request.target)
        {
            let joined = match (last.change, request.change) {
                (Some(first), Some(then)) => Some(Some(merge(first, then))),
                (change, None) => Some(change),
                (None, Some(_)) => None,
            };
            if let Some(change) = joined {
                last.change = change;
                last.replies.extend(request.replies);
                continue;
            }
        }
        batched.push(request);
    }
    batched
}

/// One change doing what `first` and then `then` would.
pub(super) fn merge(first: Change, then: Change) -> Change {
    match (first, then) {
        (Change::Step(a), Change::Step(b)) => Change::Step(a + b),
        (Change::Set(value), Change::Step(delta)) => Change::Set((value + delta).clamp(0.0, 1.0)),
        (first, Change::ToggleMute) => first,
        (_, then) => then,
    }
}

/// Looks for the screens again when the request finds none to answer it, since a monitor that was
/// off or asleep when last looked for is missing.
fn serve(
    devices: &mut Vec<Device>,
    levels: &mut Levels,
    request: &Request,
) -> Result<Report, String> {
    attempt(devices, levels, request).or_else(|_| {
        *devices = device::find();
        levels.screens = devices.iter_mut().map(read).collect();
        attempt(devices, levels, request)
    })
}

fn attempt(
    devices: &mut [Device],
    levels: &mut Levels,
    request: &Request,
) -> Result<Report, String> {
    let chosen = choose(devices, &request.target)?;
    let mut changed = Vec::new();
    for index in chosen.iter().copied() {
        let device = &mut devices[index];
        let level = match (device.read(), request.change) {
            (Some(current), Some(change)) => {
                let next = apply(current, change);
                device.write(next).map(|()| next)
            }
            (level, _) => level,
        };
        if level.is_some() && request.change.is_some() {
            changed.push(device.name.clone());
        }
        if let Some(screen) = levels
            .screens
            .iter_mut()
            .find(|screen| screen.name == device.name)
        {
            screen.level = level;
        }
    }
    let rows: Vec<Row> = chosen
        .iter()
        .filter_map(|index| {
            let name = &devices[*index].name;
            levels.screens.iter().find(|screen| &screen.name == name)
        })
        .map(|screen| row(screen, levels.screens.len()))
        .collect();
    if rows.iter().all(|row| matches!(row.value, Shown::Text(_))) {
        return Err(match chosen.as_slice() {
            [index] => format!("{} does not answer", devices[*index].name),
            _ => "No screen answers".to_owned(),
        });
    }
    if !changed.is_empty() {
        levels.changed = changed;
        levels.changes += 1;
    }
    Ok(Report {
        title: None,
        changed: request.change.is_some(),
        rows,
    })
}

fn choose(devices: &[Device], target: &Target) -> Result<Vec<usize>, String> {
    if devices.is_empty() {
        return Err("No screen to set the brightness of".to_owned());
    }
    match target {
        Target::All => Ok((0..devices.len()).collect()),
        // The only screen is the one meant, whichever monitor is focused.
        Target::Focused if devices.len() == 1 => Ok(vec![0]),
        Target::Focused => {
            let name = compositor::backend()
                .and_then(|backend| backend.focused_monitor())
                .ok_or_else(|| "Cannot tell which monitor is focused".to_owned())?;
            named(devices, &name)
        }
        Target::Named(name) => named(devices, name),
    }
}

fn named(devices: &[Device], name: &str) -> Result<Vec<usize>, String> {
    devices
        .iter()
        .position(|device| device.name.eq_ignore_ascii_case(name))
        .map(|index| vec![index])
        .ok_or_else(|| {
            let names: Vec<&str> = devices.iter().map(|device| device.name.as_str()).collect();
            format!("No screen {name} to set; there is {}", names.join(", "))
        })
}

fn apply(current: f32, change: Change) -> f32 {
    match change {
        Change::Step(delta) => (current + delta).clamp(0.0, 1.0),
        Change::Set(value) => value,
        Change::ToggleMute => current,
    }
}

/// Labelled with the screen's name when there are several, so each has its own `--json` key.
fn row(screen: &Screen, screens: usize) -> Row {
    let label = if screens > 1 {
        format!("Brightness {}", screen.name)
    } else {
        "Brightness".to_owned()
    };
    let value = screen.level.map_or_else(
        || Shown::Text("no answer".to_owned()),
        |value| Shown::Level {
            value: value.clamp(0.0, 1.0),
            muted: false,
        },
    );
    Row::new(label, value).detail(screen.model.clone())
}

/// As last read or set, without asking the screens.
pub(super) fn status(levels: &Levels) -> Vec<Row> {
    levels
        .screens
        .iter()
        .map(|screen| row(screen, levels.screens.len()))
        .collect()
}

#[cfg(test)]
#[path = "runtime_test.rs"]
mod tests;
