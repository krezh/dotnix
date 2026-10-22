use super::*;

#[test]
fn help_is_pointed_at_by_the_command_s_path() {
    let mistake = Mistake::new(Line::new().plain("Wrong")).see_help("island logs");
    assert_eq!(
        crate::cli::render(&mistake.lines(), false),
        "✗ Wrong\n  See island logs --help"
    );
}

#[test]
fn a_slip_is_suggested_and_a_different_word_is_not() {
    let known = ["volume", "mic", "media", "brightness"];
    assert_eq!(closest("volum", known), Some("volume"));
    assert_eq!(closest("mci", known), Some("mic"));
    assert_eq!(closest("weather", known), None);
}

#[test]
fn choices_read_as_a_list() {
    assert_eq!(either(&["on"]), "on");
    assert_eq!(either(&["on", "off", "toggle"]), "on, off or toggle");
}
