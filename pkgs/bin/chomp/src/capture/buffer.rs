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
    pub fn new(
        data: Vec<u8>,
        width: u32,
        height: u32,
        stride: u32,
        format: wl_shm::Format,
    ) -> Self {
        Self {
            data,
            width,
            height,
            stride,
            format,
        }
    }

    /// Crops the image to the specified rectangular region, clipped to the image.
    ///
    /// The rectangle may start outside the image — a window can hang off the edge
    /// of its output, and a selection can start on a neighbouring monitor — so the
    /// intersection is computed in `i64`, keeping a negative origin from wrapping
    /// into a huge offset.
    pub fn crop(&self, rect: Rect) -> Result<CapturedImage> {
        let left = (rect.x as i64).max(0);
        let top = (rect.y as i64).max(0);
        let right = (rect.x as i64 + rect.width as i64).min(self.width as i64);
        let bottom = (rect.y as i64 + rect.height as i64).min(self.height as i64);

        if right <= left || bottom <= top {
            anyhow::bail!(
                "Crop region {} lies outside the {}x{} capture",
                rect.describe(),
                self.width,
                self.height
            );
        }

        let left = left as u32;
        let top = top as u32;
        let rect_width = (right as u32) - left;
        let rect_height = (bottom as u32) - top;

        log::debug!(
            "Cropping {}x{} region at ({},{}) from {}x{} image (stride: {}, format: {:?})",
            rect_width,
            rect_height,
            left,
            top,
            self.width,
            self.height,
            self.stride,
            self.format
        );

        let expected_size = rect_width
            .checked_mul(rect_height)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| {
                anyhow::anyhow!("Crop region size overflow: {}x{}", rect_width, rect_height)
            })? as usize;

        let row_start = |y: u32| (top + y) as usize * self.stride as usize + left as usize * 4;

        let last_row_offset = row_start(rect_height - 1);
        let row_size = (rect_width * 4) as usize;

        if last_row_offset + row_size > self.data.len() {
            anyhow::bail!(
                "Crop region extends beyond buffer bounds: last_row_offset={}, row_size={}, buffer_len={}",
                last_row_offset,
                row_size,
                self.data.len()
            );
        }

        let mut cropped_data = vec![0u8; expected_size];

        for y in 0..rect_height {
            let src_offset = row_start(y);
            let dst_offset = (y * rect_width * 4) as usize;
            cropped_data[dst_offset..dst_offset + row_size]
                .copy_from_slice(&self.data[src_offset..src_offset + row_size]);
        }

        log::debug!(
            "Cropped buffer size: {}, expected: {}",
            cropped_data.len(),
            expected_size
        );

        Ok(CapturedImage {
            data: cropped_data,
            width: rect_width,
            height: rect_height,
            stride: rect_width * 4,
            format: self.format,
        })
    }
}
