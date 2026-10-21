use super::{Hook, Reading, parse};

#[test]
fn the_status_line_gives_each_window_it_reports_in_milliseconds() {
    let json = r#"{
        "session_id": "abc",
        "cwd": "/tmp",
        "rate_limits": {
            "five_hour": { "used_percentage": 23.5, "resets_at": 1800000000 },
            "seven_day": { "used_percentage": 41 }
        }
    }"#;
    let Ok(Hook::Usage { readings, .. }) = parse("StatusLine", json) else {
        panic!("not read as usage");
    };
    assert_eq!(
        readings,
        vec![
            Reading {
                id: "five_hour".to_owned(),
                used: 23.5,
                resets_at: Some(1_800_000_000_000),
            },
            Reading {
                id: "seven_day".to_owned(),
                used: 41.0,
                resets_at: None,
            },
        ]
    );
}

#[test]
fn a_status_line_without_limits_reads_as_none() {
    let json = r#"{ "session_id": "abc", "rate_limits": { "five_hour": null } }"#;
    let Ok(Hook::Usage { readings, .. }) = parse("StatusLine", json) else {
        panic!("not read as usage");
    };
    assert!(readings.is_empty());
}
