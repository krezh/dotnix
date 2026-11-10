use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
    },
    time::{Duration, Instant},
};

use futures::channel::oneshot;
use tracing::{debug, warn};

use super::{
    device::{self, Screen as Device},
    state::{self, Levels, Screen},
    uevent::{Socket, Uevent},
};
use crate::{
    compositor,
    control::{self, Change, LEVEL, Reply, Report, Row, Shown},
    features::Feed,
    resume, session_lock,
    supervisor::{self, Ended},
};

/// How often the screens are read again while the dashboard shows them, counted from when island
/// last talked to them, so a poll never comes between presses.
const POLL_EVERY: Duration = Duration::from_secs(2);
/// For monitors to wake and answer after a plug, a resume or an unlock.
const SETTLE: Duration = Duration::from_secs(2);

pub struct Brightness {
    messages: Sender<Message>,
    watchers: Arc<AtomicUsize>,
}

pub(super) enum Message {
    Request(Request),
    /// The dashboard opened.
    Watch,
    /// The kernel said a backlight changed.
    Backlight,
    /// Monitors may have come, gone or woken.
    Replug,
    /// The screens may have been changed while the session was locked.
    Unlocked,
}

/// Looking at the screens again once they have settled: reading the ones known, or finding them
/// all afresh.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Look {
    Read,
    Find,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Due {
    pub(super) at: Instant,
    pub(super) look: Look,
}

/// A burst of events waits for its last one, and finding the screens covers reading them.
pub(super) fn postpone(due: Option<Due>, at: Instant, look: Look) -> Due {
    match due {
        Some(due) => Due {
            at: due.at.max(at),
            look: due.look.max(look),
        },
        None => Due { at, look },
    }
}

/// Held while the dashboard is open.
pub(super) struct Watch(Arc<AtomicUsize>);

impl Drop for Watch {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
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
        let (messages, arrived) = mpsc::channel();
        let watchers = Arc::new(AtomicUsize::new(0));
        listen(&messages);
        let watched = watchers.clone();
        feed.supervise(move |feed| run(feed, &arrived, &watched));
        Self { messages, watchers }
    }

    /// One screen's level, 0 to 1, without waiting to hear it was set.
    pub(super) fn set(&self, screen: &str, level: f32) {
        let _ = self.messages.send(Message::Request(Request {
            target: Target::Named(screen.to_owned()),
            change: Some(Change::Set(level.clamp(0.0, 1.0))),
            replies: Vec::new(),
        }));
    }

    /// The monitors are read again every so often until the returned [`Watch`] is dropped.
    pub(super) fn watch(&self) -> Watch {
        if self.watchers.fetch_add(1, Ordering::SeqCst) == 0 {
            let _ = self.messages.send(Message::Watch);
        }
        Watch(self.watchers.clone())
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
        if self.messages.send(Message::Request(request)).is_err() {
            return Report::shows(vec![Row::new(
                "Brightness",
                Shown::Text("stopped".to_owned()),
            )])
            .into();
        }
        Reply::Later(answer)
    }
}

/// Hears what may have changed the screens without island: the kernel's events, the machine
/// waking and the session unlocking.
fn listen(messages: &Sender<Message>) {
    let woken = messages.clone();
    resume::listen(move || {
        let _ = woken.send(Message::Replug);
    });
    let unlocked = messages.clone();
    session_lock::listen_unlocked(move || {
        let _ = unlocked.send(Message::Unlocked);
    });
    let heard = messages.clone();
    supervisor::spawn("island-brightness-uevents", move || {
        let socket = match Socket::open() {
            Ok(socket) => socket,
            Err(error) => {
                warn!("cannot hear the kernel's device events: {error}");
                return;
            }
        };
        loop {
            let message = match socket.next() {
                Ok(Uevent::Backlight) => Message::Backlight,
                Ok(Uevent::Hotplug) => Message::Replug,
                Err(error) => {
                    warn!("stopped hearing the kernel's device events: {error}");
                    return;
                }
            };
            if heard.send(message).is_err() {
                return;
            }
        }
    });
}

