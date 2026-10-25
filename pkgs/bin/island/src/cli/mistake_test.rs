use super::*;

#[test]
fn a_slip_is_suggested_and_a_different_word_is_not() {
    let known = ["volume", "mic", "media", "brightness"];
    assert_eq!(closest("volum", known), Some("volume"));
    assert_eq!(closest("mci", known), Some("mic"));
    assert_eq!(closest("weather", known), None);
}
