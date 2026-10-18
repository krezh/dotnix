use serde::Deserialize;

use crate::features::parse_settings;

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Settings {
    pub(super) terminal: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            terminal: vec!["xdg-terminal-exec".to_owned()],
        }
    }
}

impl Settings {
    pub(super) fn check(&self) -> Result<(), String> {
        if self
            .terminal
            .first()
            .is_none_or(|program| program.trim().is_empty())
        {
            return Err(
                "terminal must contain a nonempty program followed by optional arguments"
                    .to_owned(),
            );
        }
        Ok(())
    }
}

pub(super) fn check(settings: &toml::Value) -> Result<(), String> {
    parse_settings::<Settings>(settings)?.check()
}
