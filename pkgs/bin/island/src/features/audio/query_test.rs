use libpulse_binding::sample::{Format, Spec};

use super::{format, ours};

#[test]
fn formats_read_short() {
    let spec = |rate, channels| Spec {
        format: Format::S16le,
        channels,
        rate,
    };
    assert_eq!(format(&spec(48_000, 2)), "48 kHz · stereo");
    assert_eq!(format(&spec(44_100, 1)), "44.1 kHz · mono");
    assert_eq!(format(&spec(96_000, 6)), "96 kHz · 6ch");
}

#[test]
fn leaves_out_its_own_meters() {
    assert!(ours("island"));
    assert!(ours("island visualizer"));
    assert!(!ours("islander"));
    assert!(!ours("Firefox"));
}
