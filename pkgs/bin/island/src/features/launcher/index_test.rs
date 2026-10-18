use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use super::*;

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "island-index-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn dir(&self, path: &str) -> PathBuf {
        let path = self.0.join(path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn app(path: &Path, name: &str, exec: &str) {
        fs::write(
            path,
            format!("[Desktop Entry]\nType=Application\nName={name}\nExec={exec}\n"),
        )
        .unwrap();
    }

    fn switch(link: &Path, target: &Path) {
        let pending = link.with_extension("pending");
        symlink(target, &pending).unwrap();
        fs::rename(pending, link).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

struct Running {
    index: Index,
    requests: Receiver<()>,
}

impl Running {
    fn new(dirs: Vec<PathBuf>) -> Self {
        let (refresh, requests) = Refresh::channel();
        let mut index = Index::new(dirs, refresh).unwrap();
        index.scan().unwrap();
        Self { index, requests }
    }

    fn observe(&mut self, expected: &[(&str, &str)]) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            self.requests
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("application change did not wake the index");
            if let Some(apps) = self.index.scan().unwrap() {
                let actual: Vec<_> = apps
                    .iter()
                    .map(|app| (app.name.as_str(), app.exec.as_str()))
                    .collect();
                if actual == expected {
                    return;
                }
                assert!(
                    Instant::now() < deadline,
                    "expected {expected:?}, got {actual:?}"
                );
            }
        }
    }
}

#[test]
fn installed_edited_and_removed_apps_update_without_restarting() {
    let fixture = Fixture::new();
    let apps = fixture.dir("applications");
    let mut running = Running::new(vec![apps.clone()]);
    let entry = apps.join("example.desktop");
    Fixture::app(&entry, "Example", "example");
    running.observe(&[("Example", "example")]);
    Fixture::app(&entry, "Renamed", "example --new-command");
    running.observe(&[("Renamed", "example --new-command")]);
    fs::remove_file(entry).unwrap();
    running.observe(&[]);
}

#[test]
fn removing_a_hidden_user_override_reveals_the_system_app() {
    let fixture = Fixture::new();
    let user = fixture.dir("user/applications");
    let system = fixture.dir("system/applications");
    Fixture::app(&system.join("example.desktop"), "System", "system-app");
    let hidden = user.join("example.desktop");
    fs::write(&hidden, "[Desktop Entry]\nHidden=true\n").unwrap();
    let mut running = Running::new(vec![user, system]);
    assert!(running.index.apps.as_ref().unwrap().is_empty());
    fs::remove_file(hidden).unwrap();
    running.observe(&[("System", "system-app")]);
}

#[test]
fn nixos_system_and_static_links_switch_without_user_nix_profiles() {
    let fixture = Fixture::new();
    let run = fixture.dir("run");
    let per_user = fixture.dir("etc/profiles/per-user");
    for generation in ["one", "two"] {
        let system = fixture.dir(&format!("store/system-{generation}"));
        let system_apps = fixture.dir(&format!("store/packages-{generation}/share/applications"));
        Fixture::app(
            &system_apps.join("system.desktop"),
            &format!("System {generation}"),
            "system-app",
        );
        symlink(
            system_apps.parent().unwrap().parent().unwrap(),
            system.join("sw"),
        )
        .unwrap();
        let users = fixture.dir(&format!("store/etc-{generation}/etc/profiles/per-user"));
        let user_apps = fixture.dir(&format!("store/user-{generation}/share/applications"));
        Fixture::app(
            &user_apps.join("user.desktop"),
            &format!("User {generation}"),
            "user-app",
        );
        symlink(
            user_apps.parent().unwrap().parent().unwrap(),
            users.join("test"),
        )
        .unwrap();
    }
    symlink("../../static/profiles/per-user/test", per_user.join("test")).unwrap();
    let current = run.join("current-system");
    let static_link = fixture.0.join("etc/static");
    symlink("../store/system-one", &current).unwrap();
    symlink("../store/etc-one/etc", &static_link).unwrap();
    let mut running = Running::new(vec![
        per_user.join("test/share/applications"),
        current.join("sw/share/applications"),
    ]);
    let initial: Vec<_> = running
        .index
        .apps
        .as_ref()
        .unwrap()
        .iter()
        .map(|app| app.name.as_str())
        .collect();
    assert_eq!(initial, ["System one", "User one"]);
    Fixture::switch(&static_link, Path::new("../store/etc-two/etc"));
    running.observe(&[("System one", "system-app"), ("User two", "user-app")]);
    Fixture::switch(&current, Path::new("../store/system-two"));
    running.observe(&[("System two", "system-app"), ("User two", "user-app")]);
}

#[test]
fn absent_and_recreated_application_directories_are_monitored() {
    let fixture = Fixture::new();
    let apps = fixture.0.join("data/applications");
    let mut running = Running::new(vec![apps.clone()]);
    fs::create_dir_all(&apps).unwrap();
    Fixture::app(&apps.join("first.desktop"), "First", "first");
    running.observe(&[("First", "first")]);
    fs::remove_dir_all(&apps).unwrap();
    running.observe(&[]);
    fs::create_dir(&apps).unwrap();
    Fixture::app(&apps.join("second.desktop"), "Second", "second");
    running.observe(&[("Second", "second")]);
    Fixture::app(&apps.join("second.desktop"), "Changed", "changed");
    running.observe(&[("Changed", "changed")]);
}

#[test]
fn linked_desktop_targets_can_appear_change_and_be_replaced_atomically() {
    let fixture = Fixture::new();
    let apps = fixture.dir("applications");
    let external = fixture.dir("external");
    let target = external.join("source");
    symlink(&target, apps.join("linked.desktop")).unwrap();
    let mut running = Running::new(vec![apps.clone()]);
    Fixture::app(&target, "Linked", "linked");
    running.observe(&[("Linked", "linked")]);
    Fixture::app(&target, "Edited", "edited");
    running.observe(&[("Edited", "edited")]);
    let replacement = external.join("replacement");
    Fixture::app(&replacement, "Replacement", "replacement");
    Fixture::switch(&apps.join("linked.desktop"), &replacement);
    running.observe(&[("Replacement", "replacement")]);
}

#[test]
fn unrelated_parent_events_do_not_request_an_index_refresh() {
    let fixture = Fixture::new();
    let apps = fixture.dir("applications");
    let running = Running::new(vec![apps]);
    fs::write(fixture.0.join("unrelated"), "not an application").unwrap();
    assert_eq!(
        running.requests.recv_timeout(Duration::from_millis(80)),
        Err(mpsc::RecvTimeoutError::Timeout)
    );
}
