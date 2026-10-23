//! How much went over the network each day, kept in island's database so a week survives
//! restarts.

use rusqlite::params;

use crate::db;

/// Only ever append.
const MIGRATIONS: &[&str] = &["
CREATE TABLE usage (
    day TEXT PRIMARY KEY,
    rx INTEGER NOT NULL,
    tx INTEGER NOT NULL
);
"];

pub const DAYS: i64 = 7;

/// Adds to the day's totals.
pub fn add(day: &str, rx: u64, tx: u64) -> Result<(), String> {
    let db = db::open("network", MIGRATIONS)?;
    db.execute(
        "INSERT INTO usage (day, rx, tx) VALUES (?1, ?2, ?3)
         ON CONFLICT (day) DO UPDATE SET rx = rx + excluded.rx, tx = tx + excluded.tx",
        params![day, rx as i64, tx as i64],
    )
    .map(drop)
    .map_err(|error| format!("cannot save the data used: {error}"))
}

/// Received and sent on each day from `since` on, as `YYYY-MM-DD`.
pub fn read(since: &str) -> Result<Vec<(String, u64, u64)>, String> {
    let db = db::open("network", MIGRATIONS)?;
    let fail = |error: rusqlite::Error| format!("cannot read the data used: {error}");
    db.execute("DELETE FROM usage WHERE day < ?1", params![since])
        .map_err(fail)?;
    let mut statement = db
        .prepare("SELECT day, rx, tx FROM usage WHERE day >= ?1 ORDER BY day")
        .map_err(fail)?;
    let rows = statement
        .query_map(params![since], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?.max(0) as u64,
                row.get::<_, i64>(2)?.max(0) as u64,
            ))
        })
        .map_err(fail)?;
    rows.collect::<Result<_, _>>().map_err(fail)
}
