use std::path::PathBuf;
use std::time::Duration;

use super::*;
use crate::cleanup::{CleanupItem, DiskUsage};

fn policy() -> CleanupPolicy {
    CleanupPolicy {
        keep_generations: 3,
        journal_max_age: "7d".to_owned(),
    }
}

fn group(kind: CleanupKind, bytes: u64) -> CleanupGroup {
    CleanupGroup {
        kind,
        available: true,
        reclaimable: bytes,
        items: vec![CleanupItem {
            label: "item".to_owned(),
            detail: "detail".to_owned(),
            bytes: Some(bytes),
            path: None,
        }],
        note: None,
        volumes: Vec::new(),
    }
}

fn volume(available: u64) -> Volume {
    Volume {
        mount: PathBuf::from("/"),
        device: 1,
        usage: DiskUsage {
            total: 1_000,
            used: 1_000 - available,
            available,
        },
        kinds: CleanupKind::ALL.to_vec(),
    }
}

/// A screen whose first scan found Nix, Trash and old game prefixes.
fn scanned() -> Cleanup {
    let (mut screen, _) = Cleanup::open(policy());
    for (kind, bytes) in [
        (CleanupKind::Nix, 300),
        (CleanupKind::Trash, 100),
        (CleanupKind::GamePrefixes, 50),
    ] {
        screen.update(Msg::Group(0, group(kind, bytes), 3));
    }
    screen.update(Msg::Group(
        0,
        CleanupGroup::unavailable(CleanupKind::Docker, "no docker"),
        3,
    ));
    let effects = screen.update(Msg::Scanned(0, vec![volume(200)]));
    assert_eq!(effects, vec![Effect::Estimate(Some(450))]);
    screen
}

#[test]
fn first_scan_selects_what_it_finds_but_prefixes() {
    let screen = scanned();
    assert!(!screen.scanning);
    assert_eq!(screen.selected, vec![CleanupKind::Nix, CleanupKind::Trash]);
    assert_eq!(
        screen.plan(),
        vec![(CleanupKind::Nix, 300), (CleanupKind::Trash, 100)]
    );
}

#[test]
fn unavailable_categories_cannot_be_picked() {
    let mut screen = scanned();
    screen.update(Msg::Toggle(CleanupKind::Docker));
    assert!(!screen.is_selected(CleanupKind::Docker));
    screen.update(Msg::Toggle(CleanupKind::GamePrefixes));
    assert!(screen.is_selected(CleanupKind::GamePrefixes));
}

#[test]
fn superseded_scans_are_dropped() {
    let mut screen = scanned();
    screen.update(Msg::Group(7, group(CleanupKind::Caches, 10), 3));
    assert!(screen.group(CleanupKind::Caches).is_none());
}

#[test]
fn cleaning_takes_two_presses() {
    let mut screen = scanned();
    assert!(screen.update(Msg::Act).is_empty());
    assert!(screen.armed);
    let effects = screen.update(Msg::Act);
    let [Effect::Clean { groups, policy, .. }] = effects.as_slice() else {
        panic!("expected a cleanup, got {effects:?}");
    };
    let kinds: Vec<_> = groups.iter().map(|group| group.kind).collect();
    assert_eq!(kinds, vec![CleanupKind::Nix, CleanupKind::Trash]);
    assert_eq!(policy.keep_generations, 3);
    assert_eq!(policy.journal_max_age, "0");
    assert_eq!(screen.phase, Phase::Cleaning);
}

#[test]
fn a_change_disarms() {
    let mut screen = scanned();
    screen.update(Msg::Act);
    screen.update(Msg::Toggle(CleanupKind::Trash));
    assert!(!screen.armed);
}

#[test]
fn stepping_generations_rescans_nix_once_settled() {
    let mut screen = scanned();
    let effects = screen.update(Msg::StepGenerations(-1));
    assert_eq!(effects, vec![Effect::Settle(1)]);
    assert_eq!(screen.generations, "2");
    assert!(!screen.plan_is_current());
    assert!(!screen.can_act());
    assert!(screen.update(Msg::Settled(0)).is_empty());
    let effects = screen.update(Msg::Settled(1));
    let [
        Effect::Scan {
            serial,
            kinds,
            policy,
        },
    ] = effects.as_slice()
    else {
        panic!("expected a rescan, got {effects:?}");
    };
    assert_eq!(kinds, &vec![CleanupKind::Nix]);
    assert_eq!(policy.keep_generations, 2);
    let serial = *serial;
    screen.update(Msg::Group(serial, group(CleanupKind::Nix, 400), 2));
    screen.update(Msg::Scanned(serial, vec![volume(200)]));
    assert!(screen.plan_is_current());
    assert!(screen.can_act());
    assert_eq!(
        screen
            .group(CleanupKind::Nix)
            .map(|group| group.reclaimable),
        Some(400)
    );
}

#[test]
fn generations_stay_between_none_and_a_hundred() {
    let mut screen = scanned();
    screen.generations = "0".to_owned();
    assert!(screen.update(Msg::StepGenerations(-1)).is_empty());
    screen.generations = "100".to_owned();
    assert!(screen.update(Msg::StepGenerations(1)).is_empty());
}

#[test]
fn a_finished_cleanup_reports_what_it_freed() {
    let mut screen = scanned();
    screen.update(Msg::Act);
    screen.update(Msg::Act);
    let serial = screen.serial;
    screen.update(Msg::Started(serial, CleanupKind::Nix));
    assert_eq!(screen.running, vec![CleanupKind::Nix]);
    screen.update(Msg::Finished(serial, CleanupKind::Nix, Ok(300)));
    screen.update(Msg::Finished(
        serial,
        CleanupKind::Trash,
        Err("denied".to_owned()),
    ));
    let effects = screen.update(Msg::Cleaned {
        serial,
        before: vec![volume(200)],
        after: vec![volume(500)],
        elapsed: Duration::from_millis(1_500),
    });
    assert_eq!(effects, vec![Effect::Estimate(None)]);
    assert_eq!(screen.phase, Phase::Complete);
    assert_eq!(
        screen.freed,
        Some(Freed {
            bytes: 300,
            elapsed: Duration::from_millis(1_500),
            failures: 1,
        })
    );
}

#[test]
fn scanning_again_starts_over() {
    let mut screen = scanned();
    screen.phase = Phase::Complete;
    let before = screen.serial;
    let effects = screen.update(Msg::Act);
    assert!(matches!(
        effects.as_slice(),
        [Effect::Scan { serial, kinds, .. }] if *serial == before + 1 && kinds.len() == 8
    ));
    assert!(screen.scanning);
    assert!(screen.groups.is_empty());
}
