use super::{Window, values};

#[test]
fn values_run_from_nothing_at_the_start_to_the_latest_sample() {
    let window = Window {
        provider: "claude".to_owned(),
        limit: "five_hour".to_owned(),
        start: 0,
        duration: 100,
        samples: vec![(50, 40.0)],
    };
    // Five slots at 0, 25, 50, 75 and 100; the last sample is at 50.
    assert_eq!(values(&window, 5, 60), vec![0.0, 20.0, 40.0]);
}

#[test]
fn values_stop_at_now_and_follow_each_sample() {
    let window = Window {
        provider: "claude".to_owned(),
        limit: "five_hour".to_owned(),
        start: 0,
        duration: 100,
        samples: vec![(25, 10.0), (75, 60.0), (100, 70.0)],
    };
    assert_eq!(values(&window, 5, 80), vec![0.0, 10.0, 35.0, 60.0]);
}
