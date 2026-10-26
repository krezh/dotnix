use std::path::Path;

use rusqlite::{Connection, OpenFlags, Transaction};

const NIX_DATABASE: &str = "file:/nix/var/nix/db/db.sqlite";

#[derive(Debug, PartialEq, Eq)]
pub(super) struct StorePath {
    pub(super) path: String,
    pub(super) nar_size: i64,
}

pub(super) fn query_closures(
    old: &Path,
    new: &Path,
) -> Result<(Vec<StorePath>, Vec<StorePath>), String> {
    query_database(Path::new(NIX_DATABASE), old, new)
}

fn query_database(
    database: &Path,
    old: &Path,
    new: &Path,
) -> Result<(Vec<StorePath>, Vec<StorePath>), String> {
    let canonicalize = |root: &Path| {
        root.canonicalize()
            .map_err(|error| format!("cannot resolve closure root {}: {error}", root.display()))
    };
    let old = canonicalize(old)?;
    let new = canonicalize(new)?;
    let mut connection = open_database(database)
        .map_err(|error| format!("cannot open Nix store database: {error}"))?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("cannot start closure snapshot: {error}"))?;
    let old = query_closure(&transaction, &old)?;
    let new = query_closure(&transaction, &new)?;
    Ok((old, new))
}

fn open_database(database: &Path) -> rusqlite::Result<Connection> {
    let connection = Connection::open_with_flags(
        database,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_URI,
    )?;
    connection
        .execute_batch("PRAGMA mmap_size=268435456; PRAGMA temp_store=2; PRAGMA query_only=ON;")?;
    Ok(connection)
}

fn query_closure(transaction: &Transaction<'_>, root: &Path) -> Result<Vec<StorePath>, String> {
    let root = root
        .to_str()
        .ok_or_else(|| format!("closure root is not UTF-8: {}", root.display()))?;
    let query = || -> rusqlite::Result<Vec<StorePath>> {
        let mut statement = transaction.prepare(
            "WITH RECURSIVE graph(p) AS (
                 SELECT id FROM ValidPaths WHERE path = ?1
                 UNION
                 SELECT reference FROM Refs JOIN graph ON referrer = p
             )
             SELECT path, narSize FROM graph JOIN ValidPaths ON p = id ORDER BY path",
        )?;
        let rows = statement.query_map([root], |row| {
            Ok(StorePath {
                path: row.get(0)?,
                nar_size: row.get(1)?,
            })
        })?;
        rows.collect()
    };
    let paths = query().map_err(|error| format!("cannot read closure for {root}: {error}"))?;
    if paths.is_empty() {
        return Err(format!(
            "closure root is not registered in the Nix store: {root}"
        ));
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DATABASE: AtomicUsize = AtomicUsize::new(0);

    struct Database {
        directory: PathBuf,
        path: PathBuf,
        connection: Connection,
    }

    impl Database {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!(
                "molt-closure-db-test-{}-{}",
                std::process::id(),
                NEXT_DATABASE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&directory).unwrap();
            let path = directory.join("db.sqlite");
            let connection = Connection::open(&path).unwrap();
            connection
                .execute_batch(
                    "CREATE TABLE ValidPaths (
                         id INTEGER PRIMARY KEY, path TEXT UNIQUE NOT NULL, narSize INTEGER NOT NULL
                     );
                     CREATE TABLE Refs (referrer INTEGER NOT NULL, reference INTEGER NOT NULL);",
                )
                .unwrap();
            Self {
                directory,
                path,
                connection,
            }
        }

        fn root(&self, id: i64, name: &str, size: i64) -> PathBuf {
            let path = self.directory.join(name);
            fs::write(&path, []).unwrap();
            self.connection
                .execute(
                    "INSERT INTO ValidPaths (id, path, narSize) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id, path.to_str().unwrap(), size],
                )
                .unwrap();
            path
        }

        fn references(&self, edges: &[(i64, i64)]) {
            for (referrer, reference) in edges {
                self.connection
                    .execute(
                        "INSERT INTO Refs (referrer, reference) VALUES (?1, ?2)",
                        [referrer, reference],
                    )
                    .unwrap();
            }
        }
    }

    impl Drop for Database {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.directory).unwrap();
        }
    }

    #[test]
    fn reads_unique_sorted_closures_through_cycles_and_diamonds() {
        let database = Database::new();
        let old = database.root(1, "old", 10);
        database.root(2, "left", 20);
        database.root(3, "right", 30);
        database.root(4, "shared", 40);
        let new = database.root(5, "new", 50);
        database.references(&[(1, 2), (1, 3), (2, 4), (3, 4), (4, 2), (5, 3)]);

        let (old, new) = query_database(&database.path, &old, &new).unwrap();
        let names = |paths: &[StorePath]| {
            paths
                .iter()
                .map(|path| {
                    (
                        Path::new(&path.path)
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_owned(),
                        path.nar_size,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names(&old),
            [("left", 20), ("old", 10), ("right", 30), ("shared", 40)]
                .map(|(name, size)| (name.to_owned(), size))
        );
        assert_eq!(
            names(&new),
            [("left", 20), ("new", 50), ("right", 30), ("shared", 40)]
                .map(|(name, size)| (name.to_owned(), size))
        );
    }

    #[test]
    fn resolves_profile_symlinks_and_rejects_missing_or_unregistered_roots() {
        let database = Database::new();
        let root = database.root(1, "root", 42);
        let profile = database.directory.join("profile");
        symlink(&root, &profile).unwrap();
        let (old, new) = query_database(&database.path, &profile, &root).unwrap();
        assert_eq!(old, new);
        assert_eq!(old[0].path, root.to_str().unwrap());
        let missing = database.directory.join("missing");
        assert!(query_database(&database.path, &missing, &root).is_err());
        assert!(query_database(&database.path, &root, &missing).is_err());
        fs::write(&missing, []).unwrap();
        assert!(
            query_database(&database.path, &missing, &root)
                .unwrap_err()
                .contains("not registered")
        );
        assert!(query_database(&database.path, &root, &missing).is_err());
    }

    #[test]
    fn opens_read_only_without_creating_a_missing_database() {
        let database = Database::new();
        let root = database.root(1, "root", 42);
        fs::set_permissions(&database.path, fs::Permissions::from_mode(0o444)).unwrap();
        assert!(query_database(&database.path, &root, &root).is_ok());
        let connection = open_database(&database.path).unwrap();
        assert!(connection.execute("DELETE FROM ValidPaths", []).is_err());
        let missing = database.directory.join("missing.sqlite");
        assert!(query_database(&missing, &root, &root).is_err());
        assert!(!missing.exists());
    }

    #[test]
    fn reads_both_roots_from_one_snapshot() {
        let database = Database::new();
        database
            .connection
            .execute_batch("PRAGMA journal_mode=WAL;")
            .unwrap();
        let old = database.root(1, "old", 10);
        let new = database.root(2, "new", 20);
        let dependency = database.root(3, "dependency", 30);
        let mut connection = open_database(&database.path).unwrap();
        let transaction = connection.transaction().unwrap();
        assert_eq!(query_closure(&transaction, &old).unwrap().len(), 1);
        database.references(&[(2, 3)]);
        let snapshot = query_closure(&transaction, &new).unwrap();
        assert_eq!(snapshot.len(), 1);
        drop(transaction);
        let (_, current) = query_database(&database.path, &old, &new).unwrap();
        assert_eq!(current.len(), 2);
        assert!(
            current
                .iter()
                .any(|path| path.path == dependency.to_str().unwrap())
        );
    }
}
