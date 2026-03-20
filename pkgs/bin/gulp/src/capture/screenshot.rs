//! Screenshot capture and saving

use anyhow::{Context, Result};
use wayland_client::Connection;

use crate::compositor::get_outputs;
use crate::render::{convert_argb_to_rgba, Rect};

/// Captures a screenshot directly given a rect and saves it to a file.
pub fn capture_screenshot(rect: Rect, output_path: &str) -> Result<()> {
    let conn = Connection::connect_to_env().context("Failed to connect to Wayland")?;
    let outputs = get_outputs(&conn)?;
    
    capture_and_save(&conn, &outputs, rect, Some(output_path))
}

/// Captures a screen region and saves it to a file or stdout.
///
/// Public API for use by ui/wayland/capture.rs
pub fn capture_and_save(
    conn: &Connection,
    outputs: &[crate::compositor::protocol::outputs::OutputInfo],
    rect: Rect,
    output_path: Option<&str>,
) -> Result<()> {
    log::info!(
        "Capturing region: {}x{} at ({},{})",
        rect.width,
        rect.height,
        rect.x,
        rect.y
    );

    // Capture and crop to the selected region
    let cropped = crate::capture::capture_region(conn, outputs, rect)?;

    // Convert ARGB to RGBA for image library
    let rgba_buffer = convert_argb_to_rgba(&cropped.data);

    // Create image from buffer
    let img = image::RgbaImage::from_raw(cropped.width, cropped.height, rgba_buffer)
        .context("Failed to create image from buffer")?;

    // Save or write to stdout
    match output_path {
        Some("-") => {
            use image::{codecs::png::PngEncoder, ImageEncoder};

            let mut stdout = std::io::stdout().lock();
            let encoder = PngEncoder::new(&mut stdout);
            encoder.write_image(
                img.as_raw(),
                cropped.width,
                cropped.height,
                image::ExtendedColorType::Rgba8,
            ).context("Failed to write image to stdout")?;
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
