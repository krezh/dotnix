use rusqlite::{Connection, OptionalExtension, params};

use super::{
    history::Window,
    state::{Freshness, Provider, Source},
};

const MIGRATIONS: &[&str] = &[
    "
CREATE TABLE ai_usage (
    identity TEXT PRIMARY KEY NOT NULL,
    snapshot TEXT NOT NULL
);
",
    "
CREATE TABLE ai_samples (
    provider TEXT NOT NULL,
    limit_id TEXT NOT NULL,
    start INTEGER NOT NULL,
    duration INTEGER NOT NULL,
    at INTEGER NOT NULL,
    used REAL NOT NULL,
    PRIMARY KEY (provider, limit_id, at)
);
",
];

/// The windows still running at `now`, as the refreshes before island last stopped saw them.
pub(super) fn windows(now: i64) -> Result<Vec<Window>, String> {
    let db = crate::db::open("ai", MIGRATIONS)?;
    windows_from(&db, now)
}

/// Keeps a window's latest sample, and lets go of the limit's earlier windows.
pub(super) fn keep(window: &Window) -> Result<(), String> {
    let db = crate::db::open("ai", MIGRATIONS)?;
    keep_to(&db, window)
}

fn windows_from(db: &Connection, now: i64) -> Result<Vec<Window>, String> {
    let fail = |error: rusqlite::Error| format!("cannot read AI usage samples: {error}");
    let mut query = db
        .prepare(
            "SELECT provider, limit_id, start, duration, at, used FROM ai_samples
             WHERE start + duration > ?1 ORDER BY provider, limit_id, start, at",
        )
        .map_err(fail)?;
    let rows = query
        .query_map(params![now], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                (row.get::<_, i64>(4)?, row.get::<_, f64>(5)?),
            ))
        })
        .map_err(fail)?;
    let mut windows: Vec<Window> = Vec::new();
    for row in rows {
        let (provider, limit, start, duration, sample) = row.map_err(fail)?;
        match windows.last_mut() {
            Some(window) if window.provider == provider && window.limit == limit => {
                // A later window of the same limit replaces an earlier one left behind.
                if window.start != start {
                    window.start = start;
                    window.duration = duration;
                    window.samples.clear();
                }
                window.samples.push(sample);
            }
            _ => windows.push(Window {
                provider,
                limit,
                start,
                duration,
                samples: vec![sample],
            }),
        }
    }
    Ok(windows)
}

fn keep_to(db: &Connection, window: &Window) -> Result<(), String> {
    let Some((at, used)) = window.samples.last().copied() else {
        return Ok(());
    };
    let fail = |error: rusqlite::Error| format!("cannot write AI usage samples: {error}");
    db.execute(
        "DELETE FROM ai_samples WHERE provider = ?1 AND limit_id = ?2 AND start != ?3",
        params![window.provider, window.limit, window.start],
    )
    .map_err(fail)?;
    db.execute(
        "INSERT OR REPLACE INTO ai_samples (provider, limit_id, start, duration, at, used)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            window.provider,
            window.limit,
            window.start,
            window.duration,
            at,
            used
        ],
    )
    .map_err(fail)?;
    Ok(())
}

pub(super) fn load(identity: &str) -> Result<Option<Provider>, String> {
    let db = crate::db::open("ai", MIGRATIONS)?;
    load_from(&db, identity)
}

pub(super) fn save(provider: &Provider) -> Result<(), String> {
    if provider.source != Source::Claude || !provider.freshness.is_fresh() {
        return Ok(());
    }
    let db = crate::db::open("ai", MIGRATIONS)?;
    save_to(&db, provider)
}

fn load_from(db: &Connection, identity: &str) -> Result<Option<Provider>, String> {
    let snapshot: Option<String> = db
        .query_row(
            "SELECT snapshot FROM ai_usage WHERE identity = ?1",
            params![identity],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("cannot read AI usage cache: {error}"))?;
    let Some(snapshot) = snapshot else {
        return Ok(None);
    };
    let mut provider: Provider = serde_json::from_str(&snapshot)
        .map_err(|error| format!("cannot decode AI usage cache: {error}"))?;
    if provider.source != Source::Claude || provider.id != identity {
        return Err("AI usage cache identity or source does not match".to_owned());
    }
    provider.freshness = Freshness::Cached;
    for limit in &mut provider.limits {
        limit.exhausts_at = None;
    }
    Ok(Some(provider))
}

fn save_to(db: &Connection, provider: &Provider) -> Result<(), String> {
    if provider.source != Source::Claude || !provider.freshness.is_fresh() {
        return Ok(());
    }
    let snapshot = serde_json::to_string(provider)
        .map_err(|error| format!("cannot encode AI usage cache: {error}"))?;
    db.execute(
        "INSERT INTO ai_usage (identity, snapshot) VALUES (?1, ?2)
         ON CONFLICT (identity) DO UPDATE SET snapshot = excluded.snapshot",
        params![provider.id, snapshot],
    )
    .map_err(|error| format!("cannot write AI usage cache: {error}"))?;
    Ok(())
}

#[cfg(test)]
#[path = "cache_test.rs"]
mod tests;
