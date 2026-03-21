//! CLI argument parsing and configuration merging

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};

use crate::config::{Config, FontWeight};

/// Command-line arguments for chomp
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

    /// Text Font weight
    #[arg(long, value_enum)]
    pub font_weight: Option<FontWeight>,

    /// Border color in hex
    #[arg(short, long)]
    pub border_color: Option<String>,

    /// Border thickness in pixels
    #[arg(long)]
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

    /// Delay in milliseconds before starting capture
    #[arg(long)]
    pub delay: Option<u64>,

    /// Freeze screen before selection (captures snapshot)
    #[arg(long)]
    pub freeze: Option<bool>,

    /// Enable OCR mode (extract text from selected region)
    #[arg(long)]
    pub ocr: bool,

    /// Screenshot output file path (use '-' for stdout in PNG format)
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// Capture mode: image-area, image-window, image-screen, video-area, video-window, video-screen
    #[arg(long, short = 'm')]
    pub mode: Option<String>,

    /// Show recording status
    #[arg(long)]
    pub status: bool,

    /// Zipline server URL (overrides config)
    #[arg(long, short = 'u')]
    pub zipline_url: Option<String>,

    /// Zipline token file path (overrides config)
    #[arg(long, short = 't')]
    pub zipline_token: Option<String>,

    /// Use original filename on Zipline (overrides config)
    #[arg(long)]
    pub original_name: Option<bool>,

    /// Save path directory (overrides config)
    #[arg(long, short = 'p')]
    pub save_path: Option<String>,

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
        // Merge font settings
        self.font_family.get_or_insert(config.font.family);
        self.font_size.get_or_insert(config.font.size);
        self.font_weight.get_or_insert(config.font.weight);

        // Merge border settings
        self.border_color.get_or_insert(config.border.color);
        self.border_thickness.get_or_insert(config.border.thickness);
        self.border_rounding.get_or_insert(config.border.rounding);

        // Merge display settings
        self.dim_opacity.get_or_insert(config.display.dim_opacity);
        self.log.get_or_insert(config.display.log);
        self.freeze.get_or_insert(config.display.freeze);

        // Merge upload settings
        self.zipline_url.get_or_insert(config.upload.zipline.url);
        self.zipline_token.get_or_insert(config.upload.zipline.token);
        self.original_name.get_or_insert(config.upload.zipline.use_original_name);

        // Merge capture settings
        self.save_path.get_or_insert(config.capture.save_path);

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
