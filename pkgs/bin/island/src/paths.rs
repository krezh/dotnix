use std::{env, path::PathBuf};

use crate::profile;

const ISLAND: &str = "island";

pub fn home() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

fn base(variable: &str, fallback: &str) -> Option<PathBuf> {
    env::var_os(variable)
        .map(PathBuf::from)
        .or_else(|| home().map(|home| home.join(fallback)))
}

pub fn config() -> Option<PathBuf> {
    base("XDG_CONFIG_HOME", ".config").map(|base| base.join(ISLAND))
}

pub fn state() -> Option<PathBuf> {
    base("XDG_STATE_HOME", ".local/state").map(|base| base.join(profile::folder()))
}

pub fn cache() -> Option<PathBuf> {
    base("XDG_CACHE_HOME", ".cache").map(|base| base.join(profile::folder()))
}

pub fn runtime_root() -> Option<PathBuf> {
    env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from)
}

/// In memory, only the user's, and gone at logout, for things decrypted from the database.
pub fn runtime() -> Option<PathBuf> {
    runtime_root().map(|base| base.join(profile::folder()))
}

pub fn data_dirs() -> Vec<PathBuf> {
    let shared =
        env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".to_owned());
    base("XDG_DATA_HOME", ".local/share")
        .into_iter()
        .chain(
            shared
                .split(':')
                .filter(|dir| !dir.is_empty())
                .map(PathBuf::from),
        )
        .collect()
}
