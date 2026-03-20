//! OCR text extraction using Tesseract
//!
//! This module extracts text from screen regions using Tesseract OCR.

mod tesseract;

pub use tesseract::extract_text;

use anyhow::Result;
use wayland_client::Connection;

use crate::render::Rect;

/// Captures a screen region and extracts text from it using OCR.
///
/// Identifies which output contains the specified region, captures that output,
/// crops to the selection area, and performs OCR on the resulting image.
pub fn capture_and_ocr(
    conn: &Connection,
    outputs: &[crate::compositor::protocol::outputs::OutputInfo],
    rect: Rect,
) -> Result<String> {
    log::info!(
        "Capturing region for OCR: {}x{} at ({},{})",
        rect.width,
        rect.height,
        rect.x,
        rect.y
    );

    // Capture and crop to the selected region
    let cropped = crate::capture::capture_region(conn, outputs, rect)?;

    // Perform OCR
    extract_text(&cropped)
}
