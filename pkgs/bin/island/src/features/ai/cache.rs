use rusqlite::{Connection, OptionalExtension, params};

use super::state::{Freshness, Provider, Source};

const MIGRATIONS: &[&str] = &["
CREATE TABLE ai_usage (
    identity TEXT PRIMARY KEY NOT NULL,
    snapshot TEXT NOT NULL
);
"];

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
