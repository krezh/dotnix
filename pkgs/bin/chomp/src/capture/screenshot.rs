//! Screenshot capture and saving

use anyhow::{Context, Result};
use image::RgbaImage;
use std::io::Write;
use std::path::Path;
use wayland_client::Connection;

use crate::capture::CapturedImage;
use crate::compositor::protocol::outputs::OutputInfo;
use crate::compositor::{Screencopy, get_outputs};
use crate::render::Rect;

/// Captures a screenshot directly given a rect and saves it to a file.
pub fn capture_screenshot(rect: Rect, output_path: &Path) -> Result<()> {
    let (mut screencopy, outputs) = connect()?;

    capture_and_save(&mut screencopy, &outputs, rect, Some(output_path))
}

/// Captures a screen region and returns it as PNG-encoded bytes.
pub fn capture_png_bytes(rect: Rect) -> Result<Vec<u8>> {
    let (mut screencopy, outputs) = connect()?;

    encode_png(&capture_image(&mut screencopy, &outputs, rect)?)
}

/// Opens a Wayland connection of its own, binds screencopy on it and enumerates
/// the outputs.
fn connect() -> Result<(Screencopy, Vec<OutputInfo>)> {
    let conn = Connection::connect_to_env().context("Failed to connect to Wayland")?;
    let outputs = get_outputs(&conn)?;
    let screencopy = Screencopy::new(&conn)?;

    Ok((screencopy, outputs))
}

/// Captures a screen region and saves it to a file or stdout.
///
/// Public API for use by ui/wayland/capture.rs
pub fn capture_and_save(
    screencopy: &mut Screencopy,
    outputs: &[OutputInfo],
    rect: Rect,
    output_path: Option<&Path>,
) -> Result<()> {
    let img = capture_image(screencopy, outputs, rect)?;

    match output_path {
        Some(path) if path.as_os_str() == "-" => {
            std::io::stdout()
                .lock()
                .write_all(&encode_png(&img)?)
                .context("Failed to write image to stdout")?;
        }
        Some(path) => {
            atomic_write(path, &encode_png(&img)?)
                .with_context(|| format!("Failed to save screenshot to {}", path.display()))?;
            log::info!("Screenshot saved to {}", path.display());
        }
        None => {
            anyhow::bail!("No output path specified for screenshot");
        }
    }

    Ok(())
}

/// Captures and crops a screen region into an RGBA image.
fn capture_image(
    screencopy: &mut Screencopy,
    outputs: &[OutputInfo],
    rect: Rect,
) -> Result<RgbaImage> {
    log::info!(
        "Capturing region: {}x{} at ({},{})",
        rect.width,
        rect.height,
        rect.x,
        rect.y
    );

    let cropped = crate::capture::capture_region(screencopy, outputs, rect)?;
    RgbaImage::from_raw(cropped.width, cropped.height, cropped.to_rgba())
        .context("Failed to create image from buffer")
}

/// Saves a `CapturedImage` (ARGB8888) directly to a PNG file.
pub fn save_captured_image(img: CapturedImage, output_path: &Path) -> Result<()> {
    atomic_write(output_path, &captured_image_to_png(&img)?)
        .with_context(|| format!("Failed to save screenshot to {}", output_path.display()))
}

/// Encodes a `CapturedImage` (ARGB8888) as PNG bytes (for annotate path).
pub fn captured_image_to_png(img: &CapturedImage) -> Result<Vec<u8>> {
    let rgba_image = RgbaImage::from_raw(img.width, img.height, img.to_rgba())
        .context("Failed to construct RGBA image from captured buffer")?;
    encode_png(&rgba_image)
}

/// Encodes an RGBA image as PNG bytes.
fn encode_png(img: &RgbaImage) -> Result<Vec<u8>> {
    use image::{ImageEncoder, codecs::png::PngEncoder};

    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgba8,
        )
        .context("Failed to encode PNG")?;

    Ok(png)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::fs::OpenOptions;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));

    for _ in 0..100 {
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(".chomp-{}-{}.tmp", std::process::id(), sequence));
        match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(mut file) => {
                if let Err(error) = file.write_all(bytes).and_then(|_| file.sync_all()) {
                    let _ = std::fs::remove_file(&temp);
                    return Err(error).context("Failed to write temporary screenshot");
                }
                if let Err(error) = std::fs::rename(&temp, path) {
                    let _ = std::fs::remove_file(&temp);
                    return Err(error).context("Failed to publish screenshot");
                }
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error).context("Failed to create temporary screenshot"),
        }
    }

    anyhow::bail!("Could not allocate a temporary screenshot file")
}

#[cfg(test)]
#[path = "screenshot_test.rs"]
mod tests;
