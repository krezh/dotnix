use std::{fs, path::PathBuf, time::Duration};

use serde::Deserialize;

use super::state::Endpoint;
use crate::features::parse_settings;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub url: String,
    pub token_file: PathBuf,
    pub min_priority: i64,
    pub auto_delete: bool,
    /// How often, in seconds, the server is asked what it still has, so messages deleted
    /// elsewhere leave island; 0 never.
    pub sync_seconds: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            url: String::new(),
            token_file: PathBuf::new(),
            min_priority: 0,
            auto_delete: false,
            sync_seconds: DEFAULT_SYNC_SECONDS,
        }
    }
}

const DEFAULT_SYNC_SECONDS: u64 = 30;

impl Settings {
    pub fn sync_interval(&self) -> Option<Duration> {
        (self.sync_seconds > 0).then(|| Duration::from_secs(self.sync_seconds))
    }

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
