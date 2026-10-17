use std::{fs, path::PathBuf};

use rusqlite::ErrorCode;

use super::{PLAIN_HEADER, unlock};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("island-db-test-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir.join("island.db")
}

#[test]
fn encrypts_the_file_and_refuses_the_wrong_key() {
    let path = scratch("key");
    let db = unlock(&path, &[7; 32]).unwrap();
    db.execute_batch("CREATE TABLE secret (text TEXT); INSERT INTO secret VALUES ('hunter2');").unwrap();
    drop(db);

    let raw = fs::read(&path).unwrap();
    assert_ne!(&raw[..16], PLAIN_HEADER);
    assert!(!raw.windows(7).any(|window| window == b"hunter2"));

    let wrong = unlock(&path, &[8; 32]).err().unwrap();
    assert!(matches!(wrong, rusqlite::Error::SqliteFailure(error, _) if error.code == ErrorCode::NotADatabase));
    let right = unlock(&path, &[7; 32]).unwrap();
    let text: String = right.query_row("SELECT text FROM secret", [], |row| row.get(0)).unwrap();
    assert_eq!(text, "hunter2");
}

#[test]
fn an_unencrypted_database_reads_as_not_a_database() {
    let path = scratch("plain");
    fs::write(&path, [PLAIN_HEADER.as_slice(), &[0; 100]].concat()).unwrap();
    let error = unlock(&path, &[7; 32]).err().unwrap();
    assert!(matches!(error, rusqlite::Error::SqliteFailure(error, _) if error.code == ErrorCode::NotADatabase));
}
