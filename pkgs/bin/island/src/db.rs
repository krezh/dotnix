//! The `schema` table remembers how many of each owner's migrations have run, so owners change
//! their own tables without knowing of each other.

use std::{fmt::Display, fs, io::Read, path::Path, sync::Mutex, time::Duration};

use rusqlite::{Connection, ErrorCode, OptionalExtension, params};
use tracing::{info, warn};

use crate::{keyring, paths, sync::lock, time};

const BUSY: Duration = Duration::from_secs(5);
const PLAIN_HEADER: &[u8; 16] = b"SQLite format 3\0";

static KEY: Mutex<Option<Vec<u8>>> = Mutex::new(None);
static OPENING: Mutex<()> = Mutex::new(());

pub struct Owner {
    name: &'static str,
    migrations: &'static [&'static str],
    shared: Mutex<Option<Connection>>,
}

impl Owner {
    /// Only ever append to `migrations`.
    pub const fn new(name: &'static str, migrations: &'static [&'static str]) -> Self {
        Self {
            name,
            migrations,
            shared: Mutex::new(None),
        }
    }

    /// Each migration that has not run before runs once, in its own transaction.
    pub fn open(&self) -> Result<Connection, String> {
        let state = paths::state().ok_or("no home directory")?;
        fs::create_dir_all(&state)
            .map_err(|error| format!("cannot create {}: {error}", state.display()))?;
        open_at(
            &state.join("island.db"),
            &key()?,
            self.name,
            self.migrations,
        )
    }

    /// On the owner's shared connection, opened on first use and again after `work` fails.
    pub fn with<T, E: Display>(
        &self,
        doing: &str,
        work: impl FnOnce(&Connection) -> Result<T, E>,
    ) -> Result<T, String> {
        self.shared(|| self.open(), doing, work)
    }

    fn shared<T, E: Display>(
        &self,
        open: impl FnOnce() -> Result<Connection, String>,
        doing: &str,
        work: impl FnOnce(&Connection) -> Result<T, E>,
    ) -> Result<T, String> {
        let mut shared = lock(&self.shared);
        let db = match shared.take() {
            Some(db) => db,
            None => open()?,
        };
        let done = work(&db).map_err(|error| format!("cannot {doing}: {error}"))?;
        *shared = Some(db);
        Ok(done)
    }
}

#[cfg(test)]
pub fn memory(migrations: &[&str]) -> Connection {
    let db = Connection::open_in_memory().unwrap();
    for migration in migrations {
        db.execute_batch(migration).unwrap();
    }
    db
}

fn key() -> Result<Vec<u8>, String> {
    let mut cached = lock(&KEY);
    if let Some(key) = cached.as_ref() {
        return Ok(key.clone());
    }
    let (key, _) = keyring::database_key()?;
    *cached = Some(key.clone());
    Ok(key)
}

fn unlock(path: &Path, key: &[u8]) -> rusqlite::Result<Connection> {
    let db = Connection::open(path)?;
    db.busy_timeout(BUSY)?;
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
        let seconds = time::now();
        let aside = path.with_extension(format!("db.unreadable-{seconds}"));
        fs::rename(path, &aside)
            .map_err(|error| format!("cannot move {} aside: {error}", path.display()))?;
        warn!(kept = %aside.display(), "the keyring's key does not open the database; starting a new one");
    }
    Ok(())
}

fn open_at(
    path: &Path,
    key: &[u8],
    owner: &str,
    migrations: &[&str],
) -> Result<Connection, String> {
    let fail = |error: rusqlite::Error| format!("{}: {error}", path.display());
    // Switching to WAL and migrating both fail if workers race on a new database.
    let _opening = lock(&OPENING);
    let mut db = match unlock(path, key) {
        Ok(db) => db,
        Err(rusqlite::Error::SqliteFailure(error, _)) if error.code == ErrorCode::NotADatabase => {
            set_aside(path)?;
            unlock(path, key).map_err(fail)?
        }
        Err(error) => return Err(fail(error)),
    };
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
