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
