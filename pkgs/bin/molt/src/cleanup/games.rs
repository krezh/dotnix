//! What Steam and Proton leave behind: shader caches and Proton prefixes of games no longer
//! installed, and Proton builds nothing is set to use.
//!
//! Everything comes from Steam's own files, offline: `libraryfolders.vdf` for the libraries, app
//! manifests for what is installed, `appinfo.vdf` for names, `config.vdf` for the Proton build
//! each game runs with. Other launchers' settings (Faugus, Heroic, Lutris) count as uses of a
//! Proton build too.
//!
//! Prefixes are trimmed rather than removed: Windows, redistributables, and Proton's own files go,
//! while `drive_c/users`, `drive_c/ProgramData`, and the user registry stay, since games keep
//! saves and settings there. Proton rebuilds the rest around them if the game is installed again.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use super::Freed;
use super::filesystem::{directory_size, display_path, home, remove_counted};
use super::model::{CleanupGroup, CleanupItem, CleanupKind};

/// What a trimmed prefix keeps, relative to `compatdata/<appid>`.
const KEPT: [&str; 3] = ["pfx/drive_c/users", "pfx/drive_c/ProgramData", "pfx/user.reg"];

/// Proton builds umu-launcher downloads, updates, and removes on its own.
const UMU_PROTON: &str = "UMU-Proton";

const APPINFO_28: u32 = 0x0756_4428;
const APPINFO_29: u32 = 0x0756_4429;

/// A Steam installation: its root, its libraries, and the apps installed in them.
struct Steam {
    root: PathBuf,
    libraries: Vec<PathBuf>,
    installed: HashSet<u32>,
}

impl Steam {
    fn find() -> Option<Self> {
        let home = home()?;
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"));
        [data.join("Steam"), home.join(".steam/steam")]
            .into_iter()
            .find(|root| root.join("steamapps").is_dir())
            .and_then(|root| fs::canonicalize(root).ok())
            .map(Self::at)
    }

    fn at(root: PathBuf) -> Self {
        let mut libraries = vec![root.clone()];
        if let Ok(text) = fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) {
            let tokens = vdf_tokens(&text);
            for pair in tokens.windows(2) {
                if pair[0] == "path"
                    && let Ok(library) = fs::canonicalize(&pair[1])
                    && !libraries.contains(&library)
                {
                    libraries.push(library);
                }
            }
        }
        let installed = libraries
            .iter()
            .flat_map(|library| app_entries(&library.join("steamapps"), "appmanifest_", ".acf"))
            .map(|(appid, _)| appid)
            .collect();
        Self {
            root,
            libraries,
            installed,
        }
    }

    /// Each library's `steamapps/<directory>`.
    fn library_dirs(&self, directory: &str) -> impl Iterator<Item = PathBuf> + '_ {
        let directory = directory.to_owned();
        self.libraries
            .iter()
            .map(move |library| library.join("steamapps").join(&directory))
    }

    fn compat_tools(&self) -> PathBuf {
        self.root.join("compatibilitytools.d")
    }
}

/// The Steam libraries, for the disk usage of the filesystems this cleanup frees space on.
pub(super) fn library_roots() -> Vec<PathBuf> {
    Steam::find().map(|steam| steam.libraries).unwrap_or_default()
}

pub(super) fn scan_leftovers(cancellation: &AtomicBool) -> CleanupGroup {
    let (Some(steam), Some(home)) = (Steam::find(), home()) else {
        return CleanupGroup::unavailable(CleanupKind::Games, "Steam is not installed");
    };
    let names = app_names(&steam.root.join("appcache/appinfo.vdf"));
    let mut items = Vec::new();
    for shaders in steam.library_dirs("shadercache") {
        for (appid, path) in app_entries(&shaders, "", "") {
            if steam.installed.contains(&appid) {
                continue;
            }
            let label = format!("Shader cache · {}", app_name(&names, appid));
            items.extend(item(path, &home, label, cancellation));
        }
    }
    let uses = proton_uses(&steam.root, &home);
    for tool in unused_tools(&steam.compat_tools(), &uses) {
        let label = format!("Proton · {}", file_name(&tool));
        items.extend(item(tool, &home, label, cancellation));
    }
    group(
        CleanupKind::Games,
        items,
        "Shader caches rebuild when a game is installed again; Proton builds a game, Steam, Faugus, Heroic, or Lutris is set to use are kept",
    )
}

