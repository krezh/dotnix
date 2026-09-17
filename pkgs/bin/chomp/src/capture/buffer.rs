//! Raw image buffer handling and pixel format conversion

use anyhow::{Context, Result};
use nix::sys::mman;
use std::ffi::c_void;
use std::num::NonZeroUsize;
use std::os::fd::OwnedFd;
use std::ptr::NonNull;
use wayland_client::protocol::wl_shm;

use crate::render::Rect;

/// Pixels held either in a normal allocation or in the shared memory the
/// compositor captured into.
///
/// A capture is the size of the whole screen, so mapping the compositor's buffer
/// rather than reading it into a fresh allocation saves both a copy and the
/// zeroing of the destination.
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

impl Pixels {
    /// Returns the pixels as an owned buffer, copying only a mapping.
    pub fn into_vec(self) -> Vec<u8> {
        match self {
            Self::Owned(data) => data,
            Self::Mapped(mapping) => mapping.to_vec(),
        }
    }
}

/// A read-only memory mapping of a capture, unmapped when dropped.
pub struct MappedPixels {
    ptr: NonNull<c_void>,
    len: usize,
}

// SAFETY: the mapping is private and read-only, and nothing else holds the
// pointer, so it is sound to move between threads.
unsafe impl Send for MappedPixels {}

impl MappedPixels {
    /// Maps `len` bytes of `fd` for reading.
    ///
    /// The mapping stays valid after `fd` is closed, so the caller may drop the
    /// descriptor as soon as this returns.
    pub fn map(fd: &OwnedFd, len: usize) -> Result<Self> {
        let size = NonZeroUsize::new(len).context("Cannot map an empty capture")?;

        // SAFETY: `fd` is a sealed memfd of at least `len` bytes, and the mapping
        // is only read through the slice handed out by `Deref`.
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
        // SAFETY: the pointer and length are the ones mmap returned, and no
        // slice handed out by `Deref` outlives `self`.
        if let Err(e) = unsafe { mman::munmap(self.ptr, self.len) } {
            log::warn!("Failed to unmap a captured buffer: {}", e);
        }
    }
}

/// Represents captured image data with metadata
pub struct CapturedImage {
    pub data: Pixels,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: wl_shm::Format,
}

impl CapturedImage {
    /// Creates a new CapturedImage.
    pub fn new(
        data: impl Into<Pixels>,
        width: u32,
        height: u32,
        stride: u32,
        format: wl_shm::Format,
    ) -> Self {
        Self {
            data: data.into(),
            width,
            height,
            stride,
            format,
        }
    }

    /// Copies `source` out of this image into `dst` at (`dst_x`, `dst_y`).
    ///
    /// `source` is given in the output's layout coordinates; `scale` converts it
    /// to the captured pixels, which are denser on a scaled output. At scale 1
    /// this is a row-by-row copy; otherwise each destination pixel takes the
    /// nearest source pixel, keeping the geometry right without resampling.
    ///
    /// Anything that would fall outside either image is skipped.
    pub fn copy_into(
        &self,
        dst: &mut [u8],
        dst_stride: usize,
        dst_x: usize,
        dst_y: usize,
        source: Rect,
        scale: (f64, f64),
    ) {
        let unscaled = (scale.0 - 1.0).abs() < f64::EPSILON && (scale.1 - 1.0).abs() < f64::EPSILON;

        for row in 0..source.height.max(0) as usize {
            let dst_start = (dst_y + row) * dst_stride + dst_x * 4;
            let row_bytes = source.width.max(0) as usize * 4;

            let Some(dst_row) = dst.get_mut(dst_start..dst_start + row_bytes) else {
                break;
            };

            if unscaled {
                let src_start =
                    (source.y as usize + row) * self.stride as usize + source.x as usize * 4;

                let Some(src_row) = self.data.get(src_start..src_start + row_bytes) else {
                    break;
                };

                dst_row.copy_from_slice(src_row);
                continue;
            }

            let src_y = ((source.y as f64 + row as f64) * scale.1) as usize;

            for (column, pixel) in dst_row.chunks_exact_mut(4).enumerate() {
                let src_x = ((source.x as f64 + column as f64) * scale.0) as usize;
                let src_start = src_y * self.stride as usize + src_x * 4;

                if let Some(src_pixel) = self.data.get(src_start..src_start + 4) {
                    pixel.copy_from_slice(src_pixel);
                }
            }
        }
    }
}
