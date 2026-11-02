use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use super::state::Report;
use crate::paths;

#[derive(Default, Serialize, Deserialize)]
pub(super) struct Cache {
    pub(super) attempted_at: u64,
    pub(super) report: Option<Report>,
}

fn cache_path() -> Option<PathBuf> {
    paths::cache().map(|cache| cache.join("molt.json"))
}

pub(super) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

pub(super) fn load() -> Cache {
    cache_path()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub(super) fn save(cache: &Cache) {
    let Some(path) = cache_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(raw) = serde_json::to_string(cache) {
        let _ = fs::write(path, raw);
    }
}
