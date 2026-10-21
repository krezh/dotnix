use image::RgbaImage;

use super::*;

fn pixels(picture: Picture) -> (u32, u32, Vec<u8>) {
    let Picture::Pixels(image) = picture else {
        panic!("expected pixels");
    };
    let size = image.size(0);
    (
        size.width.0 as u32,
        size.height.0 as u32,
        image.as_bytes(0).unwrap().to_vec(),
    )
}

fn argb(side: i32, pixel: [u8; 4]) -> (i32, i32, Vec<u8>) {
    (side, side, pixel.repeat((side * side) as usize))
}

#[test]
fn picks_the_smallest_pixmap_that_covers_the_size_and_converts_to_bgra() {
    let picture = pixmap(vec![
        argb(16, [255, 0, 0, 255]),
        argb(1024, [255, 0, 255, 0]),
        argb(48, [255, 255, 0, 0]),
    ])
    .unwrap();
    let (width, height, bytes) = pixels(picture);
    assert_eq!((width, height), (SIZE, SIZE));
    // The 48px pixmap is the red one; BGRA puts red third.
    assert!(bytes.chunks_exact(4).all(|pixel| pixel == [0, 0, 255, 255]));
}

#[test]
fn falls_back_to_the_largest_pixmap_when_all_are_small() {
    let picture = pixmap(vec![argb(16, [255, 0, 0, 255]), argb(32, [255, 0, 255, 0])]).unwrap();
    let (width, _, bytes) = pixels(picture);
    assert_eq!(width, SIZE);
    assert!(
        bytes
            .chunks_exact(4)
            .all(|pixel| pixel[1] > 250 && pixel[0] < 5)
    );
}

#[test]
fn transparent_pixels_do_not_darken_the_edges() {
    let mut buffer = RgbaImage::new(64, 64);
    for (x, _, pixel) in buffer.enumerate_pixels_mut() {
        pixel.0 = if x < 32 {
            [255, 255, 255, 255]
        } else {
            [0, 0, 0, 0]
        };
    }
    let (_, _, bytes) = pixels(resampled(buffer).unwrap());
    for pixel in bytes.chunks_exact(4).filter(|pixel| pixel[3] > 8) {
        assert!(pixel[..3].iter().all(|channel| *channel > 240), "{pixel:?}");
    }
}

#[test]
fn keeps_the_aspect_ratio() {
    let (width, height, _) = pixels(resampled(RgbaImage::new(128, 64)).unwrap());
    assert_eq!((width, height), (SIZE, SIZE / 2));
}
