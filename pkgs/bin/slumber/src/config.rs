use std::fs;
use std::path::{Path, PathBuf};

use clap::{ArgAction, Parser, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum AnimationChoice {
    Waves,
    Starfield,
    Clock,
    #[default]
    Random,
}

impl AnimationChoice {
    fn as_str(self) -> &'static str {
        match self {
            Self::Waves => "waves",
            Self::Starfield => "starfield",
            Self::Clock => "clock",
            Self::Random => "random",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    #[default]
    Mocha,
    Macchiato,
    Frappe,
    Latte,
}

impl ThemeChoice {
    fn resolve(self) -> Theme {
        match self {
            Self::Mocha => Theme::mocha(),
            Self::Macchiato => Theme::macchiato(),
            Self::Frappe => Theme::frappe(),
            Self::Latte => Theme::latte(),
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "slumber",
    author = "krezh",
    version,
    about = "Fast, reactive Wayland/Hyprland screensaver with Catppuccin themes and animations"
)]
pub struct Cli {
    /// Animation to display
    #[arg(short, long, value_enum)]
    pub animation: Option<AnimationChoice>,

    /// Catppuccin theme flavor
    #[arg(short, long, value_enum)]
    pub theme: Option<ThemeChoice>,

    /// Target frame rate (15–240), rounded per output to a whole divisor of its refresh rate
    #[arg(long)]
    pub fps: Option<u32>,

    /// Nonnegative mouse motion threshold in pixels to trigger wake-up
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

    /// Validate configuration without connecting to Wayland or acquiring a lock
    #[arg(long)]
    pub check_config: bool,

    /// List all available animations and exit
    #[arg(long)]
    pub list_animations: bool,

    /// List all built-in themes and exit
    #[arg(long)]
    pub list_themes: bool,

    /// Overlay digital clock on top of the animation
    #[arg(short = 'k', long = "clock", aliases = ["show-clock"], action = ArgAction::Set, num_args = 0..=1, default_missing_value = "true")]
    pub show_clock: Option<bool>,

    /// Background opacity from 0.0 (see-through) to 1.0 (opaque)
    #[arg(short = 'o', long = "opacity", aliases = ["background-opacity"])]
    pub opacity: Option<f32>,

    /// Font family or path to TTF/OTF font file [default: Rubik]
    #[arg(short = 'f', long = "font", aliases = ["font-family"])]
    pub font: Option<String>,

    /// Animation speed multiplier (0.05–10.0) [default: 1.0]
    #[arg(short = 's', long = "speed")]
    pub speed: Option<f32>,

    /// Slowly move the clock to reduce static pixels [default: true]
    #[arg(long, action = ArgAction::Set, num_args = 0..=1, default_missing_value = "true")]
    pub clock_drift: Option<bool>,

    /// Entry and animation transition duration in milliseconds (0 to disable)
    #[arg(long)]
    pub transition_ms: Option<u64>,

    /// Synchronize animation choices and timing across outputs
    #[arg(long, action = ArgAction::Set, num_args = 0..=1, default_missing_value = "true")]
    pub sync_outputs: Option<bool>,

    /// Draw waves as one scene across all outputs; implies --sync-outputs
    #[arg(long, action = ArgAction::Set, num_args = 0..=1, default_missing_value = "true")]
    pub span_outputs: Option<bool>,

    /// Chrono strftime format for the clock [default: %H:%M]
    #[arg(long)]
    pub time_format: Option<String>,

    /// Chrono strftime format for the date [default: %A, %d %B %Y]
    #[arg(long)]
    pub date_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FileConfig {
    pub animation: AnimationChoice,
    pub theme: ThemeChoice,
    pub fps: u32,
    pub motion_threshold: f64,
    pub grace_ms: u64,
    pub cycle: u64,
    pub show_clock: bool,
    pub opacity: f32,
    pub font: String,
    pub speed: f32,
    pub clock_drift: bool,
    pub transition_ms: u64,
    pub sync_outputs: bool,
    pub span_outputs: bool,
    pub time_format: String,
    pub date_format: String,
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            animation: AnimationChoice::Random,
            theme: ThemeChoice::Mocha,
            fps: 60,
            motion_threshold: 5.0,
            grace_ms: 200,
            cycle: 0,
            show_clock: false,
            opacity: 1.0,
            font: "Rubik".into(),
            speed: 1.0,
            clock_drift: true,
            transition_ms: 600,
            sync_outputs: false,
            span_outputs: false,
            time_format: "%H:%M".into(),
            date_format: "%A, %d %B %Y".into(),
        }
    }
}

struct FormatValidator;

impl std::fmt::Write for FormatValidator {
    fn write_str(&mut self, _: &str) -> std::fmt::Result {
        Ok(())
    }
}

impl FileConfig {
    fn validate(&self) -> Result<(), String> {
        if !(15..=240).contains(&self.fps) {
            return Err(format!("fps must be between 15 and 240, got {}", self.fps));
        }
        if !self.motion_threshold.is_finite() || self.motion_threshold < 0.0 {
            return Err(format!(
                "motion_threshold must be finite and nonnegative, got {}",
                self.motion_threshold
            ));
        }
        if !self.opacity.is_finite() || !(0.0..=1.0).contains(&self.opacity) {
            return Err(format!(
                "opacity must be finite and between 0.0 and 1.0, got {}",
                self.opacity
            ));
        }
        if !self.speed.is_finite() || !(0.05..=10.0).contains(&self.speed) {
            return Err(format!(
                "speed must be finite and between 0.05 and 10.0, got {}",
                self.speed
            ));
        }
        if self.font.trim().is_empty() {
            return Err("font must be a nonempty family name or font path".into());
        }
        for (name, format) in [
            ("time_format", &self.time_format),
            ("date_format", &self.date_format),
        ] {
            let timestamp = chrono::DateTime::from_timestamp(0, 0).expect("Unix epoch is valid");
            if timestamp
                .format(format)
                .write_to(&mut FormatValidator)
                .is_err()
            {
                return Err(format!(
                    "{name} contains an invalid chrono strftime directive: {format:?}"
                ));
            }
        }
        Ok(())
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
    pub clock_drift: bool,
    pub transition_ms: u64,
    pub sync_outputs: bool,
    pub span_outputs: bool,
    pub time_format: String,
    pub date_format: String,
}

impl AppConfig {
    pub fn load(cli: &Cli) -> Result<Self, Box<dyn std::error::Error>> {
        let default_path = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|p| p.join("slumber").join("config.toml"));
        Self::load_with_default(cli, default_path.as_deref())
    }

    fn load_with_default(
        cli: &Cli,
        default_path: Option<&Path>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = cli.config.as_deref().or(default_path);
        let mut cfg: FileConfig = match config_path {
            Some(path) => match fs::read_to_string(path) {
                Ok(content) => {
                    let cfg: FileConfig = toml::from_str(&content)
                        .map_err(|err| format!("invalid config {}: {err}", path.display()))?;
                    cfg.validate()
                        .map_err(|err| format!("invalid config {}: {err}", path.display()))?;
                    cfg
                }
                Err(err) if cli.config.is_none() && err.kind() == std::io::ErrorKind::NotFound => {
                    FileConfig::default()
                }
                Err(err) => {
                    return Err(format!("cannot read config {}: {err}", path.display()).into());
                }
            },
            None => FileConfig::default(),
        };

        cfg.animation = cli.animation.unwrap_or(cfg.animation);
        cfg.theme = cli.theme.unwrap_or(cfg.theme);
        cfg.fps = cli.fps.unwrap_or(cfg.fps);
        cfg.motion_threshold = cli.motion_threshold.unwrap_or(cfg.motion_threshold);
        cfg.grace_ms = cli.grace_ms.unwrap_or(cfg.grace_ms);
        cfg.cycle = cli.cycle.unwrap_or(cfg.cycle);
        cfg.show_clock = cli.show_clock.unwrap_or(cfg.show_clock);
        cfg.opacity = cli.opacity.unwrap_or(cfg.opacity);
        cfg.font = cli.font.clone().unwrap_or(cfg.font);
        cfg.speed = cli.speed.unwrap_or(cfg.speed);
        cfg.clock_drift = cli.clock_drift.unwrap_or(cfg.clock_drift);
        cfg.transition_ms = cli.transition_ms.unwrap_or(cfg.transition_ms);
        cfg.sync_outputs = cli.sync_outputs.unwrap_or(cfg.sync_outputs);
        cfg.span_outputs = cli.span_outputs.unwrap_or(cfg.span_outputs);
        cfg.time_format = cli.time_format.clone().unwrap_or(cfg.time_format);
        cfg.date_format = cli.date_format.clone().unwrap_or(cfg.date_format);
        cfg.validate()
            .map_err(|err| format!("invalid command-line configuration: {err}"))?;

        let theme: Theme = if let Some(path) = &cli.custom_theme {
            let content = fs::read_to_string(path)
                .map_err(|err| format!("cannot read custom theme {}: {err}", path.display()))?;
            toml::from_str(&content)
                .map_err(|err| format!("invalid custom theme {}: {err}", path.display()))?
        } else {
            cfg.theme.resolve()
        };

        Ok(Self {
            animation: cfg.animation.as_str().into(),
            theme: theme.with_background_opacity(cfg.opacity),
            fps: cfg.fps,
            motion_threshold: cfg.motion_threshold,
            grace_ms: cfg.grace_ms,
            cycle: cfg.cycle,
            show_clock: cfg.show_clock,
            font: cfg.font,
            speed: cfg.speed,
            clock_drift: cfg.clock_drift,
            transition_ms: cfg.transition_ms,
            // Spanning draws one scene across outputs, so their scenes must be shared.
            sync_outputs: cfg.sync_outputs || cfg.span_outputs,
            span_outputs: cfg.span_outputs,
            time_format: cfg.time_format,
            date_format: cfg.date_format,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct ConfigDirectory(PathBuf);

    impl ConfigDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "slumber-config-{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn write(&self, name: &str, content: &str) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, content).unwrap();
            path
        }
    }

    impl Drop for ConfigDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn only_missing_implicit_config_is_optional() {
        let directory = ConfigDirectory::new();
        let missing = directory.0.join("missing.toml");
        let mut cli = Cli::try_parse_from(["slumber"]).unwrap();
        AppConfig::load_with_default(&cli, Some(&missing)).unwrap();

        cli.config = Some(missing.clone());
        let error = AppConfig::load_with_default(&cli, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains(&missing.display().to_string()));
        assert!(error.contains("cannot read config"));

        cli.config = None;
        let error = AppConfig::load_with_default(&cli, Some(&directory.0))
            .unwrap_err()
            .to_string();
        assert!(error.contains("cannot read config"));
    }

    #[test]
    fn invalid_file_settings_report_the_path_and_field() {
        let directory = ConfigDirectory::new();
        for (content, expected) in [
            ("fps = [", "invalid config"),
            ("unknown = true", "unknown"),
            ("animation = 'typo'", "typo"),
            ("theme = 'typo'", "typo"),
            ("fps = 0", "fps"),
            ("fps = 241", "fps"),
            ("motion_threshold = -1.0", "motion_threshold"),
            ("motion_threshold = inf", "motion_threshold"),
            ("motion_threshold = nan", "motion_threshold"),
            ("opacity = -0.1", "opacity"),
            ("opacity = 1.1", "opacity"),
            ("opacity = nan", "opacity"),
            ("speed = 0.0", "speed"),
            ("speed = 10.1", "speed"),
            ("speed = inf", "speed"),
            ("font = ''", "font"),
            ("grace_ms = -1", "grace_ms"),
            ("cycle = -1", "cycle"),
            ("transition_ms = -1", "transition_ms"),
            ("time_format = '%Q'", "time_format"),
            ("date_format = '%'", "date_format"),
            ("time_format = '%#z'", "time_format"),
        ] {
            let path = directory.write("config.toml", content);
            let mut cli = Cli::try_parse_from(["slumber"]).unwrap();
            cli.config = Some(path.clone());
            let error = AppConfig::load_with_default(&cli, None)
                .unwrap_err()
                .to_string();
            assert!(error.contains(expected), "{content}: {error}");
            assert!(error.contains(&path.display().to_string()), "{error}");
        }
    }

    #[test]
    fn invalid_cli_settings_are_not_clamped() {
        for args in [
            vec!["--fps=14"],
            vec!["--fps=241"],
            vec!["--motion-threshold=-1"],
            vec!["--motion-threshold=NaN"],
            vec!["--opacity=1.1"],
            vec!["--opacity=NaN"],
            vec!["--speed=0"],
            vec!["--speed=inf"],
            vec!["--time-format=%Q"],
            vec!["--date-format=%"],
        ] {
            let cli = Cli::try_parse_from(std::iter::once("slumber").chain(args)).unwrap();
            let error = AppConfig::load_with_default(&cli, None)
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("invalid command-line configuration"),
                "{error}"
            );
        }
        assert!(Cli::try_parse_from(["slumber", "--animation=typo"]).is_err());
        assert!(Cli::try_parse_from(["slumber", "--theme=typo"]).is_err());
        assert!(Cli::try_parse_from(["slumber", "--clock=maybe"]).is_err());
    }

    #[test]
    fn cli_overrides_file_values_including_false() {
        let directory = ConfigDirectory::new();
        let path = directory.write(
            "config.toml",
            "show_clock = true\nclock_drift = true\nsync_outputs = true\n",
        );
        let mut cli = Cli::try_parse_from([
            "slumber",
            "--clock=false",
            "--clock-drift=false",
            "--sync-outputs",
            "false",
        ])
        .unwrap();
        cli.config = Some(path);
        let cfg = AppConfig::load_with_default(&cli, None).unwrap();
        assert!(!cfg.show_clock);
        assert!(!cfg.clock_drift);
        assert!(!cfg.sync_outputs);
        for flag in ["--clock", "--show-clock", "-k"] {
            let cli = Cli::try_parse_from(["slumber", flag]).unwrap();
            assert!(AppConfig::load_with_default(&cli, None).unwrap().show_clock);
        }
    }

    #[test]
    fn invalid_file_values_cannot_be_hidden_by_cli_overrides() {
        let directory = ConfigDirectory::new();
        let mut cli = Cli::try_parse_from(["slumber", "--fps=60"]).unwrap();
        cli.config = Some(directory.write("config.toml", "fps = 0"));
        assert!(AppConfig::load_with_default(&cli, None).is_err());
    }

    #[test]
    fn custom_theme_errors_include_the_requested_path() {
        let directory = ConfigDirectory::new();
        let mut cli = Cli::try_parse_from(["slumber"]).unwrap();
        let missing = directory.0.join("missing-theme.toml");
        cli.custom_theme = Some(missing.clone());
        let error = AppConfig::load_with_default(&cli, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains(&missing.display().to_string()));
        assert!(error.contains("cannot read custom theme"));

        for content in [
            "name = [".into(),
            "name = 'incomplete'".into(),
            format!(
                "unknown = true\n{}",
                toml::to_string(&Theme::mocha()).unwrap()
            ),
            toml::to_string(&Theme::mocha())
                .unwrap()
                .replace("[base]", "[base]\nunknown = 1"),
        ] {
            let path = directory.write("theme.toml", &content);
            cli.custom_theme = Some(path.clone());
            let error = AppConfig::load_with_default(&cli, None)
                .unwrap_err()
                .to_string();
            assert!(error.contains(&path.display().to_string()));
            assert!(error.contains("invalid custom theme"));
        }
        cli.custom_theme =
            Some(directory.write("theme.toml", &toml::to_string(&Theme::latte()).unwrap()));
        assert_eq!(
            AppConfig::load_with_default(&cli, None).unwrap().theme.name,
            "latte"
        );
    }
}
