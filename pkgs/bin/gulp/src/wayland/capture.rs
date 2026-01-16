//! Screenshot capture and OCR completion handling

use anyhow::{Context, Result};
use wayland_client::{protocol::wl_output, Connection};

use crate::{cli::Args, ocr, selection::Rect};

use super::output::OutputSurface;

/// Copies text to the Wayland clipboard using wl-copy.
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .context("Failed to spawn wl-copy process")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .context("Failed to write to wl-copy stdin")?;
    }

    let status = child.wait().context("Failed to wait for wl-copy")?;

    if status.success() {
        log::info!("Text copied to clipboard ({} bytes)", text.len());
        Ok(())
    } else {
        anyhow::bail!("wl-copy exited with status: {}", status)
    }
}

/// Handles selection completion including screenshot capture, OCR, or coordinate output.
pub fn complete_selection(
    conn: &Connection,
    output_surfaces: &mut [OutputSurface],
    outputs_map: &[(wl_output::WlOutput, String)],
    args: &Args,
    rect: Rect,
) -> Result<()> {
    // Clear overlays before capturing
    clear_overlays(output_surfaces);

    // Flush and ensure transparent frames are committed
    let _ = conn.flush();
    let _ = conn.roundtrip();

    // Minimal delay for compositor to render the transparent frame
    std::thread::sleep(std::time::Duration::from_millis(16)); // One frame at 60fps

    // Collect outputs with their names and positions
    let outputs_list: Vec<(wl_output::WlOutput, String, i32, i32, u32, u32)> = output_surfaces
        .iter()
        .map(|surf| {
            let name = outputs_map
                .iter()
                .find(|(out, _)| out == &surf._output)
                .map(|(_, n)| n.clone())
                .unwrap_or_default();
            (
                surf._output.clone(),
                name,
                surf.x,
                surf.y,
                surf.width,
                surf.height,
            )
        })
        .collect();

    if args.ocr {
        // OCR mode: capture and extract text
        match ocr::capture_and_ocr(conn, &outputs_list, rect) {
            Ok(text) => {
                println!("{}", text);

                // Copy to clipboard using wl-copy
                if let Err(e) = copy_to_clipboard(&text) {
                    log::warn!("Failed to copy to clipboard: {}", e);
                }
            }
            Err(e) => {
                eprintln!("OCR failed: {}", e);
                std::process::exit(1);
            }
        }
    } else if let Some(ref output_path) = args.output {
        // Screenshot mode: capture and save to file or stdout
        match ocr::capture_and_save(conn, &outputs_list, rect, Some(output_path)) {
            Ok(()) => {
                // Success - file saved or written to stdout
            }
            Err(e) => {
                eprintln!("Screenshot capture failed: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // Coordinate output mode: output coordinates only
        let output = format_output(rect.x, rect.y, rect.width, rect.height);
        println!("{}", output);
    }

    Ok(())
}

/// Clears all output surfaces by rendering fully transparent overlays.
fn clear_overlays(output_surfaces: &mut [OutputSurface]) {
    for output_surface in output_surfaces {
        let width = output_surface.width as i32;
        let height = output_surface.height as i32;
        let stride = width * 4;

        if let Some(pool) = output_surface.pool.as_mut() {
            if let Ok((buffer, canvas)) = pool.create_buffer(
                width,
                height,
                stride,
                wayland_client::protocol::wl_shm::Format::Argb8888,
            ) {
                // Fill with fully transparent pixels
                for byte in canvas.iter_mut() {
                    *byte = 0;
                }

                output_surface
                    .surface
                    .attach(Some(buffer.wl_buffer()), 0, 0);
                output_surface.surface.damage_buffer(0, 0, width, height);
                output_surface.surface.commit();
            }
        }
    }
}

/// Formats selection coordinates into output string.
fn format_output(x: i32, y: i32, width: i32, height: i32) -> String {
    "%x,%y %wx%h"
        .replace("%x", &x.to_string())
        .replace("%y", &y.to_string())
        .replace("%w", &width.to_string())
        .replace("%h", &height.to_string())
        .replace("%X", &(x + width).to_string())
        .replace("%Y", &(y + height).to_string())
}
