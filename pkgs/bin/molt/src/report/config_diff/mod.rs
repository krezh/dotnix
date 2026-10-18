//! Generated configuration files that differ between the active and the reviewed closure: a
//! NixOS system's `etc` plus each Home Manager user's `home-files`, or a standalone Home Manager
//! generation's `home-files`.
//!
//! Store paths are normalised before comparing, so a file that only points at rebuilt
//! dependencies is set aside instead of listed as a configuration change.

mod diff;

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::build::Target;
use crate::report::ChangeStatus;
use diff::line_diff;

const STORE: &str = "/nix/store";
const STORE_PREFIX: &[u8] = b"/nix/store/";
const STORE_HASH_LENGTH: usize = 32;
/// Shown in place of a store hash, so `/nix/store/<hash>-foo` reads `/nix/store/…-foo`.
const HASH_PLACEHOLDER: &str = "…";
/// `etc` entries that are package content dix already reports.
const ETC_SKIP: &[&str] = &["profiles"];
/// Larger files are compared and listed, but not diffed.
const MAX_TEXT_BYTES: u64 = 4 * 1024 * 1024;
const COMPARE_CHUNK_BYTES: usize = 64 * 1024;
const MAX_DEPTH: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DiffLineKind {
    Hunk,
    Context,
    Removed,
    Added,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DiffLine {
    pub(crate) kind: DiffLineKind,
    pub(crate) text: String,
}

#[derive(Clone, Debug)]
pub(crate) enum ConfigContent {
    Text {
        lines: Vec<DiffLine>,
        added: usize,
        removed: usize,
    },
    Binary,
}

#[derive(Clone, Debug)]
pub(crate) struct ConfigChange {
    /// `Added`, `Removed`, or `Changed`.
    pub(crate) status: ChangeStatus,
    /// Where the file is installed: `/etc/nix/nix.conf`, `~/.config/ghostty/config`.
    pub(crate) path: String,
    /// Name of the store path the file resolves to, without its hash: `slumber-config.toml`. dix
    /// lists a generated file under this name as a package without a version.
    pub(crate) store_name: Option<String>,
    pub(crate) content: ConfigContent,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ConfigDiff {
    pub(crate) changes: Vec<ConfigChange>,
    /// Files whose only difference is the store paths they mention.
    pub(crate) store_path_only: Vec<String>,
}

pub(crate) fn compare(
    target: Target,
    old: &Path,
    new: &Path,
    cancellation: &AtomicBool,
) -> Result<ConfigDiff, String> {
    let mut diff = ConfigDiff::default();
    for root in roots(target, old, new) {
        let mut walk = Walk {
            cancellation,
            skip: root.skip,
            diff: &mut diff,
        };
        walk.pair(root.old.as_deref(), root.new.as_deref(), &root.display, 0)?;
    }
    Ok(diff)
}

struct Root {
    display: String,
    old: Option<PathBuf>,
    new: Option<PathBuf>,
    skip: &'static [&'static str],
}

fn roots(target: Target, old: &Path, new: &Path) -> Vec<Root> {
    match target {
        Target::HomeManager => vec![Root {
            display: "~".to_owned(),
            old: Some(old.join("home-files")),
            new: Some(new.join("home-files")),
            skip: &[],
        }],
        Target::NixOs => {
            let mut roots = vec![Root {
                display: "/etc".to_owned(),
                old: Some(old.join("etc")),
                new: Some(new.join("etc")),
                skip: ETC_SKIP,
            }];
            let old_homes = home_generations(old);
            let mut new_homes = home_generations(new);
            let current_user = std::env::var("USER").ok();
            let mut users = old_homes.keys().cloned().collect::<Vec<_>>();
            users.extend(
                new_homes
                    .keys()
                    .filter(|user| !old_homes.contains_key(*user))
                    .cloned(),
            );
            users.sort();
            for user in users {
                let display = if current_user.as_deref() == Some(user.as_str()) {
                    "~".to_owned()
                } else {
                    format!("~{user}")
                };
                roots.push(Root {
                    display,
                    old: old_homes
                        .get(&user)
                        .map(|generation| generation.join("home-files")),
                    new: new_homes
                        .remove(&user)
                        .map(|generation| generation.join("home-files")),
                    skip: &[],
                });
            }
            roots
        }
    }
}

/// Home Manager generations a NixOS system activates, by user, read from the
/// `home-manager-<user>.service` units the Home Manager NixOS module installs.
fn home_generations(system: &Path) -> BTreeMap<String, PathBuf> {
    let mut generations = BTreeMap::new();
    let Ok(entries) = fs::read_dir(system.join("etc/systemd/system")) else {
        return generations;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("home-manager-") || !name.ends_with(".service") {
            continue;
        }
        let Ok(unit) = fs::read_to_string(entry.path()) else {
            continue;
        };
        if let Some((user, generation)) = home_unit(&unit) {
            generations.insert(user, generation);
        }
    }
    generations
}

fn home_unit(unit: &str) -> Option<(String, PathBuf)> {
    let user = unit
        .lines()
        .find_map(|line| line.strip_prefix("User="))?
        .trim()
        .to_owned();
    let generation = unit
        .lines()
        .filter_map(|line| line.strip_prefix("ExecStart="))
        .flat_map(str::split_whitespace)
        .find(|word| word.starts_with('/') && word.ends_with("-home-manager-generation"))?;
    Some((user, PathBuf::from(generation)))
}

enum Node {
    Directory,
    File(u64),
    /// A symlink leaving the store or pointing nowhere: compared by its target, not followed.
    Link(String),
}

fn node(path: &Path) -> Option<Node> {
    let link = fs::symlink_metadata(path).ok()?;
    if link.file_type().is_symlink() {
        match fs::canonicalize(path) {
            Ok(target) if target.starts_with(STORE) => {}
            Ok(target) => return Some(Node::Link(target.display().to_string())),
            Err(_) => {
                let target = fs::read_link(path).ok()?;
                return Some(Node::Link(target.display().to_string()));
            }
        }
    }
    let metadata = fs::metadata(path).ok()?;
    Some(if metadata.is_dir() {
        Node::Directory
    } else {
        Node::File(metadata.len())
    })
}

struct Walk<'a> {
    cancellation: &'a AtomicBool,
    skip: &'static [&'static str],
    diff: &'a mut ConfigDiff,
}

