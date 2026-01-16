//! gulp: Enhanced region selector for Wayland
//!
//! A compositor-agnostic screen selection tool with features like:
//! - Window snapping and hover detection (works with Hyprland, Niri, and other compositors)
//! - Fullscreen window support (can select windows even in fullscreen mode)
//! - Smooth animations
//! - Configurable appearance
//! - Multi-monitor support

mod animation;
mod cli;
mod config;
mod ocr;
mod render;
mod selection;
mod wayland;
mod windows;

use anyhow::Result;
use clap::Parser;
use cli::{Args, parse_log_level};
use config::Config;

fn main() -> Result<()> {
    let args = Args::parse();

    // Handle shell completion generation
    if let Some(shell) = args.generate_completions {
        Args::generate_completions(shell);
        return Ok(());
    }

    // Handle config generation early (doesn't need the rest of the setup)
    if args.generate_config {
        let config_path = Config::write_defaults_to_file(None)?;
        println!("Default config file created at: {}", config_path.display());
        return Ok(());
    }

    // Load config from file or use defaults if loading fails
    let config = Config::load().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load config: {}. Using defaults.", e);
        Config::default()
    });

    // Merge CLI args with config (CLI overrides config)
    let args = args.merge_with_config(config);

    // Initialize logging system
    let log_level = args.log.as_deref().unwrap_or("off");
    env_logger::Builder::from_default_env()
        .filter_level(parse_log_level(log_level))
        .init();

    log::info!("Starting gulp with args: {:?}", args);

    // Run the Wayland application (blocks until selection or cancel)
    wayland::App::run(args)?;

    Ok(())
}
