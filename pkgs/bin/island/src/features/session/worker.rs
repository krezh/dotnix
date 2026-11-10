//! Leaving the session on a thread of its own: finding the open apps, closing them as Hyprland and
//! their processes say they have gone, then, after the island has switched off, the action.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, Sender, channel},
    },
    thread,
    time::{Duration, Instant},
};

use tracing::{info, warn};

use super::{
    Action,
    apps::Apps,
    hyprland::Events,
    logind,
    process::{self, Bell},
    settings::Settings,
    state::{App, Leaving, Mood, Phase, State},
};
use crate::{
    features::Feed,
    launch::{self, Launch},
    stage::SWITCH_OFF,
    sync::lock,
};

/// How long an app may take before it reads as not responding.
const LINGER: Duration = Duration::from_millis(3500);
/// Between the last app going and the island switching off.
const SETTLE: Duration = Duration::from_millis(450);
/// Between one forced app and the next going.
const KILL_STEP: Duration = Duration::from_millis(70);
/// In a rehearsal, when the first app pretends to close, and how long after it each next one does.
const FIRST_CLOSE: Duration = Duration::from_millis(600);
const NEXT_CLOSE: Duration = Duration::from_millis(350);

pub enum Order {
    Cancel,
    Force,
}

/// The state every island sees, which the feature and the worker both change.
pub struct Shared {
    state: Mutex<State>,
    feed: Feed<State>,
}

impl Shared {
    pub fn new(feed: Feed<State>) -> Self {
        Self {
            state: Mutex::default(),
            feed,
        }
    }

    pub fn update(&self, change: impl FnOnce(&mut State)) {
        let mut state = lock(&self.state);
        let before = state.clone();
        change(&mut state);
        if *state != before {
            self.feed.publish(state.clone());
        }
    }

    pub fn get(&self) -> State {
        lock(&self.state).clone()
    }

    fn leaving(&self, change: impl FnOnce(&mut Leaving)) {
        self.update(|state| {
            if let Some(leaving) = &mut state.leaving {
                change(leaving);
            }
        });
    }
}

/// A worker under way, and how to reach it.
pub struct Leaver {
    orders: Sender<Order>,
    bell: Arc<Bell>,
}

impl Leaver {
    pub fn order(&self, order: Order) {
        if self.orders.send(order).is_ok() {
            self.bell.ring();
        }
    }
}

enum Ended {
    Closed,
    Cancelled,
}

/// Starts ending the session with `action`; `done` runs once it has ended or stopped.
pub fn start(
    action: Action,
    settings: Settings,
    shared: Arc<Shared>,
    rehearsal: bool,
    done: impl FnOnce() + Send + 'static,
) -> Result<Leaver, String> {
    let bell = Arc::new(Bell::new()?);
    let (orders, received) = channel();
    let ringing = bell.clone();
    thread::Builder::new()
        .name("session-leaving".to_owned())
        .spawn(move || {
            run(action, &settings, &shared, rehearsal, &received, &ringing);
            done();
        })
        .map_err(|error| format!("cannot start leaving the session: {error}"))?;
    Ok(Leaver { orders, bell })
}

fn run(
    action: Action,
    settings: &Settings,
    shared: &Shared,
    rehearsal: bool,
    orders: &Receiver<Order>,
    bell: &Bell,
) {
    shared.update(|state| {
        state.armed = None;
        state.leaving = Some(Leaving {
            action,
            since: Instant::now(),
            total: 0,
            apps: Vec::new(),
            phase: Phase::Finding,
            rehearsal,
        });
    });
    info!(action = action.word(), rehearsal, "leaving the session");
    let ended = match Apps::find() {
        Ok(apps) => {
            let rows: Vec<App> = apps
                .open()
                .iter()
                .map(|open| App::new(open.key.clone(), &open.class, &open.title))
                .collect();
            shared.leaving(|leaving| {
                leaving.total = rows.len();
                leaving.apps = rows.clone();
                leaving.phase = Phase::Closing;
            });
            let rows: HashMap<String, App> =
                rows.into_iter().map(|row| (row.key.clone(), row)).collect();
            let ended = if rehearsal {
                rehearse(shared, orders, bell)
            } else {
                close(apps, &rows, shared, orders, bell, action, settings)
            };
            match ended {
                Ok(ended) => ended,
                Err(error) => fail(shared, orders, error),
            }
        }
        Err(error) => fail(shared, orders, error),
    };
    if let Ended::Cancelled = ended {
        info!("leaving the session was cancelled");
        shared.update(|state| state.leaving = None);
    }
}

/// Shows what went wrong until it is cancelled, as nothing else is safe.
fn fail(shared: &Shared, orders: &Receiver<Order>, error: String) -> Ended {
    warn!("cannot leave the session: {error}");
    shared.leaving(|leaving| leaving.phase = Phase::Failed(error));
    loop {
        match orders.recv() {
            Ok(Order::Cancel) | Err(_) => return Ended::Cancelled,
            Ok(Order::Force) => {}
        }
    }
}

/// Shows `apps` still open, in the order found, those open past `LINGER` as not responding.
fn show(shared: &Shared, rows: &HashMap<String, App>, open: &[String], lingering: bool) {
    shared.leaving(|leaving| {
        leaving.apps = leaving
            .apps
            .iter()
            .filter(|app| open.contains(&app.key))
            .map(|app| {
                let mut app = rows.get(&app.key).cloned().unwrap_or_else(|| app.clone());
                app.mood = if lingering {
                    Mood::Lingering
                } else {
                    Mood::Closing
                };
                app
            })
            .collect();
    });
}

