///! Configuration file handling for hypr-slurp
///!
///! Supports loading from JSON files in XDG-compliant locations:
///! - $XDG_CONFIG_HOME/hypr-slurp/config.json
///! - ~/.config/hypr-slurp/config.json (fallback)
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main configuration structure with nested groups
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct Config {
    /// Text/font configuration
    pub font: FontConfig,

    /// Border configuration
    pub border: BorderConfig,

    /// Display configuration
    pub display: DisplayConfig,

    /// Feature flags
    pub features: FeatureConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct FontConfig {
    /// Font family
    pub family: String,

    /// Font size
    pub size: u32,

    /// Font weight (Normal, Bold)
    pub weight: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct BorderConfig {
    /// Border color in hex
    pub color: String,

    /// Border thickness in pixels
    pub thickness: u32,

    /// Border rounding in pixels
    pub rounding: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct DisplayConfig {
    /// Dimming opacity (0.0-1.0)
    pub dim_opacity: f64,

    /// Maximum frames per second (0 = auto-detect)
    pub fps: u32,

    /// Log level (off, info, debug, warn, error)
    pub log: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct FeatureConfig {
    /// Disable window snapping
    pub no_snap: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font: FontConfig::default(),
            border: BorderConfig::default(),
            display: DisplayConfig::default(),
            features: FeatureConfig::default(),
        }
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "Inter Nerd Font".to_string(),
            size: 16,
            weight: "Bold".to_string(),
        }
    }
}

impl Default for BorderConfig {
    fn default() -> Self {
        Self {
            color: "#FFFFFF".to_string(),
            thickness: 2,
            rounding: 0,
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            dim_opacity: 0.5,
            fps: 0,
            log: "off".to_string(),
        }
    }
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self { no_snap: false }
    }
}

impl Config {
    /// Loads configuration from file, falling back to defaults if not found
    ///
    /// Searches for config.json in XDG-compliant locations.
    /// Returns default config if no file is found (not an error).
    pub fn load() -> Result<Self> {
        match Self::find_config_file() {
            Some(path) => {
                log::info!("Loading config from: {}", path.display());
                let content = fs::read_to_string(&path)
                    .context(format!("Failed to read config file: {}", path.display()))?;

                serde_json::from_str(&content)
                    .context(format!("Failed to parse config file: {}", path.display()))
            }
            None => {
                log::info!("No config file found, using defaults");
                Ok(Self::default())
            }
        }
    }

    /// Writes default configuration to file
    ///
    /// Creates the config directory if it doesn't exist.
    /// Returns the path where the config was written.
    pub fn write_defaults_to_file(path: Option<PathBuf>) -> Result<PathBuf> {
        let config_path = path.unwrap_or_else(Self::default_config_path);

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).context(format!(
                "Failed to create config directory: {}",
                parent.display()
            ))?;
        }

        // Serialize default config to pretty JSON
        let json_content = serde_json::to_string_pretty(&Self::default())
            .context("Failed to serialize default config to JSON")?;

        fs::write(&config_path, json_content).context(format!(
            "Failed to write config file to: {}",
            config_path.display()
        ))?;

        Ok(config_path)
    }

    /// Searches for config file in XDG-compliant locations
    ///
    /// Priority order:
    /// 1. $XDG_CONFIG_HOME/hypr-slurp/config.json
    /// 2. ~/.config/hypr-slurp/config.json
    fn find_config_file() -> Option<PathBuf> {
        const CONFIG_FILE: &str = "hypr-slurp/config.json";

        // Try XDG_CONFIG_HOME first
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(|dir| PathBuf::from(dir).join(CONFIG_FILE))
            .filter(|path| path.exists())
            .or_else(|| {
                // Fall back to ~/.config
                std::env::var("HOME")
                    .ok()
                    .map(|home| PathBuf::from(home).join(".config").join(CONFIG_FILE))
                    .filter(|path| path.exists())
            })
    }

    /// Returns the default path where config file should be created
    ///
    /// Uses XDG_CONFIG_HOME if set, otherwise ~/.config
    pub fn default_config_path() -> PathBuf {
        const CONFIG_FILE: &str = "hypr-slurp/config.json";

        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(|dir| PathBuf::from(dir).join(CONFIG_FILE))
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|home| PathBuf::from(home).join(".config").join(CONFIG_FILE))
            })
            .unwrap_or_else(|| PathBuf::from("~/.config/hypr-slurp/config.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.font.family, "Inter Nerd Font");
        assert_eq!(config.font.size, 16);
        assert_eq!(config.border.color, "#FFFFFF");
        assert_eq!(config.display.dim_opacity, 0.5);
    }

    #[test]
    fn test_parse_config() {
        let json_str = r##"{
  "font": {
    "family": "JetBrains Mono",
    "size": 14
  },
  "border": {
    "color": "#FF0000"
  },
  "display": {
    "dim_opacity": 0.7
  }
}"##;

        let config: Config = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.font.family, "JetBrains Mono");
        assert_eq!(config.font.size, 14);
        assert_eq!(config.border.color, "#FF0000");
        assert_eq!(config.display.dim_opacity, 0.7);
        // Defaults should still work for unspecified values
        assert_eq!(config.border.thickness, 2);
    }
}
