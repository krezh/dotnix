use super::*;
use std::env;
use std::fs;

use crate::test_support::PlainUnits;

/// Direct inputs as `inputs::load` reads them from a lock file with these GitHub nodes.
fn inputs(nodes: &[(&str, &str, &str, &str)]) -> Vec<FlakeInput> {
    let directory = env::temp_dir().join(format!(
        "molt-build-progress-test-{}-{}",
        std::process::id(),
        nodes.len()
    ));
    fs::create_dir_all(&directory).unwrap();
    let mut lock = serde_json::json!({
        "nodes": { "root": { "inputs": {} } },
        "root": "root",
        "version": 7,
    });
    for (name, owner, repo, rev) in nodes {
        lock["nodes"]["root"]["inputs"][name] = serde_json::json!(name);
        lock["nodes"][name] = serde_json::json!({
            "locked": { "type": "github", "owner": owner, "repo": repo, "rev": rev },
        });
    }
    fs::write(directory.join("flake.lock"), lock.to_string()).unwrap();
    let (_, inputs) = crate::inputs::load(&directory).unwrap();
    fs::remove_dir_all(&directory).unwrap();
    inputs
}

fn fetch(input: &str, started: Instant, finished: Option<Instant>) -> FlakeFetch {
    FlakeFetch {
        input: input.to_owned(),
        started,
        finished,
    }
}

#[test]
fn fetches_map_to_direct_inputs_by_source_and_revision() {
    let now = Instant::now();
    let inputs = inputs(&[
        ("nixpkgs", "NixOS", "nixpkgs", "aaa111"),
        ("nixpkgs-stable", "NixOS", "nixpkgs", "bbb222"),
        ("nixpkgs-wayland", "nix-community", "nixpkgs-wayland", "ccc"),
    ]);
    let fetches = [
        fetch("github:NixOS/nixpkgs/bbb222?narHash=x", now, None),
        fetch(
            "git+file:///home/me/dotnix",
            now,
            Some(now + Duration::from_millis(800)),
        ),
    ];
    let rows = input_rows(&inputs, &fetches, None);
    assert_eq!(rows[0].state, InputState::Cached);
    assert_eq!(rows[1].state, InputState::Fetching(now));
    assert_eq!(rows[1].reference, "github:NixOS/nixpkgs/bbb222");
    assert_eq!(rows[2].state, InputState::Cached);
    assert_eq!(rows[3].name, "dotnix");
    assert_eq!(
        rows[3].state,
        InputState::Fetched(Duration::from_millis(800))
    );

    let done = now + Duration::from_secs(2);
    let rows = input_rows(&inputs, &fetches, Some(done));
    assert_eq!(rows[1].state, InputState::Fetched(Duration::from_secs(2)));
}

#[test]
fn source_matches_need_whole_path_segments() {
    assert!(mentions("github:nixos/nixpkgs/rev", "nixos/nixpkgs"));
    assert!(!mentions(
        "github:nixos/nixpkgs-wayland/rev",
        "nixos/nixpkgs"
    ));
    assert!(!mentions("github:xnixos/nixpkgs/rev", "nixos/nixpkgs"));
    assert_eq!(fetch_name("github:catppuccin/nix/abc"), "nix");
    assert_eq!(fetch_name("git+https://example.com/a/repo.git"), "repo");
}

#[test]
fn open_fetches_fail_with_evaluation() {
    let now = Instant::now();
    let fetches = [
        fetch("github:NixOS/nixpkgs/aaa", now, None),
        fetch("git+file:///home/me/dotnix", now, Some(now)),
    ];
    let mut rows = input_rows(&[], &fetches, None);
    fail_open_fetches(&mut rows);
    assert_eq!(rows[0].state, InputState::Failed);
    assert_eq!(rows[0].content(now, &PlainUnits).group, 0);
    assert_eq!(rows[1].state, InputState::Fetched(Duration::ZERO));
}
