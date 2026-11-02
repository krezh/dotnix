use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn reports_a_nonzero_clipboard_exit() {
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("wl-copy");
    std::fs::write(&script, "#!/bin/sh\ncat >/dev/null\nexit 9\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();

    assert!(copy_text(script.to_str().unwrap(), "text").is_err());
}

#[test]
fn encodes_file_uris() {
    assert_eq!(
        file_uri(Path::new("/home/me/Videos/clip 1 (ä).mp4")),
        "file:///home/me/Videos/clip%201%20%28%C3%A4%29.mp4"
    );
}
