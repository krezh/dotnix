use super::{device, of_headset};
use crate::features::audio::state::{Device, Form};

const HEADSET: &str = "Logitech ASTRO A50 Gen 5";

fn named(description: &str, form: Form) -> Device {
    Device {
        description: description.to_owned(),
        form,
        ..Device::default()
    }
}

#[test]
fn matches_a_shared_word() {
    assert!(of_headset("A50 Chat", HEADSET));
    assert!(of_headset("a50 system", HEADSET));
    assert!(!of_headset("Argon Speakers", HEADSET));
    assert!(!of_headset("5 1", "Gen 5"));
}

#[test]
fn a_formless_device_of_the_headset_is_a_headset() {
    assert_eq!(device(&named("A50 System", Form::Card), Some(HEADSET)), "device-headset");
    assert_eq!(device(&named("A50 System", Form::Card), None), "device-card");
    assert_eq!(device(&named("Argon Speakers", Form::Speakers), Some(HEADSET)), "device-speakers");
}
