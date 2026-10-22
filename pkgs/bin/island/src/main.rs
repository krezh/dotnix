mod activation;
mod bar;
mod cli;
mod components;
mod compositor;
mod config;
mod control;
mod copy;
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
mod markdown;
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

#[derive(Default)]
struct Args {
    config: Option<PathBuf>,
    radius: Option<f32>,
    power: Option<f32>,
    opacity: Option<f32>,
    icon_theme: Option<String>,
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Args, cli::Mistake> {
    let commands: Vec<&str> = cli::commands().iter().map(|sub| sub.name).collect();
    let mut parsed = Args::default();
    for (opt, value) in cli::parse_options(cli::ISLAND, args, "island", &commands)? {
        let value = value.unwrap_or_default();
        match opt.long {
            "config" => parsed.config = Some(PathBuf::from(value)),
            "radius" => parsed.radius = Some(number(opt, &value, 0.0..=f32::MAX)?),
            "power" => parsed.power = Some(number(opt, &value, 1.0..=f32::MAX)?),
            "opacity" => parsed.opacity = Some(number(opt, &value, 0.0..=1.0)?),
            _ => parsed.icon_theme = Some(value),
        }
    }
    Ok(parsed)
}

fn number(
    opt: &cli::Opt,
    value: &str,
    range: std::ops::RangeInclusive<f32>,
) -> Result<f32, cli::Mistake> {
    match value.parse::<f32>() {
        Ok(number) if range.contains(&number) => Ok(number),
        _ => {
            let range = if *range.end() == f32::MAX {
                format!("{} or more", range.start())
            } else {
                format!("from {} to {}", range.start(), range.end())
            };
            Err(cli::Mistake::new(
                cli::Line::new()
                    .push(cli::Tone::Command, opt.spelled())
                    .plain(format!(" needs a number {range}, got "))
                    .push(cli::Tone::Strong, value),
            ))
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1).peekable();
    match args.peek().map(String::as_str) {
        Some("-h" | "--help") => {
            cli::print_help(None);
            std::process::exit(0);
        }
        Some("ctl") => {
            args.next();
            std::process::exit(control::run(args.collect()));
        }
        Some("completions") => {
            args.next();
            std::process::exit(cli::completions(args.collect()));
        }
        // What the completion scripts ask; not for typing.
        Some("__complete") => {
            let words: Vec<String> = args.skip(1).skip_while(|arg| arg != "--").skip(1).collect();
            cli::print_completions(&words);
            std::process::exit(0);
        }
        Some("logs") => {
            args.next();
            logging::logs(args.collect());
        }
        Some(word) => {
            if let Some(client) = features::client(word) {
                args.next();
                std::process::exit(client(&args.collect::<Vec<_>>()));
            }
        }
        _ => {}
    }
    let args: Vec<String> = args.collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        cli::print_help(None);
        std::process::exit(0);
    }
    let parsed = match parse_args(args.into_iter()) {
        Ok(parsed) => parsed,
        Err(mistake) => {
            cli::print_error(&mistake.lines());
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
    shape::set_border(appearance.border);

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
        shadow: appearance.shadow.max(0.0),
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
