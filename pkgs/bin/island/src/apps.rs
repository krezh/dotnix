//! What a window's class stands for: its app's name and icon, from the app's desktop entry.

use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::LazyLock,
};

use crate::{desktop, icons};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct App {
    pub name: Option<String>,
    pub icon: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    name: String,
    icon: String,
    class: Option<String>,
}

/// Every desktop entry, by its window class and by its file's name, both in lower case; the first
/// of a name in the data directories wins, as it does for the desktop.
#[derive(Default)]
struct Entries {
    by_class: HashMap<String, Entry>,
    by_id: HashMap<String, Entry>,
}

static ENTRIES: LazyLock<Entries> = LazyLock::new(read);

fn parse(raw: &str) -> Option<Entry> {
    let mut name = None;
    let mut icon = String::new();
    let mut class = None;
    for (key, value) in desktop::keys(raw) {
        match key {
            "Name" => name = Some(value.to_owned()),
            "Icon" => value.clone_into(&mut icon),
            "StartupWMClass" => class = Some(value.to_owned()),
            _ => {}
        }
    }
    Some(Entry {
        name: name.filter(|name| !name.is_empty())?,
        icon,
        class,
    })
}

fn read() -> Entries {
    let mut entries = Entries::default();
    for (file, path) in desktop::files(&desktop::dirs()) {
        let id = file.trim_end_matches(".desktop").to_lowercase();
        let Some(entry) = fs::read_to_string(&path).ok().as_deref().and_then(parse) else {
            continue;
        };
        if let Some(class) = &entry.class {
            entries
                .by_class
                .entry(class.to_lowercase())
                .or_insert_with(|| entry.clone());
        }
        entries.by_id.entry(id).or_insert(entry);
    }
    entries
}

/// The last part of a reverse-domain name, such as `ghostty` of `com.mitchellh.ghostty`.
pub fn last_part(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

fn entry(class: &str) -> Option<&'static Entry> {
    let class = class.to_lowercase();
    let last = last_part(&class);
    let entries = &*ENTRIES;
    entries
        .by_class
        .get(&class)
        .or_else(|| entries.by_id.get(&class))
        .or_else(|| entries.by_id.get(last))
        .or_else(|| {
            entries
                .by_id
                .iter()
                .find(|(id, _)| last_part(id) == last)
                .map(|(_, entry)| entry)
        })
}

/// The app a window of `class` belongs to, or what can be found of it.
pub fn app(class: &str) -> App {
    let entry = entry(class);
    App {
        name: entry.map(|entry| entry.name.clone()),
        icon: entry
            .and_then(|entry| icons::lookup(&entry.icon))
            .or_else(|| icons::lookup(class))
            .or_else(|| icons::lookup(&class.to_lowercase())),
    }
}

/// What a window of `class` is called: its app's name, or else the class.
pub fn name(app: &App, class: &str) -> String {
    app.name.clone().unwrap_or_else(|| class.to_owned())
}

#[cfg(test)]
#[path = "apps_test.rs"]
mod tests;
