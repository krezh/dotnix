use std::time::{Duration, SystemTime};

use rusqlite::Connection;

use super::{Earlier, MIGRATIONS, Write, aged, read, write};
use crate::features::notifications::notification::{Expire, Notification};

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
        },
    }
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
