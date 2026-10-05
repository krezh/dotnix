//! Raw image buffer handling and pixel format conversion

use anyhow::{Context, Result};
use nix::sys::mman;
use std::ffi::c_void;
use std::num::NonZeroUsize;
use std::os::fd::OwnedFd;
use std::ptr::NonNull;
use wayland_client::protocol::wl_output;

use crate::render::Rect;

pub enum Pixels {
    Owned(Vec<u8>),
    Mapped(MappedPixels),
}

impl std::ops::Deref for Pixels {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match self {
            Self::Owned(data) => data,
            Self::Mapped(mapping) => mapping,
        }
    }
}

impl From<Vec<u8>> for Pixels {
    fn from(data: Vec<u8>) -> Self {
        Self::Owned(data)
    }
}

impl From<MappedPixels> for Pixels {
    fn from(mapping: MappedPixels) -> Self {
        Self::Mapped(mapping)
    }
}

pub struct MappedPixels {
    ptr: NonNull<c_void>,
    len: usize,
}

// SAFETY: the mapping is private, read-only, and has no aliased mutable access.
unsafe impl Send for MappedPixels {}

impl MappedPixels {
    pub fn map(fd: &OwnedFd, len: usize) -> Result<Self> {
        let size = NonZeroUsize::new(len).context("Cannot map an empty capture")?;
        // SAFETY: `fd` is a sealed memfd of at least `len` bytes.
        let ptr = unsafe {
            mman::mmap(
                None,
                size,
                mman::ProtFlags::PROT_READ,
                mman::MapFlags::MAP_PRIVATE,
                fd,
                0,
            )
            .context("Failed to map the captured buffer")?
        };
        Ok(Self { ptr, len })
    }
}

impl std::ops::Deref for MappedPixels {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        // SAFETY: the mapping covers `len` bytes and lives as long as `self`.
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr().cast::<u8>(), self.len) }
    }
}

