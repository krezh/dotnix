//! Screenshot capture and OCR completion handling

use anyhow::Result;
use wayland_client::{Connection, protocol::wl_output};

use crate::{capture, capture::CaptureMode, cli::Settings, ocr, render::Rect, system};

use super::output::OutputSurface;

/// Handles selection completion including screenshot capture, OCR, or coordinate output.
///
/// Returns Some(geometry) if in area mode for the caller to handle, otherwise None.
pub fn complete_selection(
    conn: &Connection,
    output_surfaces: &mut [OutputSurface],
    outputs_map: &[(wl_output::WlOutput, String)],
    settings: &Settings,
    rect: Rect,
) -> Result<Option<String>> {
    clear_overlays(output_surfaces);
    let _ = conn.roundtrip();
    let _ = conn.roundtrip();

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

    if matches!(
        settings.mode,
        Some(CaptureMode::ImageArea | CaptureMode::VideoArea)
    ) {
        // For area modes, return geometry for the caller to handle
        return Ok(Some(rect.to_geometry_string()));
    }

    if settings.ocr {
        // OCR mode: capture and extract text
        let text = ocr::capture_and_ocr(conn, &outputs_list, rect)?;
        println!("{}", text);

        // Copy to clipboard using wl-copy
        if let Err(e) = system::copy_text(&text) {
            log::warn!("Failed to copy to clipboard: {}", e);
        }
    } else if let Some(ref output_path) = settings.output {
        // Screenshot mode: capture and save to file or stdout
        capture::capture_and_save(conn, &outputs_list, rect, Some(output_path))?;
    } else {
        // Coordinate output mode: output coordinates only
        println!("{}", rect.to_geometry_string());
    }

    Ok(None)
}

/// Unmaps all output surfaces by attaching a null buffer, removing them from compositor scene.
pub(super) fn clear_overlays(output_surfaces: &mut [OutputSurface]) {
    for output_surface in output_surfaces {
        output_surface.surface.attach(None, 0, 0);
        output_surface.surface.commit();
    }
}
