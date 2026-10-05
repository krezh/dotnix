//! CLI argument parsing and configuration merging

use clap::{CommandFactory, Parser};
use clap_complete::{Shell, generate};

use crate::capture::CaptureMode;
use crate::config::{Config, FontWeight, KeybindsConfig, LogLevel, ModeSelectConfig};
use std::path::PathBuf;

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
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub font_size: Option<u32>,

    /// Text Font weight
    #[arg(long, value_enum)]
    pub font_weight: Option<FontWeight>,

    /// Border color in hex
    #[arg(short, long, value_parser = parse_hex_color)]
    pub border_color: Option<String>,

    /// Border thickness in pixels
    #[arg(long)]
    pub border_thickness: Option<u32>,

    /// Border rounding in pixels (for rounded corners)
    #[arg(short = 'r', long)]
    pub border_rounding: Option<u32>,

    /// Dimming opacity (0.0-1.0)
    #[arg(short, long, value_parser = parse_unit)]
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
    #[arg(long, conflicts_with = "mode")]
    pub ocr: bool,

    /// Annotate the screenshot with satty before saving/uploading
    #[arg(short = 'a', long)]
    pub annotate: bool,

    /// Copy the capture to the clipboard instead of uploading it
    #[arg(short = 'c', long)]
    pub clipboard: bool,

    /// Path to the satty binary (overrides config)
    #[arg(long)]
    pub satty_path: Option<String>,

    /// Screenshot output file path (use '-' for stdout in PNG format)
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,

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
    pub zipline_token: Option<PathBuf>,

    /// Use original filename on Zipline (overrides config)
    #[arg(long)]
    pub original_name: Option<bool>,

    /// Save path directory (overrides config)
    #[arg(long, short = 'p')]
    pub save_path: Option<PathBuf>,

    /// Generate default config file and exit
    #[arg(long)]
    pub generate_config: bool,

    /// Overwrite existing config file when used with --generate-config
    #[arg(long)]
    pub force: bool,

    /// Generate shell completion script and exit
    #[arg(long, value_name = "SHELL", value_enum)]
    pub generate_completions: Option<Shell>,

    /// Internal: show an upload notification and wait for its action button
    #[arg(long, hide = true, num_args = 3, value_names = ["TITLE", "MESSAGE", "URL"])]
    pub await_notification_action: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageDestination {
    SaveOrUpload,
    Clipboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureAction {
    Image,
    Ocr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureRequest {
    pub mode: Option<CaptureMode>,
    pub action: CaptureAction,
    pub destination: ImageDestination,
}

impl CaptureRequest {
    pub fn is_ocr(self) -> bool {
        self.action == CaptureAction::Ocr
    }

    pub fn to_clipboard(self) -> bool {
        self.destination == ImageDestination::Clipboard
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordingOptions {
    pub max_fps: u32,
    pub encode_resolution: String,
    pub bitrate: String,
    pub codec: String,
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
    pub request: CaptureRequest,
    pub annotate: bool,
    pub satty_path: String,
    pub wl_copy: String,
    pub wl_screenrec: String,
    pub ocr_language: String,
    pub recording: RecordingOptions,
    pub output: Option<PathBuf>,
    pub zipline_url: String,
    pub zipline_token: PathBuf,
    pub original_name: bool,
    pub save_path: PathBuf,
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
            delay: self.delay.or(config.capture.delay),
            freeze: self.freeze.unwrap_or(config.display.freeze),
            request: CaptureRequest {
                mode: self.mode,
                action: if self.ocr {
                    CaptureAction::Ocr
                } else {
                    CaptureAction::Image
                },
                destination: if self.clipboard {
                    ImageDestination::Clipboard
                } else {
                    ImageDestination::SaveOrUpload
                },
            },
            annotate: self.annotate,
            satty_path: self.satty_path.unwrap_or(config.tools.satty),
            wl_copy: config.tools.wl_copy,
            wl_screenrec: config.tools.wl_screenrec,
            ocr_language: config.ocr.language,
            recording: RecordingOptions {
                max_fps: config.capture.video.max_fps,
                encode_resolution: config.capture.video.encode_resolution,
                bitrate: config.capture.video.bitrate,
                codec: config.capture.video.codec,
            },
            output: self.output,
            zipline_url: self.zipline_url.unwrap_or(config.upload.zipline.url),
            zipline_token: expand_home(
                self.zipline_token
                    .unwrap_or_else(|| PathBuf::from(config.upload.zipline.token)),
            ),
            original_name: self
                .original_name
                .unwrap_or(config.upload.zipline.use_original_name),
            save_path: expand_home(
                self.save_path
                    .unwrap_or_else(|| PathBuf::from(config.capture.save_path)),
            ),
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

fn parse_unit(value: &str) -> Result<f64, String> {
    let value = value
        .parse::<f64>()
        .map_err(|_| "expected a number between 0.0 and 1.0".to_string())?;
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(value)
    } else {
        Err("expected a number between 0.0 and 1.0".to_string())
    }
}

fn parse_hex_color(value: &str) -> Result<String, String> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    if matches!(hex.len(), 3 | 6) && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(value.to_string())
    } else {
        Err("expected a three- or six-digit hexadecimal color".to_string())
    }
}

fn expand_home(path: PathBuf) -> PathBuf {
    let Some(path_text) = path.to_str() else {
        return path;
    };
    let Some(rest) = path_text
        .strip_prefix("~/")
        .or_else(|| (path_text == "~").then_some(""))
    else {
        return path;
    };
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(rest))
        .unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_ocr_as_a_region_request() {
        let settings = Args::parse_from(["chomp", "--ocr"]).resolve(Config::default());

        assert!(settings.request.is_ocr());
        assert_eq!(settings.request.mode, None);
    }

    #[test]
    fn preserves_clipboard_destination_without_an_explicit_mode() {
        let settings = Args::parse_from(["chomp", "--clipboard"]).resolve(Config::default());

        assert!(settings.request.to_clipboard());
    }

    #[test]
    fn rejects_ocr_with_an_explicit_capture_mode() {
        assert!(Args::try_parse_from(["chomp", "--ocr", "--mode", "image-area"]).is_err());
    }

    #[test]
    fn rejects_invalid_visual_values() {
        assert!(Args::try_parse_from(["chomp", "--dim-opacity", "1.2"]).is_err());
        assert!(Args::try_parse_from(["chomp", "--border-color", "not-a-color"]).is_err());
        assert!(Args::try_parse_from(["chomp", "--font-size", "0"]).is_err());
    }
}
