//! Small facts Swix remembers between runs, kept as plain-text files under
//! `$XDG_CACHE_HOME/swix` (falling back to `~/.cache/swix`).

use std::env;
use std::fs;
use std::path::PathBuf;

/// Path of a cache entry, or `None` when neither cache root can be resolved.
pub(crate) fn path(name: &str) -> Option<PathBuf> {
    let root = env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;
    Some(root.join("swix").join(name))
}

/// Best effort: a cache that cannot be written only costs the next run its hint.
pub(crate) fn write(name: &str, contents: &str) {
    let Some(path) = path(name) else {
        return;
    };
    if let Some(parent) = path.parent()
        && fs::create_dir_all(parent).is_ok()
    {
        let _ = fs::write(path, contents);
    }
}

pub(crate) fn read(name: &str) -> Option<String> {
    fs::read_to_string(path(name)?).ok()
}

pub(crate) fn remove(name: &str) {
    if let Some(path) = path(name) {
        let _ = fs::remove_file(path);
    }
}
