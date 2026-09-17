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
use cli::{Args, Settings};
use config::Config;
use std::io::Write;

pub const APP_NAME: &str = "chomp";

fn main() -> Result<()> {
    let args = Args::parse();

    if let Some(shell) = args.generate_completions {
        Args::generate_completions(shell);
        return Ok(());
    }

    // The detached process serving an upload notification's action button.
    if let Some(action) = args.await_notification_action.as_deref() {
        let [title, message, url] = action else {
            anyhow::bail!("--await-notification-action takes a title, a message and a URL");
        };
        ui::Notifier::await_action(title, message, url);
        return Ok(());
    }

    // Handle config generation early (doesn't need the rest of the setup)
    if args.generate_config {
        let config_path = Config::default_config_path();
        if !args.force && config_path.exists() {
            // Merge: load existing file (serde fills missing keys with defaults), write back
            let existing = Config::load()?;
            let path = Config::write_config_to_file(&existing, None)?;
            println!(
                "Config merged (new keys added with defaults): {}",
                path.display()
            );
        } else {
            let path = Config::write_defaults_to_file(None)?;
            if args.force {
                println!("Config overwritten with defaults: {}", path.display());
            } else {
                println!("Default config created: {}", path.display());
            }
        }
        return Ok(());
    }

    let config = Config::load().unwrap_or_else(|e| {
        // Alternate form prints the whole chain, so an unknown or malformed key
        // is named rather than just "failed to parse".
        eprintln!("Warning: Failed to load config: {:#}. Using defaults.", e);
        Config::default()
    });

    let show_status = args.status;
    let settings = args.resolve(config);

    env_logger::Builder::from_default_env()
        .filter_level(settings.log.to_filter())
        .init();

    // Shown regardless of log level: a keybind that cannot fire is invisible
    // otherwise, and the selector would just ignore the key.
    for problem in settings.keybinds.problems() {
        eprintln!("chomp: {}", problem);
    }

    log::info!("Starting chomp with settings: {:?}", settings);

    if show_status {
        return handle_status(&settings);
    }

    if let Some(mode) = settings.mode {
        let notifier = ui::Notifier::new();

        if mode.is_video() && capture::recording(&settings).is_some() {
            return handle_stop_recording(&settings, &notifier);
        }

        apply_delay(&settings);

        if mode.is_video() {
            return handle_video_mode(&settings, &mode, &notifier, None);
        }

        // Area mode runs the selector anyway: take the image it cropped along with
        // the geometry, instead of capturing the region a second time.
        if mode == capture::CaptureMode::ImageArea {
            let selected = ui::App::run(settings.clone())?;
            if selected.cancelled() {
                return Ok(());
            }

            return handle_image_mode(
                &settings,
                &mode,
                &notifier,
                selected.image,
                selected.geometry,
                settings.clipboard,
            );
        }

        return handle_image_mode(&settings, &mode, &notifier, None, None, settings.clipboard);
    }

    apply_delay(&settings);

    let selected = ui::App::run(settings.clone())?;
    if let Some(mode) = selected.mode {
        let notifier = ui::Notifier::new();
        if mode == capture::CaptureMode::StopRecording {
            return handle_stop_recording(&settings, &notifier);
        } else if mode.is_video() {
            return handle_video_mode(&settings, &mode, &notifier, selected.geometry);
        } else {
            return handle_image_mode(
                &settings,
                &mode,
                &notifier,
                selected.image,
                selected.geometry,
                selected.to_clipboard,
            );
        }
    }

    Ok(())
}

/// Applies delay if specified in settings.
fn apply_delay(settings: &Settings) {
    if let Some(delay_ms) = settings.delay {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    }
}

/// Attempts to upload file if configured, logging and notifying on error.
fn handle_upload(settings: &Settings, file_path: &str, notifier: &ui::Notifier) {
    if should_upload(settings) {
        if let Err(e) = upload_file(settings, file_path, notifier) {
            log::error!("Upload failed: {}", e);
            notifier.send_error("Upload failed", Some(&e.to_string()));
        }
    }
}

/// Handles --status flag to show recording status.
fn handle_status(settings: &Settings) -> Result<()> {
    match capture::recording(settings) {
        Some(file) => {
            println!("🔴 Recording in progress");
            println!("📁 Output: {}", file);
            println!("Run chomp -m video-<mode> to stop");
        }
        None => println!("⏹️ No recording active"),
    }

    Ok(())
}

