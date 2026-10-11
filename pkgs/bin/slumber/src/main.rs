mod animations;
mod canvas;
mod config;
mod font;
mod lock;
mod rng;
mod theme;
mod wayland;
#[cfg(test)]
mod tests;

use clap::Parser;
use config::{AppConfig, Cli};
use lock::SingleInstanceLock;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

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

    let _lock = match SingleInstanceLock::acquire() {
        Ok(l) => l,
        Err(_) => return Ok(()),
    };
    let config = AppConfig::load(&cli);
    wayland::run(config)?;

    Ok(())
}
