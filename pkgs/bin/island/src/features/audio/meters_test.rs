use super::loudness;

#[test]
fn loudness_spans_sixty_decibels() {
    assert_eq!(loudness(1.0), 1.0);
    assert_eq!(loudness(2.0), 1.0);
    assert!((loudness(0.1) - 2.0 / 3.0).abs() < 1e-4);
    assert_eq!(loudness(0.001), 0.0);
    assert_eq!(loudness(0.0), 0.0);
    assert_eq!(loudness(f32::NAN), 0.0);
}
