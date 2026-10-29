//! A worker owns the connection: it reads the history once at start, dropping what has aged out,
//! then writes as notifications arrive or are forgotten.

use std::{
    path::PathBuf,
    sync::{Arc, OnceLock, mpsc::Receiver},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, params};
use tracing::{debug, warn};

use super::{
    Event, Notifications, State,
    cards::Filter,
    notification::{Expire, Notification, Posted},
};
use crate::{
    components::icon::Picture,
    db,
    features::{Feed, notice::Origin},
    supervisor::Ended,
};

/// Only ever append.
const MIGRATIONS: &[&str] = &[
    "
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
",
    "
ALTER TABLE history ADD COLUMN lead TEXT;
",
    "
ALTER TABLE history ADD COLUMN feature TEXT;
ALTER TABLE history ADD COLUMN origin TEXT;
ALTER TABLE history ADD COLUMN source TEXT;
ALTER TABLE history ADD COLUMN via INTEGER NOT NULL DEFAULT 0;
ALTER TABLE history ADD COLUMN detail TEXT;
CREATE INDEX history_origin ON history (feature, origin);
",
];

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

    fn origin(&self) -> Option<&Origin> {
        self.notification
            .posted
            .as_ref()
            .map(|posted| &posted.origin)
    }
}

pub enum Write {
    Record(Earlier),
    Forget(Vec<(i64, u32)>),
}

fn seconds(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |elapsed| {
        i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
    })
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
        "SELECT session, id, app, summary, body, critical, icon, received, lead, feature, origin, source, via, detail
         FROM history ORDER BY received DESC",
    )?;
    let rows = statement.query_map([], |row| {
        let received: i64 = row.get(7)?;
        let posted = match (row.get(9)?, row.get(10)?, row.get(11)?) {
            (Some(feature), Some(key), Some(source)) => Some(Posted {
                origin: Origin {
                    feature,
                    source,
                    key,
                },
                via: row.get(12)?,
                detail: row.get(13)?,
            }),
            _ => None,
        };
        Ok(Earlier {
            session: row.get(0)?,
            notification: Notification {
                id: row.get(1)?,
                app: row.get(2)?,
                summary: row.get(3)?,
                body: row.get(4)?,
                critical: row.get(5)?,
                icon: row
                    .get::<_, Option<String>>(6)?
                    .map(|path| Picture::File(PathBuf::from(path))),
                image: None,
                lead: row.get(8)?,
                transient: false,
                resident: false,
                expire: Expire::Default,
                actions: Vec::new(),
                received: UNIX_EPOCH + Duration::from_secs(u64::try_from(received).unwrap_or(0)),
                posted,
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
            let posted = notification.posted.as_ref();
            let origin = posted.map(|posted| &posted.origin);
            if let Some(origin) = origin {
                // Posted again in a later session, such as a message still on its server.
                db.execute(
                    "DELETE FROM history WHERE feature = ?1 AND origin = ?2 AND NOT (session = ?3 AND id = ?4)",
                    params![origin.feature, origin.key, earlier.session, notification.id],
                )?;
            }
            db.execute(
                "INSERT INTO history (session, id, app, summary, body, critical, icon, received, lead,
                    feature, origin, source, via, detail)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                 ON CONFLICT (session, id) DO UPDATE SET app = excluded.app, summary = excluded.summary,
                    body = excluded.body, critical = excluded.critical, icon = excluded.icon,
                    received = excluded.received, lead = excluded.lead, feature = excluded.feature,
                    origin = excluded.origin, source = excluded.source, via = excluded.via,
                    detail = excluded.detail",
                params![
                    earlier.session,
                    notification.id,
                    notification.app,
                    notification.summary,
                    notification.body,
                    notification.critical,
                    icon,
                    seconds(notification.received),
                    notification.lead,
                    origin.map(|origin| &origin.feature),
                    origin.map(|origin| &origin.key),
                    origin.map(|origin| &origin.source),
                    posted.is_some_and(|posted| posted.via),
                    posted.and_then(|posted| posted.detail.as_ref()),
                ],
            )?;
        }
        Write::Forget(keys) => {
            let transaction = db.transaction()?;
            for (session, id) in keys {
                transaction.execute(
                    "DELETE FROM history WHERE session = ?1 AND id = ?2",
                    params![session, id],
                )?;
            }
            transaction.commit()?;
        }
    }
    Ok(())
}

pub fn work(feed: &Feed<State, Event>, writes: &Receiver<Write>, days: u32) -> Ended {
    let mut db = match db::open("notifications", MIGRATIONS) {
        Ok(db) => db,
        Err(error) => {
            return Ended::Failed(format!("cannot open the notification history: {error}"));
        }
    };
    let earlier = match read(&db, days) {
        Ok(earlier) => earlier,
        Err(error) => {
            return Ended::Failed(format!("cannot read the notification history: {error}"));
        }
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

impl Notifications {
    /// In place of one it replaces.
    pub(super) fn remember(&self, notification: &Notification) {
        let earlier = Earlier {
            session: session(),
            notification: notification.clone(),
        };
        let days = self.history_days;
        let _ = self.history.send(Write::Record(earlier.clone()));
        let origin = earlier.origin().cloned();
        self.change(|store| {
            let history = Arc::make_mut(&mut store.history);
            history.retain(|kept| {
                kept.key() != earlier.key()
                    && (origin.is_none() || kept.origin() != origin.as_ref())
                    && !aged(kept.notification.received, days)
            });
            history.insert(0, earlier);
        });
    }

    /// Keeps what this session added meanwhile.
    pub(super) fn load_history(&self, read: Vec<Earlier>) {
        self.change(|store| {
            let mut merged = read;
            for kept in store.history.iter() {
                if !merged.iter().any(|read| read.key() == kept.key()) {
                    merged.push(kept.clone());
                }
            }
            merged.sort_by(|a, b| b.notification.received.cmp(&a.notification.received));
            store.history = Arc::new(merged);
        });
    }

    /// Without what is missed, and without actions, whose apps may be long gone.
    pub(super) fn earlier(state: &State) -> Vec<Earlier> {
        let session = session();
        state
            .history
            .iter()
            .filter(|kept| {
                !(kept.session == session
                    && state
                        .missed
                        .iter()
                        .any(|missed| missed.id == kept.notification.id))
            })
            .map(|kept| Earlier {
                session: kept.session,
                notification: Notification {
                    actions: Vec::new(),
                    ..kept.notification.clone()
                },
            })
            .collect()
    }

    pub(super) fn forget(&self, keys: Vec<(i64, u32)>) {
        if keys.is_empty() {
            return;
        }
        self.change(|store| {
            Arc::make_mut(&mut store.history).retain(|kept| !keys.contains(&kept.key()))
        });
        let _ = self.history.send(Write::Forget(keys));
    }

    /// Those `filter` shows.
    pub(super) fn clear_earlier(&self, filter: &Filter) {
        let keys = Self::earlier(&self.store.borrow())
            .iter()
            .filter(|kept| filter.shows(&kept.notification))
            .map(Earlier::key)
            .collect();
        self.forget(keys);
    }
}

#[cfg(test)]
#[path = "history_test.rs"]
mod tests;
