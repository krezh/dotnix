use serde::Deserialize;

use super::Action;

/// An argv per action, run instead of asking logind, or for logging out instead of exiting
/// Hyprland; empty does the usual.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Commands {
    lock: Vec<String>,
    logout: Vec<String>,
    suspend: Vec<String>,
    hibernate: Vec<String>,
    reboot: Vec<String>,
    shutdown: Vec<String>,
}

impl Commands {
    pub fn of(&self, action: Action) -> &[String] {
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
pub struct Settings {
    pub actions: Vec<Action>,
    pub confirm: bool,
    pub commands: Commands,
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
