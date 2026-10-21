use super::{DAY_MS, forecast};
use crate::features::ai::state::{Limit, Severity};

const HOUR_MS: i64 = 60 * 60 * 1000;
/// Midnight, UTC.
const MONDAY: i64 = 1_799_971_200_000;

fn hour(at: i64) -> u32 {
    ((at / HOUR_MS) % 24) as u32
}

fn limit(used: f64, start: i64, duration: i64) -> Limit {
    Limit {
        id: "limit".to_owned(),
        label: "Limit".to_owned(),
        used,
        remaining: 100.0 - used,
        resets_at: Some(start + duration),
        duration_ms: Some(duration),
        severity: Severity::Ok,
        exhausts_at: None,
        expected: None,
    }
}

#[test]
fn without_readings_a_short_window_keeps_the_pace_it_had() {
    let window = 5 * HOUR_MS;
    let now = MONDAY + 2 * HOUR_MS;
    let seen = forecast(&limit(20.0, MONDAY, window), &[], now, hour).unwrap();
    assert!((seen.expected - 50.0).abs() < 0.01);
    assert_eq!(seen.exhausts_at, None);
}

#[test]
fn running_out_comes_where_the_pace_reaches_the_top() {
    let window = 5 * HOUR_MS;
    let now = MONDAY + HOUR_MS;
    let seen = forecast(&limit(40.0, MONDAY, window), &[], now, hour).unwrap();
    assert!((seen.expected - 200.0).abs() < 0.01);
    let out = seen.exhausts_at.unwrap();
    assert!((out - (MONDAY + 2 * HOUR_MS + HOUR_MS / 2)).abs() < 1_000);
}

#[test]
fn nights_count_for_little_in_a_week() {
    // A week that started at 08:00 and has seen one night so far has six more ahead, which a
    // straight line would count as working hours.
    let week = 7 * DAY_MS;
    let start = MONDAY + 8 * HOUR_MS;
    let now = start + 34 * HOUR_MS;
    let used = 30.0;
    let seen = forecast(&limit(used, start, week), &[], now, hour).unwrap();
    let straight = used * week as f64 / (now - start) as f64;
    assert!(seen.expected < straight);
}

#[test]
fn a_recent_burst_raises_the_forecast() {
    let window = 5 * HOUR_MS;
    let now = MONDAY + 3 * HOUR_MS;
    let quiet = forecast(&limit(30.0, MONDAY, window), &[], now, hour).unwrap();
    let burst = forecast(
        &limit(30.0, MONDAY, window),
        &[(now - 20 * 60 * 1000, 10.0)],
        now,
        hour,
    )
    .unwrap();
    assert!(burst.expected > quiet.expected);
}

#[test]
fn too_early_or_too_little_says_nothing() {
    let window = 5 * HOUR_MS;
    assert_eq!(
        forecast(&limit(20.0, MONDAY, window), &[], MONDAY + 60_000, hour),
        None
    );
    assert_eq!(
        forecast(&limit(2.0, MONDAY, window), &[], MONDAY + 2 * HOUR_MS, hour),
        None
    );
}
