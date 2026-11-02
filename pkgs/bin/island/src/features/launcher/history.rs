//! Read once in the background at startup and kept in memory; each launch counts there at once and
//! is written to the database in the background.

use std::{collections::HashMap, sync::Mutex};

use rusqlite::{Connection, params};
use tracing::{debug, warn};

use crate::{db, supervisor, sync::lock, time::now};

/// Only ever append.
const MIGRATIONS: &[&str] = &["
CREATE TABLE launches (
    app TEXT PRIMARY KEY,
    count INTEGER NOT NULL,
    last INTEGER NOT NULL
);
"];

static DB: db::Owner = db::Owner::new("launcher", MIGRATIONS);

/// Seconds.
const HALF_LIFE: f64 = 14.0 * 86_400.0;
/// Seconds after an app's last launch.
const FORGET: i64 = 365 * 86_400;

/// The last launch is in Unix seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launches {
    pub count: u32,
    pub last: i64,
}

static HISTORY: Mutex<Option<HashMap<String, Launches>>> = Mutex::new(None);

/// Every launch counts, each weighing less the longer ago the last one was.
pub fn frecency(launches: Launches, now: i64) -> f64 {
    let age = (now - launches.last).max(0) as f64;
    f64::from(launches.count) * 0.5_f64.powf(age / HALF_LIFE)
}

fn read(db: &Connection) -> rusqlite::Result<HashMap<String, Launches>> {
    db.execute(
        "DELETE FROM launches WHERE last < ?1",
        params![now() - FORGET],
    )?;
    let mut statement = db.prepare("SELECT app, count, last FROM launches")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            Launches {
                count: row.get(1)?,
                last: row.get(2)?,
            },
        ))
    })?;
    rows.collect()
}

/// Call it off the UI thread, as it may wait on the keyring. A launch counted meanwhile is kept:
/// each app has the larger of the two counts, since the database may or may not have it already.
pub fn load() {
    let history = DB
        .with("read the launch history", read)
        .unwrap_or_else(|error| {
            warn!("{error}");
            HashMap::new()
        });
    debug!(apps = history.len(), "read the launch history");
    let mut shared = lock(&HISTORY);
    let counted = shared.take().unwrap_or_default();
    *shared = Some(merge(history, counted));
}

/// Each app keeps the larger count and the later launch.
fn merge(
    mut stored: HashMap<String, Launches>,
    counted: HashMap<String, Launches>,
) -> HashMap<String, Launches> {
    for (app, launches) in counted {
        let entry = stored.entry(app).or_insert(launches);
        entry.count = entry.count.max(launches.count);
        entry.last = entry.last.max(launches.last);
    }
    stored
}

pub fn ranks() -> HashMap<String, f64> {
    let now = now();
    lock(&HISTORY)
        .iter()
        .flatten()
        .map(|(app, launches)| (app.clone(), frecency(*launches, now)))
        .collect()
}

pub fn record(app: &str) {
    let at = now();
    {
        let mut shared = lock(&HISTORY);
        let launches = shared
            .get_or_insert_default()
            .entry(app.to_owned())
            .or_insert(Launches { count: 0, last: at });
        launches.count += 1;
        launches.last = at;
    }
    let app = app.to_owned();
    supervisor::spawn("island-launches", move || {
        let written = DB.with("save a launch", |db| {
            db.execute(
                "INSERT INTO launches (app, count, last) VALUES (?1, 1, ?2)
                 ON CONFLICT (app) DO UPDATE SET count = count + 1, last = excluded.last",
                params![app, at],
            )
        });
        if let Err(error) = written {
            warn!(app, "{error}");
        }
    });
}

#[cfg(test)]
#[path = "history_test.rs"]
mod tests;
