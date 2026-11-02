use super::pixels;

#[test]
fn a_whole_image_is_read() {
    assert!(pixels((2, 2, 8, true, 8, 4, vec![0; 16])).is_some());
}

#[test]
fn the_last_row_needs_no_padding() {
    assert!(pixels((1, 2, 8, false, 8, 3, vec![0; 11])).is_some());
}

#[test]
fn sizes_the_data_cannot_back_are_refused() {
    assert!(pixels((i32::MAX, i32::MAX, i32::MAX, true, 8, 4, vec![0; 16])).is_none());
    assert!(pixels((65_535, 65_535, 262_140, true, 8, 4, vec![0; 16])).is_none());
}

#[test]
fn rows_cannot_overlap() {
    assert!(pixels((1, i32::MAX, 0, true, 8, 4, vec![0; 16])).is_none());
}