/// Kills what is still open one after another, each shown killed before it goes.
fn forced(
    shared: &Shared,
    keys: &[String],
    mut kill: impl FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    shared.leaving(|leaving| {
        for app in &mut leaving.apps {
            app.mood = Mood::Killed;
        }
    });
    for key in keys {
        kill(key)?;
        thread::sleep(KILL_STEP);
        shared.leaving(|leaving| leaving.apps.retain(|app| app.key != *key));
    }
    Ok(())
}

fn close(
    mut apps: Apps,
    rows: &HashMap<String, App>,
    shared: &Shared,
    orders: &Receiver<Order>,
    bell: &Bell,
    action: Action,
    settings: &Settings,
) -> Result<Ended, String> {
    let started = Instant::now();
    let mut events: Option<Events> = apps.hyprland().events().ok();
    apps.close()?;
    loop {
        let open: Vec<String> = apps.open().iter().map(|app| app.key.clone()).collect();
        show(shared, rows, &open, started.elapsed() >= LINGER);
        if apps.done() {
            break;
        }
        let linger = (started.elapsed() < LINGER).then_some(started + LINGER);
        let until = [apps.next_ask(), linger].into_iter().flatten().min();
        let mut fds = apps.fds();
        fds.push(bell.fd());
        fds.extend(events.as_ref().map(Events::fd));
        process::wait(&fds, until)?;
        bell.quiet();
        if events.as_mut().is_some_and(|events| !events.drain()) {
            events = None;
        }
        for order in orders.try_iter() {
            match order {
                Order::Cancel => return Ok(Ended::Cancelled),
                Order::Force => {
                    let keys: Vec<String> = apps.open().iter().map(|app| app.key.clone()).collect();
                    forced(shared, &keys, |key| apps.kill(key))?;
                    apps.forced();
                }
            }
        }
        if !apps.done() {
            apps.close()?;
        }
    }
    if let Some(Order::Cancel) = settle(orders, bell) {
        return Ok(Ended::Cancelled);
    }
    shared.leaving(|leaving| leaving.phase = Phase::Off);
    thread::sleep(SWITCH_OFF);
    match finish(action, settings, &apps) {
        Ok(()) => Ok(Ended::Closed),
        Err(error) => {
            warn!(action = action.word(), "cannot end the session: {error}");
            Ok(Ended::Cancelled)
        }
    }
}

/// Waits `SETTLE` after the last app has gone, so the empty list is seen; a cancel still counts.
fn settle(orders: &Receiver<Order>, bell: &Bell) -> Option<Order> {
    let until = Instant::now() + SETTLE;
    while Instant::now() < until {
        let _ = process::wait(&[bell.fd()], Some(until));
        bell.quiet();
        if let Some(order @ Order::Cancel) = orders
            .try_iter()
            .find(|order| matches!(order, Order::Cancel))
        {
            return Some(order);
        }
    }
    None
}

/// Runs the action once the apps have closed: its own command, or exiting Hyprland to log out, or
/// logind.
fn finish(action: Action, settings: &Settings, apps: &Apps) -> Result<(), String> {
    let argv = settings.commands.of(action).to_vec();
    if !argv.is_empty() {
        info!(action = action.word(), command = ?argv, "running the session command");
        launch::spawn(Launch {
            id: format!("io.github.krezh.Island.{}", action.word()),
            name: action.label().to_owned(),
            argv,
            directory: None,
        });
        return Ok(());
    }
    match action {
        Action::Logout => apps.hyprland().exit(),
        _ => logind(action).map_err(|error| error.to_string()),
    }?;
    info!(action = action.word(), "asked to end the session");
    Ok(())
}

/// As closing, but every app only pretends to close, one after another, and nothing runs at the
/// end: the island comes back.
fn rehearse(shared: &Shared, orders: &Receiver<Order>, bell: &Bell) -> Result<Ended, String> {
    let started = Instant::now();
    let keys: Vec<String> = shared
        .get()
        .leaving
        .map(|leaving| leaving.apps.into_iter().map(|app| app.key).collect())
        .unwrap_or_default();
    let mut gone = 0;
    while gone < keys.len() {
        let due = started + FIRST_CLOSE + NEXT_CLOSE * gone as u32;
        let _ = process::wait(&[bell.fd()], Some(due));
        bell.quiet();
        for order in orders.try_iter() {
            match order {
                Order::Cancel => return Ok(Ended::Cancelled),
                Order::Force => {
                    forced(shared, &keys[gone..], |_| Ok(()))?;
                    gone = keys.len();
                }
            }
        }
        if gone < keys.len() && Instant::now() >= due {
            let key = &keys[gone];
            shared.leaving(|leaving| leaving.apps.retain(|app| app.key != *key));
            gone += 1;
        }
    }
    if let Some(Order::Cancel) = settle(orders, bell) {
        return Ok(Ended::Cancelled);
    }
    shared.leaving(|leaving| leaving.phase = Phase::Off);
    thread::sleep(SWITCH_OFF + SETTLE);
    info!("rehearsal over; nothing was closed");
    Ok(Ended::Cancelled)
}
