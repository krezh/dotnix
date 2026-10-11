use std::fs;
use std::path::PathBuf;

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::theme::Theme;

#[derive(Parser, Debug)]
#[command(
    name = "slumber",
    author = "krezh",
    version,
    about = "Fast, reactive Wayland/Hyprland screensaver with Catppuccin themes and animations"
)]
pub struct Cli {
    /// Animation name: waves, starfield, clock, random
    #[arg(short, long)]
    pub animation: Option<String>,

    /// Catppuccin theme flavor: mocha, macchiato, frappe, latte
    #[arg(short, long)]
    pub theme: Option<String>,

    /// Target frame rate
    #[arg(long)]
    pub fps: Option<u32>,

    /// Mouse motion threshold in pixels to trigger wake-up
    #[arg(long)]
    pub motion_threshold: Option<f64>,

    /// Milliseconds to ignore initial mouse motion after launch
    #[arg(long)]
    pub grace_ms: Option<u64>,

    /// Cycle animations every N seconds (0 to disable)
    #[arg(long)]
    pub cycle: Option<u64>,

    /// Path to TOML config file
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Path to custom TOML theme file
    #[arg(long)]
    pub custom_theme: Option<PathBuf>,

    /// List all available animations and exit
    #[arg(long)]
    pub list_animations: bool,

    /// List all built-in themes and exit
    #[arg(long)]
    pub list_themes: bool,

    /// Overlay digital clock on top of the animation
    #[arg(short = 'k', long = "clock", aliases = ["show-clock"])]
    pub show_clock: bool,
    /// Background opacity from 0.0 (see-through) to 1.0 (opaque)
    #[arg(short = 'o', long = "opacity", aliases = ["background-opacity"])]
    pub opacity: Option<f32>,
    /// Font family or path to TTF/OTF font file [default: Rubik]
    #[arg(short = 'f', long = "font", aliases = ["font-family"])]
    pub font: Option<String>,
    /// Animation speed multiplier [default: 1.0]
    #[arg(short = 's', long = "speed")]
    pub speed: Option<f32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConfig {
    #[serde(default = "default_animation")]
    pub animation: String,

    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default = "default_fps")]
    pub fps: u32,

    #[serde(default = "default_motion_threshold")]
    pub motion_threshold: f64,

    #[serde(default = "default_grace_ms")]
    pub grace_ms: u64,

    #[serde(default)]
    pub cycle: u64,
    #[serde(default)]
    pub show_clock: bool,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default = "default_font")]
    pub font: String,
    #[serde(default = "default_speed")]
    pub speed: f32,
}

fn default_animation() -> String {
    "random".into()
}

fn default_theme() -> String {
    "mocha".into()
}
fn default_font() -> String {
    "Rubik".into()
}
fn default_speed() -> f32 {
    1.0
}



fn default_fps() -> u32 {
    60
}

fn default_motion_threshold() -> f64 {
    5.0
}

fn default_opacity() -> f32 {
    1.0
}

fn default_grace_ms() -> u64 {
    200
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            animation: default_animation(),
            theme: default_theme(),
            fps: default_fps(),
            motion_threshold: default_motion_threshold(),
            grace_ms: default_grace_ms(),
            cycle: 0,
            show_clock: false,
            opacity: default_opacity(),
            font: default_font(),
            speed: default_speed(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub animation: String,
    pub theme: Theme,
    pub fps: u32,
    pub motion_threshold: f64,
    pub grace_ms: u64,
    pub cycle: u64,
    pub show_clock: bool,
    pub font: String,
    pub speed: f32,
}

impl AppConfig {
    pub fn load(cli: &Cli) -> Self {
        let config_path = cli.config.clone().or_else(|| {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
                .map(|p| p.join("slumber").join("config.toml"))
        });

        let file_cfg: FileConfig = if let Some(path) = config_path {
            if path.exists() {
                match fs::read_to_string(&path) {
                    Ok(content) => toml::from_str(&content).unwrap_or_default(),
                    Err(_) => FileConfig::default(),
                }
            } else {
                FileConfig::default()
            }
        } else {
            FileConfig::default()
        };

        let animation = cli
            .animation
            .clone()
            .unwrap_or_else(|| file_cfg.animation.clone());

        let theme_name = cli.theme.clone().unwrap_or_else(|| file_cfg.theme.clone());

        let mut theme = if let Some(custom_path) = &cli.custom_theme {
            if let Ok(content) = fs::read_to_string(custom_path) {
                toml::from_str(&content).unwrap_or_else(|_| Theme::mocha())
            } else {
                Theme::mocha()
            }
        } else {
            Theme::from_name(&theme_name).unwrap_or_else(Theme::mocha)
        };

        let opacity = cli.opacity.unwrap_or(file_cfg.opacity).clamp(0.0, 1.0);
        theme = theme.with_background_opacity(opacity);

        let fps = cli.fps.unwrap_or(file_cfg.fps).clamp(15, 240);
        let motion_threshold = cli.motion_threshold.unwrap_or(file_cfg.motion_threshold).max(0.0);
        let grace_ms = cli.grace_ms.unwrap_or(file_cfg.grace_ms);
        let cycle = cli.cycle.unwrap_or(file_cfg.cycle);

        let show_clock = cli.show_clock || file_cfg.show_clock;
        let font = cli.font.clone().unwrap_or_else(|| file_cfg.font.clone());
        let speed = cli.speed.unwrap_or(file_cfg.speed).clamp(0.05, 10.0);
        Self {
            animation,
            theme,
            fps,
            motion_threshold,
            grace_ms,
            cycle,
            show_clock,
            font,
            speed,
        }
    }
}
