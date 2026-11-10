use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use tracing::{info, warn};

use super::{
    Action, logind,
    settings::Settings,
    state::State,
    worker::{self, Leaver, Order, Shared},
};
use crate::{
    control::{Command, Group, Report, Row, Takes, Value, form},
    features::{Feature, Feed, parse_settings},
    launch::{self, Launch},
    sync::lock,
};

const CONFIRM: Duration = Duration::from_secs(4);

#[derive(Clone)]
pub struct Session {
    pub(super) settings: Settings,
    shared: Arc<Shared>,
    leaver: Arc<Mutex<Option<Leaver>>>,
    /// Which arming is the latest, so an old timeout leaves a newer one alone.
    armings: Arc<Mutex<u64>>,
}

impl Session {
    /// Locks, sleeps or ends the session; ending it closes the apps first.
    pub(super) fn run(&self, action: Action) {
        self.disarm();
        if action.ends_session() {
            self.leave(action, false);
            return;
        }
        let argv = self.settings.commands.of(action).to_vec();
        if argv.is_empty() {
            thread::spawn(move || match logind(action) {
                Ok(()) => info!(action = action.word(), "asked logind"),
                Err(error) => warn!(action = action.word(), "logind refused: {error}"),
            });
        } else {
            info!(action = action.word(), command = ?argv, "running the session command");
            launch::spawn(Launch {
                id: format!("io.github.krezh.Island.{}", action.word()),
                name: action.label().to_owned(),
                argv,
                directory: None,
            });
        }
    }

    /// Starts closing the apps, unless the session is already ending.
    fn leave(&self, action: Action, rehearsal: bool) {
        let mut leaver = lock(&self.leaver);
        if leaver.is_some() {
            return;
        }
        let held = self.leaver.clone();
        let done = move || *lock(&held) = None;
        match worker::start(
            action,
            self.settings.clone(),
            self.shared.clone(),
            rehearsal,
            done,
        ) {
            Ok(started) => *leaver = Some(started),
            Err(error) => warn!("{error}"),
        }
    }

    pub(super) fn cancel(&self) {
        if let Some(leaver) = &*lock(&self.leaver) {
            leaver.order(Order::Cancel);
        }
    }

    pub(super) fn force(&self) {
        if let Some(leaver) = &*lock(&self.leaver) {
            leaver.order(Order::Force);
        }
    }

    fn disarm(&self) {
        self.shared.update(|state| state.armed = None);
    }

    /// True when it ran rather than waiting for a second click.
    pub(super) fn press(&self, action: Action) -> bool {
        if !(self.settings.confirm && action.ends_session()) {
            self.run(action);
            return true;
        }
        if self.shared.get().armed == Some(action) {
            self.run(action);
            return true;
        }
        let arming = {
            let mut armings = lock(&self.armings);
            *armings += 1;
            *armings
        };
        self.shared.update(|state| state.armed = Some(action));
        let (shared, armings) = (self.shared.clone(), self.armings.clone());
        thread::spawn(move || {
            thread::sleep(CONFIRM);
            if *lock(&armings) == arming {
                shared.update(|state| state.armed = None);
            }
        });
        false
    }
}

impl Feature for Session {
    const NAME: &'static str = "session";

    const CONTROLS: &'static [Command] = &[Command {
        name: "session",
        about: "Lock, log out, sleep or power off, without asking first",
        group: Group::System,
        takes: Takes::Forms {
            forms: &[
                form("lock", Value::Nothing, "Lock the screen"),
                form("logout", Value::Nothing, "Close every app, then log out"),
                form("suspend", Value::Nothing, "Suspend to memory"),
                form("hibernate", Value::Nothing, "Hibernate to disk"),
                form(
                    "reboot",
                    Value::Nothing,
                    "Close every app, then restart the computer",
                ),
                form(
                    "shutdown",
                    Value::Nothing,
                    "Close every app, then shut the computer down",
                ),
                form(
                    "rehearse",
                    Value::Nothing,
                    "Show shutting down without closing anything or shutting down",
                ),
                form(
                    "cancel",
                    Value::Nothing,
                    "Stop ending the session, leaving what is open",
                ),
            ],
            bare: None,
        },
        examples: &["island ctl session lock", "island ctl session rehearse"],
    }];

    type State = State;

    type Event = ();

    fn start(feed: Feed<State>) -> Self {
        Self {
            settings: feed.settings(),
            shared: Arc::new(Shared::new(feed)),
            leaver: Arc::default(),
            armings: Arc::default(),
        }
    }

    fn check_settings(settings: &toml::Value) -> Result<(), String> {
        parse_settings::<Settings>(settings).map(drop)
    }

    fn command(&self, _: &State, args: &[&str]) -> Report {
        match args.get(1).copied() {
            Some("rehearse") => {
                self.leave(Action::Shutdown, true);
                Report::did(Row::said("Rehearsing shutting down"))
            }
            Some("cancel") => {
                self.cancel();
                Report::did(Row::said("Stopped ending the session"))
            }
            word => {
                let Some(action) = word.and_then(Action::parse) else {
                    return Report::default();
                };
                self.run(action);
                Report::did(Row::said(action.doing()))
            }
        }
    }
}
