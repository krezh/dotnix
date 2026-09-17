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

/// Log level options.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    #[default]
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    /// Converts the log level to the corresponding log filter.
    pub fn to_filter(self) -> log::LevelFilter {
        match self {
            Self::Off => log::LevelFilter::Off,
            Self::Error => log::LevelFilter::Error,
            Self::Warn => log::LevelFilter::Warn,
            Self::Info => log::LevelFilter::Info,
            Self::Debug => log::LevelFilter::Debug,
            Self::Trace => log::LevelFilter::Trace,
        }
    }
}

/// Keybindings for the mode selector overlay
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default, deny_unknown_fields)]
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

impl KeybindsConfig {
    /// Returns each binding as (name, key).
    pub fn entries(&self) -> [(&'static str, &str); 8] {
        [
            ("screenshot_area", &self.screenshot_area),
            ("screenshot_screen", &self.screenshot_screen),
            ("screenshot_window", &self.screenshot_window),
            ("ocr", &self.ocr),
            ("record_area", &self.record_area),
            ("record_screen", &self.record_screen),
            ("record_window", &self.record_window),
            ("stop_recording", &self.stop_recording),
        ]
    }

    /// Returns the bindings that cannot work, as messages to show the user.
    ///
    /// A binding is one character: chomp has no table of key names, so anything
    /// longer never fires, and two bindings on the same key leave the second one
    /// unreachable. Both are silent at runtime, hence reporting them up front.
    pub fn problems(&self) -> Vec<String> {
        let entries = self.entries();
        let mut problems = Vec::new();

        for (name, key) in entries {
            if key.chars().count() != 1 {
                problems.push(format!(
                    "keybind {} is {:?}, which is not a single key, so it will never fire",
                    name, key
                ));
            }
        }

        for (index, (name, key)) in entries.iter().enumerate() {
            if let Some((earlier, _)) = entries[..index].iter().find(|(_, other)| other == key) {
                problems.push(format!(
                    "keybinds {} and {} are both {:?}; only {} will fire",
                    earlier, name, key, earlier
                ));
            }
        }

        problems
    }
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
#[serde(default, deny_unknown_fields)]
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
#[serde(default, deny_unknown_fields)]
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

    /// OCR configuration
    pub ocr: OcrConfig,

    /// External programs chomp runs
    pub tools: ToolsConfig,

    /// Mode selector keybindings
    pub keybinds: KeybindsConfig,

    /// Mode selector bar appearance
    pub mode_select: ModeSelectConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct FontConfig {
    /// Font family
    pub family: String,

    /// Font size
    pub size: u32,

    /// Font weight
    pub weight: FontWeight,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct BorderConfig {
    /// Border color in hex
    pub color: String,

    /// Border thickness in pixels
    pub thickness: u32,

    /// Border rounding in pixels
    pub rounding: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct DisplayConfig {
    /// Dimming opacity (0.0-1.0)
    pub dim_opacity: f64,

    /// Log level
    pub log: LogLevel,

    /// Freeze screen before selection (captures snapshot)
    pub freeze: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default, deny_unknown_fields)]
pub struct UploadConfig {
    /// Zipline upload settings
    pub zipline: ZiplineConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ZiplineConfig {
    /// Zipline server URL (auto-uploads if both url and token are set)
    pub url: String,

    /// Path to Zipline token file (e.g., "~/.config/zipline/token")
    pub token: String,

    /// Use original filename on Zipline
    pub use_original_name: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct CaptureConfig {
    /// Default save directory for captures
    pub save_path: String,

    /// Delay before capturing, in milliseconds
    pub delay: Option<u64>,

    /// Screen recording settings
    pub video: VideoConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct VideoConfig {
    /// Frame rate ceiling passed to the recorder
    pub max_fps: u32,

    /// Encoder resolution, e.g. "1920x1080". Empty records at the native size.
    pub encode_resolution: String,

    /// Encoder bitrate, in the recorder's own units, e.g. "10 MB" for 80 Mbps.
    /// Empty derives one from the recorded area and frame rate.
    pub bitrate: String,

    /// Video codec: "auto", "avc", "hevc", "vp8", "vp9" or "av1". At a given
    /// bitrate "hevc" holds up better in motion than "avc".
    pub codec: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct OcrConfig {
    /// Tesseract language, which must be installed in its data directory
    pub language: String,
}

/// Paths of the external programs chomp runs.
///
/// Each is resolved through `PATH` when left as a bare name.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct ToolsConfig {
    /// Annotation editor, used by --annotate
    pub satty: String,

    /// Clipboard tool
    pub wl_copy: String,

    /// Screen recorder
    pub wl_screenrec: String,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            save_path: "/tmp".to_string(),
            delay: None,
            video: VideoConfig::default(),
        }
    }
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            max_fps: 60,
            encode_resolution: String::new(),
            bitrate: String::new(),
            codec: "auto".to_string(),
        }
    }
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            language: "eng".to_string(),
        }
    }
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            satty: "satty".to_string(),
            wl_copy: "wl-copy".to_string(),
            wl_screenrec: "wl-screenrec".to_string(),
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
            log: LogLevel::Off,
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

        let json_content =
            serde_json::to_string_pretty(config).context("Failed to serialize config to JSON")?;

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

    /// Returns the config file locations, in priority order:
    ///
    /// 1. `$XDG_CONFIG_HOME/chomp/config.json`
    /// 2. `~/.config/chomp/config.json`
    fn config_paths() -> Vec<PathBuf> {
        const CONFIG_FILE: &str = "chomp/config.json";

        let xdg = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(|dir| PathBuf::from(dir).join(CONFIG_FILE));

        let home = std::env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join(".config").join(CONFIG_FILE));

        xdg.into_iter().chain(home).collect()
    }

    /// Returns the first config file that exists, if any.
    fn find_config_file() -> Option<PathBuf> {
        Self::config_paths().into_iter().find(|path| path.exists())
    }

    /// Returns the path where the config file should be created.
    pub fn default_config_path() -> PathBuf {
        Self::config_paths()
            .into_iter()
            .next()
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