impl Drop for MappedPixels {
    fn drop(&mut self) {
        // SAFETY: these are the pointer and length returned by `mmap`.
        if let Err(error) = unsafe { mman::munmap(self.ptr, self.len) } {
            log::warn!("Failed to unmap a captured buffer: {}", error);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Argb8888,
    Xrgb8888,
}

pub struct CapturedImage {
    pub data: Pixels,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
}

pub struct CopyPlan {
    pub dst_stride: usize,
    pub dst_x: usize,
    pub dst_y: usize,
    pub source: Rect,
    pub source_scale: (f64, f64),
    pub target_scale: f64,
}
impl CapturedImage {
    pub fn new(
        data: impl Into<Pixels>,
        width: u32,
        height: u32,
        stride: u32,
        format: PixelFormat,
    ) -> Result<Self> {
        let data = data.into();
        anyhow::ensure!(width > 0 && height > 0, "Captured image is empty");
        anyhow::ensure!(stride >= width * 4, "Captured image stride is too short");
        anyhow::ensure!(
            data.len() >= stride as usize * height as usize,
            "Captured image buffer is shorter than its dimensions"
        );
        Ok(Self {
            data,
            width,
            height,
            stride,
            format,
        })
    }

    pub fn normalize(self, transform: wl_output::Transform, y_inverted: bool) -> Result<Self> {
        if transform == wl_output::Transform::Normal && !y_inverted {
            return Ok(self);
        }

        let swaps_axes = matches!(
            transform,
            wl_output::Transform::_90
                | wl_output::Transform::_270
                | wl_output::Transform::Flipped90
                | wl_output::Transform::Flipped270
        );
        let (width, height) = if swaps_axes {
            (self.height, self.width)
        } else {
            (self.width, self.height)
        };
        let stride = width * 4;
        let mut data = vec![0; stride as usize * height as usize];

        for y in 0..height {
            for x in 0..width {
                let (source_x, mut source_y) =
                    inverse_transform(transform, x, y, self.width, self.height);
                if y_inverted {
                    source_y = self.height - 1 - source_y;
                }
                let source = source_y as usize * self.stride as usize + source_x as usize * 4;
                let destination = y as usize * stride as usize + x as usize * 4;
                data[destination..destination + 4].copy_from_slice(&self.data[source..source + 4]);
            }
        }

        Self::new(data, width, height, stride, self.format)
    }

    pub fn to_rgba(&self) -> Vec<u8> {
        let mut rgba = vec![0; self.width as usize * self.height as usize * 4];
        for y in 0..self.height as usize {
            let source = &self.data
                [y * self.stride as usize..y * self.stride as usize + self.width as usize * 4];
            let destination =
                &mut rgba[y * self.width as usize * 4..(y + 1) * self.width as usize * 4];
            for (source, destination) in source
                .as_chunks::<4>()
                .0
                .iter()
                .zip(destination.as_chunks_mut::<4>().0)
            {
                destination[0] = source[2];
                destination[1] = source[1];
                destination[2] = source[0];
                destination[3] = match self.format {
                    PixelFormat::Argb8888 => source[3],
                    PixelFormat::Xrgb8888 => 255,
                };
            }
        }
        rgba
    }

    pub fn copy_into(&self, dst: &mut [u8], plan: CopyPlan) {
        let CopyPlan {
            dst_stride,
            dst_x,
            dst_y,
            source,
            source_scale,
            target_scale,
        } = plan;
        let width = (source.width.max(0) as f64 * target_scale).round() as usize;
        let height = (source.height.max(0) as f64 * target_scale).round() as usize;

        for row in 0..height {
            let destination_y = dst_y + row;
            let source_y =
                ((source.y as f64 + row as f64 / target_scale) * source_scale.1).floor() as usize;
            for column in 0..width {
                let destination = destination_y * dst_stride + (dst_x + column) * 4;
                let source_x = ((source.x as f64 + column as f64 / target_scale) * source_scale.0)
                    .floor() as usize;
                let source = source_y * self.stride as usize + source_x * 4;
                if let (Some(destination), Some(source)) = (
                    dst.get_mut(destination..destination + 4),
                    self.data.get(source..source + 4),
                ) {
                    destination.copy_from_slice(source);
                    if self.format == PixelFormat::Xrgb8888 {
                        destination[3] = 255;
                    }
                }
            }
        }
    }
}

fn inverse_transform(
    transform: wl_output::Transform,
    x: u32,
    y: u32,
    source_width: u32,
    source_height: u32,
) -> (u32, u32) {
    match transform {
        wl_output::Transform::Normal => (x, y),
        wl_output::Transform::_90 => (y, source_height - 1 - x),
        wl_output::Transform::_180 => (source_width - 1 - x, source_height - 1 - y),
        wl_output::Transform::_270 => (source_width - 1 - y, x),
        wl_output::Transform::Flipped => (source_width - 1 - x, y),
        wl_output::Transform::Flipped90 => (source_width - 1 - y, source_height - 1 - x),
        wl_output::Transform::Flipped180 => (x, source_height - 1 - y),
        wl_output::Transform::Flipped270 => (y, x),
        _ => (x, y),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_padded_xrgb_rows_without_skewing() {
        let data = vec![
            1, 2, 3, 0, 4, 5, 6, 0, 99, 99, 99, 99, 7, 8, 9, 0, 10, 11, 12, 0, 99, 99, 99, 99,
        ];
        let image = CapturedImage::new(data, 2, 2, 12, PixelFormat::Xrgb8888).unwrap();

        assert_eq!(
            image.to_rgba(),
            vec![3, 2, 1, 255, 6, 5, 4, 255, 9, 8, 7, 255, 12, 11, 10, 255]
        );
    }

    #[test]
    fn normalizes_rotated_outputs() {
        let image = CapturedImage::new(
            vec![1, 0, 0, 255, 2, 0, 0, 255],
            2,
            1,
            8,
            PixelFormat::Argb8888,
        )
        .unwrap()
        .normalize(wl_output::Transform::_90, false)
        .unwrap();

        assert_eq!((image.width, image.height), (1, 2));
        assert_eq!(image.data[0], 1);
        assert_eq!(image.data[4], 2);
    }

    #[test]
    fn normalizes_y_inverted_frames() {
        let image = CapturedImage::new(
            vec![1, 0, 0, 255, 2, 0, 0, 255],
            1,
            2,
            4,
            PixelFormat::Argb8888,
        )
        .unwrap()
        .normalize(wl_output::Transform::Normal, true)
        .unwrap();

        assert_eq!(image.data[0], 2);
        assert_eq!(image.data[4], 1);
    }
}
