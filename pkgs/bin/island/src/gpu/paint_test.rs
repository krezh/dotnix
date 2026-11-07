use super::*;

#[test]
fn shelves_stop_at_the_bottom_of_the_atlas() {
    let mut shelves = Shelves::default();
    assert!(shelves.place(ATLAS / 2, ATLAS / 2).is_some());
    assert!(shelves.place(ATLAS / 2 - 1, 10).is_some());
    assert!(shelves.place(10, ATLAS / 2).is_none());
}

#[test]
fn nothing_is_emptied_until_something_did_not_fit() {
    let mut painter = Painter::new();
    assert_eq!(painter.reclaim_at(), None);
    assert!(!painter.reclaim(Instant::now()));
}

#[test]
fn emptying_starts_the_atlas_over() {
    let now = Instant::now();
    let mut painter = Painter::new();
    painter.glow_room();
    painter.drawings.insert(
        "drawing".to_owned(),
        Entry {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
            left: 0,
            top: 0,
            colored: false,
        },
    );
    painter.full = true;
    assert!(painter.reclaim(now));
    assert_eq!(painter.reclaim_at(), None);
    assert!(painter.drawings.is_empty());
    assert!(painter.glows.is_empty());
    assert_eq!(painter.shelves.place(1, 1), Some((0, 0)));
}

#[test]
fn what_cannot_fit_empties_the_atlas_only_so_often() {
    let now = Instant::now();
    let mut painter = Painter::new();
    painter.full = true;
    assert!(painter.reclaim(now));
    painter.full = true;
    assert!(!painter.reclaim(now));
    assert_eq!(painter.reclaim_at(), Some(now + RECLAIM_EVERY));
    assert!(painter.reclaim(now + RECLAIM_EVERY));
}

#[test]
fn what_is_bigger_than_the_atlas_never_fits() {
    assert!(Shelves::fits(ATLAS - 1, ATLAS - 1));
    assert!(!Shelves::fits(ATLAS, 1));
    assert!(!Shelves::fits(1, ATLAS));
}

#[test]
fn a_clear_pixel_of_a_colored_glyph_adds_nothing() {
    assert_eq!(premultiplied(&[200, 100, 50, 0]), [0, 0, 0, 0]);
    assert_eq!(premultiplied(&[200, 100, 50, 255]), [200, 100, 50, 255]);
    assert_eq!(premultiplied(&[200, 100, 50, 128]), [100, 50, 25, 128]);
}

#[test]
fn a_symbol_is_the_mean_of_what_was_drawn_at_twice_its_size() {
    let pixel = |alpha: u8| [0, 0, 0, alpha];
    let rgba: Vec<u8> = [255, 255, 0, 0, 255, 255, 0, 0, 0, 0, 255, 0, 0, 0, 255, 255]
        .into_iter()
        .flat_map(pixel)
        .collect();
    assert_eq!(
        halved(&rgba, 2),
        [[255; 4], [0; 4], [0; 4], [191; 4]].concat()
    );
}
