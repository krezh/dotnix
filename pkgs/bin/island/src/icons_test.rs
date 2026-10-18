use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use super::{Index, MAX_SUCCESSES};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "island-icons-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.0.join(relative)
    }

    fn file(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        path
    }

    fn index(&self, roots: &[&str], theme: &str) -> Index {
        Index::new(
            roots.iter().map(|root| self.path(root)).collect(),
            theme.to_owned(),
            self.path("pixmaps"),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn canonical(path: &Path) -> Option<PathBuf> {
    Some(fs::canonicalize(path).unwrap())
}

#[test]
fn a_miss_does_not_hide_icons_installed_in_declared_directories_later() {
    let fixture = Fixture::new();
    fixture.file(
        "icons/custom/index.theme",
        "[Icon Theme]\nDirectories=32x32/apps,scalable/apps\n",
    );
    fs::create_dir_all(fixture.path("icons/custom/32x32/apps")).unwrap();
    let mut index = fixture.index(&["icons"], "custom");

    assert_eq!(index.lookup("late"), None);
    assert!(index.successes.is_empty());
    let png = fixture.file("icons/custom/32x32/apps/late.png", "png");
    assert_eq!(index.lookup("late"), canonical(&png));

    assert_eq!(index.lookup("later"), None);
    let svg = fixture.file("icons/custom/scalable/apps/later.svg", "svg");
    assert_eq!(index.lookup("later"), canonical(&svg));
}

#[test]
fn unindexed_directories_discover_new_subdirectories_after_a_miss() {
    let fixture = Fixture::new();
    fixture.file("icons/custom/existing/old.svg", "old");
    let mut index = fixture.index(&["icons"], "custom");
    assert_eq!(index.lookup("late"), None);

    let path = fixture.file("icons/custom/new/deep/late.svg", "late");
    assert_eq!(index.lookup("late"), canonical(&path));
    assert_eq!(index.lookup("another"), None);
    let another = fixture.file("icons/custom/new/deep/another.png", "another");
    assert_eq!(index.lookup("another"), canonical(&another));
}

#[test]
fn stable_intermediate_symlink_switches_update_cached_image_paths() {
    let fixture = Fixture::new();
    let first = fixture.file(
        "generation-one/share/icons/custom/32x32/apps/same.png",
        "first",
    );
    let second = fixture.file(
        "generation-two/share/icons/custom/scalable/apps/same.svg",
        "second",
    );
    let added = fixture.file(
        "generation-two/share/icons/custom/scalable/apps/added.svg",
        "added",
    );
    let first_stable = fixture.file(
        "generation-one/share/icons/custom/scalable/apps/stable.svg",
        "first",
    );
    let second_stable = fixture.file(
        "generation-two/share/icons/custom/scalable/apps/stable.svg",
        "second",
    );
    for generation in ["generation-one", "generation-two"] {
        fixture.file(
            &format!("{generation}/share/icons/custom/index.theme"),
            "[Icon Theme]\nDirectories=32x32/apps,scalable/apps\n",
        );
    }
    symlink(
        fixture.path("generation-one"),
        fixture.path("current-system"),
    )
    .unwrap();
    let mut index = fixture.index(&["current-system/share/icons"], "custom");
    assert_eq!(index.lookup("same"), canonical(&first));
    assert_eq!(index.lookup("stable"), canonical(&first_stable));
    assert_eq!(index.lookup("added"), None);

    symlink(fixture.path("generation-two"), fixture.path("next-system")).unwrap();
    fs::rename(fixture.path("next-system"), fixture.path("current-system")).unwrap();

    assert_eq!(index.lookup("same"), canonical(&second));
    assert_eq!(index.lookup("stable"), canonical(&second_stable));
    assert_eq!(index.lookup("added"), canonical(&added));
    assert_ne!(canonical(&first), canonical(&second));
}

#[test]
fn changing_an_icon_symlink_updates_a_cached_hit_without_replacing_its_root() {
    let fixture = Fixture::new();
    let first = fixture.file("store/first.svg", "first");
    let second = fixture.file("store/second.svg", "second");
    fixture.file("icons/custom/index.theme", "Directories=scalable/apps\n");
    let logical = fixture.path("icons/custom/scalable/apps/icon.svg");
    fs::create_dir_all(logical.parent().unwrap()).unwrap();
    symlink(&first, &logical).unwrap();
    let mut index = fixture.index(&["icons"], "custom");
    assert_eq!(index.lookup("icon"), canonical(&first));

    fs::remove_file(&logical).unwrap();
    symlink(&second, &logical).unwrap();
    assert_eq!(index.lookup("icon"), canonical(&second));
}

#[test]
fn theme_precedence_beats_format_and_size_and_inherits_in_order() {
    let fixture = Fixture::new();
    fixture.file(
        "icons/custom/index.theme",
        "Inherits=first,second\nDirectories=128x128/apps\n",
    );
    let primary = fixture.file("icons/custom/128x128/apps/icon.png", "primary");
    fixture.file("icons/first/scalable/apps/icon.svg", "parent");
    let inherited = fixture.file("icons/first/128x128/apps/inherited.png", "first");
    fixture.file("icons/first/index.theme", "Inherits=custom\n");
    fixture.file("icons/second/scalable/apps/inherited.svg", "second");
    fixture.file("icons/hicolor/scalable/apps/inherited.svg", "hicolor");
    let fallback = fixture.file("icons/hicolor/scalable/apps/fallback.svg", "fallback");
    let mut index = fixture.index(&["icons"], "custom");

    assert_eq!(index.lookup("icon"), canonical(&primary));
    assert_eq!(index.lookup("inherited"), canonical(&inherited));
    assert_eq!(index.lookup("fallback"), canonical(&fallback));
}

#[test]
fn svg_and_nearest_size_beat_root_order_but_equal_ranks_keep_root_order() {
    let fixture = Fixture::new();
    fixture.file("first/custom/32x32/apps/vector.png", "first");
    let svg = fixture.file("second/custom/scalable/apps/vector.svg", "second");
    fixture.file("first/custom/128x128/apps/raster.png", "large");
    let nearest = fixture.file("second/custom/32x32/apps/raster.png", "target");
    let tie = fixture.file("first/custom/24x24/apps/tied.png", "first");
    fixture.file("second/custom/40x40/apps/tied.png", "second");
    let mut index = fixture.index(&["first", "second"], "custom");

    assert_eq!(index.lookup("vector"), canonical(&svg));
    assert_eq!(index.lookup("raster"), canonical(&nearest));
    assert_eq!(index.lookup("tied"), canonical(&tie));
}

#[test]
fn deleted_successes_are_resolved_again_and_misses_are_not_retained() {
    let fixture = Fixture::new();
    let primary = fixture.file("icons/custom/scalable/apps/icon.svg", "primary");
    let fallback = fixture.file("icons/hicolor/32x32/apps/icon.png", "fallback");
    let mut index = fixture.index(&["icons"], "custom");
    assert_eq!(index.lookup("icon"), canonical(&primary));
    fs::remove_file(primary).unwrap();
    assert_eq!(index.lookup("icon"), canonical(&fallback));
    for number in 0..100 {
        assert_eq!(index.lookup(&format!("missing-{number}")), None);
    }
    assert_eq!(index.successes.len(), 1);
}

#[test]
fn pixmaps_remain_a_late_discovered_fallback_and_absolute_paths_are_unchanged() {
    let fixture = Fixture::new();
    let mut index = fixture.index(&["icons"], "custom");
    assert_eq!(index.lookup("late"), None);
    let pixmap = fixture.file("pixmaps/late.xpm", "pixmap");
    assert_eq!(index.lookup("late"), canonical(&pixmap));

    let target = fixture.file("store/absolute.svg", "absolute");
    let logical = fixture.path("absolute.svg");
    symlink(&target, &logical).unwrap();
    assert_eq!(index.lookup(logical.to_str().unwrap()), Some(logical));
    assert_eq!(
        index.lookup(fixture.path("missing.svg").to_str().unwrap()),
        None
    );
    assert_eq!(index.lookup(""), None);
    assert_eq!(index.lookup("../absolute"), None);
}

#[test]
fn explicit_theme_directories_do_not_catalog_unrelated_files() {
    let fixture = Fixture::new();
    fixture.file(
        "icons/custom/index.theme",
        "Directories=32x32/apps\nScaledDirectories=32x32@2/apps\n",
    );
    fixture.file("icons/custom/32x32/apps/known.png", "known");
    let scaled = fixture.file("icons/custom/32x32@2/apps/scaled.png", "scaled");
    fixture.file("icons/custom/unrelated/deep/hidden.svg", "not declared");
    let mut index = fixture.index(&["icons"], "custom");
    assert_eq!(index.lookup("missing"), None);
    assert_eq!(index.lookup("scaled"), canonical(&scaled));
    let catalog = index.catalogs.get(&fixture.path("icons/custom")).unwrap();
    assert!(catalog.tree.is_empty());
    assert_eq!(catalog.directories.len(), 3);
}

#[test]
fn success_cache_has_a_fixed_upper_bound() {
    let fixture = Fixture::new();
    fixture.file("icons/custom/index.theme", "Directories=32x32/apps\n");
    let icon = fixture.file("icons/custom/32x32/apps/icon.png", "icon");
    let mut index = fixture.index(&["icons"], "custom");
    for number in 0..MAX_SUCCESSES {
        index
            .successes
            .insert(format!("cached-{number}"), icon.clone());
    }
    assert_eq!(index.lookup("icon"), canonical(&icon));
    assert!(index.successes.len() <= MAX_SUCCESSES);
}

#[test]
fn tray_lookup_keeps_custom_directory_precedence_and_discovers_late_icons() {
    let fixture = Fixture::new();
    let first = fixture.file("tray/128x128/icon.png", "first");
    let directory = fixture.path("tray");
    assert_eq!(
        super::lookup_in(directory.to_str().unwrap(), "icon"),
        canonical(&first)
    );
    let mut index = fixture.index(&[], "custom");
    assert!(
        index
            .in_directory(&directory, "late-tray-fixture")
            .is_none()
    );
    let late = fixture.file("tray/scalable/late-tray-fixture.svg", "late");
    assert_eq!(
        super::lookup_in(directory.to_str().unwrap(), "late-tray-fixture"),
        canonical(&late),
    );
    let vector = fixture.file("tray/scalable/icon.svg", "vector");
    assert_eq!(
        super::lookup_in(directory.to_str().unwrap(), "icon"),
        canonical(&vector)
    );
}

#[test]
fn unindexed_directory_symlink_cycles_are_ignored() {
    let fixture = Fixture::new();
    let icon = fixture.file("icons/custom/32x32/apps/icon.png", "icon");
    symlink(
        fixture.path("icons/custom"),
        fixture.path("icons/custom/32x32/apps/cycle"),
    )
    .unwrap();
    let mut index = fixture.index(&["icons"], "custom");
    assert_eq!(index.lookup("absent"), None);
    assert_eq!(index.lookup("icon"), canonical(&icon));
}
