use super::*;

#[test]
fn uses_the_protocol_window_handle_for_toplevel_capture() {
    let windows = parse_tagged_windows(
        r#"[{
            "address": "0x5a672d726da0",
            "at": [10, 40],
            "size": [2540, 1390],
            "class": "steam_app_default",
            "title": "World of Warcraft",
            "tags": ["games*"],
            "focusHistoryID": 3
        }]"#,
        "games",
    )
    .unwrap();

    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].address, 0x5a672d726da0);
    assert_eq!(windows[0].capture_handle, 0x2d72_6da0);
}

#[test]
fn resolves_scaled_rotated_and_negative_monitor_geometry() {
    let monitors = r#"[
        {
            "name": "LEFT",
            "x": -1536,
            "y": 0,
            "width": 1920,
            "height": 1080,
            "scale": 1.25,
            "transform": 0
        },
        {
            "name": "MAIN",
            "x": 0,
            "y": 0,
            "width": 2560,
            "height": 1440,
            "scale": 1.0,
            "transform": 0
        },
        {
            "name": "ROTATED",
            "x": 2560,
            "y": 0,
            "width": 1440,
            "height": 2560,
            "scale": 1.0,
            "transform": 1
        }
    ]"#;

    assert_eq!(
        parse_cursor_monitor(r#"{"x":-1,"y":100}"#, monitors).unwrap(),
        "LEFT"
    );
    assert_eq!(
        parse_cursor_monitor(r#"{"x":0,"y":100}"#, monitors).unwrap(),
        "MAIN"
    );
    assert_eq!(
        parse_cursor_monitor(r#"{"x":3000,"y":100}"#, monitors).unwrap(),
        "ROTATED"
    );
    assert!(parse_cursor_monitor(r#"{"x":6000,"y":100}"#, monitors).is_err());
}
