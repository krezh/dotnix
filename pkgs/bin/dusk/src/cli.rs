use std::path::PathBuf;

use crate::config::{self, Action, Options};

pub enum Command {
    Run(Options),
    Help,
}

pub const HELP: &str = "Dusk — graceful session exit for Hyprland

Usage: dusk <logout|shutdown|reboot> [OPTIONS]

Actions:
  logout                Exit Hyprland (no additional command by default)
  shutdown              Exit Hyprland, then run systemctl poweroff
  reboot                Exit Hyprland, then run systemctl reboot

Closes applications gracefully, waits for them to exit, then exits Hyprland.
The selected action's command runs only after compositor exit is confirmed.
Cancel stops session exit but cannot restore applications already closed.
Bare dusk shows help. Runtime options require exactly one action.
Options may precede or follow the action.

Options:
  --config PATH         Read an alternate TOML configuration file
  --dry-run[=BOOL]       Show the UI without closing apps or running commands
  --no-exit[=BOOL]       Close apps without exiting Hyprland or running commands
  -t, --top-label TEXT   Override the selected action's heading
  --command CMD         Override its shell command; empty disables it
  --verbose[=BOOL]       Print session-exit diagnostics
  --no-fork[=BOOL]       Run in the foreground instead of detaching
  --vt N|none            Switch VT using sudo -n chvt after exit; none clears config
  --opacity A           Main panel opacity, 0–1 (default: 0.75)
  --panel-opacity A     Inner panel opacity, 0–1 (default: 0.60)
  -h, --help            Show this help without loading configuration

Configuration:
  $XDG_CONFIG_HOME/dusk/config.toml, or ~/.config/dusk/config.toml
  Precedence: explicit CLI values > config file > selected action's defaults.
  CLI overrides apply regardless of position; repeated options use the last value.
  A missing default file is optional; a missing --config file is an error.
  Unknown keys, invalid values, and unreadable files stop startup before app closure.
  Bare boolean flags mean true; use =false to override a configured true value.
  Settings are read at startup. The fullscreen background stays transparent.
  The panel pops in unless GTK animations are disabled.

TOML keys (defaults shown; omit vt to leave VT switching disabled):
  dry_run = false
  no_exit = false
  verbose = false
  no_fork = false
  # vt = 1

  [appearance]
  opacity = 0.75
  panel_opacity = 0.60

  [actions.logout]
  top_label = 'Logging out...'
  command = ''

  [actions.shutdown]
  top_label = 'Shutting down...'
  command = 'systemctl poweroff'

  [actions.reboot]
  top_label = 'Restarting...'
  command = 'systemctl reboot'

Action settings are independent; omitted fields inherit that action's defaults.
An empty command disables the additional command, not the built-in Hyprland exit.
Dry run suppresses all side effects, including commands and VT switching.
No-exit, cancellation, and errors also suppress completion commands.
When configured, VT switching runs after compositor exit and before the command.

Examples:
  dusk logout
  dusk shutdown --dry-run --no-fork
  dusk --config ~/.config/dusk/preview.toml reboot
  dusk logout -t 'See you soon...' --command 'notify-session-ended'
  dusk reboot --command=";

#[derive(Default)]
struct Overrides {
    dry_run: Option<bool>,
    no_exit: Option<bool>,
    top_label: Option<String>,
    command: Option<String>,
    verbose: Option<bool>,
    no_fork: Option<bool>,
    vt: Option<Option<u32>>,
    opacity: Option<f64>,
    panel_opacity: Option<f64>,
}

impl Overrides {
    fn apply(self, options: &mut Options) {
        if let Some(value) = self.dry_run {
            options.dry_run = value;
        }
        if let Some(value) = self.no_exit {
            options.no_exit = value;
        }
        if let Some(value) = self.top_label {
            options.top_label = value;
        }
        if let Some(value) = self.command {
            options.command = Some(value).filter(|command| !command.is_empty());
        }
        if let Some(value) = self.verbose {
            options.verbose = value;
        }
        if let Some(value) = self.no_fork {
            options.no_fork = value;
        }
        if let Some(value) = self.vt {
            options.vt = value;
        }
        if let Some(value) = self.opacity {
            options.appearance.opacity = value;
        }
        if let Some(value) = self.panel_opacity {
            options.appearance.panel_opacity = value;
        }
    }
}

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    parse_with_path(args, config::default_path())
}

