use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
thread_local! {
    static HOST_OVERRIDE: RefCell<Option<String>> = const { RefCell::new(None) };
    static KEYBINDS_OVERRIDE: RefCell<Option<bool>> = const { RefCell::new(None) };
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Config {
    pub(crate) flake_dir: PathBuf,
    #[serde(default)]
    pub(crate) home_flake: Option<String>,
    pub(crate) nixos_flake: String,
    #[serde(default = "default_sans_font")]
    pub(crate) sans_font: String,
    #[serde(default = "default_mono_font")]
    pub(crate) mono_font: String,
    #[serde(default = "default_rounding")]
    pub(crate) rounding: i32,
    #[serde(default = "default_keybinds")]
    pub(crate) keybinds: bool,
    #[serde(default)]
    pub(crate) cleanup: CleanupPolicy,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct CleanupPolicy {
    #[serde(default = "default_keep_generations")]
    pub(crate) keep_generations: usize,
    #[serde(default = "default_journal_max_age")]
    pub(crate) journal_max_age: String,
}

#[derive(Clone, Debug)]
pub(crate) struct Appearance {
    pub(crate) sans_font: String,
    pub(crate) mono_font: String,
    pub(crate) rounding: i32,
    pub(crate) keybinds: bool,
}

fn default_sans_font() -> String {
    "sans-serif".to_owned()
}

fn default_mono_font() -> String {
    "monospace".to_owned()
}

const fn default_rounding() -> i32 {
    15
}

const fn default_keybinds() -> bool {
    true
}
const fn default_keep_generations() -> usize {
    0
}

fn default_journal_max_age() -> String {
    "0".to_owned()
}

impl Default for CleanupPolicy {
    fn default() -> Self {
        Self {
            keep_generations: default_keep_generations(),
            journal_max_age: default_journal_max_age(),
        }
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            sans_font: default_sans_font(),
            mono_font: default_mono_font(),
            rounding: default_rounding(),
            keybinds: default_keybinds(),
        }
    }
}
pub(crate) fn set_host_override(host: Option<String>) {
    HOST_OVERRIDE.with(|override_host| override_host.replace(host));
}

pub(crate) fn set_keybinds_override(keybinds: Option<bool>) {
    KEYBINDS_OVERRIDE.with(|override_keybinds| override_keybinds.replace(keybinds));
}

pub(crate) fn load_config() -> Result<Config, String> {
    let path = Path::new("/etc/swix/swix.toml");
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut config = parse_config(&content)?;
    if let Some(host) = HOST_OVERRIDE.with(|host| host.borrow().clone()) {
        config.nixos_flake = host;
    }
    if let Some(keybinds) = KEYBINDS_OVERRIDE.with(|k| *k.borrow()) {
        config.keybinds = keybinds;
    }
    Ok(config)
}

pub(crate) fn parse_config(content: &str) -> Result<Config, String> {
    let mut config: Config =
        toml::from_str(content).map_err(|error| format!("invalid Swix configuration: {error}"))?;
    if config.nixos_flake.trim().is_empty() {
        return Err("nixos_flake must not be empty".to_owned());
    }
    if config
        .home_flake
        .as_ref()
        .is_some_and(|name| name.trim().is_empty())
    {
        return Err("home_flake must not be empty when configured".to_owned());
    }
    if !(0..=64).contains(&config.rounding) {
        return Err("rounding must be between 0 and 64".to_owned());
    }
    config.cleanup.validate()?;
    let flake_dir = config.flake_dir.to_string_lossy();
    if flake_dir == "$HOME"
        || flake_dir == "~"
        || flake_dir.starts_with("$HOME/")
        || flake_dir.starts_with("~/")
    {
        let home = env::var_os("HOME").ok_or("HOME is unset but flake_dir uses it")?;
        config.flake_dir = expand_home(&flake_dir, Path::new(&home));
    }
    Ok(config)
}

fn expand_home(value: &str, home: &Path) -> PathBuf {
    if matches!(value, "$HOME" | "~") {
        return home.to_owned();
    }
    value
        .strip_prefix("$HOME/")
        .or_else(|| value.strip_prefix("~/"))
        .map_or_else(|| PathBuf::from(value), |rest| home.join(rest))
}

impl CleanupPolicy {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.keep_generations > 100 {
            return Err("cleanup.keep_generations must be between 0 and 100".to_owned());
        }
        if !valid_journal_max_age(&self.journal_max_age) {
            return Err(
                "cleanup.journal_max_age must be 0 or a positive duration such as 14d, 12h, or 30min"
                    .to_owned(),
            );
        }
        Ok(())
    }
}

fn valid_journal_max_age(value: &str) -> bool {
    if value == "0" {
        return true;
    }
    let Some(unit_start) = value.find(|character: char| !character.is_ascii_digit()) else {
        return false;
    };
    let (amount, unit) = value.split_at(unit_start);
    !amount.is_empty()
        && amount.parse::<u64>().is_ok_and(|amount| amount > 0)
        && matches!(unit, "s" | "min" | "h" | "d" | "w" | "month" | "y")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_toml_escaping_and_validates_values() {
        let config = parse_config(
            r#"
                flake_dir = "/tmp/flake"
                nixos_flake = "host"
                sans_font = "Font \"Quoted\""
                mono_font = "Mono"
                rounding = 12
            "#,
        )
        .unwrap();
        assert_eq!(config.sans_font, "Font \"Quoted\"");
        assert_eq!(config.rounding, 12);
        assert!(config.keybinds);
        assert_eq!(config.cleanup.keep_generations, 0);
        assert_eq!(config.cleanup.journal_max_age, "0");
        let disabled_config = parse_config(
            r#"
                flake_dir = "/tmp/flake"
                nixos_flake = "host"
                keybinds = false
            "#,
        )
        .unwrap();
        assert!(!disabled_config.keybinds);
        assert!(
            parse_config(
                r#"flake_dir = "/tmp/flake"
                   nixos_flake = ""
                   rounding = 100"#
            )
            .is_err()
        );
        assert!(
            parse_config(
                r#"flake_dir = "/tmp/flake"
                   nixos_flake = "host"
                   [cleanup]
                   keep_generations = 0
                   journal_max_age = "--vacuum-size=1""#
            )
            .is_err()
        );
    }
}
