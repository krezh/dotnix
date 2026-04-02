//! chomp: Enhanced region selector for Wayland
//!
//! A compositor-agnostic screen selection tool with features like:
//! - Interactive area selection with live preview
//! - Window and screen capture modes
//! - Video recording with wl-screenrec
//! - OCR text extraction with Tesseract
//! - Configurable appearance
//! - Multi-monitor support

mod capture;
mod cli;
mod compositor;
mod config;
mod ocr;
mod render;
mod system;
mod ui;
mod upload;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Args, parse_log_level};
use config::Config;

pub const APP_NAME: &str = "chomp";

fn main() -> Result<()> {
    let args = Args::parse();

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

    let config = Config::load().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load config: {}. Using defaults.", e);
        Config::default()
    });

    let args = args.merge_with_config(config);

    let log_level = args.log.as_deref().unwrap_or("off");
    env_logger::Builder::from_default_env()
        .filter_level(parse_log_level(log_level))
        .init();

    log::info!("Starting chomp with args: {:?}", args);

    if args.status {
        return handle_status();
    }

    if let Some(ref mode_str) = args.mode {
        let mode = capture::CaptureMode::from_str(mode_str)?;
        let notifier = ui::Notifier::new();

        if mode.is_video() {
            return handle_video_mode(&args, &mode, &notifier);
        } else {
            return handle_image_mode(&args, &mode, &notifier);
        }
    }

    apply_delay(&args);

    let _ = ui::App::run(args)?;

    Ok(())
}

/// Applies delay if specified in args.
fn apply_delay(args: &Args) {
    if let Some(delay_ms) = args.delay {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    }
}

/// Attempts to upload file if configured, logging and notifying on error.
fn handle_upload(args: &Args, file_path: &str, notifier: &ui::Notifier) {
    if should_upload(args) {
        if let Err(e) = upload_file(args, file_path, notifier) {
            log::error!("Upload failed: {}", e);
            notifier.send_error("Upload failed", Some(&e.to_string()));
        }
    }
}

/// Handles --status flag to show recording status.
fn handle_status() -> Result<()> {
    let recorder = capture::VideoRecorder::new();
    let (is_recording, output_file) = recorder.is_recording()?;

    if is_recording {
        println!("🔴 Recording in progress");
        if let Some(file) = output_file {
            println!("📁 Output: {}", file);
        }
        println!("Run chomp -m video-<mode> to stop");
    } else {
        println!("⏹️ No recording active");
    }

    Ok(())
}

/// Handles video recording modes.
fn handle_video_mode(
    args: &Args,
    mode: &capture::CaptureMode,
    notifier: &ui::Notifier,
) -> Result<()> {
    let recorder = capture::VideoRecorder::new();

    let (is_recording, _) = recorder.is_recording()?;

    if is_recording {
        let output_file = recorder.stop_recording()?;
        notifier.send_success("Screen recording saved");

        if let Some(file) = &output_file {
            println!("Recording saved to: {}", file);
            handle_upload(args, file, notifier);
        }

        return Ok(());
    }

    apply_delay(args);

    let (geometry, monitor): (Option<String>, Option<String>) = match mode {
        capture::CaptureMode::VideoArea => {
            let geo = ui::App::run(args.clone())?
                .context("Selection cancelled or failed")?;
            (Some(geo), None)
        }
        capture::CaptureMode::VideoWindow => {
            let geo = compositor::get_active_window()?;
            (Some(geo), None)
        }
        capture::CaptureMode::VideoScreen => {
            let mon = compositor::get_active_monitor()?;
            (None, Some(mon))
        }
        _ => unreachable!(),
    };

    let output_file = generate_output_path(args, "mp4")?;

    recorder.start_recording(geometry.as_deref(), monitor.as_deref(), &output_file)?;

    let mode_name = match mode {
        capture::CaptureMode::VideoArea => "area",
        capture::CaptureMode::VideoWindow => "window",
        capture::CaptureMode::VideoScreen => "screen",
        _ => unreachable!(),
    };

    notifier.send_info(&format!("Recording {}, run again to stop", mode_name));
    println!(
        "Recording started - run chomp -m video-{} to stop",
        mode_name
    );

    Ok(())
}

/// Handles image capture modes.
fn handle_image_mode(
    args: &Args,
    mode: &capture::CaptureMode,
    notifier: &ui::Notifier,
) -> Result<()> {
    apply_delay(args);

    let output_file = generate_output_path(args, "png")?;

    let rect = match mode {
        capture::CaptureMode::ImageArea => {
            let geometry = ui::App::run(args.clone())?
                .context("Selection cancelled or failed")?;
            render::Rect::from_geometry_string(&geometry)?
        }
        capture::CaptureMode::ImageWindow => {
            let geometry_str = compositor::get_active_window()?;
            render::Rect::from_geometry_string(&geometry_str)?
        }
        capture::CaptureMode::ImageScreen => {
            // For screen mode, use a large rect that covers typical screens
            render::Rect::new(0, 0, 3840, 2160)
        }
        _ => unreachable!(),
    };

    capture::capture_screenshot(rect, &output_file)?;

    log::info!("Screenshot saved to {}", output_file);

    if should_upload(args) {
        if let Err(e) = upload_file(args, &output_file, notifier) {
            log::error!("Upload failed: {}", e);
            // Fallback: copy image to clipboard
            if let Err(clip_err) = system::copy_image(&output_file) {
                notifier.send_error(
                    "Upload and clipboard copy failed",
                    Some(&clip_err.to_string()),
                );
            } else {
                notifier.send_info("Upload failed - image copied to clipboard");
            }
        }
    } else {
        notifier.send_success(&format!("Screenshot saved: {}", output_file));
        println!("Screenshot saved: {}", output_file);
    }

    Ok(())
}



/// Generates output file path with timestamp.
fn generate_output_path(args: &Args, extension: &str) -> Result<String> {
    let save_path = args.save_path.as_ref()
        .expect("save_path must be set after merge_with_config");
    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    Ok(format!("{}/{}.{}", save_path, timestamp, extension))
}

/// Checks if upload is configured.
fn should_upload(args: &Args) -> bool {
    !args.zipline_url.as_ref()
        .expect("zipline_url must be set after merge_with_config").is_empty()
        && !args.zipline_token.as_ref()
        .expect("zipline_token must be set after merge_with_config").is_empty()
}

/// Uploads file to Zipline.
fn upload_file(args: &Args, file_path: &str, notifier: &ui::Notifier) -> Result<String> {
    let url = upload::upload_to_zipline(
        args.zipline_url.as_ref()
            .expect("zipline_url must be set after merge_with_config"),
        args.zipline_token.as_ref()
            .expect("zipline_token must be set after merge_with_config"),
        *args.original_name.as_ref()
            .expect("original_name must be set after merge_with_config"),
        file_path,
    )?;

    // Upload succeeded - try to copy URL to clipboard
    let clipboard_failed = if let Err(e) = system::copy_text(&url) {
        log::warn!("Failed to copy URL to clipboard: {}", e);
        true
    } else {
        false
    };

    // Show notification with URL action button
    let message = if clipboard_failed {
        "Upload successful (clipboard copy failed)"
    } else {
        "Upload successful"
    };
    notifier.send_with_action(message, &url);

    println!("Uploaded: {}", url);

    Ok(url)
}
