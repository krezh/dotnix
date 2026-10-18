use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    env, fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex, OnceLock},
};

use gpui::{AssetSource, SharedString};

pub struct Files;

impl AssetSource for Files {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = BUNDLED
            .lock()
            .ok()
            .and_then(|bundled| bundled.get(path).copied())
        {
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
static INDEX: LazyLock<Mutex<Index>> = LazyLock::new(|| {
    Mutex::new(Index::new(
        roots(),
        THEME.get().map_or("hicolor", String::as_str).to_owned(),
        PathBuf::from("/usr/share/pixmaps"),
    ))
});
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
    let data_dirs =
        env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".to_owned());
    home.map(|home| home.join(".icons"))
        .into_iter()
        .chain(data_home.map(|dir| dir.join("icons")))
        .chain(
            data_dirs
                .split(':')
                .filter(|dir| !dir.is_empty())
                .map(|dir| Path::new(dir).join("icons")),
        )
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

#[derive(Clone, Copy, PartialEq, Eq)]
struct Stamp {
    device: u64,
    inode: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

impl Stamp {
    fn read(path: &Path) -> Option<Self> {
        let metadata = fs::metadata(path).ok()?;
        Some(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        })
    }
}

#[derive(Default)]
struct Directory {
    stamp: Option<Stamp>,
    children: Vec<PathBuf>,
}

#[derive(Default)]
struct Catalog {
    initialized: bool,
    index: Option<Stamp>,
    declared: bool,
    directories: Vec<PathBuf>,
    tree: HashMap<PathBuf, Directory>,
}

impl Catalog {
    fn directories(&mut self, root: &Path) -> &[PathBuf] {
        let index_path = root.join("index.theme");
        let stamp = Stamp::read(&index_path);
        if !self.initialized || self.index != stamp {
            self.initialized = true;
            self.index = stamp;
            self.directories.clear();
            self.declared = false;
            if let Ok(index) = fs::read_to_string(index_path) {
                for line in index.lines() {
                    if let Some(value) = line
                        .strip_prefix("Directories=")
                        .or_else(|| line.strip_prefix("ScaledDirectories="))
                    {
                        self.declared = true;
                        self.directories.extend(
                            value
                                .split(',')
                                .map(str::trim)
                                .filter(|dir| !dir.is_empty())
                                .map(|dir| root.join(dir)),
                        );
                    }
                }
            }
            if self.declared {
                self.directories.push(root.to_path_buf());
                self.directories.sort();
                self.directories.dedup();
                self.tree.clear();
            }
        }
        if !self.declared {
            self.directories.clear();
            collect_directories(
                root,
                &mut self.tree,
                &mut HashSet::new(),
                &mut self.directories,
            );
            self.directories.sort();
            self.tree
                .retain(|path, _| self.directories.binary_search(path).is_ok());
        }
        &self.directories
    }
}

fn collect_directories(
    path: &Path,
    tree: &mut HashMap<PathBuf, Directory>,
    seen: &mut HashSet<(u64, u64)>,
    directories: &mut Vec<PathBuf>,
) {
    let Some(stamp) = Stamp::read(path) else {
        return;
    };
    if !seen.insert((stamp.device, stamp.inode)) {
        return;
    }
    directories.push(path.to_path_buf());
    let directory = tree.entry(path.to_path_buf()).or_default();
    if directory.stamp != Some(stamp) {
        directory.stamp = Some(stamp);
        directory.children.clear();
        if let Ok(entries) = fs::read_dir(path) {
            directory
                .children
                .extend(entries.flatten().filter_map(|entry| {
                    let kind = entry.file_type().ok()?;
                    (kind.is_dir() || (kind.is_symlink() && entry.path().is_dir()))
                        .then(|| entry.path())
                }));
        }
        directory.children.sort();
    }
    let children = std::mem::take(&mut directory.children);
    for child in &children {
        collect_directories(child, tree, seen, directories);
    }
    tree.get_mut(path).unwrap().children = children;
}

const MAX_SUCCESSES: usize = 2048;
const MAX_CATALOGS: usize = 128;

struct Index {
    roots: Vec<PathBuf>,
    theme: String,
    pixmaps: PathBuf,
    root_stamps: Vec<Option<Stamp>>,
    catalogs: HashMap<PathBuf, Catalog>,
    successes: HashMap<String, PathBuf>,
    pixmap_stamp: Option<Stamp>,
    pixmap_extensions: Vec<Option<std::ffi::OsString>>,
}

impl Index {
    fn new(roots: Vec<PathBuf>, theme: String, pixmaps: PathBuf) -> Self {
        let root_stamps = roots
            .iter()
            .chain([&pixmaps])
            .map(|root| Stamp::read(root))
            .collect();
        Self {
            roots,
            theme,
            pixmaps,
            root_stamps,
            catalogs: HashMap::new(),
            successes: HashMap::new(),
            pixmap_stamp: None,
            pixmap_extensions: Vec::new(),
        }
    }

    fn refresh_roots(&mut self) {
        let mut changed = false;
        for (root, previous) in self
            .roots
            .iter()
            .chain([&self.pixmaps])
            .zip(&mut self.root_stamps)
        {
            let stamp = Stamp::read(root);
            if stamp != *previous {
                *previous = stamp;
                changed = true;
            }
        }
        if changed {
            self.successes.clear();
        }
    }

    fn in_directory(&mut self, root: &Path, name: &str) -> Option<(u32, PathBuf)> {
        if !self.catalogs.contains_key(root) {
            if self.catalogs.len() == MAX_CATALOGS {
                self.catalogs.clear();
            }
            self.catalogs.insert(root.to_path_buf(), Catalog::default());
        }
        let catalog = self.catalogs.get_mut(root).unwrap();
        let svg_name = format!("{name}.svg");
        let png_name = format!("{name}.png");
        let mut candidate = PathBuf::new();
        let mut best: Option<(u32, PathBuf)> = None;
        for directory in catalog.directories(root) {
            candidate.clone_from(directory);
            candidate.push(&svg_name);
            if candidate.is_file() {
                return Some((0, candidate));
            }
            candidate.set_file_name(&png_name);
            let rank = score(&candidate);
            if best.as_ref().is_none_or(|(current, _)| rank < *current) && candidate.is_file() {
                best = Some((rank, candidate.clone()));
            }
        }
        best
    }

    fn in_pixmaps(&mut self, name: &str) -> Option<PathBuf> {
        let stamp = Stamp::read(&self.pixmaps);
        if stamp != self.pixmap_stamp {
            self.pixmap_stamp = stamp;
            self.pixmap_extensions.clear();
            if let Ok(entries) = fs::read_dir(&self.pixmaps) {
                self.pixmap_extensions.extend(
                    entries
                        .flatten()
                        .map(|entry| entry.path().extension().map(ToOwned::to_owned)),
                );
                self.pixmap_extensions.sort();
                self.pixmap_extensions.dedup();
            }
        }
        self.pixmap_extensions.iter().find_map(|extension| {
            let mut filename = std::ffi::OsString::from(name);
            if let Some(extension) = extension {
                filename.push(".");
                filename.push(extension);
            }
            let path = self.pixmaps.join(filename);
            path.exists().then_some(path)
        })
    }

    fn lookup(&mut self, name: &str) -> Option<PathBuf> {
        let path = Path::new(name);
        if name.is_empty() {
            return None;
        }
        if path.is_absolute() {
            return path.exists().then(|| path.to_path_buf());
        }
        if path.file_name() != Some(std::ffi::OsStr::new(name)) {
            return None;
        }
        self.refresh_roots();
        if let Some(logical) = self.successes.get(name) {
            // Retain logical paths so Nix generation switches change the image asset key.
            if let Ok(current) = fs::canonicalize(logical) {
                return Some(current);
            }
            self.successes.remove(name);
        }
        let mut found = None;
        for theme in chain(&self.roots, &self.theme) {
            let mut best: Option<(u32, PathBuf)> = None;
            for index in 0..self.roots.len() {
                let directory = self.roots[index].join(&theme);
                if let Some(candidate) = self.in_directory(&directory, name) {
                    if best.as_ref().is_none_or(|(rank, _)| candidate.0 < *rank) {
                        best = Some(candidate);
                    }
                    if best.as_ref().is_some_and(|(rank, _)| *rank == 0) {
                        break;
                    }
                }
            }
            if let Some((_, path)) = best {
                found = Some(path);
                break;
            }
        }
        let logical = found.or_else(|| self.in_pixmaps(name))?;
        let current = fs::canonicalize(&logical).ok()?;
        if self.successes.len() == MAX_SUCCESSES {
            self.successes.clear();
        }
        self.successes.insert(name.to_owned(), logical);
        Some(current)
    }
}

pub fn lookup(name: &str) -> Option<PathBuf> {
    INDEX.lock().ok()?.lookup(name)
}

pub fn lookup_in(dir: &str, name: &str) -> Option<PathBuf> {
    let mut index = INDEX.lock().ok()?;
    if !dir.is_empty() && Path::new(name).file_name() == Some(std::ffi::OsStr::new(name)) {
        if let Some((_, path)) = index.in_directory(Path::new(dir), name) {
            if let Ok(path) = fs::canonicalize(path) {
                return Some(path);
            }
        }
    }
    index.lookup(name)
}

pub fn is_svg(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "svg")
}

#[cfg(test)]
#[path = "icons_test.rs"]
mod tests;
