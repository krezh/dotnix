use serde::Deserialize;
use std::{env, fs, io, path::PathBuf};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Appearance {
    pub opacity: f64,
    pub panel_opacity: f64,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            opacity: 0.75,
            panel_opacity: 0.60,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Logout,
    Shutdown,
    Reboot,
}

impl Action {
    fn defaults(self) -> (&'static str, &'static str) {
        match self {
            Self::Logout => ("Logging out...", ""),
            Self::Shutdown => ("Shutting down...", "systemctl poweroff"),
            Self::Reboot => ("Restarting...", "systemctl reboot"),
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ActionConfig {
    top_label: Option<String>,
    command: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Actions {
    logout: ActionConfig,
    shutdown: ActionConfig,
    reboot: ActionConfig,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    dry_run: bool,
    no_exit: bool,
    verbose: bool,
    no_fork: bool,
    vt: Option<u32>,
    appearance: Appearance,
    actions: Actions,
}

impl Config {
    pub fn resolve(self, action: Action) -> Options {
        let selected = match action {
            Action::Logout => self.actions.logout,
            Action::Shutdown => self.actions.shutdown,
            Action::Reboot => self.actions.reboot,
        };
        let (top_label, command) = action.defaults();
        Options {
            dry_run: self.dry_run,
            no_exit: self.no_exit,
            top_label: selected.top_label.unwrap_or_else(|| top_label.into()),
            command: selected
                .command
                .or_else(|| (!command.is_empty()).then(|| command.into()))
                .filter(|command| !command.is_empty()),
            verbose: self.verbose,
            no_fork: self.no_fork,
            vt: self.vt,
            appearance: self.appearance,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub dry_run: bool,
    pub no_exit: bool,
    pub top_label: String,
    pub command: Option<String>,
    pub verbose: bool,
    pub no_fork: bool,
    pub vt: Option<u32>,
    pub appearance: Appearance,
}

impl Options {
    pub fn validate(&self) -> Result<(), String> {
        validate(self.vt, self.appearance)
    }
}

fn validate(vt: Option<u32>, appearance: Appearance) -> Result<(), String> {
    if vt.is_some_and(|vt| !(1..=63).contains(&vt)) {
        return Err("vt must be between 1 and 63".into());
    }
    for (key, value) in [
        ("appearance.opacity", appearance.opacity),
        ("appearance.panel_opacity", appearance.panel_opacity),
    ] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(format!("{key} must be a finite number between 0 and 1"));
        }
    }
    Ok(())
}

pub fn default_path() -> Option<PathBuf> {
    let base = env::var_os("XDG_CONFIG_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            env::var_os("HOME")
                .filter(|path| !path.is_empty())
                .map(|home| PathBuf::from(home).join(".config"))
        })?;
    Some(base.join("dusk/config.toml"))
}

pub fn load(path: Option<PathBuf>, explicit: bool) -> Result<Config, String> {
    let Some(path) = path else {
        return Ok(Config::default());
    };
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if !explicit && error.kind() == io::ErrorKind::NotFound => {
            return Ok(Config::default());
        }
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    let config: Config =
        toml::from_str(&content).map_err(|error| format!("{}: {error}", path.display()))?;
    validate(config.vt, config.appearance)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_keys_and_wrong_types() {
        for content in [
            "dry_rnu = true",
            "dry_run = 'true'",
            "post_cmd = 'systemctl poweroff'",
            "top_label = 'Legacy heading'",
            "command = 'systemctl poweroff'",
            "[actions.logout]\npost_cmd = 'exit 7'",
            "[actions.shutdown]\ncommand = false",
            "[actions.reboot]\nunknown = true",
            "[actions.suspend]\ncommand = ''",
            "vt = -1",
            "[appearance]\npanel_opactiy = 0.5",
            "[appearance]\nopacity = '0.5'",
        ] {
            assert!(toml::from_str::<Config>(content).is_err(), "{content}");
        }
    }

    #[test]
    fn validates_numeric_boundaries_and_nonfinite_opacity() {
        for value in ["-0.1", "1.1", "nan", "inf", "-inf"] {
            for key in ["opacity", "panel_opacity"] {
                let config: Config =
                    toml::from_str(&format!("[appearance]\n{key} = {value}")).unwrap();
                assert!(
                    config.resolve(Action::Logout).validate().is_err(),
                    "{key} = {value}"
                );
            }
        }
        for value in ["0", "1", "0.0", "1.0"] {
            let config: Config = toml::from_str(&format!(
                "[appearance]\nopacity = {value}\npanel_opacity = {value}"
            ))
            .unwrap();
            assert!(config.resolve(Action::Logout).validate().is_ok(), "{value}");
        }
        for (vt, valid) in [(0, false), (1, true), (63, true), (64, false)] {
            let config = Config {
                vt: Some(vt),
                ..Config::default()
            };
            assert_eq!(
                config.resolve(Action::Logout).validate().is_ok(),
                valid,
                "vt = {vt}"
            );
        }
    }

    #[test]
    fn absent_and_partial_actions_use_their_own_defaults() {
        for (action, label, command) in [
            (Action::Logout, "Logging out...", None),
            (
                Action::Shutdown,
                "Shutting down...",
                Some("systemctl poweroff"),
            ),
            (Action::Reboot, "Restarting...", Some("systemctl reboot")),
        ] {
            for content in ["", "[actions.logout]\n[actions.shutdown]\n[actions.reboot]"] {
                let config: Config = toml::from_str(content).unwrap();
                let options = config.resolve(action);
                assert_eq!(options.top_label, label);
                assert_eq!(options.command.as_deref(), command);
            }
            let config: Config = toml::from_str(
                "[actions.logout]\ntop_label = 'Custom'\n\
                 [actions.shutdown]\ntop_label = 'Custom'\n\
                 [actions.reboot]\ntop_label = 'Custom'",
            )
            .unwrap();
            let options = config.resolve(action);
            assert_eq!(options.top_label, "Custom");
            assert_eq!(options.command.as_deref(), command);

            let config: Config = toml::from_str(
                "[actions.logout]\ncommand = 'custom'\n\
                 [actions.shutdown]\ncommand = 'custom'\n\
                 [actions.reboot]\ncommand = 'custom'",
            )
            .unwrap();
            let options = config.resolve(action);
            assert_eq!(options.top_label, label);
            assert_eq!(options.command.as_deref(), Some("custom"));
        }
    }

    #[test]
    fn action_settings_are_independent_and_empty_commands_disable_defaults() {
        let content = "\
            [actions.logout]\ntop_label = 'Bye'\ncommand = 'logout-hook'\n\
            [actions.shutdown]\ntop_label = 'Power off'\ncommand = 'shutdown-hook'\n\
            [actions.reboot]\ntop_label = 'Restart'\ncommand = 'reboot-hook'";
        for (action, label, command) in [
            (Action::Logout, "Bye", "logout-hook"),
            (Action::Shutdown, "Power off", "shutdown-hook"),
            (Action::Reboot, "Restart", "reboot-hook"),
        ] {
            let config: Config = toml::from_str(content).unwrap();
            let options = config.resolve(action);
            assert_eq!(options.top_label, label);
            assert_eq!(options.command.as_deref(), Some(command));
            let config: Config = toml::from_str(
                "[actions.logout]\ncommand = ''\n\
                 [actions.shutdown]\ncommand = ''\n\
                 [actions.reboot]\ncommand = ''",
            )
            .unwrap();
            assert_eq!(config.resolve(action).command, None);
        }
    }
}
