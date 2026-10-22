use super::{Series, sample, text};

#[test]
fn scrolling_samples_follow_the_painted_positions() {
    let values = [10.0, 20.0, 30.0, 40.0];
    assert_eq!(sample(&values, 6, 0.5, 0.375), Some(10.0));
    assert_eq!(sample(&values, 6, 0.5, 0.625), Some(20.0));
    assert_eq!(sample(&values, 6, 0.5, 0.875), Some(30.0));
    assert_eq!(sample(&values, 6, 0.5, 1.0), Some(35.0));
    assert_eq!(sample(&values, 6, 0.5, 0.95), Some(35.0));
    assert_eq!(sample(&values, 6, 0.5, 0.9), Some(30.0));
    assert_eq!(sample(&values, 6, 0.0, 1.0), Some(30.0));
    assert_eq!(sample(&values, 6, 1.0, 1.0), Some(40.0));
}

#[test]
fn unpainted_history_has_no_sample() {
    assert_eq!(sample(&[], 40, 0.5, 1.0), None);
    assert_eq!(sample(&[1.0], 40, 0.5, 1.0), None);
    assert_eq!(sample(&[10.0, 20.0], 6, 0.5, 0.5), None);
    assert_eq!(sample(&[10.0, 20.0], 6, 0.5, -0.1), None);
    assert_eq!(sample(&[10.0, 20.0], 6, 0.5, 1.1), None);
    assert_eq!(sample(&[10.0, 20.0], 0, 1.0, 0.0), Some(10.0));
}

#[test]
fn tooltip_formats_each_series_without_inventing_missing_history() {
    let series = [
        Series {
            values: vec![0.25, 0.5, 0.75],
            max: 1.0,
            color: 0,
            label: "Usage".into(),
            format: |value| format!("{:.0}%", value * 100.0),
        },
        Series {
            values: vec![40.0, 60.0],
            max: 100.0,
            color: 0,
            label: "Temperature".into(),
            format: |value| format!("{value:.0}°C"),
        },
    ];
    assert_eq!(text(&series, 6, 0.5, 1.0), "Usage: 62%\nTemperature: 50°C");
    assert_eq!(text(&series, 6, 0.5, 0.625), "Usage: 25%");
    assert_eq!(text(&series, 6, 0.5, 0.0), "");
}
