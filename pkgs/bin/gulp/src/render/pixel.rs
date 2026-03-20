//! Pixel format conversion utilities

/// Converts ARGB8888 pixel data to RGBA8888 format.
///
/// This is commonly needed when converting Wayland's native ARGB format
/// to the RGBA format expected by image processing libraries.
///
/// Optimized for bulk conversion with pre-allocated buffer and direct indexing.
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
