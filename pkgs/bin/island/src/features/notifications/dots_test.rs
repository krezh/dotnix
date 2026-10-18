use super::{BOX, MOST, layout};

#[test]
fn one_dot_per_missed_notification_up_to_nine() {
    assert_eq!(layout(0).len(), 1);
    for count in 1..=MOST {
        assert_eq!(layout(count).len(), count);
    }
    assert_eq!(layout(40).len(), MOST);
}

#[test]
fn only_shrinks_as_more_join() {
    let size = |count| layout(count)[0].2;
    assert!(size(1) > size(2));
    assert_eq!(size(2), size(4));
    assert!(size(4) > size(5));
}

#[test]
fn every_dot_sits_on_whole_pixels_inside_the_square() {
    for count in 0..=MOST {
        for (left, top, size) in layout(count) {
            for value in [left, top, size] {
                assert_eq!(value.fract(), 0.0, "{count} dots: {value} is not a whole pixel");
            }
            assert!(left >= 0.0 && top >= 0.0 && left + size <= BOX && top + size <= BOX);
        }
    }
}
