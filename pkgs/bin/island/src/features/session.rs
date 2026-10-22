use std::{
    fs,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use gpui::{AnyElement, prelude::*};
use serde::Deserialize;
use tracing::{info, warn};
use zbus::{Connection, block_on};

use super::{Feature, Feed, GlyphSpec, PanelCx, WidgetCx, parse_settings};
use crate::{
    bar::GLYPH,
    components::{icon, tooltip::Tip},
    control::{Command, Group, Report, Row, Takes, Value, form},
    dbus::uncached,
    island::{Mode, Panel},
    launch::{self, Launch},
    panel::{ActionTile, Block, Header, IconButton, PanelView, on},
    theme::{BLUE, MAUVE, PEACH, RED, TEAL, TEXT, YELLOW},
};

const CONFIRM: Duration = Duration::from_secs(4);
/// So the uptime keeps up.
const REFRESH: Duration = Duration::from_secs(30);

const LOGIN: &str = "org.freedesktop.login1";
const SESSION_PATH: &str = "/org/freedesktop/login1/session/auto";
const SESSION: &str = "org.freedesktop.login1.Session";
const MANAGER_PATH: &str = "/org/freedesktop/login1";
const MANAGER: &str = "org.freedesktop.login1.Manager";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Lock,
    Logout,
    Suspend,
    Hibernate,
    Reboot,
    Shutdown,
}

impl Action {
    const ALL: [Action; 6] = [
        Action::Lock,
        Action::Logout,
        Action::Suspend,
        Action::Hibernate,
        Action::Reboot,
        Action::Shutdown,
    ];

    fn word(self) -> &'static str {
        match self {
            Action::Lock => "lock",
            Action::Logout => "logout",
            Action::Suspend => "suspend",
            Action::Hibernate => "hibernate",
            Action::Reboot => "reboot",
            Action::Shutdown => "shutdown",
        }
    }

    fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.word() == word)
    }

    fn label(self) -> &'static str {
        match self {
            Action::Lock => "Lock",
            Action::Logout => "Log out",
            Action::Suspend => "Suspend",
            Action::Hibernate => "Hibernate",
            Action::Reboot => "Reboot",
            Action::Shutdown => "Shut down",
        }
    }

    fn color(self) -> u32 {
        match self {
            Action::Lock => BLUE,
            Action::Logout => PEACH,
            Action::Suspend => MAUVE,
            Action::Hibernate => TEAL,
            Action::Reboot => YELLOW,
            Action::Shutdown => RED,
        }
    }

    /// A click on one needs confirming.
    fn ends_session(self) -> bool {
        matches!(self, Action::Logout | Action::Reboot | Action::Shutdown)
    }

    /// Object, interface and method.
    fn logind(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Action::Lock => (SESSION_PATH, SESSION, "Lock"),
            Action::Logout => (SESSION_PATH, SESSION, "Terminate"),
            Action::Suspend => (MANAGER_PATH, MANAGER, "Suspend"),
            Action::Hibernate => (MANAGER_PATH, MANAGER, "Hibernate"),
            Action::Reboot => (MANAGER_PATH, MANAGER, "Reboot"),
            Action::Shutdown => (MANAGER_PATH, MANAGER, "PowerOff"),
        }
    }
}

/// An argv per action, run instead of asking logind; empty asks logind.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Commands {
    lock: Vec<String>,
    logout: Vec<String>,
    suspend: Vec<String>,
    hibernate: Vec<String>,
    reboot: Vec<String>,
    shutdown: Vec<String>,
}

