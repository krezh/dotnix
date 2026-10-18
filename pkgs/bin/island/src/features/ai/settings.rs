use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use super::state::Source;
use crate::features::parse_settings;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(super) enum Display {
    #[default]
    Worst,
    Providers,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Settings {
    pub(super) sources: Vec<Source>,
    pub(super) disabled_limits: HashMap<Source, HashMap<String, HashSet<String>>>,
    pub(super) refresh_minutes: u64,
    pub(super) widget: Widget,
    pub(super) alerts: Alerts,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sources: vec![Source::Omp, Source::Claude],
            disabled_limits: HashMap::new(),
            refresh_minutes: 5,
            widget: Widget::default(),
            alerts: Alerts::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Widget {
    pub(super) display: Display,
    pub(super) show_badge: bool,
}

impl Default for Widget {
    fn default() -> Self {
        Self {
            display: Display::Worst,
            show_badge: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Alerts {
    pub(super) notifications: bool,
    pub(super) high_usage_threshold: f64,
    pub(super) pace_alerts: bool,
}

impl Default for Alerts {
    fn default() -> Self {
        Self {
            notifications: true,
            high_usage_threshold: 75.0,
            pace_alerts: true,
        }
    }
}

pub(super) fn check(settings: &toml::Value) -> Result<(), String> {
    let settings: Settings = parse_settings(settings)?;
    if !(1..=60).contains(&settings.refresh_minutes) {
        return Err("refresh_minutes must be from 1 to 60".to_owned());
    }
    if !(50.0..=99.0).contains(&settings.alerts.high_usage_threshold) {
        return Err("alerts.high_usage_threshold must be from 50 to 99".to_owned());
    }
    if settings
        .sources
        .iter()
        .enumerate()
        .any(|(index, source)| settings.sources[..index].contains(source))
    {
        return Err("sources must not contain duplicates".to_owned());
    }
    Ok(())
}
