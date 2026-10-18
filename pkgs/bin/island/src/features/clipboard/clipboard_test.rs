use std::path::PathBuf;

use super::files;

#[test]
fn reads_local_files_from_a_uri_list() {
    assert_eq!(
        files(b"# comment\r\nfile:///home/me/My%20Notes.txt\r\nfile://localhost/tmp/a.png\r\n"),
        vec![
            PathBuf::from("/home/me/My Notes.txt"),
            PathBuf::from("/tmp/a.png")
        ]
    );
}

#[test]
fn rejects_lists_with_other_uris() {
    assert!(files(b"https://example.com/\nfile:///tmp/a\n").is_empty());
    assert!(files(b"").is_empty());
}