impl Walk<'_> {
    fn pair(
        &mut self,
        old: Option<&Path>,
        new: Option<&Path>,
        display: &str,
        depth: usize,
    ) -> Result<(), String> {
        if self.cancellation.load(Ordering::Relaxed) {
            return Err(molt::command::CANCELLED.to_owned());
        }
        if depth > MAX_DEPTH {
            return Ok(());
        }
        if let (Some(old), Some(new)) = (old, new)
            && let (Ok(old), Ok(new)) = (fs::canonicalize(old), fs::canonicalize(new))
            && old == new
        {
            return Ok(());
        }
        let old_node = old.and_then(node);
        let new_node = new.and_then(node);
        match (&old_node, &new_node) {
            (None, None) => Ok(()),
            (Some(Node::Directory), Some(Node::Directory)) => {
                self.directory(old, new, display, depth)
            }
            (Some(Node::Directory), _) => {
                self.directory(old, None, display, depth)?;
                self.leaf(None, new.zip(new_node), display);
                Ok(())
            }
            (_, Some(Node::Directory)) => {
                self.leaf(old.zip(old_node), None, display);
                self.directory(None, new, display, depth)
            }
            _ => {
                self.leaf(old.zip(old_node), new.zip(new_node), display);
                Ok(())
            }
        }
    }

    fn directory(
        &mut self,
        old: Option<&Path>,
        new: Option<&Path>,
        display: &str,
        depth: usize,
    ) -> Result<(), String> {
        let mut names = std::collections::BTreeSet::new();
        for directory in [old, new].into_iter().flatten() {
            if let Ok(entries) = fs::read_dir(directory) {
                names.extend(entries.flatten().map(|entry| entry.file_name()));
            }
        }
        for name in names {
            if depth == 0 && self.skip.iter().any(|skip| name == *skip) {
                continue;
            }
            let child = format!("{display}/{}", name.to_string_lossy());
            let old = old.map(|old| old.join(&name));
            let new = new.map(|new| new.join(&name));
            self.pair(old.as_deref(), new.as_deref(), &child, depth + 1)?;
        }
        Ok(())
    }

    fn leaf(&mut self, old: Option<(&Path, Node)>, new: Option<(&Path, Node)>, display: &str) {
        let store_name = new
            .as_ref()
            .or(old.as_ref())
            .and_then(|(path, _)| fs::canonicalize(path).ok())
            .and_then(|path| store_name(&path));
        let (old, new) = (old.map(contents), new.map(contents));
        let status = match (&old, &new) {
            (Some(_), Some(_)) => ChangeStatus::Changed,
            (None, Some(_)) => ChangeStatus::Added,
            (Some(_), None) => ChangeStatus::Removed,
            (None, None) => return,
        };
        let text = |contents: &Option<Contents>| match contents {
            None => Some(String::new()),
            Some(Contents::Text(text)) => Some(normalize(text)),
            Some(Contents::Binary(_) | Contents::Large(..) | Contents::Directory) => None,
        };
        let content = match (text(&old), text(&new)) {
            (Some(old_text), Some(new_text)) => {
                if status == ChangeStatus::Changed && old_text == new_text {
                    if old != new {
                        self.diff.store_path_only.push(display.to_owned());
                    }
                    return;
                }
                let (lines, added, removed) = line_diff(&old_text, &new_text);
                ConfigContent::Text {
                    lines,
                    added,
                    removed,
                }
            }
            _ => {
                let unchanged = match (&old, &new) {
                    (Some(Contents::Binary(old)), Some(Contents::Binary(new))) => old == new,
                    (
                        Some(Contents::Large(old, old_size)),
                        Some(Contents::Large(new, new_size)),
                    ) => old_size == new_size && same_bytes(old, new),
                    (Some(Contents::Directory), Some(Contents::Directory)) => true,
                    _ => false,
                };
                if unchanged {
                    return;
                }
                ConfigContent::Binary
            }
        };
        self.diff.changes.push(ConfigChange {
            status,
            path: display.to_owned(),
            store_name,
            content,
        });
    }
}

