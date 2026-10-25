use super::of_headset;

const HEADSET: &str = "Logitech ASTRO A50 Gen 5";

#[test]
fn matches_a_shared_word() {
    assert!(of_headset("A50 Chat", HEADSET));
    assert!(of_headset("a50 system", HEADSET));
    assert!(!of_headset("Argon Speakers", HEADSET));
    assert!(!of_headset("5 1", "Gen 5"));
}
