use serde::Deserialize;

use crate::features::parse_settings;

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Settings {
    pub(super) privacy: Privacy,
    pub(super) history_days: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            privacy: Privacy::default(),
            history_days: 7,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Privacy {
    pub(super) private: bool,
    pub(super) private_apps: Vec<String>,
    pub(super) hide_on_lock: bool,
}

impl Default for Privacy {
    fn default() -> Self {
        Self {
            private: false,
            private_apps: Vec::new(),
            hide_on_lock: true,
        }
    }
}

pub(super) fn check(settings: &toml::Value) -> Result<(), String> {
    parse_settings::<Settings>(settings).map(drop)
}
