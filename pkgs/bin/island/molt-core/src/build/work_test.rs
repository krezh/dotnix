use super::*;
use crate::nix::NixActivityProgress;
use crate::test_support::PlainUnits;

#[test]
fn work_plan_tracks_states_and_finishes_vanished_work() {
    let started = Instant::now();
    let mut progress = NixBuildProgress::default();
    progress.planned.insert("/nix/store/h-bravo.drv".to_owned());
    progress.planned.insert("/nix/store/h-alpha.drv".to_owned());
    progress.items.push(NixActivityProgress {
        kind: 105,
        path: Some("/nix/store/h-bravo.drv".to_owned()),
        started: Some(started),
        ..NixActivityProgress::default()
    });
    let mut plan = WorkPlan::default();
    let changed = plan.update(&progress, started);
    assert_eq!(
        changed,
        ["/nix/store/h-alpha.drv", "/nix/store/h-bravo.drv"]
    );
    assert_eq!(
        plan.get("/nix/store/h-bravo.drv").unwrap().state,
        WorkState::Building
    );
    assert!(plan.update(&progress, started).is_empty());

    progress.items.clear();
    let later = started + Duration::from_secs(5);
    assert_eq!(plan.update(&progress, later), ["/nix/store/h-bravo.drv"]);
    let bravo = plan.get("/nix/store/h-bravo.drv").unwrap();
    assert_eq!(bravo.state, WorkState::Done);
    assert_eq!(bravo.took, Some(Duration::from_secs(5)));
    let summary = plan.summary();
    assert_eq!((summary.total, summary.done, summary.in_flight), (2, 1, 0));
    assert_eq!(summary.mean_build, None);
}

fn building(path: &str, detail: &str, started: Instant) -> NixActivityProgress {
    NixActivityProgress {
        kind: 105,
        path: Some(path.to_owned()),
        detail: Some(detail.to_owned()),
        started: Some(started),
        ..NixActivityProgress::default()
    }
}

#[test]
fn work_plan_keeps_one_row_per_derivation_with_its_latest_line() {
    let started = Instant::now();
    let path = "/nix/store/3k9f2a1abc-molt-0.1.0.drv";
    let mut progress = NixBuildProgress::default();
    // Newest activity first, as snapshots list them.
    progress
        .items
        .push(building(path, "Compiling gtk4 v0.11.5", started));
    progress
        .items
        .push(building(path, "Compiling glib v0.22.0", started));
    let mut plan = WorkPlan::default();
    assert_eq!(plan.update(&progress, started), [path]);
    let content = plan.content(path, started, &PlainUnits).unwrap();
    assert_eq!(content.detail, "Compiling gtk4 v0.11.5");
    assert_eq!(content.tag, "");
    assert_eq!(plan.summary().total, 1);

    // The failure record keeps the row's last line and moves it to the Failed group.
    progress.items.clear();
    progress.failed.insert(path.to_owned());
    assert_eq!(plan.update(&progress, started), [path]);
    let content = plan.content(path, started, &PlainUnits).unwrap();
    assert_eq!(content.group, 0);
    assert_eq!(content.look, RowLook::Failed);
    assert_eq!(content.detail, "Compiling gtk4 v0.11.5");

    // Work nix was still doing when it stopped returns to Queued.
    let other = "/nix/store/bbbbbbbbbb-glib-2.84.drv";
    progress
        .items
        .push(building(other, "Compiling glib", started));
    plan.update(&progress, started);
    assert_eq!(plan.interrupt(), [other]);
    assert_eq!(plan.get(other).unwrap().state, WorkState::Queued);
    assert_eq!(plan.get(path).unwrap().state, WorkState::Failed);
}

#[test]
fn work_plan_tags_distinct_derivations_sharing_a_name() {
    let started = Instant::now();
    let first = "/nix/store/3k9f2a1abc-molt-0.1.0.drv";
    let second = "/nix/store/zz11yy2def-molt-0.1.0.drv";
    let other = "/nix/store/aaaaaaaaaa-glib-2.84.drv";
    let mut progress = NixBuildProgress::default();
    progress.planned.insert(first.to_owned());
    progress.planned.insert(other.to_owned());
    let mut plan = WorkPlan::default();
    plan.update(&progress, started);
    let tag = |plan: &WorkPlan, path| plan.content(path, started, &PlainUnits).unwrap().tag;
    assert_eq!(tag(&plan, first), "");

    progress
        .items
        .push(building(second, "Compiling molt", started));
    let mut changed = plan.update(&progress, started);
    changed.sort();
    assert_eq!(changed, [first, second]);
    assert_eq!(tag(&plan, first), "· 3k9f2a1");
    assert_eq!(tag(&plan, second), "· zz11yy2");
    assert_eq!(tag(&plan, other), "");
}

#[test]
fn estimate_needs_three_finished_builds() {
    let mut progress = NixBuildProgress::default();
    for (name, seconds) in [("a", 10), ("b", 20)] {
        progress.completed.insert(
            format!("/nix/store/h-{name}.drv"),
            Duration::from_secs(seconds),
        );
    }
    progress.planned.insert("/nix/store/h-d.drv".to_owned());
    progress.planned.insert("/nix/store/h-e.drv".to_owned());
    let mut plan = WorkPlan::default();
    plan.update(&progress, Instant::now());
    assert_eq!(remaining_estimate(&plan.summary()), None);
    progress
        .completed
        .insert("/nix/store/h-c.drv".to_owned(), Duration::from_secs(30));
    plan.update(&progress, Instant::now());
    assert_eq!(
        remaining_estimate(&plan.summary()),
        Some(Duration::from_secs(40))
    );
}

#[test]
fn throughput_uses_a_sliding_window() {
    let start = Instant::now();
    let mut throughput = Throughput::default();
    assert!(throughput.sample(start, 0).abs() < 1e-6);
    let rate = throughput.sample(start + Duration::from_secs(1), 1024);
    assert!((rate - 1024.0).abs() < 1e-6);
    throughput.sample(start + Duration::from_secs(5), 1024);
    let rate = throughput.sample(start + Duration::from_secs(6), 4096);
    // The window reaches back to the last sample before it (at one second), so the rate
    // spans five seconds.
    assert!((rate - 3072.0 / 5.0).abs() < 1e-6);
    assert_eq!(throughput.history.len(), 4);
}

#[test]
fn store_paths_read_as_names() {
    assert_eq!(
        activity_name("/nix/store/hash-molt-0.1.0.drv"),
        "molt-0.1.0"
    );
}