struct Worker {
    devices: Vec<Device>,
    levels: Levels,
    /// When island last talked to the monitors.
    talked: Instant,
    due: Option<Due>,
}

fn run(feed: &Feed<Levels>, arrived: &Receiver<Message>, watchers: &AtomicUsize) -> Ended {
    let mut devices = device::find();
    let mut worker = Worker {
        levels: Levels {
            screens: devices.iter_mut().map(read).collect(),
            ..Levels::default()
        },
        devices,
        talked: Instant::now(),
        due: None,
    };
    if !feed.publish(worker.levels.clone()) {
        return Ended::Unwanted;
    }
    loop {
        let watching = watchers.load(Ordering::SeqCst) > 0;
        let poll = watching.then(|| worker.talked + POLL_EVERY);
        let deadline = [poll, worker.due.map(|due| due.at)]
            .into_iter()
            .flatten()
            .min();
        let first = match deadline {
            Some(deadline) => {
                match arrived.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                    Ok(message) => Some(message),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => return Ended::Unwanted,
                }
            }
            None => match arrived.recv() {
                Ok(message) => Some(message),
                Err(_) => return Ended::Unwanted,
            },
        };
        let before = worker.levels.clone();
        match first {
            Some(first) => {
                let queued: Vec<Message> =
                    std::iter::once(first).chain(arrived.try_iter()).collect();
                if !worker.handle(queued, feed) {
                    return Ended::Unwanted;
                }
            }
            None => worker.elapse(watchers.load(Ordering::SeqCst) > 0),
        }
        if worker.levels != before && !feed.publish(worker.levels.clone()) {
            return Ended::Unwanted;
        }
    }
}

impl Worker {
    /// Serves requests in turn, publishing after each so presses show as they land.
    fn handle(&mut self, queued: Vec<Message>, feed: &Feed<Levels>) -> bool {
        let mut requests = Vec::new();
        let (mut watch, mut backlight) = (false, false);
        for message in queued {
            match message {
                Message::Request(request) => requests.push(request),
                Message::Watch => watch = true,
                Message::Backlight => backlight = true,
                Message::Replug => {
                    self.due = Some(postpone(self.due, Instant::now() + SETTLE, Look::Find));
                }
                Message::Unlocked => {
                    self.due = Some(postpone(self.due, Instant::now() + SETTLE, Look::Read));
                }
            }
        }
        if backlight {
            self.backlight();
        }
        for request in batch(requests) {
            let answer = serve(&mut self.devices, &mut self.levels, &request);
            self.talked = Instant::now();
            for reply in request.replies {
                let _ = reply.send(answer.clone());
            }
            if !feed.publish(self.levels.clone()) {
                return false;
            }
        }
        if watch && self.talked.elapsed() >= POLL_EVERY {
            self.reread();
        }
        true
    }

    /// A deadline passed: a look that was due, or a poll while the dashboard is open.
    fn elapse(&mut self, watching: bool) {
        if let Some(due) = self.due.filter(|due| Instant::now() >= due.at) {
            self.due = None;
            match due.look {
                Look::Find => self.find(),
                Look::Read => self.reread(),
            }
        } else if watching && self.talked.elapsed() >= POLL_EVERY {
            self.reread();
        }
    }

    fn reread(&mut self) {
        let readings = self
            .devices
            .iter_mut()
            .map(|device| (device.name.clone(), device.read()))
            .collect();
        state::reread(&mut self.levels, readings);
        self.talked = Instant::now();
    }

    /// Monitors may have come or gone, so they are looked for afresh.
    fn find(&mut self) {
        debug!("looking for the screens again");
        self.devices = device::find();
        self.levels.screens = self.devices.iter_mut().map(read).collect();
        self.talked = Instant::now();
    }

    fn backlight(&mut self) {
        for device in self
            .devices
            .iter_mut()
            .filter(|device| device.is_backlight())
        {
            if let Some(level) = device.read() {
                state::moved(&mut self.levels, &device.name, level);
            }
        }
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
            (Some(current), Some(change)) => device.write(apply(current, change)),
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
