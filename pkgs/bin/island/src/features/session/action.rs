use serde::Deserialize;

use crate::theme::{BLUE, MAUVE, PEACH, RED, TEAL, YELLOW};

pub const LOGIN: &str = "org.freedesktop.login1";
pub const SESSION_PATH: &str = "/org/freedesktop/login1/session/auto";
pub const SESSION: &str = "org.freedesktop.login1.Session";
pub const MANAGER_PATH: &str = "/org/freedesktop/login1";
pub const MANAGER: &str = "org.freedesktop.login1.Manager";

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
    pub const ALL: [Action; 6] = [
        Action::Lock,
        Action::Logout,
        Action::Suspend,
        Action::Hibernate,
        Action::Reboot,
        Action::Shutdown,
    ];

    pub fn word(self) -> &'static str {
        match self {
            Action::Lock => "lock",
            Action::Logout => "logout",
            Action::Suspend => "suspend",
            Action::Hibernate => "hibernate",
            Action::Reboot => "reboot",
            Action::Shutdown => "shutdown",
        }
    }

    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.word() == word)
    }

    pub fn label(self) -> &'static str {
        match self {
            Action::Lock => "Lock",
            Action::Logout => "Log out",
            Action::Suspend => "Suspend",
            Action::Hibernate => "Hibernate",
            Action::Reboot => "Reboot",
            Action::Shutdown => "Shut down",
        }
    }

    /// What it says while under way.
    pub fn doing(self) -> &'static str {
        match self {
            Action::Lock => "Locking the screen",
            Action::Logout => "Logging out",
            Action::Suspend => "Suspending",
            Action::Hibernate => "Hibernating",
            Action::Reboot => "Restarting",
            Action::Shutdown => "Shutting down",
        }
    }

    pub fn color(self) -> u32 {
        match self {
            Action::Lock => BLUE,
            Action::Logout => PEACH,
            Action::Suspend => MAUVE,
            Action::Hibernate => TEAL,
            Action::Reboot => YELLOW,
            Action::Shutdown => RED,
        }
    }

    /// Ends the session, so the apps are closed first.
    pub fn ends_session(self) -> bool {
        matches!(self, Action::Logout | Action::Reboot | Action::Shutdown)
    }

    /// Object, interface and method.
    pub fn logind(self) -> (&'static str, &'static str, &'static str) {
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
