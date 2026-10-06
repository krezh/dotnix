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
