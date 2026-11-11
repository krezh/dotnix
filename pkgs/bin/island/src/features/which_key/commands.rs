use super::{feature::WhichKey, menu, settings::Entry, state::Local};
use crate::{components::list, features::Outcome, key::Key, launch};

/// What picking an entry does.
pub(super) enum Picked {
    Run,
    Enter,
}

impl WhichKey {
    /// Runs the entry's command, or says to go into its submenu.
    pub(super) fn pick(&self, entry: &Entry) -> Picked {
        match &entry.cmd {
            Some(cmd) if entry.submenu.is_empty() => {
                launch::spawn(launch::Launch {
                    id: "which-key".to_owned(),
                    name: entry.desc.clone(),
                    argv: vec!["sh".to_owned(), "-c".to_owned(), cmd.clone()],
                    directory: None,
                });
                Picked::Run
            }
            _ => Picked::Enter,
        }
    }

    pub(super) fn handle_key(&self, local: &mut Local, key: &Key) -> Outcome {
        match key.name.as_str() {
            "escape" => return Outcome::Close,
            "backspace" => {
                return if local.back() {
                    Outcome::Open
                } else {
                    Outcome::Close
                };
            }
            _ => {}
        }
        let Some(typed) = list::typed(key) else {
            return Outcome::Open;
        };
        let Some(level) = menu::level(&self.settings, local.menu(), &local.keys) else {
            return Outcome::Open;
        };
        let Some(entry) = level.entries.iter().find(|entry| entry.key == typed) else {
            return Outcome::Open;
        };
        match self.pick(entry) {
            Picked::Run => Outcome::Close,
            Picked::Enter => {
                local.enter(&entry.key);
                Outcome::Open
            }
        }
    }
}
