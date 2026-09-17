//! Pixel format conversion utilities

/// Returns a copy of ARGB8888 `buffer` darkened by `opacity`.
///
/// This is the same blend the overlay paints outside the selection. Applying it
/// to the frozen screen once keeps it off the drag path, where it would be a
/// full-screen alpha blend on every frame.
pub fn dim_argb(buffer: &[u8], opacity: f64) -> Vec<u8> {
    let keep = ((1.0 - opacity.clamp(0.0, 1.0)) * 255.0).round() as u32;
    let mut dimmed = buffer.to_vec();

    for pixel in dimmed.chunks_exact_mut(4) {
        // ARGB8888 is little-endian: blue, green, red, then alpha, left as is.
        for channel in &mut pixel[..3] {
            *channel = (*channel as u32 * keep / 255) as u8;
        }
    }

    dimmed
}

/// Copies a rectangle of pixels from `src` to the same position in `dst`.
///
/// Rows are addressed through each buffer's own stride. A row reaching past
/// either buffer ends the copy, so an oversized rectangle clips rather than
/// panicking.
pub fn blit(
    dst: &mut [u8],
    dst_stride: usize,
    src: &[u8],
    src_stride: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) {
    let row_bytes = width * 4;
    let x_bytes = x * 4;

    for row in y..y + height {
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

/// Converts ARGB8888 pixel data to RGBA8888 format.
///
/// This is commonly needed when converting Wayland's native ARGB format
/// to the RGBA format expected by image processing libraries.
///
/// Expects packed rows, as `CapturedImage::crop` produces: a buffer whose stride
/// exceeds `width * 4` keeps its padding here and comes out skewed.
pub fn convert_argb_to_rgba(buffer: &[u8]) -> Vec<u8> {
    let mut rgba_buffer = vec![0u8; buffer.len()];

    for (src, dst) in buffer.chunks_exact(4).zip(rgba_buffer.chunks_exact_mut(4)) {
        dst[0] = src[2]; // R
        dst[1] = src[1]; // G
        dst[2] = src[0]; // B
        dst[3] = src[3]; // A
    }

    rgba_buffer
}
