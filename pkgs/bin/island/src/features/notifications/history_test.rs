use std::time::{Duration, SystemTime};

use rusqlite::Connection;

use super::{Earlier, MIGRATIONS, Write, aged, read, write};
use crate::features::{
    notice::Origin,
    notifications::notification::{Expire, Notification, Posted},
};

fn database() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    for migration in MIGRATIONS {
        db.execute_batch(migration).unwrap();
    }
    db
}

fn entry(session: i64, id: u32, summary: &str, age: Duration) -> Earlier {
    Earlier {
        session,
        notification: Notification {
            id,
            app: "app".to_owned(),
            summary: summary.to_owned(),
            body: "body".to_owned(),
            critical: false,
            icon: None,
            image: None,
            lead: None,
            transient: false,
            resident: false,
            expire: Expire::Default,
            actions: Vec::new(),
            received: SystemTime::now() - age,
            posted: None,
        },
    }
}

fn posted(session: i64, id: u32, key: &str, summary: &str) -> Earlier {
    let mut kept = entry(session, id, summary, Duration::ZERO);
    kept.notification.posted = Some(Posted {
        origin: Origin {
            feature: "feature".to_owned(),
            source: "Source".to_owned(),
            key: key.to_owned(),
        },
        via: true,
        detail: Some("priority 5".to_owned()),
    });
    kept
}

fn summaries(db: &Connection, days: u32) -> Vec<String> {
    read(db, days)
        .unwrap()
        .into_iter()
        .map(|kept| kept.notification.summary)
        .collect()
}

#[test]
fn keeps_notifications_newest_first() {
    let mut db = database();
    write(
        &mut db,
        Write::Record(entry(1, 1, "older", Duration::from_secs(60))),
    )
    .unwrap();
    write(
        &mut db,
        Write::Record(entry(2, 1, "newer", Duration::from_secs(5))),
    )
    .unwrap();
    assert_eq!(summaries(&db, 7), ["newer", "older"]);
}

#[test]
fn a_replacement_takes_its_place() {
    let mut db = database();
    write(
        &mut db,
        Write::Record(entry(1, 4, "first", Duration::from_secs(9))),
    )
    .unwrap();
    write(
        &mut db,
        Write::Record(entry(1, 4, "replaced", Duration::from_secs(1))),
    )
    .unwrap();
    assert_eq!(summaries(&db, 7), ["replaced"]);
}

#[test]
fn forgets_what_it_is_told() {
    let mut db = database();
    write(&mut db, Write::Record(entry(1, 1, "a", Duration::ZERO))).unwrap();
    write(&mut db, Write::Record(entry(1, 2, "b", Duration::ZERO))).unwrap();
    write(&mut db, Write::Forget(vec![(1, 1)])).unwrap();
    assert_eq!(summaries(&db, 7), ["b"]);
}

#[test]
fn a_posted_one_is_kept_once_however_often_it_is_posted() {
    let mut db = database();
    write(&mut db, Write::Record(posted(1, 3, "41", "first"))).unwrap();
    write(&mut db, Write::Record(posted(2, 9, "41", "again"))).unwrap();
    write(&mut db, Write::Record(posted(2, 10, "42", "other"))).unwrap();
    let kept = read(&db, 7).unwrap();
    let mut summaries: Vec<_> = kept
        .iter()
        .map(|kept| kept.notification.summary.as_str())
        .collect();
    summaries.sort_unstable();
    assert_eq!(summaries, ["again", "other"]);
    let again = kept
        .iter()
        .find(|kept| kept.notification.summary == "again")
        .unwrap();
    assert_eq!(
        again.notification.posted,
        posted(2, 9, "41", "again").notification.posted
    );
}

#[test]
fn drops_what_has_aged_out() {
    let mut db = database();
    let day = Duration::from_secs(86_400);
    write(&mut db, Write::Record(entry(1, 1, "fresh", day))).unwrap();
    write(&mut db, Write::Record(entry(1, 2, "stale", day * 8))).unwrap();
    assert_eq!(summaries(&db, 7), ["fresh"]);
    assert_eq!(summaries(&db, 30), ["fresh"]);
    assert!(aged(SystemTime::now() - day * 8, 7));
    assert!(!aged(SystemTime::now() - day, 7));
}