#[derive(PartialEq, Eq)]
enum Contents<'a> {
    Text(String),
    Binary(Vec<u8>),
    /// A file over `MAX_TEXT_BYTES`, with its size: compared in chunks, never read whole.
    Large(&'a Path, u64),
    Directory,
}

fn contents((path, node): (&Path, Node)) -> Contents<'_> {
    match node {
        Node::Link(target) => Contents::Text(format!("symlink → {target}\n")),
        Node::Directory => Contents::Directory,
        Node::File(size) if size > MAX_TEXT_BYTES => Contents::Large(path, size),
        Node::File(_) => match fs::read(path) {
            Ok(bytes) if !bytes.iter().take(8192).any(|byte| *byte == 0) => {
                match String::from_utf8(bytes) {
                    Ok(text) => Contents::Text(text),
                    Err(error) => Contents::Binary(error.into_bytes()),
                }
            }
            Ok(bytes) => Contents::Binary(bytes),
            Err(error) => Contents::Text(format!("unreadable: {error}\n")),
        },
    }
}

/// Whether two files hold the same bytes, read a buffer at a time. Read errors count as a
/// difference.
fn same_bytes(old: &Path, new: &Path) -> bool {
    let (Ok(old), Ok(new)) = (fs::File::open(old), fs::File::open(new)) else {
        return false;
    };
    let mut old = BufReader::with_capacity(COMPARE_CHUNK_BYTES, old);
    let mut new = BufReader::with_capacity(COMPARE_CHUNK_BYTES, new);
    loop {
        let (Ok(old_bytes), Ok(new_bytes)) = (old.fill_buf(), new.fill_buf()) else {
            return false;
        };
        let length = old_bytes.len().min(new_bytes.len());
        if length == 0 {
            return old_bytes.is_empty() && new_bytes.is_empty();
        }
        if old_bytes[..length] != new_bytes[..length] {
            return false;
        }
        old.consume(length);
        new.consume(length);
    }
}

/// `/nix/store/<hash>-name/…` → `name`.
fn store_name(path: &Path) -> Option<String> {
    let entry = path.strip_prefix(STORE).ok()?.components().next()?;
    let entry = entry.as_os_str().to_str()?;
    let name = entry.get(STORE_HASH_LENGTH..)?.strip_prefix('-')?;
    (!name.is_empty()).then(|| name.to_owned())
}

