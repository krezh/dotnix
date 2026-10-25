use std::{fs, path::PathBuf};

use serde::Deserialize;

use super::state::Endpoint;
use crate::features::parse_settings;

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub url: String,
    pub token_file: PathBuf,
    pub min_priority: i64,
    pub auto_delete: bool,
}

impl Settings {
    pub fn is_enabled(&self) -> bool {
        !self.url.trim().is_empty() || !self.token_file.as_os_str().is_empty()
    }

    pub fn check(&self) -> Result<(), String> {
        if !self.is_enabled() {
            return Ok(());
        }
        Endpoint::parse(&self.url)?;
        if self.token_file.as_os_str().is_empty() {
            return Err("token_file must not be empty".to_owned());
        }
        Ok(())
    }

    pub fn read_token(&self) -> Result<String, String> {
        let raw = fs::read_to_string(&self.token_file).map_err(|error| {
            format!(
                "cannot read Gotify token from {}: {error}",
                self.token_file.display()
            )
        })?;
        let token = raw.trim();
        if token.is_empty() {
            return Err(format!(
                "Gotify token file {} is empty",
                self.token_file.display()
            ));
        }
        if token.bytes().any(|byte| byte < 0x20 || byte == 0x7f) {
            return Err(format!(
                "Gotify token file {} contains control characters",
                self.token_file.display()
            ));
        }
        Ok(token.to_owned())
    }
}

pub fn check(settings: &toml::Value) -> Result<(), String> {
    parse_settings::<Settings>(settings).and_then(|settings| settings.check())
}

#[cfg(test)]
#[path = "settings_test.rs"]
mod tests;
