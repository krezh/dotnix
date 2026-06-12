//! Configuration file handling for chomp
//!
//! Supports loading from JSON files in XDG-compliant locations:
//! - $XDG_CONFIG_HOME/chomp/config.json
//! - ~/.config/chomp/config.json (fallback)
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Font weight options.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, clap::ValueEnum)]
pub enum FontWeight {
    Normal,
    #[default]
    Bold,
}

impl FontWeight {
    /// Converts the font weight to Cairo's FontWeight type.
    pub fn to_cairo(self) -> cairo::FontWeight {
        match self {
            Self::Normal => cairo::FontWeight::Normal,
            Self::Bold => cairo::FontWeight::Bold,
        }
    }
}

/// Keybindings for the mode selector overlay
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct KeybindsConfig {
    pub screenshot_area: String,
    pub screenshot_screen: String,
    pub screenshot_window: String,
    pub ocr: String,
    pub record_area: String,
    pub record_screen: String,
    pub record_window: String,
    pub stop_recording: String,
}

impl Default for KeybindsConfig {
    fn default() -> Self {
        Self {
            screenshot_area: "a".to_string(),
            screenshot_screen: "s".to_string(),
            screenshot_window: "w".to_string(),
            ocr: "c".to_string(),
            record_area: "A".to_string(),
            record_screen: "S".to_string(),
            record_window: "W".to_string(),
            stop_recording: "x".to_string(),
        }
    }
}

/// Visual style for the mode selector bottom bar
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct ModeSelectConfig {
    /// Bar background color (hex)
    pub background_color: String,
    /// Bar background opacity (0.0–1.0)
    pub background_opacity: f64,
    /// Bar height in pixels
    pub bar_height: u32,
    /// Top border opacity (applied to the existing border_color)
    pub border_opacity: f64,
    /// Key label color (hex); empty string falls back to border_color
    pub key_color: String,
    /// Description text color (hex)
    pub description_color: String,
    /// Description text opacity (0.0–1.0)
    pub description_opacity: f64,
    /// Group separator opacity (0.0–1.0)
    pub separator_opacity: f64,
    /// Color of the recording-active indicator dot (hex)
    pub recording_dot_color: String,
    /// Color of the stop-recording key label and description (hex)
    pub recording_highlight_color: String,
}

impl Default for ModeSelectConfig {
    fn default() -> Self {
        Self {
            background_color: "#0D0D14".to_string(),
            background_opacity: 0.95,
            bar_height: 56,
            border_opacity: 0.35,
            key_color: String::new(), // empty = use border_color
            description_color: "#FFFFFF".to_string(),
            description_opacity: 0.85,
            separator_opacity: 0.18,
            recording_dot_color: "#F24040".to_string(),
            recording_highlight_color: "#F2BF33".to_string(),
        }
    }
}

/// Main configuration structure with nested groups
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct Config {
    /// Text/font configuration
    pub font: FontConfig,

    /// Border configuration
    pub border: BorderConfig,

    /// Display configuration
    pub display: DisplayConfig,

    /// Upload configuration
    pub upload: UploadConfig,

    /// Capture configuration
    pub capture: CaptureConfig,

    /// Annotation configuration
    pub annotate: AnnotateConfig,

    /// Mode selector keybindings
    pub keybinds: KeybindsConfig,

    /// Mode selector bar appearance
    pub mode_select: ModeSelectConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct FontConfig {
    /// Font family
    pub family: String,

    /// Font size
    pub size: u32,

    /// Font weight
    pub weight: FontWeight,
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

    /// Log level (off, info, debug, warn, error)
    pub log: String,

    /// Freeze screen before selection (captures snapshot)
    pub freeze: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct UploadConfig {
    /// Zipline upload settings
    pub zipline: ZiplineConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct ZiplineConfig {
    /// Zipline server URL (auto-uploads if both url and token are set)
    pub url: String,

    /// Path to Zipline token file (e.g., "~/.config/zipline/token")
    pub token: String,

    /// Use original filename on Zipline
    pub use_original_name: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct CaptureConfig {
    /// Default save directory for captures
    pub save_path: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct AnnotateConfig {
    /// Path to the satty binary used for annotation
    pub satty_path: String,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            save_path: "/tmp".to_string(),
        }
    }
}

impl Default for AnnotateConfig {
    fn default() -> Self {
        Self {
            satty_path: "satty".to_string(),
        }
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "Inter".to_string(),
            size: 16,
            weight: FontWeight::Bold,
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
            log: "off".to_string(),
            freeze: true,
        }
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

    /// Writes a config instance to file, creating parent directories as needed.
    pub fn write_config_to_file(config: &Self, path: Option<PathBuf>) -> Result<PathBuf> {
        let config_path = path.unwrap_or_else(Self::default_config_path);

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).context(format!(
                "Failed to create config directory: {}",
                parent.display()
            ))?;
        }

        let json_content = serde_json::to_string_pretty(config)
            .context("Failed to serialize config to JSON")?;

        fs::write(&config_path, json_content).context(format!(
            "Failed to write config file to: {}",
            config_path.display()
        ))?;

        Ok(config_path)
    }

    /// Writes the default config to file (convenience wrapper around write_config_to_file).
    pub fn write_defaults_to_file(path: Option<PathBuf>) -> Result<PathBuf> {
        Self::write_config_to_file(&Self::default(), path)
    }

    /// Searches for config file in XDG-compliant locations
    ///
    /// Priority order:
    /// 1. $XDG_CONFIG_HOME/chomp/config.json
    /// 2. ~/.config/chomp/config.json
    fn find_config_file() -> Option<PathBuf> {
        const CONFIG_FILE: &str = "chomp/config.json";

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
        const CONFIG_FILE: &str = "chomp/config.json";

        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(|dir| PathBuf::from(dir).join(CONFIG_FILE))
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|home| PathBuf::from(home).join(".config").join(CONFIG_FILE))
            })
            .unwrap_or_else(|| PathBuf::from("~/.config/chomp/config.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
