use super::*;
use crate::report::flow::requires_host_switch_confirmation;

fn change(status: ChangeStatus, old: &str, new: &str) -> Change {
    Change {
        status,
        name: "demo".to_owned(),
        old: old.to_owned(),
        new: new.to_owned(),
        size: 0,
    }
}

#[test]
fn enables_changelogs_only_for_known_target_versions() {
    let upgraded = |old, new| change(ChangeStatus::Upgraded, old, new);
    assert!(has_changelog(&upgraded("1.0", "2.0")));
    assert!(has_changelog(&change(ChangeStatus::Added, "", "2.0")));
    assert!(!has_changelog(&upgraded("1.0", "")));
    assert!(!has_changelog(&change(ChangeStatus::Changed, "", "0.1.0")));
    assert!(!has_changelog(&change(
        ChangeStatus::Changed,
        "0.1.0",
        "0.1.0"
    )));
}

#[test]
fn describes_versions() {
    let plain = |status, old, new| version_text(&change(status, old, new)).1;
    assert_eq!(plain(ChangeStatus::Upgraded, "1.0", "2.0"), "1.0 → 2.0");
    assert_eq!(plain(ChangeStatus::Changed, "", "0.1.0"), "0.1.0 · rebuilt");
    assert_eq!(
        plain(ChangeStatus::Changed, "0.1.0", "0.1.0"),
        "0.1.0 · rebuilt"
    );
    assert_eq!(
        version_text(&change(ChangeStatus::Changed, "", "")).0,
        vec![("rebuilt".to_owned(), Ink::Muted)]
    );
    assert_eq!(
        version_text(&change(ChangeStatus::Upgraded, "1.0", "2.0")).0[2],
        ("2.0".to_owned(), Ink::New)
    );
    assert_eq!(plain(ChangeStatus::Added, "", ""), "no version");
}

fn joined(old: &str, new: &str) -> String {
    version_change(old, new)
        .into_iter()
        .map(|(text, _)| text)
        .collect()
}

#[test]
fn long_version_changes_show_what_they_share_once() {
    assert_eq!(joined("19.1.1", "19.2.0"), "19.1.1 → 19.2.0");
    assert_eq!(
        joined("1.15.18-unstable-2026-11-24", "1.15.19-unstable-2026-11-24"),
        "1.15.{18 → 19}-unstable-2026-11-24"
    );
    assert_eq!(
        joined("0-unstable-2026-08-28", "0-unstable-2026-11-14"),
        "0-unstable-2026-{08-28 → 11-14}"
    );
    assert_eq!(
        joined("1.0.0-nightly.20261123.102", "1.0.0-nightly.20261125.104"),
        "1.0.0-nightly.{20261123.102 → 20261125.104}"
    );
    assert_eq!(
        joined("0-unstable-9134da3", "0-unstable-789cf75"),
        "0-unstable-{9134da3 → 789cf75}"
    );
}

#[test]
fn groups_thousands() {
    assert_eq!(grouped(999), "999");
    assert_eq!(grouped(3238), "3,238");
    assert_eq!(grouped(-1_234_567), "-1,234,567");
}

#[test]
fn only_cross_host_nixos_switches_require_confirmation() {
    assert!(!requires_host_switch_confirmation(
        Target::NixOs,
        "thor",
        Some("thor")
    ));
    assert!(!requires_host_switch_confirmation(
        Target::NixOs,
        "thor",
        Some("thor.example")
    ));
    assert!(requires_host_switch_confirmation(
        Target::NixOs,
        "odin",
        Some("thor")
    ));
    assert!(requires_host_switch_confirmation(
        Target::NixOs,
        "odin",
        None
    ));
    assert!(!requires_host_switch_confirmation(
        Target::HomeManager,
        "alice",
        Some("thor")
    ));
}
