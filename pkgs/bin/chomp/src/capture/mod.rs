//! Screen capture domain
//!
//! Handles screenshot and video recording workflows.

pub mod buffer;
pub mod mode;
pub mod screenshot;
pub mod video;

pub use buffer::CapturedImage;
pub use mode::CaptureMode;
pub use screenshot::{
    capture_and_save, capture_png_bytes, capture_screenshot, captured_image_to_png,
    save_captured_image,
};
pub use video::{recording, start_recording, stop_recording};

use anyhow::{Context, Result};

use crate::compositor::{Screencopy, protocol::outputs::find_outputs_for_rect};
use crate::render::Rect;

/// Captures a screen region, from every output it covers.
///
/// This is shared logic used by both screenshot and OCR operations.
pub(crate) fn capture_region(
    screencopy: &mut Screencopy,
    outputs: &[crate::compositor::protocol::outputs::OutputInfo],
    rect: Rect,
) -> Result<CapturedImage> {
    let covering = find_outputs_for_rect(outputs, rect)?;

    log::info!(
        "Capturing {} across {} output(s)",
        rect.describe(),
        covering.len()
    );

    let mut captures = Vec::with_capacity(covering.len());
    for (output, overlap) in covering {
        let local = Rect::new(
            overlap.x - output.logical.x,
            overlap.y - output.logical.y,
            overlap.width,
            overlap.height,
        );
        captures.push((
            overlap,
            screencopy.capture_region(&output.output, local, output.transform)?,
        ));
    }

    let parts: Vec<_> = captures
        .iter()
        .map(|(geometry, image)| (*geometry, image))
        .collect();

    compose_region(rect, &parts)
}

/// Assembles the pixels of `rect` from the outputs it covers.
///
/// One capture only ever covers one output, so a selection spanning monitors is
/// put together here: each output contributes the part of the selection that
/// falls on it, placed at its own offset within the result.
///
/// The result is trimmed to the part of `rect` the outputs actually cover, so a
/// selection hanging off the edge of the desktop yields the visible part rather
/// than a full-size image with blank margins. A gap between monitors of unequal
/// size stays blank, since nothing was ever displayed there.
pub(crate) fn compose_region(
    rect: Rect,
    parts: &[(Rect, &CapturedImage)],
) -> Result<CapturedImage> {
    let covered = parts
        .iter()
        .filter_map(|(geometry, _)| rect.intersection(geometry))
        .reduce(|covered, part| covered.union(&part))
        .with_context(|| format!("Selection {} is not on any output", rect.describe()))?;

    let target_scale = parts
        .iter()
        .map(|(geometry, image)| {
            (image.width as f64 / geometry.width as f64)
                .max(image.height as f64 / geometry.height as f64)
        })
        .reduce(f64::max)
        .context("No captures to compose")?;

    let width = (covered.width as f64 * target_scale).round() as u32;
    let height = (covered.height as f64 * target_scale).round() as u32;
    let stride = width as usize * 4;
    let mut data = vec![0u8; stride * height as usize];

    for (geometry, image) in parts {
        let Some(part) = rect.intersection(geometry) else {
            continue;
        };

        image.copy_into(
            &mut data,
            buffer::CopyPlan {
                dst_stride: stride,
                dst_x: ((part.x - covered.x) as f64 * target_scale).round() as usize,
                dst_y: ((part.y - covered.y) as f64 * target_scale).round() as usize,
                source: Rect::new(
                    part.x - geometry.x,
                    part.y - geometry.y,
                    part.width,
                    part.height,
                ),
                source_scale: (
                    image.width as f64 / geometry.width as f64,
                    image.height as f64 / geometry.height as f64,
                ),
                target_scale,
            },
        );
    }

    CapturedImage::new(
        data,
        width,
        height,
        stride as u32,
        buffer::PixelFormat::Argb8888,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::buffer::PixelFormat;

    fn capture(width: u32, height: u32, fill: u8) -> CapturedImage {
        let data = vec![fill; (width * height * 4) as usize];
        CapturedImage::new(data, width, height, width * 4, PixelFormat::Argb8888).unwrap()
    }

    /// The pixel at (x, y) of a composed image, as its first byte.
    fn pixel(image: &CapturedImage, x: u32, y: u32) -> u8 {
        image.data[(y * image.stride + x * 4) as usize]
    }

    #[test]
    fn composes_a_selection_spanning_two_outputs() {
        let left = capture(4, 4, 0x11);
        let right = capture(4, 4, 0x22);
        let parts = [
            (Rect::new(0, 0, 4, 4), &left),
            (Rect::new(4, 0, 4, 4), &right),
        ];

        // Two columns from the left output, two from the right.
        let composed = compose_region(Rect::new(2, 0, 4, 4), &parts).unwrap();

        assert_eq!((composed.width, composed.height), (4, 4));
        assert_eq!(pixel(&composed, 0, 0), 0x11);
        assert_eq!(pixel(&composed, 1, 3), 0x11);
        assert_eq!(pixel(&composed, 2, 0), 0x22);
        assert_eq!(pixel(&composed, 3, 3), 0x22);
    }

    #[test]
    fn trims_a_selection_hanging_off_the_desktop() {
        let output = capture(4, 4, 0x33);
        let parts = [(Rect::new(0, 0, 4, 4), &output)];

        let composed = compose_region(Rect::new(2, 2, 4, 4), &parts).unwrap();

        assert_eq!((composed.width, composed.height), (2, 2));
        assert_eq!(pixel(&composed, 1, 1), 0x33);
    }

    #[test]
    fn rejects_a_selection_off_every_output() {
        let output = capture(4, 4, 0x44);
        let parts = [(Rect::new(0, 0, 4, 4), &output)];

        assert!(compose_region(Rect::new(10, 10, 2, 2), &parts).is_err());
    }

    #[test]
    fn preserves_the_highest_output_scale() {
        let mut data = vec![0u8; 8 * 8 * 4];
        for (index, pixel) in data.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            pixel[0] = if index % 8 >= 4 { 0xAA } else { 0x55 };
        }
        let scaled = CapturedImage::new(data, 8, 8, 8 * 4, PixelFormat::Argb8888).unwrap();
        let parts = [(Rect::new(0, 0, 4, 4), &scaled)];

        let composed = compose_region(Rect::new(0, 0, 4, 4), &parts).unwrap();

        assert_eq!((composed.width, composed.height), (8, 8));
        assert_eq!(pixel(&composed, 0, 0), 0x55);
        assert_eq!(pixel(&composed, 7, 7), 0xAA);
    }
}
