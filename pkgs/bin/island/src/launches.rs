//! The launcher's history in island's encrypted database: how often and how recently each app was
//! launched, which ranks apps by frecency. It is read once in the background at startup and kept in
//! memory; each launch counts there at once and is written to the database in the background.

use std::{
    collections::HashMap,
    sync::Mutex,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::params;
use tracing::{debug, warn};

use crate::db;

/// The launcher's migrations, oldest first; only ever append.
const MIGRATIONS: &[&str] = &["
CREATE TABLE launches (
    app TEXT PRIMARY KEY,
    count INTEGER NOT NULL,
    last INTEGER NOT NULL
);
"];

/// How long it takes a launch to count for half as much, in seconds.
const HALF_LIFE: f64 = 14.0 * 86_400.0;
/// How long an app is remembered after its last launch, in seconds.
const FORGET: i64 = 365 * 86_400;

/// How often an app was launched, and when last, in Unix seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launches {
    pub count: u32,
    pub last: i64,
}

/// Every remembered app's launches by desktop id, once read from the database.
static HISTORY: Mutex<Option<HashMap<String, Launches>>> = Mutex::new(None);

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX))
}

/// How much `launches` count at `now`: every launch, each weighing less the longer ago the last one
/// was.
pub fn frecency(launches: Launches, now: i64) -> f64 {
    let age = (now - launches.last).max(0) as f64;
    f64::from(launches.count) * 0.5_f64.powf(age / HALF_LIFE)
}

fn read() -> Result<HashMap<String, Launches>, String> {
    let db = db::open("launcher", MIGRATIONS)?;
    let fail = |error: rusqlite::Error| format!("cannot read the launch history: {error}");
    db.execute("DELETE FROM launches WHERE last < ?1", params![now() - FORGET]).map_err(fail)?;
    let mut statement = db.prepare("SELECT app, count, last FROM launches").map_err(fail)?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, String>(0)?, Launches { count: row.get(1)?, last: row.get(2)? })))
        .map_err(fail)?;
    rows.collect::<Result<_, _>>().map_err(fail)
}

/// Reads the history from the database; call it off the UI thread, as it may wait on the keyring.
/// A launch counted meanwhile is kept: each app has the larger of the two counts, since the
/// database may or may not have that launch already.
pub fn load() {
    let history = read().unwrap_or_else(|error| {
        warn!("{error}");
        HashMap::new()
    });
    debug!(apps = history.len(), "read the launch history");
    let mut shared = HISTORY.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let counted = shared.take().unwrap_or_default();
    *shared = Some(merge(history, counted));
}

/// `stored` with `counted` folded in, each app keeping the larger count and the later launch.
fn merge(mut stored: HashMap<String, Launches>, counted: HashMap<String, Launches>) -> HashMap<String, Launches> {
    for (app, launches) in counted {
        let entry = stored.entry(app).or_insert(launches);
        entry.count = entry.count.max(launches.count);
        entry.last = entry.last.max(launches.last);
    }
    stored
}

/// Every remembered app's frecency now, by desktop id.
pub fn ranks() -> HashMap<String, f64> {
    let now = now();
    HISTORY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .flatten()
        .map(|(app, launches)| (app.clone(), frecency(*launches, now)))
        .collect()
}

/// Counts a launch of `app`, by desktop id.
pub fn record(app: &str) {
    let at = now();
    {
        let mut shared = HISTORY.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let launches = shared.get_or_insert_default().entry(app.to_owned()).or_insert(Launches { count: 0, last: at });
        launches.count += 1;
        launches.last = at;
    }
    let app = app.to_owned();
    let _ = thread::Builder::new().name("island-launches".into()).spawn(move || {
        let written = db::open("launcher", MIGRATIONS).and_then(|db| {
            db.execute(
                "INSERT INTO launches (app, count, last) VALUES (?1, 1, ?2)
                 ON CONFLICT (app) DO UPDATE SET count = count + 1, last = excluded.last",
                params![app, at],
            )
            .map_err(|error| format!("cannot save a launch: {error}"))
        });
        if let Err(error) = written {
            warn!(app, "{error}");
        }
    });
}

#[cfg(test)]
#[path = "launches_test.rs"]
mod tests;
