//! Raw image buffer handling and pixel format conversion

use anyhow::Result;
use wayland_client::protocol::wl_shm;

use crate::render::Rect;

/// Represents captured image data with metadata
pub struct CapturedImage {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: wl_shm::Format,
}

impl CapturedImage {
    /// Creates a new CapturedImage.
    pub fn new(data: Vec<u8>, width: u32, height: u32, stride: u32, format: wl_shm::Format) -> Self {
        Self {
            data,
            width,
            height,
            stride,
            format,
        }
    }

    /// Returns the width of the image.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the height of the image.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Crops the image to the specified rectangular region.
    ///
    /// Returns a new `CapturedImage` containing only the pixels within the given rectangle.
    /// Adjusts dimensions automatically if the rectangle extends beyond image boundaries.
    pub fn crop(&self, rect: Rect) -> Result<CapturedImage> {
        let rect_width = rect.width.min((self.width as i32) - rect.x) as u32;
        let rect_height = rect.height.min((self.height as i32) - rect.y) as u32;

        log::debug!(
            "Cropping {}x{} region from {}x{} image (stride: {}, format: {:?})",
            rect_width, rect_height, self.width, self.height, self.stride, self.format
        );

        let expected_size = rect_width
            .checked_mul(rect_height)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| anyhow::anyhow!("Crop region size overflow: {}x{}", rect_width, rect_height))?
            as usize;

        // Pre-validate that entire crop region is within buffer bounds
        let last_row_offset = ((rect.y as u32 + rect_height - 1) * self.stride + rect.x as u32 * 4) as usize;
        let row_size = (rect_width * 4) as usize;

        if last_row_offset + row_size > self.data.len() {
            anyhow::bail!(
                "Crop region extends beyond buffer bounds: last_row_offset={}, row_size={}, buffer_len={}",
                last_row_offset, row_size, self.data.len()
            );
        }

        let mut cropped_data = vec![0u8; expected_size];

        for y in 0..rect_height {
            let src_offset = ((rect.y as u32 + y) * self.stride + rect.x as u32 * 4) as usize;
            let dst_offset = (y * rect_width * 4) as usize;
            cropped_data[dst_offset..dst_offset + row_size]
                .copy_from_slice(&self.data[src_offset..src_offset + row_size]);
        }

        log::debug!("Cropped buffer size: {}, expected: {}", cropped_data.len(), expected_size);

        Ok(CapturedImage {
            data: cropped_data,
            width: rect_width,
            height: rect_height,
            stride: rect_width * 4,
            format: self.format,
        })
    }
}