fn parse_with_path(
    args: impl IntoIterator<Item = String>,
    default_path: Option<PathBuf>,
) -> Result<Command, String> {
    let mut overrides = Overrides::default();
    let mut config_path = None;
    let mut action = None;
    let mut args = args.into_iter().peekable();
    if args.peek().is_none() {
        return Ok(Command::Help);
    }
    while let Some(arg) = args.next() {
        let (name, inline) = arg
            .split_once('=')
            .map_or((arg.as_str(), None), |(name, value)| (name, Some(value)));
        match name {
            "logout" | "shutdown" | "reboot" if inline.is_none() => {
                if action.is_some() {
                    return Err("specify exactly one action: logout, shutdown, or reboot".into());
                }
                action = Some(match name {
                    "logout" => Action::Logout,
                    "shutdown" => Action::Shutdown,
                    "reboot" => Action::Reboot,
                    _ => unreachable!(),
                });
            }
            "--dry-run" | "--no-exit" | "--verbose" | "--no-fork" => {
                let value = inline
                    .map_or(Ok(true), |value| value.parse::<bool>())
                    .map_err(|_| format!("{name} expects true or false"))?;
                match name {
                    "--dry-run" => overrides.dry_run = Some(value),
                    "--no-exit" => overrides.no_exit = Some(value),
                    "--verbose" => overrides.verbose = Some(value),
                    "--no-fork" => overrides.no_fork = Some(value),
                    _ => unreachable!(),
                }
            }
            "--config" | "-t" | "--top-label" | "--command" | "--vt" | "--opacity"
            | "--panel-opacity" => {
                let value = inline
                    .map(str::to_owned)
                    .or_else(|| args.next())
                    .ok_or_else(|| format!("{name} requires a value"))?;
                match name {
                    "--config" => config_path = Some(PathBuf::from(value)),
                    "-t" | "--top-label" => overrides.top_label = Some(value),
                    "--command" => overrides.command = Some(value),
                    "--vt" => {
                        overrides.vt = Some(if value == "none" {
                            None
                        } else {
                            Some(
                                value
                                    .parse()
                                    .map_err(|_| "--vt requires a VT number or none")?,
                            )
                        });
                    }
                    "--opacity" | "--panel-opacity" => {
                        let value = value
                            .parse()
                            .map_err(|_| format!("{name} requires a number between 0 and 1"))?;
                        if name == "--opacity" {
                            overrides.opacity = Some(value);
                        } else {
                            overrides.panel_opacity = Some(value);
                        }
                    }
                    _ => unreachable!(),
                }
            }
            "-h" | "--help" if inline.is_none() => return Ok(Command::Help),
            _ if arg.starts_with('-') => return Err(format!("unknown option: {arg}")),
            _ => return Err(format!("unknown action: {arg}")),
        }
    }
    let action = action.ok_or("an action is required: logout, shutdown, or reboot")?;
    let explicit = config_path.is_some();
    let mut options = config::load(config_path.or(default_path), explicit)?.resolve(action);
    overrides.apply(&mut options);
    options.validate()?;
    Ok(Command::Run(options))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct ConfigFile(PathBuf);

    impl ConfigFile {
        fn new(contents: &str) -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "dusk-config-{}-{}.toml",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::write(&path, contents).unwrap();
            Self(path)
        }
    }

    impl Drop for ConfigFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn run(args: &[&str], default_path: Option<PathBuf>) -> Options {
        match parse_with_path(args.iter().map(|arg| (*arg).to_owned()), default_path).unwrap() {
            Command::Run(options) => options,
            Command::Help => panic!("expected runtime options"),
        }
    }

    #[test]
    fn file_settings_are_overridden_only_by_explicit_cli_values() {
        let file = ConfigFile::new(
            "dry_run = true\nno_exit = true\nverbose = true\nno_fork = true\nvt = 3\n\
             [actions.shutdown]\ntop_label = 'From file'\ncommand = 'exit 7'\n\
             [appearance]\nopacity = 0.85\npanel_opacity = 0.35",
        );
        let options = run(
            &["--top-label", "From CLI", "shutdown", "--opacity=0.5"],
            Some(file.0.clone()),
        );
        assert_eq!(options.top_label, "From CLI");
        assert_eq!(options.appearance.opacity, 0.5);
        assert_eq!(options.appearance.panel_opacity, 0.35);
        assert!(options.dry_run && options.no_exit && options.verbose && options.no_fork);
        assert_eq!(options.command.as_deref(), Some("exit 7"));
        assert_eq!(options.vt, Some(3));
        let options = run(
            &[
                "shutdown",
                "--dry-run=false",
                "--no-exit=false",
                "--verbose=false",
                "--no-fork=false",
                "--command=",
                "--vt=none",
                "--panel-opacity=0.2",
            ],
            Some(file.0.clone()),
        );
        assert!(!options.dry_run && !options.no_exit && !options.verbose && !options.no_fork);
        assert_eq!(options.command, None);
        assert_eq!(options.vt, None);
        assert_eq!(options.appearance.panel_opacity, 0.2);
        assert_eq!(options.top_label, "From file");
    }

    #[test]
    fn explicit_config_selection_is_independent_of_argument_order() {
        let default = ConfigFile::new("[actions.reboot]\ntop_label = 'Wrong file'");
        let selected =
            ConfigFile::new("no_fork = true\n[actions.reboot]\ntop_label = 'Selected file'");
        for args in [
            vec![
                "reboot",
                "--config",
                selected.0.to_str().unwrap(),
                "--top-label",
                "Override",
            ],
            vec![
                "--top-label",
                "Override",
                "--config",
                selected.0.to_str().unwrap(),
                "reboot",
            ],
        ] {
            let options = run(&args, Some(default.0.clone()));
            assert_eq!(options.top_label, "Override");
            assert!(options.no_fork);
        }
    }

    #[test]
    fn missing_default_is_optional_but_explicit_or_invalid_config_is_fatal() {
        let file = ConfigFile::new("unknown_setting = true");
        let missing = file.0.with_extension("missing");
        let options = run(&["logout", "--dry-run"], Some(missing.clone()));
        assert!(options.dry_run);
        assert!(
            parse_with_path(
                [
                    "logout".to_owned(),
                    "--config".to_owned(),
                    missing.to_string_lossy().into_owned()
                ],
                None
            )
            .is_err()
        );
        assert!(parse_with_path(["logout".into()], Some(file.0.clone())).is_err());
        assert!(matches!(
            parse_with_path(["--help".into()], Some(file.0.clone())).unwrap(),
            Command::Help
        ));
    }

    #[test]
    fn invalid_cli_values_fail_before_startup() {
        for arg in [
            "--opacity=NaN",
            "--panel-opacity=1.1",
            "--vt=64",
            "--dry-run=no",
        ] {
            assert!(
                parse_with_path(["logout".into(), arg.into()], None).is_err(),
                "{arg}"
            );
        }
    }

    #[test]
    fn bare_invocation_and_help_do_not_load_configuration() {
        let file = ConfigFile::new("invalid configuration");
        for args in [vec![], vec!["--help"], vec!["logout", "--help"]] {
            assert!(matches!(
                parse_with_path(args.into_iter().map(str::to_owned), Some(file.0.clone())),
                Ok(Command::Help)
            ));
        }
    }

    #[test]
    fn missing_unknown_duplicate_actions_and_legacy_options_are_rejected() {
        for args in [
            vec!["--dry-run"],
            vec!["--command="],
            vec!["--top-label", "Logout"],
            vec!["--config", "missing.toml"],
            vec!["suspend"],
            vec!["shutdown=now"],
            vec!["logout", "unknown"],
            vec!["logout", "logout"],
            vec!["logout", "shutdown"],
            vec!["shutdown", "--dry-run", "reboot"],
            vec!["reboot", "--post-cmd="],
            vec!["logout", "-p", ""],
        ] {
            assert!(
                parse_with_path(args.iter().map(|arg| (*arg).to_owned()), None).is_err(),
                "{args:?}"
            );
        }
    }

    #[test]
    fn cli_action_selection_and_overrides_are_order_independent() {
        let file = ConfigFile::new(
            "[actions.logout]\ntop_label = 'Logout'\ncommand = 'logout-hook'\n\
             [actions.shutdown]\ntop_label = 'Shutdown'\ncommand = 'shutdown-hook'\n\
             [actions.reboot]\ntop_label = 'Reboot'\ncommand = 'reboot-hook'",
        );
        for (action, label, command) in [
            ("logout", "Logout", "logout-hook"),
            ("shutdown", "Shutdown", "shutdown-hook"),
            ("reboot", "Reboot", "reboot-hook"),
        ] {
            let options = run(&[action], Some(file.0.clone()));
            assert_eq!(options.top_label, label);
            assert_eq!(options.command.as_deref(), Some(command));
            for args in [
                vec!["--command", "override", "-t", "Override", action],
                vec![action, "--command=override", "--top-label=Override"],
            ] {
                let options = run(&args, Some(file.0.clone()));
                assert_eq!(options.top_label, "Override");
                assert_eq!(options.command.as_deref(), Some("override"));
            }
            for args in [
                vec!["--command=ignored", action, "--command="],
                vec!["--command", "", action],
            ] {
                assert_eq!(run(&args, Some(file.0.clone())).command, None);
            }
            let options = run(
                &["--command=", action, "--command=last"],
                Some(file.0.clone()),
            );
            assert_eq!(options.command.as_deref(), Some("last"));
        }
    }
}
