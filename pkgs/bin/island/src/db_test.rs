use std::{
    cell::Cell,
    fs,
    path::PathBuf,
    sync::{Arc, Barrier},
    thread,
};

use rusqlite::{Connection, ErrorCode};

use super::{Owner, PLAIN_HEADER, open_at, unlock};
use crate::testing;

fn scratch(name: &str) -> PathBuf {
    testing::scratch(&format!("db-{name}")).join("island.db")
}

#[test]
fn workers_can_all_create_a_new_database_at_once() {
    const OWNERS: [(&str, &str); 3] = [
        ("one", "CREATE TABLE one (id INTEGER)"),
        ("two", "CREATE TABLE two (id INTEGER)"),
        ("three", "CREATE TABLE three (id INTEGER)"),
    ];
    for round in 0..10 {
        let path = scratch(&format!("race-{round}"));
        let start = Arc::new(Barrier::new(OWNERS.len() * 2));
        let workers: Vec<_> = OWNERS
            .iter()
            .chain(&OWNERS)
            .map(|(owner, migration)| {
                let (path, start) = (path.clone(), start.clone());
                thread::spawn(move || {
                    start.wait();
                    open_at(&path, &[7; 32], owner, &[migration]).map(drop)
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap().unwrap();
        }
    }
}

#[test]
fn an_owner_keeps_its_connection_until_work_fails() {
    let owner = Owner::new("test", &[]);
    let opened = Cell::new(0);
    let open = || {
        opened.set(opened.get() + 1);
        Connection::open_in_memory().map_err(|error| error.to_string())
    };
    let works = |_: &Connection| Ok::<_, String>(());

    owner.shared(open, "work", works).unwrap();
    owner.shared(open, "work", works).unwrap();
    assert_eq!(opened.get(), 1);

    let failed = owner.shared(open, "work", |_| Err::<(), _>("it broke"));
    assert_eq!(failed, Err("cannot work: it broke".to_owned()));

    owner.shared(open, "work", works).unwrap();
    assert_eq!(opened.get(), 2);
}

#[test]
fn an_owner_tries_again_after_failing_to_open() {
    let owner = Owner::new("test", &[]);
    let works = |_: &Connection| Ok::<_, String>(());
    let failed = owner.shared(|| Err("locked".to_owned()), "work", works);
    assert_eq!(failed, Err("locked".to_owned()));

    let open = || Connection::open_in_memory().map_err(|error| error.to_string());
    owner.shared(open, "work", works).unwrap();
}

#[test]
fn encrypts_the_file_and_refuses_the_wrong_key() {
    let path = scratch("key");
    let db = unlock(&path, &[7; 32]).unwrap();
    db.execute_batch("CREATE TABLE secret (text TEXT); INSERT INTO secret VALUES ('hunter2');")
        .unwrap();
    drop(db);

    let raw = fs::read(&path).unwrap();
    assert_ne!(&raw[..16], PLAIN_HEADER);
    assert!(!raw.windows(7).any(|window| window == b"hunter2"));

    let wrong = unlock(&path, &[8; 32]).err().unwrap();
    assert!(
        matches!(wrong, rusqlite::Error::SqliteFailure(error, _) if error.code == ErrorCode::NotADatabase)
    );
    let right = unlock(&path, &[7; 32]).unwrap();
    let text: String = right
        .query_row("SELECT text FROM secret", [], |row| row.get(0))
        .unwrap();
    assert_eq!(text, "hunter2");
}

#[test]
fn an_unencrypted_database_reads_as_not_a_database() {
    let path = scratch("plain");
    fs::write(&path, [PLAIN_HEADER.as_slice(), &[0; 100]].concat()).unwrap();
    let error = unlock(&path, &[7; 32]).err().unwrap();
    assert!(
        matches!(error, rusqlite::Error::SqliteFailure(error, _) if error.code == ErrorCode::NotADatabase)
    );
}
