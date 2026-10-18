mod activation;
mod bar;
mod components;
mod compositor;
mod config;
mod control;
mod cover;
mod db;
mod dbus;
mod effects;
mod features;
mod glyphs;
mod icons;
mod island;
mod keyring;
mod launch;
mod logging;
mod menu;
mod panel;
mod panels;
mod presence;
mod run;
mod shape;
mod shell;
mod spring;
mod supervisor;
mod sys;
mod theme;
mod ui;
mod wm;

use std::{path::PathBuf, rc::Rc};

use futures::channel::mpsc::unbounded;
use gpui::App;
use tracing::{error, info, warn};

use shell::{Config, Shell};

const USAGE: &str = "\
usage: island [--config PATH] [--radius PX] [--power P] [--opacity A] [--icon-theme NAME]
       island ctl COMMAND   (see island ctl --help)
       island logs          (see island logs --help)

  --config PATH      config file (default $XDG_CONFIG_HOME/island/config.toml)
  --radius PX        corner radius, overrides the config
  --power P          corner curve: 2 circular, 4 squircle, overrides the config
  --opacity A        body opacity 0–1, overrides the config
  --icon-theme NAME  icon theme, overrides the config

One island opens on every output and serves org.freedesktop.Notifications.
ISLAND_LOG sets what is logged, overriding [log] level in the config: a level such as debug, or
per module or feature like audio=debug,tray=trace. ISLAND_LOG=frames=debug logs every animation's frame count
and worst frame gap.";

#[derive(Default)]
struct Args {
    config: Option<PathBuf>,
    radius: Option<f32>,
    power: Option<f32>,
    opacity: Option<f32>,
    icon_theme: Option<String>,
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut parsed = Args::default();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => {
                parsed.config = Some(PathBuf::from(args.next().ok_or("--config needs a path")?))
            }
            "--radius" => {
                parsed.radius = Some(parse_number(args.next(), "--radius", 0.0..=f32::MAX)?)
            }
            "--power" => parsed.power = Some(parse_number(args.next(), "--power", 1.0..=f32::MAX)?),
            "--opacity" => {
                parsed.opacity = Some(parse_number(args.next(), "--opacity", 0.0..=1.0)?)
            }
            "--icon-theme" => {
                parsed.icon_theme = Some(args.next().ok_or("--icon-theme needs a theme name")?)
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    Ok(parsed)
}

fn parse_number(
    value: Option<String>,
    flag: &str,
    range: std::ops::RangeInclusive<f32>,
) -> Result<f32, String> {
    let value = value.ok_or_else(|| format!("{flag} needs a number"))?;
    match value.parse::<f32>() {
        Ok(number) if range.contains(&number) => Ok(number),
        _ => Err(format!(
            "{flag} needs a number from {} to {}, got '{value}'",
            range.start(),
            range.end()
        )),
    }
}

fn ctl(args: Vec<String>) -> ! {
    if args.is_empty() || args.iter().any(|arg| arg == "-h" || arg == "--help") {
        println!("{}", control::usage());
        std::process::exit(0);
    }
    match control::send(args) {
        Ok(()) => std::process::exit(0),
        Err(error) => {
            eprintln!("island ctl: {error}\n\n{}", control::usage());
            std::process::exit(2);
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1).peekable();
    match args.peek().map(String::as_str) {
        Some("ctl") => {
            args.next();
            ctl(args.collect());
        }
        Some("logs") => {
            args.next();
            logging::logs(args.collect());
        }
        _ => {}
    }
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("island: {error}\n\n{USAGE}");
            std::process::exit(2);
        }
    };
    let from_env = std::env::var("ISLAND_LOG")
        .ok()
        .filter(|spec| !spec.trim().is_empty());
    logging::init(from_env.as_deref().unwrap_or(logging::DEFAULT));
    logging::catch_panics();
    info!(version = env!("CARGO_PKG_VERSION"), "starting");

    let config_path = parsed.config.clone().or_else(config::default_path);
    let settings = match config::load(parsed.config) {
        Ok(settings) => settings,
        Err(error) => {
            error!("cannot load the config: {error}");
            std::process::exit(2);
        }
    };
    if let Some(path) = &config_path {
        info!(path = %path.display(), "config loaded");
    }
    if from_env.is_none()
        && settings.log.level != logging::DEFAULT
        && let Err(error) = logging::set(&settings.log.level)
    {
        warn!(spec = settings.log.level, "ignoring [log] level: {error}");
    }
    let appearance = &settings.appearance;

    icons::set_theme(
        parsed
            .icon_theme
            .unwrap_or_else(|| appearance.icon_theme.clone()),
    );
    let (controls, incoming) = unbounded();
    let bus = match dbus::serve(controls) {
        Ok(bus) => bus,
        Err(error) => {
            error!("{error}");
            std::process::exit(1);
        }
    };
    logging::serve();

    let workspaces = compositor::start();
    let config = Config {
        layout: Rc::new(settings.bar.clone()),
        features: Rc::new(settings.features.settings),
        top: compositor::backend()
            .and_then(|backend| backend.top_gap())
            .unwrap_or(10.0),
        corner: (
            parsed.radius.unwrap_or(appearance.radius),
            parsed.power.unwrap_or(appearance.power),
        ),
        opacity: parsed.opacity.unwrap_or(appearance.opacity),
        glow: settings.features.visualizer.glow.max(0.0),
    };
    let system = settings.features.system;

    gpui_platform::application()
        .with_assets(icons::Files)
        .run(move |cx: &mut App| {
            cx.set_global(system);
            Shell::start(config, incoming, workspaces, cx);
        });
    drop(bus);
}