/// Stops an in-progress recording and handles upload/notification.
fn handle_stop_recording(settings: &Settings, notifier: &ui::Notifier) -> Result<()> {
    let output_file = capture::stop_recording(settings)?;

    notifier.send_info("Screen recording saved");
    println!("Recording saved to: {}", output_file);
    handle_upload(settings, &output_file, notifier);

    Ok(())
}

/// Handles video recording modes.
fn handle_video_mode(
    settings: &Settings,
    mode: &capture::CaptureMode,
    notifier: &ui::Notifier,
    pre_geometry: Option<String>,
) -> Result<()> {
    let (geometry, monitor): (Option<String>, Option<String>) = match mode {
        capture::CaptureMode::VideoArea => match pre_geometry {
            Some(geo) => (Some(geo), None),
            None => {
                let selected = ui::App::run(settings.clone())?;
                let Some(geo) = selected.geometry else {
                    // Cancelled: nothing to record.
                    return Ok(());
                };
                (Some(geo), None)
            }
        },
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

    let output_file = generate_output_path(settings, "mp4")?;
    if output_file == "-" {
        anyhow::bail!("stdout output is not supported for video recording");
    }

    // The recorded area, so the bitrate can be sized to it.
    let size = match geometry.as_deref() {
        Some(geo) => render::Rect::from_geometry_string(geo)
            .ok()
            .map(|rect| (rect.width.max(0) as u32, rect.height.max(0) as u32)),
        None => active_monitor_rect()
            .ok()
            .map(|rect| (rect.width.max(0) as u32, rect.height.max(0) as u32)),
    };

    capture::start_recording(
        settings,
        geometry.as_deref(),
        monitor.as_deref(),
        size,
        &output_file,
    )?;

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
    settings: &Settings,
    mode: &capture::CaptureMode,
    notifier: &ui::Notifier,
    pre_captured: Option<capture::CapturedImage>,
    pre_geometry: Option<String>,
    to_clipboard: bool,
) -> Result<()> {
    let output_file = generate_output_path(settings, "png")?;

    if output_file == "-" {
        anyhow::ensure!(
            !settings.annotate,
            "Annotation is not supported with stdout output"
        );
        let png = capture_as_png(
            settings,
            mode,
            pre_captured.as_ref(),
            pre_geometry.as_deref(),
        )?;
        std::io::stdout()
            .lock()
            .write_all(&png)
            .context("Failed to write image to stdout")?;
        return Ok(());
    }

    if settings.annotate {
        let png = capture_as_png(
            settings,
            mode,
            pre_captured.as_ref(),
            pre_geometry.as_deref(),
        )?;
        if let Err(e) = system::annotate(&settings.satty_path, &png, &output_file) {
            log::error!("Annotation failed: {}", e);
            notifier.send_error("Annotation failed", Some(&e.to_string()));
            return Ok(());
        }
        // satty writes output_file only on a save action
        if !std::path::Path::new(&output_file).exists() {
            return Ok(());
        }
        if to_clipboard {
            return copy_to_clipboard(settings, notifier, &output_file);
        }
    } else if let Some(img) = pre_captured {
        capture::save_captured_image(img, &output_file)?;
    } else {
        let rect = image_capture_rect(settings, mode, pre_geometry.as_deref())?;
        capture::capture_screenshot(rect, &output_file)?;
    }

    log::info!("Screenshot saved to {}", output_file);

    if to_clipboard {
        return copy_to_clipboard(settings, notifier, &output_file);
    }

    if should_upload(settings) {
        if let Err(e) = upload_file(settings, &output_file, notifier) {
            log::error!("Upload failed: {}", e);
            // Fallback: copy image to clipboard
            if let Err(clip_err) = system::copy_image(&settings.wl_copy, &output_file) {
                notifier.send_error(
                    "Upload and clipboard copy failed",
                    Some(&clip_err.to_string()),
                );
            } else {
                notifier.send_info("Upload failed - image copied to clipboard");
            }
        }
    } else {
        notifier.send_info(&format!("Screenshot saved: {}", output_file));
        println!("Screenshot saved: {}", output_file);
    }

    Ok(())
}

/// Copies a saved screenshot to the clipboard and notifies the user.
fn copy_to_clipboard(
    settings: &Settings,
    notifier: &ui::Notifier,
    output_file: &str,
) -> Result<()> {
    match system::copy_image(&settings.wl_copy, output_file) {
        Ok(()) => {
            notifier.send_info("Screenshot copied to clipboard");
            println!("Screenshot copied to clipboard: {}", output_file);
        }
        Err(e) => {
            log::error!("Clipboard copy failed: {}", e);
            notifier.send_error("Clipboard copy failed", Some(&e.to_string()));
        }
    }
    Ok(())
}

/// Returns the capture as PNG bytes, from the image the selector already took
/// when there is one, otherwise by capturing the region now.
fn capture_as_png(
    settings: &Settings,
    mode: &capture::CaptureMode,
    pre_captured: Option<&capture::CapturedImage>,
    pre_geometry: Option<&str>,
) -> Result<Vec<u8>> {
    match pre_captured {
        Some(img) => capture::captured_image_to_png(img),
        None => {
            let rect = image_capture_rect(settings, mode, pre_geometry)?;
            capture::capture_png_bytes(rect)
        }
    }
}

/// Computes the capture rectangle for a non-video, non-pre-captured image mode.
fn image_capture_rect(
    settings: &Settings,
    mode: &capture::CaptureMode,
    pre_geometry: Option<&str>,
) -> Result<render::Rect> {
    match mode {
        capture::CaptureMode::ImageArea => {
            if let Some(geo) = pre_geometry {
                return render::Rect::from_geometry_string(geo);
            }
            let selected = ui::App::run(settings.clone())?;
            let geometry = selected.geometry.context("Selection produced no region")?;
            render::Rect::from_geometry_string(&geometry)
        }
        capture::CaptureMode::ImageWindow => {
            let geometry_str = compositor::get_active_window()?;
            render::Rect::from_geometry_string(&geometry_str)
        }
        capture::CaptureMode::ImageScreen => active_monitor_rect(),
        _ => unreachable!(),
    }
}

/// Returns the global rectangle of the active monitor, falling back to the first output.
fn active_monitor_rect() -> Result<render::Rect> {
    let conn =
        wayland_client::Connection::connect_to_env().context("Failed to connect to Wayland")?;
    let outputs = compositor::get_outputs(&conn)?;

    match compositor::get_active_monitor() {
        Ok(name) => outputs
            .iter()
            .find(|(_, n, ..)| n == &name)
            .map(|(_, _, x, y, w, h)| render::Rect::new(*x, *y, *w as i32, *h as i32))
            .with_context(|| format!("Active monitor '{}' not found among outputs", name)),
        Err(e) => {
            log::warn!("Failed to query active monitor: {}. Using first output.", e);
            let (_, _, x, y, w, h) = &outputs[0];
            Ok(render::Rect::new(*x, *y, *w as i32, *h as i32))
        }
    }
}

/// Generates output file path, preferring an explicit --output over a timestamped name.
///
/// Creates the directory the capture will be written to, so a save path that does
/// not exist yet fails here rather than after the capture is already taken.
fn generate_output_path(settings: &Settings, extension: &str) -> Result<String> {
    let path = match settings.output {
        Some(ref output) => output.clone(),
        None => {
            let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
            format!("{}/{}.{}", settings.save_path, timestamp, extension)
        }
    };

    if path != "-" {
        if let Some(dir) = std::path::Path::new(&path).parent().filter(|d| !d.exists()) {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create output directory {}", dir.display()))?;
        }
    }

    Ok(path)
}

/// Checks if upload is configured.
fn should_upload(settings: &Settings) -> bool {
    !settings.zipline_url.is_empty() && !settings.zipline_token.is_empty()
}

/// Uploads file to Zipline.
fn upload_file(settings: &Settings, file_path: &str, notifier: &ui::Notifier) -> Result<String> {
    let (url, service_name) = upload::upload_to_zipline(
        &settings.zipline_url,
        &settings.zipline_token,
        settings.original_name,
        file_path,
    )?;

    // Upload succeeded - try to copy URL to clipboard
    let clipboard_failed = if let Err(e) = system::copy_text(&settings.wl_copy, &url) {
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
    notifier.send_with_action(service_name, message, &url);

    println!("Uploaded: {}", url);

    Ok(url)
}
