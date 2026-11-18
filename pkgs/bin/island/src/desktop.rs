//! Desktop entries, as apps describe themselves: where they are, and the keys of their
//! `[Desktop Entry]` group.

use std::{collections::HashSet, fs, path::PathBuf};

use crate::paths;

/// Where desktop entries are, the first winning.
pub fn dirs() -> Vec<PathBuf> {
    paths::data_dirs()
        .into_iter()
        .map(|dir| dir.join("applications"))
        .collect()
}

/// Every entry in `dirs` by its file's name, such as `org.gnome.Nautilus.desktop`: in the order of
/// `dirs` and by name within each, the first of a name winning.
pub fn files(dirs: &[PathBuf]) -> Vec<(String, PathBuf)> {
    let mut seen = HashSet::new();
    let mut found = Vec::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
        paths.sort();
        for path in paths {
            if path
                .extension()
                .is_none_or(|extension| extension != "desktop")
            {
                continue;
            }
            let Some(id) = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
            else {
                continue;
            };
            if seen.insert(id.clone()) {
                found.push((id, path));
            }
        }
    }
    found
}

/// The keys of the `[Desktop Entry]` group and their values, trimmed; other groups, such as
/// actions, are left out.
pub fn keys(raw: &str) -> Vec<(&str, &str)> {
    let mut in_entry = false;
    let mut keys = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if let Some((key, value)) = line.split_once('=').filter(|_| in_entry) {
            keys.push((key.trim(), value.trim()));
        }
    }
    keys
}
