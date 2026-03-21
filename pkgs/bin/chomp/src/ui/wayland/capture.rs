//! Screenshot capture and OCR completion handling

use anyhow::Result;
use wayland_client::{protocol::wl_output, Connection};

use crate::{capture, cli::Args, ocr, render::Rect, system};

use super::output::OutputSurface;

/// Handles selection completion including screenshot capture, OCR, or coordinate output.
///
/// Returns Some(geometry) if in area mode for the caller to handle, otherwise None.
pub fn complete_selection(
    conn: &Connection,
    output_surfaces: &mut [OutputSurface],
    outputs_map: &[(wl_output::WlOutput, String)],
    args: &Args,
    rect: Rect,
) -> Result<Option<String>> {
    // Clear overlays before capturing
    clear_overlays(output_surfaces);

    // Flush and ensure transparent frames are committed
    let _ = conn.flush();
    let _ = conn.roundtrip();

    // Minimal delay for compositor to render the transparent frame
    std::thread::sleep(std::time::Duration::from_millis(16)); // One frame at 60fps

    // Collect outputs with their names and positions
    let outputs_list: Vec<crate::compositor::protocol::outputs::OutputInfo> = output_surfaces
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

    // Check if we're in mode-based workflow (image-area or video-area)
    if let Some(ref mode_str) = args.mode {
        if mode_str == "image-area" || mode_str == "video-area" {
            // For area modes, return geometry for the caller to handle
            let geometry = rect.to_geometry_string();
            return Ok(Some(geometry));
        }
    }
    
    if args.ocr {
        // OCR mode: capture and extract text
        match ocr::capture_and_ocr(conn, &outputs_list, rect) {
            Ok(text) => {
                println!("{}", text);

                // Copy to clipboard using wl-copy
                if let Err(e) = system::copy_text(&text) {
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
        match capture::capture_and_save(conn, &outputs_list, rect, Some(output_path)) {
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
        let output = rect.to_geometry_string();
        println!("{}", output);
    }

    Ok(None)
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
