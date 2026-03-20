//! Tesseract OCR integration

use anyhow::Result;
use crate::capture::CapturedImage;
use crate::render::convert_argb_to_rgba;

/// Extracts text from a captured image using Tesseract OCR.
///
/// Converts the image data to RGBA format and processes it with Tesseract's English language model.
/// Returns the extracted text as a trimmed string.
pub fn extract_text(image: &CapturedImage) -> Result<String> {
    log::info!("Running OCR on {}x{} image", image.width, image.height);

    // Convert ARGB to RGBA for image library
    let rgba_buffer = convert_argb_to_rgba(&image.data);

    let expected_size = (image.width * image.height * 4) as usize;
    log::debug!("Buffer size: {}, expected: {}", rgba_buffer.len(), expected_size);

    if rgba_buffer.len() != expected_size {
        anyhow::bail!(
            "Buffer size mismatch: got {} bytes, expected {} bytes ({}x{} * 4)",
            rgba_buffer.len(),
            expected_size,
            image.width,
            image.height
        );
    }

    // Pass image data directly to Tesseract
    let mut tess = tesseract::Tesseract::new(None, Some("eng"))?;
    tess = tess.set_frame(
        &rgba_buffer,
        image.width as i32,
        image.height as i32,
        4, // bytes per pixel (RGBA)
        (image.width * 4) as i32, // bytes per line
    )?;
    let text = tess.get_text()?.trim().to_string();

    log::info!("OCR completed, extracted {} characters", text.len());

    Ok(text)
}
