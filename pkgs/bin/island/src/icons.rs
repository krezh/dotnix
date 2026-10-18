use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex, OnceLock},
};

use gpui::{AssetSource, SharedString};

pub struct Files;

impl AssetSource for Files {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = BUNDLED.lock().ok().and_then(|bundled| bundled.get(path).copied()) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        Ok(fs::read(path).ok().map(Cow::Owned))
    }

    fn list(&self, _: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

const TARGET_SIZE: u32 = 32;

static THEME: OnceLock<String> = OnceLock::new();
static INDEX: OnceLock<HashMap<String, PathBuf>> = OnceLock::new();
/// Glyphs island carries itself, by the path they are drawn from.
static BUNDLED: LazyLock<Mutex<HashMap<String, &'static [u8]>>> = LazyLock::new(Mutex::default);

/// Makes `svg` drawable as a glyph from the returned path, such as for parts of an icon that no
/// theme draws separately.
pub fn bundle(name: &str, svg: &'static str) -> PathBuf {
    let path = format!("bundled/{name}.svg");
    if let Ok(mut bundled) = BUNDLED.lock() {
        bundled.insert(path.clone(), svg.as_bytes());
    }
    PathBuf::from(path)
}

pub fn set_theme(theme: String) {
    let _ = THEME.set(theme);
}

fn roots() -> Vec<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from);
    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|home| home.join(".local/share")));
    let data_dirs = env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".to_owned());
    home.map(|home| home.join(".icons"))
        .into_iter()
        .chain(data_home.map(|dir| dir.join("icons")))
        .chain(data_dirs.split(':').filter(|dir| !dir.is_empty()).map(|dir| Path::new(dir).join("icons")))
        .chain([PathBuf::from("/usr/share/pixmaps")])
        .collect()
}

fn inherits(roots: &[PathBuf], theme: &str) -> Vec<String> {
    roots
        .iter()
        .filter_map(|root| fs::read_to_string(root.join(theme).join("index.theme")).ok())
        .find_map(|index| {
            index.lines().find_map(|line| {
                line.strip_prefix("Inherits=").map(|parents| {
                    parents
                        .split(',')
                        .map(|parent| parent.trim().to_owned())
                        .filter(|parent| !parent.is_empty())
                        .collect()
                })
            })
        })
        .unwrap_or_default()
}

fn chain(roots: &[PathBuf], theme: &str) -> Vec<String> {
    let mut themes: Vec<String> = Vec::new();
    let mut pending = vec![theme.to_owned()];
    while let Some(next) = pending.pop() {
        if themes.contains(&next) {
            continue;
        }
        let parents = inherits(roots, &next);
        themes.push(next);
        pending.extend(parents.into_iter().rev());
    }
    if !themes.iter().any(|theme| theme == "hicolor") {
        themes.push("hicolor".to_owned());
    }
    themes
}

fn score(path: &Path) -> u32 {
    if path.extension().is_some_and(|extension| extension == "svg") {
        return 0;
    }
    path.components()
        .filter_map(|component| component.as_os_str().to_str())
        .find_map(|component| component.split(['x', '@']).next()?.parse::<u32>().ok())
        .map_or(1000, |size| size.abs_diff(TARGET_SIZE) + 1)
}

fn walk(dir: &Path, found: &mut HashMap<String, (u32, PathBuf)>, seen: &mut HashSet<PathBuf>) {
    if !fs::canonicalize(dir).is_ok_and(|real| seen.insert(real)) {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, found, seen);
            continue;
        }
        let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
            continue;
        };
        if !matches!(extension, "svg" | "png") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let rank = score(&path);
        match found.get(name) {
            Some((best, _)) if *best <= rank => {}
            _ => {
                found.insert(name.to_owned(), (rank, path));
            }
        }
    }
}

fn build() -> HashMap<String, PathBuf> {
    let roots = roots();
    let theme = THEME.get().map_or("hicolor", String::as_str);
    let mut index = HashMap::new();
    for theme in chain(&roots, theme) {
        let mut found = HashMap::new();
        let mut seen = HashSet::new();
        for root in &roots {
            walk(&root.join(&theme), &mut found, &mut seen);
        }
        for (name, (_, path)) in found {
            index.entry(name).or_insert(path);
        }
    }
    let mut pixmaps = HashMap::new();
    if let Ok(entries) = fs::read_dir("/usr/share/pixmaps") {
        for path in entries.flatten().map(|entry| entry.path()) {
            if let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) {
                pixmaps.insert(name.to_owned(), path.clone());
            }
        }
    }
    for (name, path) in pixmaps {
        index.entry(name).or_insert(path);
    }
    index
}

pub fn lookup(name: &str) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }
    let path = Path::new(name);
    if path.is_absolute() {
        return path.exists().then(|| path.to_path_buf());
    }
    INDEX.get_or_init(build).get(name).cloned()
}

pub fn lookup_in(dir: &str, name: &str) -> Option<PathBuf> {
    if !dir.is_empty() {
        let mut found = HashMap::new();
        walk(Path::new(dir), &mut found, &mut HashSet::new());
        if let Some((_, path)) = found.remove(name) {
            return Some(path);
        }
    }
    lookup(name)
}

pub fn is_svg(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "svg")
}
