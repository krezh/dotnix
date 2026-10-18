//! Picture previews are files in `$XDG_RUNTIME_DIR`, which lives in memory, so nothing copied
//! reaches the disk unencrypted; they are made again from the database after a login.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use image::ImageFormat;
use rusqlite::{Connection, OptionalExtension, params};
use tracing::debug;
use uuid::Uuid;

use super::{
    entry::{Entry, Kind, files},
    settings::Settings,
};
use crate::db;

/// Twice the preview pane, for sharp scaling.
const PREVIEW_W: u32 = 860;
const PREVIEW_H: u32 = 720;

/// Only ever append.
const MIGRATIONS: &[&str] = &["
CREATE TABLE clipboard (
    id INTEGER PRIMARY KEY,
    hash TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    mime TEXT NOT NULL,
    data BLOB NOT NULL,
    text TEXT NOT NULL,
    size INTEGER NOT NULL,
    width INTEGER,
    height INTEGER,
    created INTEGER NOT NULL,
    used INTEGER NOT NULL
);
CREATE INDEX clipboard_used ON clipboard (used DESC);
"];

/// How much of an entry's text is kept for searching and previews.
const SEARCHED: usize = 4096;
const PREVIEWED_FILE: u64 = 64 * 1024 * 1024;

pub(super) struct Clip {
    pub(super) kind: Kind,
    pub(super) mime: String,
    pub(super) data: Vec<u8>,
}

pub(super) struct Store {
    db: Connection,
    previews: PathBuf,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        })
}

/// The picture's size, when it was one.
fn write_preview(path: &Path, data: Option<&[u8]>, file: Option<&Path>) -> Option<(u32, u32)> {
    let picture = match (data, file) {
        (Some(data), _) => image::load_from_memory(data).ok()?,
        (None, Some(file)) if fs::metadata(file).ok()?.len() <= PREVIEWED_FILE => {
            image::open(file).ok()?
        }
        _ => return None,
    };
    let size = (picture.width(), picture.height());
    if !path.exists() {
        let preview = picture.thumbnail(PREVIEW_W, PREVIEW_H);
        if let Err(error) = preview.save_with_format(path, ImageFormat::Png) {
            debug!("cannot write a clipboard preview: {error}");
        }
    }
    Some(size)
}

impl Store {
    pub(super) fn open() -> Result<Self, String> {
        if let Some(unencrypted) =
            db::dir("XDG_CACHE_HOME", ".cache").map(|cache| cache.join("clipboard"))
        {
            let _ = fs::remove_dir_all(unencrypted);
        }
        let previews = db::runtime_dir()
            .ok_or("XDG_RUNTIME_DIR is not set")?
            .join("clipboard");
        fs::create_dir_all(&previews)
            .map_err(|error| format!("cannot create {}: {error}", previews.display()))?;
        let db = db::open("clipboard", MIGRATIONS)?;
        let store = Self { db, previews };
        if let Err(error) = store.fill_previews() {
            debug!("cannot fill in clipboard previews: {error}");
        }
        Ok(store)
    }

