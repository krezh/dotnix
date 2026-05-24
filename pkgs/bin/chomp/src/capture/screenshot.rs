//! Screenshot capture and saving

use anyhow::{Context, Result};
use image::RgbaImage;
use std::io::Write;
use wayland_client::Connection;

use crate::compositor::get_outputs;
use crate::compositor::protocol::outputs::OutputInfo;
use crate::render::{Rect, convert_argb_to_rgba};

/// Captures a screenshot directly given a rect and saves it to a file.
pub fn capture_screenshot(rect: Rect, output_path: &str) -> Result<()> {
    let conn = Connection::connect_to_env().context("Failed to connect to Wayland")?;
    let outputs = get_outputs(&conn)?;

    capture_and_save(&conn, &outputs, rect, Some(output_path))
}

/// Captures a screen region and returns it as PNG-encoded bytes.
pub fn capture_png_bytes(rect: Rect) -> Result<Vec<u8>> {
    let conn = Connection::connect_to_env().context("Failed to connect to Wayland")?;
    let outputs = get_outputs(&conn)?;

    encode_png(&capture_image(&conn, &outputs, rect)?)
}

/// Captures a screen region and saves it to a file or stdout.
///
/// Public API for use by ui/wayland/capture.rs
pub fn capture_and_save(
    conn: &Connection,
    outputs: &[OutputInfo],
    rect: Rect,
    output_path: Option<&str>,
) -> Result<()> {
    let img = capture_image(conn, outputs, rect)?;

    match output_path {
        Some("-") => {
            std::io::stdout()
                .lock()
                .write_all(&encode_png(&img)?)
                .context("Failed to write image to stdout")?;
        }
        Some(path) => {
            img.save(path)
                .with_context(|| format!("Failed to save screenshot to {}", path))?;
            log::info!("Screenshot saved to {}", path);
        }
        None => {
            anyhow::bail!("No output path specified for screenshot");
        }
    }

    Ok(())
}

/// Captures and crops a screen region into an RGBA image.
fn capture_image(conn: &Connection, outputs: &[OutputInfo], rect: Rect) -> Result<RgbaImage> {
    log::info!(
        "Capturing region: {}x{} at ({},{})",
        rect.width,
        rect.height,
        rect.x,
        rect.y
    );

    let cropped = crate::capture::capture_region(conn, outputs, rect)?;
    let rgba_buffer = convert_argb_to_rgba(&cropped.data);

    RgbaImage::from_raw(cropped.width, cropped.height, rgba_buffer)
        .context("Failed to create image from buffer")
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