pub(super) fn scan_prefixes(cancellation: &AtomicBool) -> CleanupGroup {
    let (Some(steam), Some(home)) = (Steam::find(), home()) else {
        return CleanupGroup::unavailable(CleanupKind::GamePrefixes, "Steam is not installed");
    };
    let names = app_names(&steam.root.join("appcache/appinfo.vdf"));
    let mut items = Vec::new();
    for prefixes in steam.library_dirs("compatdata") {
        for (appid, prefix) in app_entries(&prefixes, "", "") {
            // Only Steam's own apps: non-Steam shortcuts and Steam's shared prefix 0 are not in
            // the app cache, so their prefixes are never mistaken for an uninstalled game's.
            let Some(name) = names.get(&appid) else {
                continue;
            };
            if steam.installed.contains(&appid) {
                continue;
            }
            let targets = trim_targets(&prefix);
            if targets.is_empty() {
                continue;
            }
            let Some(bytes) = paths_size(&targets, cancellation) else {
                continue;
            };
            items.push(CleanupItem {
                label: name.clone(),
                detail: display_path(&prefix, &home),
                bytes: Some(bytes),
                path: Some(prefix),
            });
        }
    }
    group(
        CleanupKind::GamePrefixes,
        items,
        "Prefixes are trimmed: drive_c/users, drive_c/ProgramData, and user.reg stay, keeping saves and settings",
    )
}

pub(super) fn clean_leftovers(
    group: &CleanupGroup,
    cancellation: &AtomicBool,
    freed: &Freed,
) -> Result<u64, String> {
    let steam = Steam::find().ok_or("Steam is not installed")?;
    let home = home().ok_or("HOME is unavailable")?;
    clean_leftovers_in(&steam, &home, group, cancellation, freed)
}

pub(super) fn clean_prefixes(
    group: &CleanupGroup,
    cancellation: &AtomicBool,
    freed: &Freed,
) -> Result<u64, String> {
    let steam = Steam::find().ok_or("Steam is not installed")?;
    clean_prefixes_in(&steam, group, cancellation, freed)
}

fn clean_leftovers_in(
    steam: &Steam,
    home: &Path,
    group: &CleanupGroup,
    cancellation: &AtomicBool,
    freed: &Freed,
) -> Result<u64, String> {
    // Checked again now: a game may have been installed, or a build put to use, since the scan.
    let shader_dirs = steam.library_dirs("shadercache").collect::<Vec<_>>();
    let tools = steam.compat_tools();
    let unused = unused_tools(&tools, &proton_uses(&steam.root, home));
    let mut removal = Removal::new(freed);
    for item in &group.items {
        if cancellation.load(Ordering::Relaxed) {
            return Err(molt::command::CANCELLED.to_owned());
        }
        let Some(path) = item.path.as_deref() else {
            continue;
        };
        let parent = path.parent().unwrap_or(path);
        if shader_dirs.iter().any(|dir| dir == parent) {
            // A shader cache, unless its game was installed again since the scan.
            if appid(path).is_some_and(|appid| !steam.installed.contains(&appid)) {
                removal.remove(path, item.bytes.unwrap_or(0));
            }
        } else if parent == tools {
            // A Proton build, unless something was set to use it since the scan.
            if unused.iter().any(|tool| tool == path) {
                removal.remove(path, item.bytes.unwrap_or(0));
            }
        } else {
            removal.refuse(path);
        }
    }
    removal.finish()
}

fn clean_prefixes_in(
    steam: &Steam,
    group: &CleanupGroup,
    cancellation: &AtomicBool,
    freed: &Freed,
) -> Result<u64, String> {
    let prefix_dirs = steam.library_dirs("compatdata").collect::<Vec<_>>();
    let mut removal = Removal::new(freed);
    for item in &group.items {
        if cancellation.load(Ordering::Relaxed) {
            return Err(molt::command::CANCELLED.to_owned());
        }
        let Some(prefix) = item.path.as_deref() else {
            continue;
        };
        if !prefix
            .parent()
            .is_some_and(|parent| prefix_dirs.iter().any(|dir| dir == parent))
        {
            removal.refuse(prefix);
            continue;
        }
        // Reinstalled since the scan: its prefix is in use again.
        if appid(prefix).is_none_or(|appid| steam.installed.contains(&appid)) {
            continue;
        }
        for target in trim_targets(prefix) {
            let bytes = paths_size(std::slice::from_ref(&target), cancellation).unwrap_or(0);
            removal.remove(&target, bytes);
        }
    }
    removal.finish()
}

