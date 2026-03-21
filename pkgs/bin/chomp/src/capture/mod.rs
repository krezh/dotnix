//! Screen capture domain
//!
//! Handles screenshot and video recording workflows.

pub mod buffer;
pub mod mode;
pub mod screenshot;
pub mod video;

pub use buffer::CapturedImage;
pub use mode::CaptureMode;
pub use screenshot::{capture_and_save, capture_screenshot};
pub use video::VideoRecorder;

use anyhow::Result;
use wayland_client::Connection;

use crate::compositor::{capture_output, protocol::outputs::find_output_for_rect};
use crate::render::Rect;

/// Captures a screen region and returns the cropped image data.
///
/// Handles the common capture workflow: finding the output, capturing it, and cropping to the region.
/// This is shared logic used by both screenshot and OCR operations.
pub(crate) fn capture_region(
    conn: &Connection,
    outputs: &[crate::compositor::protocol::outputs::OutputInfo],
    rect: Rect,
) -> Result<CapturedImage> {
    // Find which output contains the selection and get local coordinates
    let (output, local_rect) = find_output_for_rect(outputs, rect)?;

    log::info!(
        "Capturing {}x{} at ({},{}) from output",
        local_rect.width, local_rect.height,
        local_rect.x, local_rect.y
    );

    let captured = capture_output(conn, output)?;
    captured.crop(local_rect)
}
