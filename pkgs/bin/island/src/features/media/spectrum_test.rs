use super::{Analyser, BARS, Fft, HOP, RATE, WINDOW};
use std::f32::consts::PI;

#[test]
fn fft_finds_a_pure_tone_in_its_bin() {
    let mut fft = Fft::new(64);
    {
        let (re, _) = fft.buffers();
        for (index, value) in re.iter_mut().enumerate() {
            *value = (2.0 * PI * 5.0 * index as f32 / 64.0).sin();
        }
    }
    fft.run();
    let (re, im) = fft.buffers();
    let magnitude = |bin: usize| (re[bin] * re[bin] + im[bin] * im[bin]).sqrt();
    let loudest = (0..32)
        .max_by(|a, b| magnitude(*a).total_cmp(&magnitude(*b)))
        .unwrap();
    assert_eq!(loudest, 5);
    assert!((magnitude(5) - 32.0).abs() < 0.01);
}

#[test]
fn a_low_tone_lights_the_low_bands_and_silence_settles_to_zero() {
    let mut analyser = Analyser::new();
    let mut last = Vec::new();
    for index in 0..WINDOW * 4 {
        let sample = 0.5 * (2.0 * PI * 80.0 * index as f32 / RATE as f32).sin();
        if let Some(levels) = analyser.push(sample) {
            last = levels;
        }
    }
    assert_eq!(last.len(), BARS);
    let loudest = (0..BARS)
        .max_by(|a, b| last[*a].total_cmp(&last[*b]))
        .unwrap();
    assert!(
        loudest <= 1,
        "80 Hz should peak in the lowest bands, got band {loudest}: {last:?}"
    );
    assert!(last[BARS - 1] < last[loudest]);

    let mut quiet = None;
    for _ in 0..HOP * 60 {
        if let Some(levels) = analyser.push(0.0) {
            quiet = Some(levels);
        }
    }
    assert!(quiet.is_some_and(|levels| levels.iter().all(|level| *level == 0.0)));
    assert!(
        (0..HOP * 4).all(|_| analyser.push(0.0).is_none()),
        "silence is reported once, not every frame"
    );
}