/// Bytes removed, and what failed.
struct Removal<'a> {
    freed: &'a Freed,
    removed: u64,
    count: usize,
    failures: Vec<String>,
}

impl<'a> Removal<'a> {
    fn new(freed: &'a Freed) -> Self {
        Self {
            freed,
            removed: 0,
            count: 0,
            failures: Vec::new(),
        }
    }

    fn remove(&mut self, path: &Path, bytes: u64) {
        self.count += 1;
        match remove_counted(path, bytes, self.freed) {
            Ok(()) => self.removed = self.removed.saturating_add(bytes),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => self.failures.push(format!("{}: {error}", path.display())),
        }
    }

    fn refuse(&mut self, path: &Path) {
        self.count += 1;
        self.failures
            .push(format!("refused path outside the Steam libraries: {}", path.display()));
    }

    fn finish(self) -> Result<u64, String> {
        match self.failures.first() {
            None => Ok(self.removed),
            Some(first) => Err(format!(
                "removed {} item(s), but {} failed: {first}",
                self.count - self.failures.len(),
                self.failures.len()
            )),
        }
    }
}

/// Everything in `prefix` except [`KEPT`] and the directories leading to it. Symlinks are
/// removed themselves, never followed.
fn trim_targets(prefix: &Path) -> Vec<PathBuf> {
    let kept = KEPT.map(|relative| prefix.join(relative));
    let mut targets = Vec::new();
    let mut pending = vec![prefix.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if kept.contains(&path) {
                continue;
            }
            let is_directory = entry.file_type().is_ok_and(|kind| kind.is_dir());
            if is_directory && kept.iter().any(|keep| keep.starts_with(&path)) {
                pending.push(path);
            } else {
                targets.push(path);
            }
        }
    }
    targets
}

/// Compatibility tools in `tools` that nothing in `uses` names, by directory or by the name
/// Steam knows the tool by. Symlinked tools are managed elsewhere and never listed.
fn unused_tools(tools: &Path, uses: &[String]) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(tools) else {
        return Vec::new();
    };
    let mut unused = Vec::new();
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let path = entry.path();
        let directory = file_name(&path);
        if directory.starts_with(UMU_PROTON) {
            continue;
        }
        let mut names = vec![directory];
        if let Ok(text) = fs::read_to_string(path.join("compatibilitytool.vdf")) {
            let tokens = vdf_tokens(&text);
            if let Some(index) = tokens.iter().position(|token| token == "compat_tools")
                && tokens.get(index + 1).is_some_and(|token| token == "{")
                && let Some(name) = tokens.get(index + 2)
            {
                names.push(name.clone());
            }
        }
        if !names
            .iter()
            .any(|name| uses.iter().any(|text| text.contains(name.as_str())))
        {
            unused.push(path);
        }
    }
    unused
}

/// Settings that name a Proton build: Steam's per-game and default mappings, and other launchers'
/// game settings. Matching is by plain text, so anything that mentions a build keeps it.
fn proton_uses(steam_root: &Path, home: &Path) -> Vec<String> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"));
    let mut files = vec![
        steam_root.join("config/config.vdf"),
        config.join("faugus-launcher/config.json"),
        data.join("faugus-launcher/games.json"),
        config.join("heroic/config.json"),
    ];
    for directory in [config.join("heroic/GamesConfig"), config.join("lutris/games")] {
        if let Ok(entries) = fs::read_dir(directory) {
            files.extend(entries.flatten().map(|entry| entry.path()));
        }
    }
    files
        .iter()
        .filter_map(|file| fs::read_to_string(file).ok())
        .collect()
}

/// Names of the apps in Steam's app cache, `appcache/appinfo.vdf`.
fn app_names(path: &Path) -> HashMap<u32, String> {
    fs::read(path)
        .ok()
        .and_then(|data| parse_app_names(&data))
        .unwrap_or_default()
}

