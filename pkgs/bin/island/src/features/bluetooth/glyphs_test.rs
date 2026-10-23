use super::{GLYPHS, battery, device};
use crate::theme::{GREEN, OVERLAY2, PEACH, YELLOW};

#[test]
fn every_device_glyph_is_one_island_looks_up() {
    for icon in ["audio-headset", "input-gaming", "phone", "something-new", ""] {
        let key = device(icon);
        assert!(GLYPHS.iter().any(|(name, _)| *name == key), "{key}");
    }
}

#[test]
fn a_battery_turns_warmer_as_it_runs_down() {
    assert_eq!(battery(Some(80)), GREEN);
    assert_eq!(battery(Some(40)), YELLOW);
    assert_eq!(battery(Some(10)), PEACH);
    assert_eq!(battery(None), OVERLAY2);
}
