use super::{Report, Updates};

/// A report as island's cache keeps it, in the shape `molt check --json` printed, trimmed to one
/// commit.
const CHECK: &str = r#"{"vcs":"jj","upstream":"trunk","incoming":5,"local":1,"fetched":false,"commits":[{"id":"37aa009e5d23","author":"dextek-bot[bot]","time":1792215431,"summary":"fix(cargo) [island]: Update Rust crate libc (0.2.190 ➔ 0.2.191) (#2273)"}],"watchlist":{"total":1,"ready":0,"prs":[{"pr_number":573942,"repo":"NixOS/nixpkgs","title":"lixPackageSets.lix_2_95.lix: drop unsupported option mimalloc","ready":false,"state":"merged","merge_commit":"982cb8808385d73a10d9b79d7af8829b40d84cdb","branches":[{"name":"master","reached":true,"status":"reached"},{"name":"nixpkgs-unstable","reached":true,"status":"reached"},{"name":"nixos-unstable-small","reached":true,"status":"reached"},{"name":"nixos-unstable","reached":false,"status":"pending","ahead_by":63}]}]}}"#;

#[test]
fn reads_reports_from_before_the_watchlist() {
    let report: Report = serde_json::from_str(r#"{"incoming":2,"local":0}"#).unwrap();
    assert!(report.watchlist.is_none());
    assert_eq!(report.ready(), 0);
}

#[test]
fn sums_up_incoming_changes_and_ready_prs() {
    let mut report: Report = serde_json::from_str(CHECK).unwrap();
    let updates = |report: &Report| Updates {
        report: Some(report.clone()),
        ..Updates::default()
    };
    assert_eq!(updates(&report).summary(), "5 incoming changes");
    if let Some(watchlist) = report.watchlist.as_mut() {
        watchlist.ready = 1;
    }
    assert_eq!(
        updates(&report).summary(),
        "5 incoming changes · 1 watched PR ready"
    );
    report.incoming = 0;
    assert_eq!(updates(&report).summary(), "1 watched PR ready");
}
