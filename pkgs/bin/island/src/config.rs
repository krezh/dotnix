use std::{env, fs, path::PathBuf};

use serde::{Deserialize, Deserializer, de::Error as _};

use crate::{
    features::{
        self, Feature, audio::Audio, caffeine::Caffeine, clock::Clock, headset::Headset, media::Media, molt::Molt,
        notifications::Notifications, tray::Tray, workspaces::Workspaces, zowie::Zowie,
    },
    squircle, theme,
};

/// A bar widget, by the layout name its feature gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Widget(pub &'static str);

impl Widget {
    pub fn parse(name: &str) -> Option<Self> {
        features::widget_names().into_iter().find(|widget| *widget == name).map(Widget)
    }
}

impl<'de> Deserialize<'de> for Widget {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Widget::parse(&name).ok_or_else(|| {
            D::Error::custom(format!("unknown widget '{name}'; known: {}", features::widget_names().join(", ")))
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Bar {
    pub left: Vec<Widget>,
    pub middle: Vec<Widget>,
    pub right: Vec<Widget>,
    #[serde(deserialize_with = "number")]
    pub spacing: f32,
}

impl Default for Bar {
    fn default() -> Self {
        Self {
            left: vec![
                Widget(Tray::NAME),
                Widget(Molt::NAME),
                Widget(Headset::NAME),
                Widget(Zowie::NAME),
                Widget(Audio::NAME),
                Widget(Caffeine::NAME),
            ],
            middle: vec![Widget(Workspaces::NAME)],
            right: vec![Widget(Media::NAME), Widget(Clock::NAME), Widget(Notifications::NAME)],
            spacing: 4.0,
        }
    }
}

impl Bar {
    /// Every widget in the layout, left to right.
    pub fn widgets(&self) -> impl Iterator<Item = Widget> + '_ {
        self.left.iter().chain(&self.middle).chain(&self.right).copied()
    }

    pub fn section_of(&self, widget: Widget) -> Option<usize> {
        [&self.left, &self.middle, &self.right]
            .iter()
            .position(|section| section.contains(&widget))
    }
}

fn number<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<f32, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Number {
        Integer(i64),
        Float(f64),
    }
    Ok(match Number::deserialize(deserializer)? {
        Number::Integer(value) => value as f32,
        Number::Float(value) => value as f32,
    })
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Appearance {
    #[serde(deserialize_with = "number")]
    pub opacity: f32,
    #[serde(deserialize_with = "number")]
    pub radius: f32,
    #[serde(deserialize_with = "number")]
    pub power: f32,
    pub icon_theme: String,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            opacity: theme::DEFAULT_OPACITY,
            radius: squircle::DEFAULT_RADIUS,
            power: squircle::DEFAULT_POWER,
            icon_theme: "hicolor".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Visualizer {
    /// Strength of the music glow behind the bar; 0 turns it off.
    #[serde(deserialize_with = "number")]
    pub glow: f32,
}

impl Default for Visualizer {
    fn default() -> Self {
        Self { glow: 0.5 }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Log {
    /// What is logged: a level such as `debug`, or per module or feature like `audio=debug,tray=trace`.
    /// `ISLAND_LOG` overrides it.
    pub level: String,
}

impl Default for Log {
    fn default() -> Self {
        Self {
            level: crate::logging::DEFAULT.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub bar: Bar,
    pub appearance: Appearance,
    pub visualizer: Visualizer,
    pub log: Log,
    /// Every other section, each a feature's by its name, such as `[notifications]`.
    #[serde(flatten)]
    pub features: toml::Table,
}

impl Config {
    /// Checks that every feature section belongs to a feature that takes it.
    fn check(self) -> Result<Self, String> {
        for (section, settings) in &self.features {
            features::check_settings(section, settings)?;
        }
        Ok(self)
    }
}

pub fn default_path() -> Option<PathBuf> {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("island").join("config.toml"))
}

pub fn load(path: Option<PathBuf>) -> Result<Config, String> {
    let explicit = path.is_some();
    let Some(path) = path.or_else(default_path) else {
        return Ok(Config::default());
    };
    match fs::read_to_string(&path) {
        Ok(raw) => toml::from_str::<Config>(&raw)
            .map_err(|error| error.to_string())
            .and_then(Config::check)
            .map_err(|error| format!("{}: {error}", path.display())),
        Err(error) if !explicit && error.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_name_feature_widgets() {
        let bar: Bar = toml::from_str("left = [\"notifications\", \"clock\", \"zowie\"]").unwrap();
        assert_eq!(bar.left, vec![Widget("notifications"), Widget("clock"), Widget("zowie")]);
    }

    #[test]
    fn feature_sections_are_checked() {
        let check = |raw: &str| toml::from_str::<Config>(raw).map_err(|error| error.to_string()).and_then(Config::check);
        assert!(check("[notifications]\nprivate = true").is_ok());
        let typo = check("[notifications]\nprivat = true").unwrap_err();
        assert!(typo.contains("[notifications]"), "{typo}");
        let unknown = check("[notificatons]\nprivate = true").unwrap_err();
        assert!(unknown.contains("unknown section [notificatons]"), "{unknown}");
        let none = check("[caffeine]\non = true").unwrap_err();
        assert!(none.contains("takes no settings"), "{none}");
    }

    #[test]
    fn unknown_widgets_list_the_known_ones() {
        let error = toml::from_str::<Bar>("left = [\"headsett\"]").unwrap_err().to_string();
        assert!(error.contains("unknown widget 'headsett'"), "{error}");
        assert!(error.contains("headset"), "{error}");
        assert!(error.contains("zowie"), "{error}");
    }
}
