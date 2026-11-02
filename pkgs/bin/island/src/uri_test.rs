use std::path::PathBuf;

use super::local_path;

#[test]
fn escapes_are_read() {
    assert_eq!(
        local_path("file:///home/me/My%20Music/caf%C3%A9.jpg"),
        Some(PathBuf::from("/home/me/My Music/café.jpg"))
    );
}

#[test]
fn localhost_is_left_out() {
    assert_eq!(
        local_path("file://localhost/tmp/a.png"),
        Some(PathBuf::from("/tmp/a.png"))
    );
}

#[test]
fn anything_else_is_no_path() {
    assert_eq!(local_path("https://example.com/a.png"), None);
    assert_eq!(local_path("/tmp/a.png"), None);
    assert_eq!(local_path("file:///tmp/%zz"), None);
}
