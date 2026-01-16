//! CLI argument parsing and configuration merging

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};

use crate::config::Config;

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

    /// Disable snap animation
    #[arg(long)]
    pub no_animation: bool,

    /// Enable OCR mode (extract text from selected region)
    #[arg(long)]
    pub ocr: bool,

    /// Screenshot output file path (use '-' for stdout in PNG format)
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// Generate default config file and exit
    #[arg(long)]
    pub generate_config: bool,

    /// Generate shell completion script and exit
    #[arg(long, value_name = "SHELL", value_enum)]
    pub generate_completions: Option<Shell>,
}

impl Args {
    /// Merges CLI arguments with config file settings.
    ///
    /// Applies priority order: CLI args > config file > hardcoded defaults.
    /// Ensures all `Option` fields are populated with concrete values.
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
        if !self.no_animation && config.features.no_animation {
            self.no_animation = true;
        }

        self
    }

    /// Generates shell completions to stdout.
    pub fn generate_completions(shell: Shell) {
        let mut cmd = Self::command();
        let bin_name = cmd.get_name().to_string();
        generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
    }
}

/// Parses a log level string into the corresponding filter level.
pub fn parse_log_level(level: &str) -> log::LevelFilter {
    match level.to_lowercase().as_str() {
        "off" => log::LevelFilter::Off,
        "info" => log::LevelFilter::Info,
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "debug" => log::LevelFilter::Debug,
        _ => log::LevelFilter::Off,
    }
}
