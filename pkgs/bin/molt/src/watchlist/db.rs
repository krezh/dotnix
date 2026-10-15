//! SQLite persistence for tracked upstream pull requests.

use std::fs;
use std::path::PathBuf;

use rusqlite::{Connection, params};

use super::{ChannelStep, PrState, WatchedPr};

pub(crate) fn db_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))?;
    Some(base.join("molt").join("watchlist.db"))
}

pub(crate) fn open_db() -> rusqlite::Result<Option<Connection>> {
    let Some(path) = db_path() else {
        return Ok(None);
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;
    init_db(&conn)?;
    Ok(Some(conn))
}

pub(crate) fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS watched_prs (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            owner          TEXT NOT NULL,
            repo           TEXT NOT NULL,
            pr_number      INTEGER NOT NULL,
            title          TEXT NOT NULL,
            html_url       TEXT NOT NULL,
            pr_state       TEXT NOT NULL,
            merge_commit   TEXT,
            merged_at      TEXT,
            channels_json  TEXT NOT NULL,
            added_at       INTEGER NOT NULL,
            last_checked   INTEGER NOT NULL,
            UNIQUE(owner, repo, pr_number)
        );",
    )
}

pub(crate) fn list_prs(conn: &Connection) -> rusqlite::Result<Vec<WatchedPr>> {
    let mut stmt = conn.prepare(
        "SELECT id, owner, repo, pr_number, title, html_url, pr_state, merge_commit, merged_at, channels_json, added_at, last_checked
         FROM watched_prs
         ORDER BY id DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        let pr_state_str: String = row.get(6)?;
        let pr_state = match pr_state_str.as_str() {
            "merged" => PrState::Merged,
            "closed" => PrState::Closed,
            _ => PrState::Open,
        };
        let channels_json: String = row.get(9)?;
        let channels: Vec<ChannelStep> = serde_json::from_str(&channels_json).unwrap_or_default();
        Ok(WatchedPr {
            id: row.get(0)?,
            owner: row.get(1)?,
            repo: row.get(2)?,
            pr_number: row.get::<_, i64>(3)? as u64,
            title: row.get(4)?,
            html_url: row.get(5)?,
            pr_state,
            merge_commit: row.get(7)?,
            merged_at: row.get(8)?,
            channels,
            added_at: row.get(10)?,
            last_checked: row.get(11)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub(crate) fn save_pr(conn: &Connection, pr: &WatchedPr) -> rusqlite::Result<i64> {
    let pr_state_str = match pr.pr_state {
        PrState::Open => "open",
        PrState::Merged => "merged",
        PrState::Closed => "closed",
    };
    let channels_json = serde_json::to_string(&pr.channels).unwrap_or_else(|_| "[]".to_owned());
    conn.execute(
        "INSERT INTO watched_prs (owner, repo, pr_number, title, html_url, pr_state, merge_commit, merged_at, channels_json, added_at, last_checked)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(owner, repo, pr_number) DO UPDATE SET
            title = excluded.title,
            html_url = excluded.html_url,
            pr_state = excluded.pr_state,
            merge_commit = excluded.merge_commit,
            merged_at = excluded.merged_at,
            channels_json = excluded.channels_json,
            last_checked = excluded.last_checked",
        params![
            pr.owner,
            pr.repo,
            pr.pr_number as i64,
            pr.title,
            pr.html_url,
            pr_state_str,
            pr.merge_commit,
            pr.merged_at,
            channels_json,
            pr.added_at,
            pr.last_checked,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub(crate) fn delete_pr(conn: &Connection, id: i64) -> rusqlite::Result<bool> {
    let count = conn.execute("DELETE FROM watched_prs WHERE id = ?1", params![id])?;
    Ok(count > 0)
}

pub(crate) fn summary(conn: &Connection) -> rusqlite::Result<WatchlistSummary> {
    let prs = list_prs(conn)?;
    let total = prs.len();
    let ready_in_unstable = prs.iter().filter(|pr| pr.is_ready()).count();
    Ok(WatchlistSummary {
        total,
        ready_in_unstable,
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct WatchlistSummary {
    pub(crate) total: usize,
    pub(crate) ready_in_unstable: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watchlist::{ChannelStep, PrState, StepState, WatchedPr};

    #[test]
    fn sqlite_watchlist_crud_and_summary() {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();

        assert_eq!(list_prs(&conn).unwrap(), vec![]);
        assert_eq!(summary(&conn).unwrap(), WatchlistSummary::default());

        let pr1 = WatchedPr {
            id: 0,
            owner: "NixOS".to_owned(),
            repo: "nixpkgs".to_owned(),
            pr_number: 573942,
            title: "lix fix".to_owned(),
            html_url: "https://github.com/NixOS/nixpkgs/pull/573942".to_owned(),
            pr_state: PrState::Merged,
            merge_commit: Some("982cb88".to_owned()),
            merged_at: Some("2026-10-14T09:09:52Z".to_owned()),
            channels: vec![
                ChannelStep {
                    name: "master".to_owned(),
                    state: StepState::Reached,
                },
                ChannelStep {
                    name: "nixos-unstable".to_owned(),
                    state: StepState::Pending { ahead_by: 63 },
                },
            ],
            added_at: 1000,
            last_checked: 1000,
        };
        let row_id = save_pr(&conn, &pr1).unwrap();
        assert!(row_id > 0);

        let prs = list_prs(&conn).unwrap();
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].pr_number, 573942);
        assert_eq!(prs[0].channels.len(), 2);
        assert_eq!(summary(&conn).unwrap().total, 1);
        assert_eq!(summary(&conn).unwrap().ready_in_unstable, 0);

        // Update pr to reached
        let mut pr1_updated = pr1.clone();
        pr1_updated.channels[1].state = StepState::Reached;
        save_pr(&conn, &pr1_updated).unwrap();

        let prs = list_prs(&conn).unwrap();
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].channels[1].state, StepState::Reached);
        assert_eq!(summary(&conn).unwrap().ready_in_unstable, 1);

        // Delete
        assert!(delete_pr(&conn, prs[0].id).unwrap());
        assert_eq!(list_prs(&conn).unwrap(), vec![]);
        assert_eq!(summary(&conn).unwrap().total, 0);
    }
}
