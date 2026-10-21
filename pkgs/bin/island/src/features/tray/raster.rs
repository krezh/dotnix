//! Tray icons resampled once to the size they are drawn at, as the GPU shrinks images unfiltered.

use std::{path::Path, sync::Arc};

use gpui::RenderImage;
use image::{
    Frame, RgbaImage,
    imageops::{self, FilterType},
};

use super::icons::ICON;
use crate::components::icon::Picture;

/// Twice the drawn size: sharp on a 2x screen, and a 1x screen halves it evenly.
pub(super) const SIZE: u32 = (ICON * 2.0) as u32;

/// The smallest pixmap at least `SIZE` wide, else the largest, from ARGB32 in network order.
pub(super) fn pixmap(pixmaps: Vec<(i32, i32, Vec<u8>)>) -> Option<Picture> {
    let (width, height, data) = pixmaps
        .into_iter()
        .filter(|(width, height, data)| {
            *width > 0 && *height > 0 && data.len() == (*width as usize) * (*height as usize) * 4
        })
        .min_by_key(|(width, _, _)| (*width < SIZE as i32, width.abs_diff(SIZE as i32)))?;
    let mut bgra = Vec::with_capacity(data.len());
    for [a, r, g, b] in data.as_chunks::<4>().0 {
        bgra.extend_from_slice(&[*b, *g, *r, *a]);
    }
    resampled(RgbaImage::from_raw(width as u32, height as u32, bgra)?)
}

/// A raster icon file, decoded and resampled; vector files are left for GPUI to render.
pub(super) fn file(path: &Path) -> Option<Picture> {
    let mut buffer = image::open(path).ok()?.into_rgba8();
    for pixel in buffer.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    resampled(buffer)
}

/// Fits the longer side to `SIZE`. Filtering runs on premultiplied color, so transparent pixels
/// do not bleed a dark fringe into the edges.
pub(super) fn resampled(mut buffer: RgbaImage) -> Option<Picture> {
    let (width, height) = buffer.dimensions();
    if width.max(height) != SIZE {
        let fit = |side: u32| (side * SIZE).div_ceil(width.max(height)).max(1);
        for pixel in buffer.pixels_mut() {
            let alpha = u16::from(pixel.0[3]);
            for channel in &mut pixel.0[..3] {
                *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
            }
        }
        buffer = imageops::resize(&buffer, fit(width), fit(height), FilterType::Lanczos3);
        for pixel in buffer.pixels_mut() {
            let alpha = u16::from(pixel.0[3]);
            if alpha > 0 {
                for channel in &mut pixel.0[..3] {
                    *channel = ((u16::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
                }
            }
        }
    }
    Some(Picture::Pixels(Arc::new(RenderImage::new(vec![
        Frame::new(buffer),
    ]))))
}

#[cfg(test)]
#[path = "raster_test.rs"]
mod tests;
