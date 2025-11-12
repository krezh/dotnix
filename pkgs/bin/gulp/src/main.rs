//! gulp: Enhanced region selector for Wayland
//!
//! A compositor-agnostic screen selection tool with features like:
//! - Window snapping and hover detection (works with Hyprland, Niri, and other compositors)
//! - Fullscreen window support (can select windows even in fullscreen mode)
//! - Smooth animations
//! - Configurable appearance
//! - Multi-monitor support

mod animation;
mod config;
mod ocr;
mod render;
mod selection;
mod wayland;
mod windows;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use config::Config;
use std::io;

/// Command-line arguments for gulp
///
/// These override config file settings when specified.
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Text font family
    #[arg(long)]
    pub font_family: Option<String>,

    /// Text Font size
    #[arg(long)]
    pub font_size: Option<u32>,

    /// Text Font weight (Options: Normal, Bold)
    #[arg(long)]
    pub font_weight: Option<String>,

    /// Border color in hex
    #[arg(short, long)]
    pub border_color: Option<String>,

    /// Border thickness in pixels
    #[arg(short = 't', long)]
    pub border_thickness: Option<u32>,

    /// Border rounding in pixels (for rounded corners)
    #[arg(short = 'r', long)]
    pub border_rounding: Option<u32>,

    /// Dimming opacity (0.0-1.0)
    #[arg(short, long)]
    pub dim_opacity: Option<f64>,

    /// Log level (off, info, debug, warn, error)
    #[arg(short = 'l', long)]
    pub log: Option<String>,

    /// Maximum frames per second (0 = auto-detect from monitor)
    #[arg(long)]
    pub fps: Option<u32>,

    /// Disable window snapping
    #[arg(long)]
    pub no_snap: bool,

    /// Enable OCR mode (extract text from selected region)
    #[arg(long)]
    pub ocr: bool,

    /// Generate default config file and exit
    #[arg(long)]
    pub generate_config: bool,

    /// Generate shell completion script and exit
    #[arg(long, value_name = "SHELL", value_enum)]
    pub generate_completions: Option<Shell>,
}

impl Args {
    /// Merges CLI arguments with config file settings
    ///
    /// Priority order: CLI args > config file > hardcoded defaults
    /// This ensures all Option fields are populated with actual values.
    pub fn merge_with_config(mut self, config: Config) -> Self {
        // Helper macro to reduce repetition
        macro_rules! merge_option {
            ($field:expr, $config_value:expr) => {
                if $field.is_none() {
                    $field = Some($config_value);
                }
            };
        }

        // Merge font settings
        merge_option!(self.font_family, config.font.family);
        merge_option!(self.font_size, config.font.size);
        merge_option!(self.font_weight, config.font.weight);

        // Merge border settings
        merge_option!(self.border_color, config.border.color);
        merge_option!(self.border_thickness, config.border.thickness);
        merge_option!(self.border_rounding, config.border.rounding);

        // Merge display settings
        merge_option!(self.dim_opacity, config.display.dim_opacity);
        merge_option!(self.log, config.display.log);
        merge_option!(self.fps, config.display.fps);

        // Merge feature flags (boolean needs special handling)
        if !self.no_snap && config.features.no_snap {
            self.no_snap = true;
        }

        self
    }
}

/// Parses log level string into appropriate filter level
fn parse_log_level(level: &str) -> log::LevelFilter {
    match level.to_lowercase().as_str() {
        "off" => log::LevelFilter::Off,
        "info" => log::LevelFilter::Info,
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "debug" => log::LevelFilter::Debug,
        _ => log::LevelFilter::Off,
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Handle shell completion generation
    if let Some(shell) = args.generate_completions {
        let mut cmd = Args::command();
        let bin_name = cmd.get_name().to_string();
        generate(shell, &mut cmd, bin_name, &mut io::stdout());
        return Ok(());
    }

    // Handle config generation early (doesn't need the rest of the setup)
    if args.generate_config {
        let config_path = Config::write_defaults_to_file(None)?;
        println!("Default config file created at: {}", config_path.display());
        return Ok(());
    }

    // Load config from file or use defaults if loading fails
    let config = Config::load().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load config: {}. Using defaults.", e);
        Config::default()
    });

    // Merge CLI args with config (CLI overrides config)
    let args = args.merge_with_config(config);

    // Initialize logging system
    let log_level = args.log.as_deref().unwrap_or("off");
    env_logger::Builder::from_default_env()
        .filter_level(parse_log_level(log_level))
        .init();

    log::info!("Starting gulp with args: {:?}", args);

    // Run the Wayland application (blocks until selection or cancel)
    wayland::App::new(args)?;

    Ok(())
}