/// Parses `appinfo.vdf` format 28 or 29 down to each app's `appinfo/common/name`.
fn parse_app_names(data: &[u8]) -> Option<HashMap<u32, String>> {
    let mut reader = Reader { data, at: 0 };
    let magic = reader.u32()?;
    reader.u32()?;
    // Format 29 stores every key once in a table at the end; format 28 spells keys out inline.
    let table = match magic {
        APPINFO_29 => {
            let offset = usize::try_from(reader.u64()?).ok()?;
            Some(string_table(data.get(offset..)?)?)
        }
        APPINFO_28 => None,
        _ => return None,
    };
    let mut names = HashMap::new();
    loop {
        let appid = reader.u32()?;
        if appid == 0 {
            break;
        }
        let size = usize::try_from(reader.u32()?).ok()?;
        let end = reader.at.checked_add(size)?;
        // State, update time, access token, two SHA-1 hashes, and the change number precede
        // the app's key-values.
        if let Some(name) = data
            .get(reader.at..end)
            .and_then(|entry| entry.get(60..))
            .and_then(|values| common_name(values, table.as_deref()))
        {
            names.insert(appid, name);
        }
        reader.at = end;
    }
    Some(names)
}

fn string_table(data: &[u8]) -> Option<Vec<&[u8]>> {
    let mut reader = Reader { data, at: 0 };
    let count = reader.u32()?;
    (0..count).map(|_| reader.cstr()).collect()
}

/// `appinfo/common/name` from an app's binary key-values.
fn common_name(data: &[u8], table: Option<&[&[u8]]>) -> Option<String> {
    let mut reader = Reader { data, at: 0 };
    let mut path: Vec<&[u8]> = Vec::new();
    loop {
        let kind = reader.u8()?;
        if kind == 0x08 || kind == 0x0B {
            path.pop()?;
            continue;
        }
        let key = match table {
            Some(table) => *table.get(usize::try_from(reader.u32()?).ok()?)?,
            None => reader.cstr()?,
        };
        match kind {
            0x00 => path.push(key),
            0x01 => {
                let value = reader.cstr()?;
                if key == b"name" && path == [b"appinfo".as_slice(), b"common".as_slice()] {
                    return Some(String::from_utf8_lossy(value).into_owned());
                }
            }
            0x02..=0x04 | 0x06 => reader.skip(4)?,
            0x07 | 0x0A => reader.skip(8)?,
            _ => return None,
        }
    }
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, length: usize) -> Option<&'a [u8]> {
        let bytes = self.data.get(self.at..self.at.checked_add(length)?)?;
        self.at += length;
        Some(bytes)
    }

    fn skip(&mut self, length: usize) -> Option<()> {
        self.take(length).map(|_| ())
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|bytes| bytes[0])
    }

    fn u32(&mut self) -> Option<u32> {
        self.take(4)?.try_into().ok().map(u32::from_le_bytes)
    }

    fn u64(&mut self) -> Option<u64> {
        self.take(8)?.try_into().ok().map(u64::from_le_bytes)
    }

    fn cstr(&mut self) -> Option<&'a [u8]> {
        let rest = self.data.get(self.at..)?;
        let length = rest.iter().position(|byte| *byte == 0)?;
        self.at += length + 1;
        Some(&rest[..length])
    }
}

/// Quoted strings and braces of a text VDF file, with escapes resolved and comments skipped.
fn vdf_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '"' => {
                let mut token = String::new();
                while let Some(character) = chars.next() {
                    match character {
                        '\\' => token.extend(chars.next()),
                        '"' => break,
                        character => token.push(character),
                    }
                }
                tokens.push(token);
            }
            '{' | '}' => tokens.push(character.to_string()),
            '/' if chars.peek() == Some(&'/') => {
                for character in chars.by_ref() {
                    if character == '\n' {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    tokens
}

/// Entries of `directory` named `<prefix><appid><suffix>`, with their app IDs.
fn app_entries(directory: &Path, prefix: &str, suffix: &str) -> Vec<(u32, PathBuf)> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            let appid = name
                .to_str()?
                .strip_prefix(prefix)?
                .strip_suffix(suffix)?
                .parse()
                .ok()?;
            Some((appid, entry.path()))
        })
        .collect()
}

fn appid(path: &Path) -> Option<u32> {
    path.file_name()?.to_str()?.parse().ok()
}

