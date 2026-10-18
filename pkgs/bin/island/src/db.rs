//! The `schema` table remembers how many of each owner's migrations have run, so owners change
//! their own tables without knowing of each other.

use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, ErrorCode, OptionalExtension, params};
use tracing::{info, warn};

use crate::keyring;

const BUSY: Duration = Duration::from_secs(5);
const PLAIN_HEADER: &[u8; 16] = b"SQLite format 3\0";

static KEY: Mutex<Option<Vec<u8>>> = Mutex::new(None);

pub fn dir(variable: &str, fallback: &str) -> Option<PathBuf> {
    env::var_os(variable)
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(fallback)))
        .map(|base| base.join("island"))
}

/// In memory, only the user's, and gone at logout, for things decrypted from the database.
pub fn runtime_dir() -> Option<PathBuf> {
    env::var_os("XDG_RUNTIME_DIR").map(|base| PathBuf::from(base).join("island"))
}

fn key() -> Result<Vec<u8>, String> {
    let mut cached = KEY.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(key) = cached.as_ref() {
        return Ok(key.clone());
    }
    let (key, _) = keyring::database_key()?;
    *cached = Some(key.clone());
    Ok(key)
}

fn unlock(path: &Path, key: &[u8]) -> rusqlite::Result<Connection> {
    let db = Connection::open(path)?;
    let hex: String = key.iter().map(|byte| format!("{byte:02x}")).collect();
    db.execute_batch(&format!("PRAGMA key = \"x'{hex}'\";"))?;
    db.query_row("SELECT count(*) FROM sqlite_master", [], |row| {
        row.get::<_, i64>(0)
    })?;
    Ok(db)
}

/// An unencrypted one from before encryption is deleted, so its history does not stay on disk in
/// the clear; one encrypted under a key the keyring no longer has is kept beside it.
fn set_aside(path: &Path) -> Result<(), String> {
    let mut header = [0_u8; 16];
    let plain = fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .is_ok()
        && &header == PLAIN_HEADER;
    for suffix in ["-wal", "-shm"] {
        let _ = fs::remove_file(path.with_extension(format!("db{suffix}")));
    }
    if plain {
        fs::remove_file(path).map_err(|error| {
            format!("cannot remove the unencrypted {}: {error}", path.display())
        })?;
        info!(path = %path.display(), "removed the unencrypted database from before encryption");
    } else {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        let aside = path.with_extension(format!("db.unreadable-{seconds}"));
        fs::rename(path, &aside)
            .map_err(|error| format!("cannot move {} aside: {error}", path.display()))?;
        warn!(kept = %aside.display(), "the keyring's key does not open the database; starting a new one");
    }
    Ok(())
}

/// Each migration that has not run before runs once, in its own transaction. Only ever append to
/// `migrations`.
pub fn open(owner: &str, migrations: &[&str]) -> Result<Connection, String> {
    let state = dir("XDG_STATE_HOME", ".local/state").ok_or("no home directory")?;
    fs::create_dir_all(&state)
        .map_err(|error| format!("cannot create {}: {error}", state.display()))?;
    let path = state.join("island.db");
    let fail = |error: rusqlite::Error| format!("{}: {error}", path.display());
    let key = key()?;
    let mut db = match unlock(&path, &key) {
        Ok(db) => db,
        Err(rusqlite::Error::SqliteFailure(error, _)) if error.code == ErrorCode::NotADatabase => {
            set_aside(&path)?;
            unlock(&path, &key).map_err(fail)?
        }
        Err(error) => return Err(fail(error)),
    };
    db.busy_timeout(BUSY).map_err(fail)?;
    db.pragma_update(None, "temp_store", "MEMORY")
        .map_err(fail)?;
    db.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get::<_, String>(0))
        .map_err(fail)?;
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema (owner TEXT PRIMARY KEY, version INTEGER NOT NULL)",
    )
    .map_err(fail)?;
    let done: i64 = db
        .query_row(
            "SELECT version FROM schema WHERE owner = ?1",
            params![owner],
            |row| row.get(0),
        )
        .optional()
        .map_err(fail)?
        .unwrap_or(0);
    for (version, migration) in (1_i64..)
        .zip(migrations)
        .skip(usize::try_from(done).unwrap_or(0))
    {
        let transaction = db.transaction().map_err(fail)?;
        transaction.execute_batch(migration).map_err(fail)?;
        transaction
            .execute(
                "INSERT INTO schema (owner, version) VALUES (?1, ?2)
                 ON CONFLICT (owner) DO UPDATE SET version = excluded.version",
                params![owner, version],
            )
            .map_err(fail)?;
        transaction.commit().map_err(fail)?;
        info!(owner, version, "migrated the database");
    }
    Ok(db)
}

#[cfg(test)]
#[path = "db_test.rs"]
mod tests;
