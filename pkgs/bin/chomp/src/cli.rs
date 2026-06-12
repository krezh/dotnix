//! CLI argument parsing and configuration merging

use clap::{CommandFactory, Parser};
use clap_complete::{Shell, generate};

use crate::capture::CaptureMode;
use crate::config::{Config, FontWeight, KeybindsConfig, LogLevel, ModeSelectConfig};

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

    /// Log level
    #[arg(short = 'l', long, value_enum)]
    pub log: Option<LogLevel>,

    /// Delay in milliseconds before starting capture
    #[arg(long)]
    pub delay: Option<u64>,

    /// Freeze screen before selection (captures snapshot)
    #[arg(long)]
    pub freeze: Option<bool>,

    /// Enable OCR mode (extract text from selected region)
    #[arg(long)]
    pub ocr: bool,

    /// Annotate the screenshot with satty before saving/uploading
    #[arg(short = 'a', long)]
    pub annotate: bool,

    /// Path to the satty binary (overrides config)
    #[arg(long)]
    pub satty_path: Option<String>,

    /// Screenshot output file path (use '-' for stdout in PNG format)
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// Capture mode
    #[arg(long, short = 'm', value_enum)]
    pub mode: Option<CaptureMode>,

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

    /// Overwrite existing config file when used with --generate-config
    #[arg(long)]
    pub force: bool,

    /// Generate shell completion script and exit
    #[arg(long, value_name = "SHELL", value_enum)]
    pub generate_completions: Option<Shell>,
}

/// Effective settings after merging CLI arguments with the config file.
///
/// Priority order: CLI args > config file > hardcoded defaults.
#[derive(Debug, Clone)]
pub struct Settings {
    pub font_family: String,
    pub font_size: u32,
    pub font_weight: FontWeight,
    pub border_color: String,
    pub border_thickness: u32,
    pub border_rounding: u32,
    pub dim_opacity: f64,
    pub log: LogLevel,
    pub delay: Option<u64>,
    pub freeze: bool,
    pub ocr: bool,
    pub annotate: bool,
    pub satty_path: String,
    pub output: Option<String>,
    pub mode: Option<CaptureMode>,
    pub zipline_url: String,
    pub zipline_token: String,
    pub original_name: bool,
    pub save_path: String,
    pub keybinds: KeybindsConfig,
    pub mode_select: ModeSelectConfig,
}

impl Args {
    /// Merges CLI arguments with config file settings into resolved settings.
    pub fn resolve(self, config: Config) -> Settings {
        Settings {
            font_family: self.font_family.unwrap_or(config.font.family),
            font_size: self.font_size.unwrap_or(config.font.size),
            font_weight: self.font_weight.unwrap_or(config.font.weight),
            border_color: self.border_color.unwrap_or(config.border.color),
            border_thickness: self.border_thickness.unwrap_or(config.border.thickness),
            border_rounding: self.border_rounding.unwrap_or(config.border.rounding),
            dim_opacity: self.dim_opacity.unwrap_or(config.display.dim_opacity),
            log: self.log.unwrap_or(config.display.log),
            delay: self.delay,
            freeze: self.freeze.unwrap_or(config.display.freeze),
            ocr: self.ocr,
            annotate: self.annotate,
            satty_path: self.satty_path.unwrap_or(config.annotate.satty_path),
            output: self.output,
            mode: self.mode,
            zipline_url: self.zipline_url.unwrap_or(config.upload.zipline.url),
            zipline_token: self.zipline_token.unwrap_or(config.upload.zipline.token),
            original_name: self
                .original_name
                .unwrap_or(config.upload.zipline.use_original_name),
            save_path: self.save_path.unwrap_or(config.capture.save_path),
            keybinds: config.keybinds,
            mode_select: config.mode_select,
        }
    }

    /// Generates shell completions to stdout.
    pub fn generate_completions(shell: Shell) {
        let mut cmd = Self::command();
        let bin_name = cmd.get_name().to_string();
        generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
    }
}
