//! A worker owns the connection: it reads the history once at start, dropping what has aged out,
//! then writes as notifications arrive or are forgotten.

use std::{
    path::PathBuf,
    sync::{OnceLock, mpsc::Receiver},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, params};
use tracing::{debug, warn};

use super::{
    Event, State,
    notification::{Expire, Notification},
};
use crate::{components::icon::Picture, db, features::Feed, supervisor::Ended};

/// Only ever append.
const MIGRATIONS: &[&str] = &["
CREATE TABLE history (
    session INTEGER NOT NULL,
    id INTEGER NOT NULL,
    app TEXT NOT NULL,
    summary TEXT NOT NULL,
    body TEXT NOT NULL,
    critical INTEGER NOT NULL,
    icon TEXT,
    received INTEGER NOT NULL,
    PRIMARY KEY (session, id)
);
CREATE INDEX history_received ON history (received DESC);
"];

/// The session and the notification's id together name it.
#[derive(Clone, Debug)]
pub struct Earlier {
    pub session: i64,
    pub notification: Notification,
}

impl Earlier {
    pub fn key(&self) -> (i64, u32) {
        (self.session, self.notification.id)
    }
}

pub enum Write {
    Record(Earlier),
    Forget(Vec<(i64, u32)>),
}

fn seconds(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX))
}

/// Named by when it started; notification ids count afresh in each.
pub fn session() -> i64 {
    static STARTED: OnceLock<i64> = OnceLock::new();
    *STARTED.get_or_init(|| seconds(SystemTime::now()))
}

pub fn aged(received: SystemTime, days: u32) -> bool {
    SystemTime::now()
        .duration_since(received)
        .is_ok_and(|age| age > Duration::from_secs(u64::from(days) * 86_400))
}

fn read(db: &Connection, days: u32) -> rusqlite::Result<Vec<Earlier>> {
    let cutoff = seconds(SystemTime::now()) - i64::from(days) * 86_400;
    db.execute("DELETE FROM history WHERE received < ?1", params![cutoff])?;
    let mut statement = db.prepare(
        "SELECT session, id, app, summary, body, critical, icon, received FROM history ORDER BY received DESC",
    )?;
    let rows = statement.query_map([], |row| {
        let received: i64 = row.get(7)?;
        Ok(Earlier {
            session: row.get(0)?,
            notification: Notification {
                id: row.get(1)?,
                app: row.get(2)?,
                summary: row.get(3)?,
                body: row.get(4)?,
                critical: row.get(5)?,
                icon: row.get::<_, Option<String>>(6)?.map(|path| Picture::File(PathBuf::from(path))),
                image: None,
                transient: false,
                resident: false,
                expire: Expire::Default,
                actions: Vec::new(),
                received: UNIX_EPOCH + Duration::from_secs(u64::try_from(received).unwrap_or(0)),
            },
        })
    })?;
    rows.collect()
}

fn write(db: &mut Connection, write: Write) -> rusqlite::Result<()> {
    match write {
        Write::Record(earlier) => {
            let notification = &earlier.notification;
            let icon = match &notification.icon {
                Some(Picture::File(path)) => Some(path.to_string_lossy().into_owned()),
                _ => None,
            };
            db.execute(
                "INSERT INTO history (session, id, app, summary, body, critical, icon, received)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT (session, id) DO UPDATE SET app = excluded.app, summary = excluded.summary,
                    body = excluded.body, critical = excluded.critical, icon = excluded.icon,
                    received = excluded.received",
                params![
                    earlier.session,
                    notification.id,
                    notification.app,
                    notification.summary,
                    notification.body,
                    notification.critical,
                    icon,
                    seconds(notification.received)
                ],
            )?;
        }
        Write::Forget(keys) => {
            let transaction = db.transaction()?;
            for (session, id) in keys {
                transaction.execute("DELETE FROM history WHERE session = ?1 AND id = ?2", params![session, id])?;
            }
            transaction.commit()?;
        }
    }
    Ok(())
}

pub fn work(feed: &Feed<State, Event>, writes: &Receiver<Write>, days: u32) -> Ended {
    let mut db = match db::open("notifications", MIGRATIONS) {
        Ok(db) => db,
        Err(error) => return Ended::Failed(format!("cannot open the notification history: {error}")),
    };
    let earlier = match read(&db, days) {
        Ok(earlier) => earlier,
        Err(error) => return Ended::Failed(format!("cannot read the notification history: {error}")),
    };
    debug!(entries = earlier.len(), "read the notification history");
    if !feed.send(Event::History(earlier)) {
        return Ended::Unwanted;
    }
    for request in writes {
        if let Err(error) = write(&mut db, request) {
            warn!("cannot write the notification history: {error}");
        }
    }
    Ended::Unwanted
}

#[cfg(test)]
#[path = "history_test.rs"]
mod tests;