    /// Such as after the preview size changed.
    fn fill_previews(&self) -> rusqlite::Result<()> {
        let mut statement = self
            .db
            .prepare("SELECT hash, kind, data FROM clipboard WHERE kind != 'text'")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let hash: String = row.get(0)?;
            let preview = self.preview_path(&hash);
            if preview.exists() {
                continue;
            }
            let kind: String = row.get(1)?;
            let data: Vec<u8> = row.get(2)?;
            match Kind::parse(&kind) {
                Some(Kind::Image) => write_preview(&preview, Some(&data), None),
                Some(Kind::Files) => files(&data)
                    .first()
                    .and_then(|first| write_preview(&preview, None, Some(first))),
                _ => None,
            };
        }
        Ok(())
    }

    fn preview_path(&self, hash: &str) -> PathBuf {
        self.previews
            .join(format!("{hash}-{PREVIEW_W}x{PREVIEW_H}.png"))
    }

    /// Moves it to the top when it is already kept.
    pub(super) fn insert(&self, clip: &Clip) -> rusqlite::Result<()> {
        let hash = Uuid::new_v5(&Uuid::NAMESPACE_OID, &clip.data)
            .simple()
            .to_string();
        let preview = self.preview_path(&hash);
        let (text, size) = match clip.kind {
            Kind::Text => (String::from_utf8_lossy(&clip.data).into_owned(), None),
            Kind::Image => (
                String::new(),
                write_preview(&preview, Some(&clip.data), None),
            ),
            Kind::Files => {
                let paths = files(&clip.data);
                let size = paths
                    .first()
                    .and_then(|first| write_preview(&preview, None, Some(first)));
                let names: Vec<String> = paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect();
                (names.join("\n"), size)
            }
        };
        let now = now();
        self.db.execute(
            "INSERT INTO clipboard (hash, kind, mime, data, text, size, width, height, created, used)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
             ON CONFLICT (hash) DO UPDATE SET used = excluded.used",
            params![
                hash,
                clip.kind.word(),
                clip.mime,
                clip.data,
                text,
                clip.data.len() as i64,
                size.map(|(width, _)| width),
                size.map(|(_, height)| height),
                now,
            ],
        )?;
        Ok(())
    }

    pub(super) fn list(&self) -> rusqlite::Result<Vec<Entry>> {
        let mut statement = self.db.prepare(
            "SELECT id, hash, kind, mime, substr(text, 1, ?1), length(text), size, width, height, used
             FROM clipboard ORDER BY used DESC",
        )?;
        let rows = statement.query_map(params![SEARCHED as i64], |row| {
            let hash: String = row.get(1)?;
            let kind: String = row.get(2)?;
            let width: Option<u32> = row.get(7)?;
            let height: Option<u32> = row.get(8)?;
            Ok(Entry {
                id: row.get(0)?,
                kind: Kind::parse(&kind).unwrap_or(Kind::Text),
                mime: row.get(3)?,
                text: row.get(4)?,
                chars: row.get(5)?,
                size: row.get(6)?,
                dimensions: width.zip(height),
                used: row.get(9)?,
                preview: Some(self.preview_path(&hash)).filter(|path| path.exists()),
            })
        })?;
        rows.collect()
    }

    pub(super) fn get(&self, id: i64) -> rusqlite::Result<Option<Clip>> {
        self.db
            .query_row(
                "SELECT kind, mime, data FROM clipboard WHERE id = ?1",
                params![id],
                |row| {
                    let kind: String = row.get(0)?;
                    Ok(Clip {
                        kind: Kind::parse(&kind).unwrap_or(Kind::Text),
                        mime: row.get(1)?,
                        data: row.get(2)?,
                    })
                },
            )
            .optional()
    }

    pub(super) fn newest(&self) -> rusqlite::Result<Option<i64>> {
        self.db
            .query_row(
                "SELECT id FROM clipboard ORDER BY used DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
    }

    pub(super) fn touch(&self, id: i64) -> rusqlite::Result<()> {
        self.db.execute(
            "UPDATE clipboard SET used = ?1 WHERE id = ?2",
            params![now(), id],
        )?;
        Ok(())
    }

    pub(super) fn delete(&self, id: i64) -> rusqlite::Result<()> {
        let hash: Option<String> = self
            .db
            .query_row(
                "SELECT hash FROM clipboard WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(hash) = hash {
            self.db
                .execute("DELETE FROM clipboard WHERE id = ?1", params![id])?;
            let _ = fs::remove_file(self.preview_path(&hash));
        }
        Ok(())
    }

    pub(super) fn clear(&self) -> rusqlite::Result<()> {
        self.db.execute("DELETE FROM clipboard", [])?;
        if let Ok(previews) = fs::read_dir(&self.previews) {
            for preview in previews.flatten() {
                let _ = fs::remove_file(preview.path());
            }
        }
        Ok(())
    }

    pub(super) fn prune(&self, settings: &Settings) -> rusqlite::Result<()> {
        let rows: Vec<(i64, i64)> = self
            .db
            .prepare("SELECT id, size FROM clipboard ORDER BY used DESC")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let budget = i64::try_from(settings.history.max_total_mb.saturating_mul(1024 * 1024))
            .unwrap_or(i64::MAX);
        let mut total = 0_i64;
        for (index, (id, size)) in rows.into_iter().enumerate() {
            total = total.saturating_add(size);
            if index >= settings.history.max_entries || total > budget {
                self.delete(id)?;
            }
        }
        Ok(())
    }
}
