use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use image::RgbaImage;

use super::{Drawn, KEPT_FILES, UNUSED_FOR};

fn image() -> Arc<RgbaImage> {
    Arc::new(RgbaImage::new(1, 1))
}

#[test]
fn a_held_image_is_kept() {
    let mut drawn = Drawn::default();
    let held = image();
    drawn.pixels(&held);
    let now = Instant::now();
    assert!(drawn.sweep(now).0.is_empty());
    assert!(drawn.sweep(now + UNUSED_FOR * 2).0.is_empty());
    assert_eq!(drawn.pixels.len(), 1);
}

#[test]
fn an_unheld_image_goes_only_after_a_while() {
    let mut drawn = Drawn::default();
    drawn.pixels(&image());
    let now = Instant::now();
    assert!(drawn.sweep(now).0.is_empty());
    assert!(drawn.sweep(now + Duration::from_millis(1)).0.is_empty());
    assert_eq!(drawn.sweep(now + UNUSED_FOR).0.len(), 1);
    assert!(drawn.pixels.is_empty());
}

#[test]
fn only_the_newest_files_are_kept() {
    let mut drawn = Drawn::default();
    let path = |index: usize| PathBuf::from(format!("/art/{index}"));
    for index in 0..KEPT_FILES + 2 {
        drawn.file(&path(index));
    }
    drawn.file(&path(0));
    let (_, gone) = drawn.sweep(Instant::now());
    assert_eq!(gone, vec![path(1), path(2)]);
    assert_eq!(drawn.files.len(), KEPT_FILES);
    assert_eq!(drawn.files.back(), Some(&path(0)));
}