impl Commands {
    fn of(&self, action: Action) -> &[String] {
        match action {
            Action::Lock => &self.lock,
            Action::Logout => &self.logout,
            Action::Suspend => &self.suspend,
            Action::Hibernate => &self.hibernate,
            Action::Reboot => &self.reboot,
            Action::Shutdown => &self.shutdown,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Settings {
    actions: Vec<Action>,
    confirm: bool,
    commands: Commands,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            actions: vec![
                Action::Lock,
                Action::Logout,
                Action::Suspend,
                Action::Reboot,
                Action::Shutdown,
            ],
            confirm: true,
            commands: Commands::default(),
        }
    }
}

fn logind(action: Action) -> zbus::Result<()> {
    block_on(async {
        let connection = Connection::system().await?;
        let (path, interface, method) = action.logind();
        let proxy = uncached(&connection, LOGIN, path, interface).await?;
        if interface == MANAGER {
            proxy.call::<_, _, ()>(method, &(true,)).await
        } else {
            proxy.call::<_, _, ()>(method, &()).await
        }
    })
}

/// "Up 2 d 3 h", "Up 3 h 12 min", "Up 12 min".
fn uptime() -> Option<String> {
    let seconds = fs::read_to_string("/proc/uptime")
        .ok()?
        .split_whitespace()
        .next()?
        .parse::<f64>()
        .ok()? as u64;
    let (days, hours, minutes) = (
        seconds / 86_400,
        seconds % 86_400 / 3600,
        seconds % 3600 / 60,
    );
    Some(match (days, hours) {
        (0, 0) => format!("Up {minutes} min"),
        (0, _) => format!("Up {hours} h {minutes} min"),
        _ => format!("Up {days} d {hours} h"),
    })
}

/// The action waiting for its confirming click, and which arming it was, so an old timeout leaves
/// a newer one alone.
#[derive(Default)]
struct Armed {
    action: Option<Action>,
    generation: u64,
}

#[derive(Clone)]
pub struct Session {
    feed: Feed<Option<Action>>,
    settings: Settings,
    armed: Arc<Mutex<Armed>>,
}

impl Session {
    fn run(&self, action: Action) {
        self.disarm();
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

    fn disarm(&self) {
        let mut armed = self
            .armed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if armed.action.take().is_some() {
            self.feed.publish(None);
        }
    }

    /// True when it ran rather than waiting for a second click.
    fn press(&self, action: Action) -> bool {
        if !(self.settings.confirm && action.ends_session()) {
            self.run(action);
            return true;
        }
        let mut armed = self
            .armed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if armed.action == Some(action) {
            drop(armed);
            self.run(action);
            return true;
        }
        armed.action = Some(action);
        armed.generation += 1;
        let generation = armed.generation;
        drop(armed);
        self.feed.publish(Some(action));
        let (shared, feed) = (self.armed.clone(), self.feed.clone());
        thread::spawn(move || {
            thread::sleep(CONFIRM);
            let mut armed = shared
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if armed.generation == generation && armed.action.take().is_some() {
                feed.publish(None);
            }
        });
        false
    }

    fn panel_handle() -> Panel {
        Panel::Feature(<Self as Feature>::NAME)
    }
}

impl Feature for Session {
    const NAME: &'static str = "session";
    const GLYPHS: &'static [GlyphSpec] = &[
        (
            "session-lock",
            &["system-lock-screen-symbolic", "changes-prevent-symbolic"],
        ),
        (
            "session-logout",
            &["system-log-out-symbolic", "application-exit-symbolic"],
        ),
        (
            "session-suspend",
            &["system-suspend-symbolic", "media-playback-pause-symbolic"],
        ),
        (
            "session-hibernate",
            &[
                "system-hibernate-symbolic",
                "system-suspend-hibernate-symbolic",
                "drive-harddisk-symbolic",
            ],
        ),
        (
            "session-reboot",
            &["system-reboot-symbolic", "view-refresh-symbolic"],
        ),
        ("session-shutdown", &["system-shutdown-symbolic"]),
    ];
    const PANEL: Option<&'static str> = Some("session");
    const PANEL_REFRESH: Option<Duration> = Some(REFRESH);
    const CONTROLS: &'static [Command] = &[Command {
        name: "session",
        about: "Lock, log out, sleep or power off, without asking first",
        group: Group::System,
        takes: Takes::Forms {
            forms: &[
                form("lock", Value::Nothing, "Lock the screen"),
                form("logout", Value::Nothing, "Log out"),
                form("suspend", Value::Nothing, "Suspend to memory"),
                form("hibernate", Value::Nothing, "Hibernate to disk"),
                form("reboot", Value::Nothing, "Restart the computer"),
                form("shutdown", Value::Nothing, "Shut the computer down"),
            ],
            bare: None,
        },
        examples: &["island ctl session lock"],
    }];

    type State = Option<Action>;
    type Local = ();
    type Event = ();

    fn start(feed: Feed<Option<Action>>) -> Self {
        Self {
            settings: feed.settings(),
            feed,
            armed: Arc::default(),
        }
    }

    fn check_settings(settings: &toml::Value) -> Result<(), String> {
        parse_settings::<Settings>(settings).map(drop)
    }

    fn widget(&self, _: &str, _: &Option<Action>, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        Some(
            w.base("bar-session")
                .tip(w.tips(), "bar-session", "Session")
                .when_some(w.glyph("session-shutdown"), |button, path| {
                    button.child(icon::glyph(path, GLYPH, TEXT))
                })
                .on_click(w.toggle(Self::panel_handle()))
                .into_any_element(),
        )
    }

    fn panel<'a>(&self, armed: &Option<Action>, p: &PanelCx<'a>) -> Option<PanelView> {
        let tiles = self
            .settings
            .actions
            .iter()
            .map(|&action| ActionTile {
                id: format!("session-{}", action.word()).into(),
                glyph: p.glyph(&format!("session-{}", action.word())),
                label: action.label().into(),
                color: action.color(),
                armed: *armed == Some(action),
                on_click: p.on_island::<Self>(move |session, island, cx| {
                    if session.press(action) {
                        island.set_mode(Mode::Collapsed, cx);
                    }
                }),
            })
            .collect();
        let note = match armed {
            Some(action) => format!("Click {} again to confirm", action.label()),
            None => uptime().unwrap_or_default(),
        };
        Some(PanelView {
            header: Header::None,
            blocks: vec![Block::Actions(tiles), Block::note(note)],
        })
    }

    fn system_button(&self, _: &Option<Action>, p: &PanelCx<'_>) -> Option<IconButton> {
        Some(IconButton {
            id: "open-session".into(),
            glyph: p.glyph("session-shutdown"),
            rising: None,
            label: "Session".into(),
            active: false,
            badge: None,
            on_click: on(|island, cx| island.set_mode(Mode::Open(Self::panel_handle()), cx)),
        })
    }

    fn command(&self, _: &Option<Action>, args: &[&str]) -> Report {
        let Some(action) = args.get(1).and_then(|word| Action::parse(word)) else {
            return Report::default();
        };
        self.run(action);
        Report::did(Row::said(match action {
            Action::Lock => "Locking the screen",
            Action::Logout => "Logging out",
            Action::Suspend => "Suspending",
            Action::Hibernate => "Hibernating",
            Action::Reboot => "Restarting",
            Action::Shutdown => "Shutting down",
        }))
    }
}
