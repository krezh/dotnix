use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
};

use super::settings::Settings;

use crate::{
    icons,
    launch::{self, Launch},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct App {
    pub id: String,
    pub name: String,
    pub generic: String,
    pub comment: String,
    pub keywords: Vec<String>,
    pub exec: String,
    pub terminal: bool,
    pub directory: Option<PathBuf>,
    pub icon: Option<PathBuf>,
}

pub(super) fn dirs() -> Vec<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from);
    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home.map(|home| home.join(".local/share")));
    let data_dirs =
        env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".to_owned());
    data_home
        .into_iter()
        .chain(
            data_dirs
                .split(':')
                .filter(|dir| !dir.is_empty())
                .map(PathBuf::from),
        )
        .map(|dir| dir.join("applications"))
        .collect()
}

fn parse(path: &Path, id: String) -> Option<App> {
    let raw = fs::read_to_string(path).ok()?;
    let mut in_entry = false;
    let mut app = App {
        id,
        name: String::new(),
        generic: String::new(),
        comment: String::new(),
        keywords: Vec::new(),
        exec: String::new(),
        terminal: false,
        directory: None,
        icon: None,
    };
    let mut icon = String::new();
    let mut kind = String::new();
    let mut hidden = false;
    for line in raw.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().to_owned();
        match key.trim() {
            "Name" => app.name = value,
            "GenericName" => app.generic = value,
            "Comment" => app.comment = value,
            "Keywords" => {
                app.keywords = value
                    .split(';')
                    .filter(|word| !word.is_empty())
                    .map(str::to_lowercase)
                    .collect()
            }
            "Exec" => app.exec = value,
            "Terminal" => app.terminal = value == "true",
            "Path" if !value.is_empty() => app.directory = Some(PathBuf::from(value)),
            "Icon" => icon = value,
            "Type" => kind = value,
            "NoDisplay" | "Hidden" => hidden |= value == "true",
            _ => {}
        }
    }
    if hidden || kind != "Application" || app.name.is_empty() || app.exec.is_empty() {
        return None;
    }
    app.icon = icons::lookup(&icon);
    Some(app)
}

pub(super) fn load(dirs: &[PathBuf]) -> Vec<App> {
    let mut seen = HashSet::new();
    let mut apps = Vec::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(&dir) else {
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
            if !seen.insert(id.clone()) {
                continue;
            }
            if let Some(app) = parse(&path, id) {
                apps.push(app);
            }
        }
    }
    apps.sort_by_key(|app| app.name.to_lowercase());
    apps
}

pub(super) fn launch(app: &App, settings: &Settings) -> Option<Launch> {
    let mut argv = launch::parse_exec(&app.exec, &app.name)?;
    if app.terminal {
        settings.check().ok()?;
        argv.splice(0..0, settings.terminal.iter().cloned());
    }
    Some(Launch {
        id: app.id.trim_end_matches(".desktop").to_owned(),
        name: app.name.clone(),
        argv,
        directory: app.directory.clone(),
    })
}

/// Best highest: the name starts with the query, a word of it does, the name contains it, a keyword
/// or the generic name does, or the name has the query's letters in order.
fn tier(app: &App, query: &str) -> Option<u32> {
    let name = app.name.to_lowercase();
    if name.starts_with(query) {
        return Some(5);
    }
    if name.split_whitespace().any(|word| word.starts_with(query)) {
        return Some(4);
    }
    if name.contains(query) {
        return Some(3);
    }
    if app
        .keywords
        .iter()
        .any(|keyword| keyword.starts_with(query))
        || app.generic.to_lowercase().contains(query)
    {
        return Some(2);
    }
    let mut chars = name.chars();
    query
        .chars()
        .all(|wanted| chars.any(|char| char == wanted))
        .then_some(1)
}

/// The best kind of match first, then the most used, then the shortest name, then by name. With no
/// query, every app, the most used first.
pub fn search(apps: &[App], query: &str, ranks: &HashMap<String, f64>) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    let rank = |index: usize| ranks.get(&apps[index].id).copied().unwrap_or(0.0);
    let length = |index: usize| apps[index].name.chars().count();
    let mut matched: Vec<(u32, usize)> = apps
        .iter()
        .enumerate()
        .filter_map(|(index, app)| {
            Some((
                if query.is_empty() {
                    0
                } else {
                    tier(app, &query)?
                },
                index,
            ))
        })
        .collect();
    matched.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(rank(b.1).total_cmp(&rank(a.1)))
            .then_with(|| {
                if query.is_empty() {
                    std::cmp::Ordering::Equal
                } else {
                    length(a.1).cmp(&length(b.1))
                }
            })
            .then(a.1.cmp(&b.1))
    });
    matched.into_iter().map(|(_, index)| index).collect()
}

#[cfg(test)]
#[path = "desktop_test.rs"]
mod tests;
