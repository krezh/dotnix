use super::{Shots, key};
use crate::picture::Picture;

#[test]
fn identifiers_compare_without_case_prefix_or_leading_zeros() {
    assert_eq!(key("0x0000180000F4"), "180000f4");
    assert_eq!(key("180000f4"), "180000f4");
}

#[test]
fn a_window_is_found_by_its_app_and_title_when_its_identifier_is_not() {
    let mut shots = Shots::default();
    let picture = Picture::pixels(image::RgbaImage::new(1, 1));
    shots.pictures.insert("ab".to_owned(), picture.clone());
    shots
        .named
        .insert(("zen".to_owned(), "Docs".to_owned()), "ab".to_owned());
    assert!(shots.of("ab", "", "") == Some(&picture));
    assert!(shots.of("other", "zen", "Docs") == Some(&picture));
    assert!(shots.of("other", "zen", "Mail").is_none());
}
