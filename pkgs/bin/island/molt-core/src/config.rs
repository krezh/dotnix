use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Config {
    pub flake_dir: PathBuf,
    #[serde(default)]
    pub home_flake: Option<String>,
    pub nixos_flake: String,
    #[serde(default)]
    pub cleanup: CleanupPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CleanupPolicy {
    #[serde(default = "default_keep_generations")]
    pub keep_generations: usize,
    #[serde(default = "default_journal_max_age")]
    pub journal_max_age: String,
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


pub fn load_config() -> Result<Config, String> {
    let path = Path::new("/etc/molt/molt.toml");
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    parse_config(&content)
}

pub fn parse_config(content: &str) -> Result<Config, String> {
    let mut config: Config =
        toml::from_str(content).map_err(|error| format!("invalid Molt configuration: {error}"))?;
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
    pub fn validate(&self) -> Result<(), String> {
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
                nixos_flake = "host \"quoted\""
                sans_font = "left over from the GTK app"
                rounding = 12
            "#,
        )
        .unwrap();
        assert_eq!(config.nixos_flake, "host \"quoted\"");
        assert_eq!(config.cleanup.keep_generations, 0);
        assert_eq!(config.cleanup.journal_max_age, "0");
        assert!(
            parse_config(
                r#"flake_dir = "/tmp/flake"
                   nixos_flake = """#
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
