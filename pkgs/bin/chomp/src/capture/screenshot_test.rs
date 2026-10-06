use super::*;
use crate::capture::buffer::PixelFormat;

#[test]
fn saves_png_bytes_atomically_regardless_of_filename_history() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("capture.png");
    std::fs::write(&path, b"old").unwrap();
    let image = CapturedImage::new(vec![0, 0, 255, 255], 1, 1, 4, PixelFormat::Argb8888).unwrap();

    save_captured_image(image, &path).unwrap();

    let decoded = image::open(path).unwrap().to_rgba8();
    assert_eq!(decoded.get_pixel(0, 0).0, [255, 0, 0, 255]);
}
