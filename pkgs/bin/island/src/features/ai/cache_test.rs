use std::{fs, path::PathBuf};

use super::{MIGRATIONS, load_from, save_to};
use crate::features::ai::state::{Credits, Freshness, Limit, Provider, Severity, Source};
use rusqlite::{Connection, params};

fn database() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(MIGRATIONS[0]).unwrap();
    db
}

fn provider(identity: &str) -> Provider {
    Provider {
        source: Source::Claude,
        id: identity.to_owned(),
        provider_id: "claude".to_owned(),
        name: "Claude Code".to_owned(),
        account: "account@example.com".to_owned(),
        plan: "Max".to_owned(),
        limits: vec![Limit {
            id: "five_hour".to_owned(),
            label: "Session".to_owned(),
            used: 37.5,
            remaining: 62.5,
            resets_at: Some(1_800_018_000_000),
            duration_ms: Some(18_000_000),
            severity: Severity::Ok,
            exhausts_at: None,
        }],
        credits: Some(Credits {
            count: 20,
            expires_at: Some(1_800_086_400_000),
        }),
        fetched_at: 1_800_000_000_000,
        freshness: Freshness::Fresh,
    }
}

fn cached(mut provider: Provider) -> Provider {
    provider.freshness = Freshness::Cached;
    for limit in &mut provider.limits {
        limit.exhausts_at = None;
    }
    provider
}

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("island-ai-cache-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir.join("usage.db")
}

#[test]
fn snapshots_survive_reopening_without_changing_source_values() {
    let path = scratch();
    let original = provider("claude:account-a:org-a");
    {
        let db = Connection::open(&path).unwrap();
        db.execute_batch(MIGRATIONS[0]).unwrap();
        save_to(&db, &original).unwrap();
    }
    {
        let db = Connection::open(&path).unwrap();
        assert_eq!(load_from(&db, &original.id).unwrap(), Some(cached(original)));
    }
    fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn accounts_and_organizations_have_distinct_snapshots_and_misses() {
    let db = database();
    let first = provider("claude:account-a:org-a");
    let mut other_account = provider("claude:account-b:org-a");
    other_account.account = "other@example.com".to_owned();
    other_account.limits[0].used = 81.0;
    other_account.limits[0].remaining = 19.0;
    let mut other_org = provider("claude:account-a:org-b");
    other_org.fetched_at += 1_000;
    for snapshot in [&first, &other_account, &other_org] {
        save_to(&db, snapshot).unwrap();
    }
    for snapshot in [first, other_account, other_org] {
        assert_eq!(
            load_from(&db, &snapshot.id).unwrap(),
            Some(cached(snapshot))
        );
    }
    assert_eq!(load_from(&db, "claude:account-b:org-b").unwrap(), None);
    assert_eq!(load_from(&db, "claude:account-a").unwrap(), None);
}

#[test]
fn only_fresh_claude_snapshots_can_replace_last_known_good() {
    let db = database();
    let original = provider("claude:account-a:org-a");
    save_to(&db, &original).unwrap();
    for freshness in [Freshness::Cached, Freshness::Stale] {
        let mut incoming = original.clone();
        incoming.freshness = freshness;
        incoming.limits[0].used = 99.0;
        incoming.fetched_at += 10_000;
        save_to(&db, &incoming).unwrap();
        assert_eq!(
            load_from(&db, &original.id).unwrap(),
            Some(cached(original.clone()))
        );
        incoming.id = "absent".to_owned();
        save_to(&db, &incoming).unwrap();
        assert_eq!(load_from(&db, "absent").unwrap(), None);
    }
    let mut incoming = original.clone();
    incoming.source = Source::Omp;
    save_to(&db, &incoming).unwrap();
    assert_eq!(
        load_from(&db, &original.id).unwrap(),
        Some(cached(original.clone()))
    );
    incoming.source = Source::Claude;
    incoming.limits[0].used = 42.0;
    incoming.limits[0].remaining = 58.0;
    incoming.fetched_at += 20_000;
    save_to(&db, &incoming).unwrap();
    assert_eq!(load_from(&db, &incoming.id).unwrap(), Some(cached(incoming)));
}

#[test]
fn loading_clears_exhaustion_projections() {
    let db = database();
    let mut original = provider("claude:account-a:org-a");
    original.limits[0].exhausts_at = Some(original.fetched_at + 60_000);
    save_to(&db, &original).unwrap();
    assert_eq!(load_from(&db, &original.id).unwrap(), Some(cached(original)));
}

#[test]
fn invalid_snapshots_are_errors_not_cross_account_results() {
    let db = database();
    let key = "claude:account-a:org-a";
    let mut wrong_source = provider(key);
    wrong_source.source = Source::Omp;
    for snapshot in [
        serde_json::to_string(&provider("claude:account-b:org-a")).unwrap(),
        serde_json::to_string(&wrong_source).unwrap(),
        "not json".to_owned(),
    ] {
        db.execute(
            "INSERT OR REPLACE INTO ai_usage (identity, snapshot) VALUES (?1, ?2)",
            params![key, snapshot],
        )
        .unwrap();
        assert!(load_from(&db, key).is_err());
    }
}
