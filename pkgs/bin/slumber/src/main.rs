mod animations;
mod canvas;
mod config;
mod font;
mod lock;
mod rng;
#[cfg(test)]
mod tests;
mod theme;
mod wayland;

use std::process::ExitCode;

use clap::Parser;
use config::{AppConfig, Cli};
use lock::{LockError, SingleInstanceLock};

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("slumber: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    if cli.list_animations {
        println!("Available animations:");
        for anim in animations::ANIMATION_NAMES {
            println!("  - {anim}");
        }
        println!("  - random (selects an animation automatically)");
        return Ok(());
    }

    if cli.list_themes {
        println!("Available Catppuccin themes:");
        println!("  - mocha (dark, default)");
        println!("  - macchiato (dark, soft)");
        println!("  - frappe (dark, cool)");
        println!("  - latte (light)");
        return Ok(());
    }

    let config = AppConfig::load(&cli)?;
    if cli.check_config {
        println!("Configuration is valid.");
        return Ok(());
    }

    let _lock = match SingleInstanceLock::acquire() {
        Ok(lock) => lock,
        Err(LockError::AlreadyRunning) => return Ok(()),
        Err(err) => return Err(err.into()),
    };
    wayland::run(config)?;

    Ok(())
}
