use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
};

use crate::{
    icons,
    launch::{self, Launch},
};

#[derive(Clone, Debug)]
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

fn dirs() -> Vec<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from);
    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home.map(|home| home.join(".local/share")));
    let data_dirs = env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".to_owned());
    data_home
        .into_iter()
        .chain(data_dirs.split(':').filter(|dir| !dir.is_empty()).map(PathBuf::from))
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
            "Keywords" => app.keywords = value.split(';').filter(|word| !word.is_empty()).map(str::to_lowercase).collect(),
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

pub fn load() -> Vec<App> {
    let mut seen = HashSet::new();
    let mut apps = Vec::new();
    for dir in dirs() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
        paths.sort();
        for path in paths {
            if path.extension().is_none_or(|extension| extension != "desktop") {
                continue;
            }
            let Some(id) = path.file_name().map(|name| name.to_string_lossy().into_owned()) else {
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

pub fn launch(app: &App) -> Option<Launch> {
    let mut argv = launch::parse_exec(&app.exec, &app.name)?;
    if app.terminal {
        argv.insert(0, "xdg-terminal-exec".to_owned());
    }
    Some(Launch {
        id: app.id.trim_end_matches(".desktop").to_owned(),
        name: app.name.clone(),
        argv,
        directory: app.directory.clone(),
    })
}

fn score(app: &App, query: &str) -> Option<u32> {
    let name = app.name.to_lowercase();
    if name.starts_with(query) {
        return Some(1000 - name.len() as u32);
    }
    if name.split_whitespace().any(|word| word.starts_with(query)) {
        return Some(800 - name.len() as u32);
    }
    if name.contains(query) {
        return Some(600);
    }
    if app.keywords.iter().any(|keyword| keyword.starts_with(query))
        || app.generic.to_lowercase().contains(query)
    {
        return Some(400);
    }
    let mut chars = name.chars();
    query
        .chars()
        .all(|wanted| chars.any(|char| char == wanted))
        .then_some(200)
}

pub fn search(apps: &[App], query: &str) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return (0..apps.len()).collect();
    }
    let mut scored: Vec<(u32, usize)> = apps
        .iter()
        .enumerate()
        .filter_map(|(index, app)| Some((score(app, &query)?, index)))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().map(|(_, index)| index).collect()
}
