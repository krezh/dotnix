//! Screenshot capture and OCR completion handling

use anyhow::Result;
use wayland_client::Connection;

use crate::compositor::Screencopy;
use crate::compositor::protocol::outputs::OutputInfo;
use crate::{
    capture, capture::CaptureMode, capture::CapturedImage, cli::Settings, ocr, render::Rect, system,
};

use super::output::OutputSurface;

/// Handles selection completion including screenshot capture, OCR, or coordinate output.
///
/// Returns the geometry in area mode for the caller to handle, along with the
/// selected image itself when it could be cropped out of the frozen screen.
pub fn complete_selection(
    conn: &Connection,
    screencopy: &mut Screencopy,
    output_surfaces: &mut [OutputSurface],
    outputs_list: &[OutputInfo],
    settings: &Settings,
    rect: Rect,
) -> Result<(Option<String>, Option<CapturedImage>)> {
    clear_overlays(output_surfaces);
    let _ = conn.roundtrip();
    let _ = conn.roundtrip();

    if matches!(
        settings.mode,
        Some(CaptureMode::ImageArea | CaptureMode::VideoArea)
    ) {
        // Recording needs coordinates, not pixels.
        let cropped = if settings.mode == Some(CaptureMode::ImageArea) {
            crop_frozen(output_surfaces, outputs_list, settings, rect)
        } else {
            None
        };

        return Ok((Some(rect.to_geometry_string()), cropped));
    }

    if settings.ocr {
        // OCR reads the same selected pixels a screenshot would save.
        let language = &settings.ocr_language;
        let text = match crop_frozen(output_surfaces, outputs_list, settings, rect) {
            Some(image) => ocr::extract_text(&image, language)?,
            None => ocr::capture_and_ocr(screencopy, outputs_list, rect, language)?,
        };
        println!("{}", text);

        // Copy to clipboard using wl-copy
        if let Err(e) = system::copy_text(&settings.wl_copy, &text) {
            log::warn!("Failed to copy to clipboard: {}", e);
        }
    } else if let Some(ref output_path) = settings.output {
        // Screenshot mode: capture and save to file or stdout
        capture::capture_and_save(screencopy, outputs_list, rect, Some(output_path))?;
    } else {
        // Coordinate output mode: output coordinates only
        println!("{}", rect.to_geometry_string());
    }

    Ok((None, None))
}

/// Crops the selection out of the frozen screenshot it was made on.
///
/// Freezing already holds the exact pixels the user selected, so every consumer
/// of a selection takes them from here: a second capture races the compositor's
/// teardown of the overlay and can return a different frame.
///
/// Returns `None` when there is nothing to crop — freeze is off, or the selection
/// is not on an output with a frozen screenshot — leaving the caller to capture.
fn crop_frozen(
    output_surfaces: &[OutputSurface],
    outputs: &[OutputInfo],
    settings: &Settings,
    rect: Rect,
) -> Option<CapturedImage> {
    use crate::compositor::protocol::outputs::find_outputs_for_rect;

    if !settings.freeze {
        return None;
    }

    let covering = find_outputs_for_rect(outputs, rect)
        .inspect_err(|e| log::warn!("Failed to locate the selected outputs: {}", e))
        .ok()?;

    // Every covered output has to have a frozen screen, or the result would be
    // missing part of the selection; capturing it live is better than that.
    let mut parts = Vec::with_capacity(covering.len());
    for (output, geometry) in covering {
        let frozen = output_surfaces
            .iter()
            .find(|surface| &surface.output == output)?
            .frozen_buffer
            .as_ref()?;

        parts.push((geometry, frozen));
    }

    capture::compose_region(rect, &parts)
        .inspect_err(|e| {
            log::warn!(
                "Failed to assemble the selection from the frozen screen: {}. Capturing it instead.",
                e
            )
        })
        .ok()
}

/// Unmaps all output surfaces by attaching a null buffer, removing them from compositor scene.
pub(super) fn clear_overlays(output_surfaces: &mut [OutputSurface]) {
    for output_surface in output_surfaces {
        output_surface.surface.attach(None, 0, 0);
        output_surface.surface.commit();
    }
}
