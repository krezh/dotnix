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