fn app_name(names: &HashMap<u32, String>, appid: u32) -> String {
    names
        .get(&appid)
        .cloned()
        .unwrap_or_else(|| format!("App {appid}"))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

fn item(path: PathBuf, home: &Path, label: String, cancellation: &AtomicBool) -> Option<CleanupItem> {
    let bytes = paths_size(std::slice::from_ref(&path), cancellation)?;
    Some(CleanupItem {
        label,
        detail: display_path(&path, home),
        bytes: Some(bytes),
        path: Some(path),
    })
}

/// Total size of `paths`, not following symlinks; `None` once cancelled.
fn paths_size(paths: &[PathBuf], cancellation: &AtomicBool) -> Option<u64> {
    let mut total = 0_u64;
    for path in paths {
        let Ok(metadata) = fs::symlink_metadata(path) else {
            continue;
        };
        let bytes = if metadata.is_dir() {
            directory_size(path, cancellation).ok()?
        } else if metadata.file_type().is_symlink() {
            0
        } else {
            metadata.len()
        };
        total = total.saturating_add(bytes);
    }
    Some(total)
}

fn group(kind: CleanupKind, mut items: Vec<CleanupItem>, note: &str) -> CleanupGroup {
    items.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then_with(|| left.label.cmp(&right.label))
    });
    let mut volumes: Vec<(u64, u64)> = Vec::new();
    for item in &items {
        let (Some(path), Some(bytes)) = (item.path.as_deref(), item.bytes) else {
            continue;
        };
        let Ok(metadata) = fs::metadata(path) else {
            continue;
        };
        match volumes.iter_mut().find(|(device, _)| *device == metadata.dev()) {
            Some((_, total)) => *total = total.saturating_add(bytes),
            None => volumes.push((metadata.dev(), bytes)),
        }
    }
    CleanupGroup {
        kind,
        available: true,
        reclaimable: items.iter().filter_map(|item| item.bytes).sum(),
        items,
        note: Some(note.to_owned()),
        volumes,
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;
    use std::sync::atomic::AtomicU64;

    use super::*;

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir()
                .join(format!("molt-games-test-{}-{id}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// One `appinfo.vdf` entry: the app ID and its `(type, key, value)` fields.
    type AppEntry<'a> = (u32, &'a [(u8, &'a str, &'a str)]);

    fn appinfo_29(apps: &[AppEntry<'_>]) -> Vec<u8> {
        // Keys as indices into a table written after the apps.
        let mut keys: Vec<String> = Vec::new();
        let mut key = |name: &str| -> [u8; 4] {
            let index = keys.iter().position(|known| known == name).unwrap_or_else(|| {
                keys.push(name.to_owned());
                keys.len() - 1
            });
            (index as u32).to_le_bytes()
        };
        let mut body = Vec::new();
        for (appid, values) in apps {
            let mut entry = vec![0_u8; 60];
            entry.push(0x00);
            entry.extend(key("appinfo"));
            for (kind, name, value) in *values {
                match *kind {
                    0x00 => {
                        entry.push(0x00);
                        entry.extend(key(name));
                    }
                    0x08 => entry.push(0x08),
                    0x01 => {
                        entry.push(0x01);
                        entry.extend(key(name));
                        entry.extend(value.as_bytes());
                        entry.push(0);
                    }
                    0x02 => {
                        entry.push(0x02);
                        entry.extend(key(name));
                        entry.extend(7_u32.to_le_bytes());
                    }
                    _ => unreachable!(),
                }
            }
            entry.push(0x08);
            entry.push(0x08);
            body.extend(appid.to_le_bytes());
            body.extend((entry.len() as u32).to_le_bytes());
            body.extend(entry);
        }
        body.extend(0_u32.to_le_bytes());
        let mut data = Vec::new();
        data.extend(APPINFO_29.to_le_bytes());
        data.extend(1_u32.to_le_bytes());
        data.extend(((16 + body.len()) as u64).to_le_bytes());
        data.extend(body);
        data.extend((keys.len() as u32).to_le_bytes());
        for name in keys {
            data.extend(name.as_bytes());
            data.push(0);
        }
        data
    }

    #[test]
    fn app_names_come_from_the_common_section_only() {
        let data = appinfo_29(&[
            (
                10,
                &[
                    (0x00, "common", ""),
                    (0x02, "type_id", ""),
                    (0x01, "name", "Counter-Strike"),
                    (0x08, "", ""),
                ],
            ),
            (
                20,
                &[
                    (0x00, "extended", ""),
                    (0x01, "name", "not the title"),
                    (0x08, "", ""),
                ],
            ),
        ]);
        let names = parse_app_names(&data).unwrap();
        assert_eq!(names.get(&10).map(String::as_str), Some("Counter-Strike"));
        assert!(!names.contains_key(&20));
    }

    #[test]
    fn trimming_keeps_saves_settings_and_the_user_registry() {
        let fixture = Fixture::new();
        for file in [
            "pfx/drive_c/users/steamuser/AppData/Roaming/Game/save.dat",
            "pfx/drive_c/ProgramData/Game/settings.ini",
            "pfx/drive_c/windows/system32/kernel32.dll",
            "pfx/drive_c/Program Files/redist.exe",
            "pfx/user.reg",
            "pfx/system.reg",
            "version",
        ] {
            fixture.write(file, "x");
        }
        symlink("drive_c", fixture.root.join("pfx/dosdevices")).unwrap();

        let mut targets = trim_targets(&fixture.root)
            .into_iter()
            .map(|path| path.strip_prefix(&fixture.root).unwrap().to_owned())
            .collect::<Vec<_>>();
        targets.sort();
        assert_eq!(
            targets,
            [
                "pfx/dosdevices",
                "pfx/drive_c/Program Files",
                "pfx/drive_c/windows",
                "pfx/system.reg",
                "version",
            ]
            .map(PathBuf::from)
        );
    }

    #[test]
    fn prefixes_of_reinstalled_games_and_paths_outside_libraries_are_left_alone() {
        let fixture = Fixture::new();
        for prefix in ["100", "200"] {
            fixture.write(&format!("steamapps/compatdata/{prefix}/pfx/drive_c/windows/a.dll"), "x");
            fixture.write(&format!("steamapps/compatdata/{prefix}/pfx/drive_c/users/save"), "x");
        }
        fixture.write("elsewhere/100/pfx/drive_c/windows/a.dll", "x");
        let steam = Steam {
            root: fixture.root.clone(),
            libraries: vec![fixture.root.clone()],
            // 200 was installed again after the scan.
            installed: HashSet::from([200]),
        };
        let prefix = |path: PathBuf| CleanupItem {
            label: String::new(),
            detail: String::new(),
            bytes: Some(1),
            path: Some(path),
        };
        let group = CleanupGroup {
            kind: CleanupKind::GamePrefixes,
            available: true,
            reclaimable: 3,
            items: vec![
                prefix(fixture.root.join("steamapps/compatdata/100")),
                prefix(fixture.root.join("steamapps/compatdata/200")),
                prefix(fixture.root.join("elsewhere/100")),
            ],
            note: None,
            volumes: Vec::new(),
        };

        let error = clean_prefixes_in(&steam, &group, &AtomicBool::new(false), &Freed::default())
            .unwrap_err();
        assert!(error.contains("refused"), "{error}");
        let exists = |relative: &str| fixture.root.join(relative).exists();
        assert!(!exists("steamapps/compatdata/100/pfx/drive_c/windows"));
        assert!(exists("steamapps/compatdata/100/pfx/drive_c/users/save"));
        assert!(exists("steamapps/compatdata/200/pfx/drive_c/windows/a.dll"));
        assert!(exists("elsewhere/100/pfx/drive_c/windows/a.dll"));
    }

    #[test]
    fn proton_builds_named_by_any_setting_are_kept() {
        let fixture = Fixture::new();
        let tools = fixture.root.join("compatibilitytools.d");
        fixture.write(
            "compatibilitytools.d/GE-Dir/compatibilitytool.vdf",
            "\"compatibilitytools\"\n{\n  \"compat_tools\"\n  {\n    \"GE-Proton9-27\" // internal\n    {\n    }\n  }\n}\n",
        );
        fixture.write("compatibilitytools.d/Faugus-Pick/proton", "");
        fixture.write("compatibilitytools.d/Forgotten/proton", "");
        fixture.write("compatibilitytools.d/UMU-Proton-10.0-4/proton", "");
        fixture.write("store/Linked/proton", "");
        symlink(fixture.root.join("store/Linked"), tools.join("Linked")).unwrap();
        let uses = vec![
            "\"CompatToolMapping\" { \"0\" { \"name\" \"GE-Proton9-27\" } }".to_owned(),
            "{\"runner\": \"Faugus-Pick\"}".to_owned(),
        ];

        assert_eq!(unused_tools(&tools, &uses), [tools.join("Forgotten")]);
    }
}
