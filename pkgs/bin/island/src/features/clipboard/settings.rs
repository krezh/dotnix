use serde::Deserialize;

use super::paste;
use crate::features::parse_settings;

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Settings {
    pub(super) history: History,
    /// Restore the newest entry when the clipboard empties.
    pub(super) persist: bool,
    pub(super) paste: Paste,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            history: History::default(),
            persist: true,
            paste: Paste::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct History {
    pub(super) max_entries: usize,
    pub(super) max_total_mb: u64,
    pub(super) max_item_mb: u64,
    pub(super) images: bool,
}

impl Default for History {
    fn default() -> Self {
        Self {
            max_entries: 200,
            max_total_mb: 256,
            max_item_mb: 32,
            images: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Paste {
    /// Empty only puts a picked entry on the clipboard.
    pub(super) keys: String,
    pub(super) terminal_keys: String,
    /// Window classes.
    pub(super) terminals: Vec<String>,
}

impl Default for Paste {
    fn default() -> Self {
        Self {
            keys: "ctrl+v".to_owned(),
            terminal_keys: "ctrl+shift+v".to_owned(),
            terminals: [
                "com.mitchellh.ghostty",
                "kitty",
                "foot",
                "footclient",
                "Alacritty",
                "org.wezfurlong.wezterm",
                "org.gnome.Ptyxis",
                "org.gnome.Console",
                "org.kde.konsole",
                "xterm",
            ]
            .map(str::to_owned)
            .to_vec(),
        }
    }
}

pub(super) fn check(settings: &toml::Value) -> Result<(), String> {
    let settings: Settings = parse_settings(settings)?;
    paste::Shortcut::parse(&settings.paste.keys).map_err(|error| format!("paste.keys: {error}"))?;
    paste::Shortcut::parse(&settings.paste.terminal_keys)
        .map_err(|error| format!("paste.terminal_keys: {error}"))?;
    Ok(())
}
