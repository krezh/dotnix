use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
thread_local! {
    static HOST_OVERRIDE: RefCell<Option<String>> = const { RefCell::new(None) };
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
}

#[derive(Clone, Debug)]
pub(crate) struct Appearance {
    pub(crate) sans_font: String,
    pub(crate) mono_font: String,
    pub(crate) rounding: i32,
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

impl Default for Appearance {
    fn default() -> Self {
        Self {
            sans_font: default_sans_font(),
            mono_font: default_mono_font(),
            rounding: default_rounding(),
        }
    }
}
pub(crate) fn set_host_override(host: Option<String>) {
    HOST_OVERRIDE.with(|override_host| override_host.replace(host));
}

pub(crate) fn load_config() -> Result<Config, String> {
    let path = Path::new("/etc/swix/swix.toml");
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut config = parse_config(&content)?;
    if let Some(host) = HOST_OVERRIDE.with(|host| host.borrow().clone()) {
        config.nixos_flake = host;
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