/// Replaces every store hash with `…`, keeping the name after it.
fn normalize(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut normalized = String::with_capacity(text.len());
    let mut copied = 0;
    let mut search = 0;
    while let Some(offset) = find(&bytes[search..], STORE_PREFIX) {
        let hash = search + offset + STORE_PREFIX.len();
        let end = hash + STORE_HASH_LENGTH;
        if bytes.get(end) == Some(&b'-')
            && bytes[hash..end]
                .iter()
                .all(|byte| byte.is_ascii_digit() || byte.is_ascii_lowercase())
        {
            normalized.push_str(&text[copied..hash]);
            normalized.push_str(HASH_PLACEHOLDER);
            copied = end;
            search = end;
        } else {
            search = hash;
        }
    }
    normalized.push_str(&text[copied..]);
    normalized
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;

    use super::*;

    const HASH_A: &str = "0123456789abcdefghijklmnopqrstuv";
    const HASH_B: &str = "vutsrqponmlkjihgfedcba9876543210";

    #[test]
    fn hides_store_hashes_only() {
        assert_eq!(
            normalize(&format!("ExecStart=/nix/store/{HASH_A}-dbus-1.16/bin/dbus")),
            "ExecStart=/nix/store/…-dbus-1.16/bin/dbus"
        );
        // Not a hash: too short, wrong alphabet, or not followed by a name.
        for text in [
            "/nix/store/abc-short",
            "/nix/store/0123456789ABCDEFGHIJKLMNOPQRSTUV-upper",
            &format!("/nix/store/{HASH_A}"),
        ] {
            assert_eq!(normalize(text), text);
        }
    }

    #[test]
    fn names_the_store_path_a_file_resolves_to() {
        let name = |path: String| store_name(Path::new(&path));
        assert_eq!(
            name(format!("/nix/store/{HASH_A}-slumber-config.toml")).as_deref(),
            Some("slumber-config.toml")
        );
        assert_eq!(
            name(format!("/nix/store/{HASH_A}-etc/etc/nix/nix.conf")).as_deref(),
            Some("etc")
        );
        assert_eq!(name("/etc/nix/nix.conf".to_owned()), None);
        assert_eq!(name(format!("/nix/store/{HASH_A}")), None);
    }

    pub(super) fn texts(lines: &[DiffLine]) -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                let sign = match line.kind {
                    DiffLineKind::Hunk => "",
                    DiffLineKind::Context => " ",
                    DiffLineKind::Removed => "-",
                    DiffLineKind::Added => "+",
                };
                format!("{sign}{}", line.text)
            })
            .collect()
    }

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            static ID: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "molt-config-diff-test-{}-{}",
                std::process::id(),
                ID.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&root);
            Self(root)
        }

        fn write(&self, path: &str, contents: &str) {
            let path = self.0.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn classifies_system_and_home_files() {
        let fixture = Fixture::new();
        let unit = |generation: &str| {
            format!(
                "[Service]\nExecStart=/nix/store/{HASH_A}-hm-setup-env {generation}\nUser=alice\n"
            )
        };
        for (side, hash, size) in [("old", HASH_A, "12"), ("new", HASH_B, "13")] {
            let home = fixture.0.join(format!("{side}-home"));
            let generation = format!("{}-home-manager-generation", home.display());
            fixture.write(
                &format!("{side}/etc/systemd/system/home-manager-alice.service"),
                &unit(&generation),
            );
            fixture.write(
                &format!("{side}/etc/dbus-1/system.conf"),
                &format!("<include>/nix/store/{hash}-dbus/etc</include>\n"),
            );
            fixture.write(&format!("{side}/etc/profiles/per-user/alice/bin/x"), side);
            fixture.write(
                &format!("{side}-home-home-manager-generation/home-files/.config/ghostty/config"),
                &format!("theme = mocha\nfont-size = {size}\n"),
            );
        }
        fixture.write("new/etc/hosts", "127.0.0.1 localhost\n");
        fixture.write("old/etc/motd", "hello\n");

        let diff = compare(
            Target::NixOs,
            &fixture.0.join("old"),
            &fixture.0.join("new"),
            &AtomicBool::new(false),
        )
        .unwrap();
        let summary = diff
            .changes
            .iter()
            .map(|change| (change.status, change.path.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            summary,
            [
                (ChangeStatus::Added, "/etc/hosts"),
                (ChangeStatus::Removed, "/etc/motd"),
                (
                    ChangeStatus::Changed,
                    "/etc/systemd/system/home-manager-alice.service"
                ),
                (ChangeStatus::Changed, "~alice/.config/ghostty/config"),
            ]
        );
        assert_eq!(diff.store_path_only, ["/etc/dbus-1/system.conf"]);
        let ConfigContent::Text {
            lines,
            added,
            removed,
        } = &diff.changes[3].content
        else {
            panic!("ghostty config should be text");
        };
        assert_eq!((*added, *removed), (1, 1));
        assert_eq!(
            texts(lines),
            [
                "@@ -1,2 +1,2 @@",
                " theme = mocha",
                "-font-size = 12",
                "+font-size = 13"
            ]
        );
    }

    #[test]
    fn compares_oversized_files_by_content() {
        let fixture = Fixture::new();
        let large = "a".repeat(MAX_TEXT_BYTES as usize + 1);
        let mut changed = large.clone();
        changed.replace_range(changed.len() - 1.., "b");
        for side in ["old", "new"] {
            fixture.write(&format!("{side}/home-files/same"), &large);
        }
        fixture.write("old/home-files/changed", &large);
        fixture.write("new/home-files/changed", &changed);

        let diff = compare(
            Target::HomeManager,
            &fixture.0.join("old"),
            &fixture.0.join("new"),
            &AtomicBool::new(false),
        )
        .unwrap();
        let summary = diff
            .changes
            .iter()
            .map(|change| (change.status, change.path.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(summary, [(ChangeStatus::Changed, "~/changed")]);
        assert!(matches!(diff.changes[0].content, ConfigContent::Binary));
    }
}
