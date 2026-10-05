//! Pixel buffer utilities

use super::Rect;

pub fn dim_argb(buffer: &[u8], opacity: f64) -> Vec<u8> {
    let keep = ((1.0 - opacity.clamp(0.0, 1.0)) * 255.0).round() as u32;
    let mut dimmed = buffer.to_vec();

    for pixel in dimmed.as_chunks_mut::<4>().0 {
        for channel in &mut pixel[..3] {
            *channel = (*channel as u32 * keep / 255) as u8;
        }
    }

    dimmed
}

pub fn blit(dst: &mut [u8], dst_stride: usize, src: &[u8], src_stride: usize, rect: Rect) {
    let row_bytes = rect.width.max(0) as usize * 4;
    let x_bytes = rect.x.max(0) as usize * 4;
    let start_y = rect.y.max(0) as usize;
    let end_y = start_y + rect.height.max(0) as usize;

    for row in start_y..end_y {
        let dst_start = row * dst_stride + x_bytes;
        let src_start = row * src_stride + x_bytes;
        let (Some(dst_row), Some(src_row)) = (
            dst.get_mut(dst_start..dst_start + row_bytes),
            src.get(src_start..src_start + row_bytes),
        ) else {
            break;
        };
        dst_row.copy_from_slice(src_row);
    }
}
