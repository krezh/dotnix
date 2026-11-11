use std::collections::{BTreeMap, HashSet};

use serde::Deserialize;

use crate::features::parse_settings;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Settings {
    pub(super) menus: BTreeMap<String, Vec<Entry>>,
}

/// One choice in a menu: the key that picks it, and either a command or a menu of its own.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry {
    pub(super) key: String,
    pub(super) desc: String,
    #[serde(default)]
    pub(super) cmd: Option<String>,
    #[serde(default)]
    pub(super) submenu: Vec<Entry>,
}

impl Settings {
    pub(super) fn check(&self) -> Result<(), String> {
        for (name, entries) in &self.menus {
            check_level(entries, name)?;
        }
        Ok(())
    }
}

fn check_level(entries: &[Entry], at: &str) -> Result<(), String> {
    if entries.is_empty() {
        return Err(format!("menu {at} has no entries"));
    }
    let mut keys = HashSet::new();
    for entry in entries {
        let mut chars = entry.key.chars();
        let one = chars
            .next()
            .filter(|key| !key.is_whitespace() && !key.is_control());
        if one.is_none() || chars.next().is_some() {
            return Err(format!(
                "menu {at}: key {:?} must be one character",
                entry.key
            ));
        }
        if !keys.insert(entry.key.as_str()) {
            return Err(format!("menu {at}: key {:?} is used twice", entry.key));
        }
        if entry.desc.trim().is_empty() {
            return Err(format!("menu {at}: key {:?} needs a desc", entry.key));
        }
        let command = entry
            .cmd
            .as_deref()
            .is_some_and(|cmd| !cmd.trim().is_empty());
        match (command, entry.submenu.is_empty()) {
            (true, true) => {}
            (false, false) => check_level(&entry.submenu, &format!("{at} › {}", entry.desc))?,
            (true, false) => {
                return Err(format!(
                    "menu {at}: key {:?} has both a cmd and a submenu",
                    entry.key
                ));
            }
            (false, true) => {
                return Err(format!(
                    "menu {at}: key {:?} needs a cmd or a submenu",
                    entry.key
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn check(settings: &toml::Value) -> Result<(), String> {
    parse_settings::<Settings>(settings)?.check()
}

#[cfg(test)]
#[path = "settings_test.rs"]
mod tests;
