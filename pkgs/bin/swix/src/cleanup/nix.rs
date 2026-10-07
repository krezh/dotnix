use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::model::{CleanupGroup, CleanupItem, CleanupKind};
use super::run_command;

const SCAN_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const CLEAN_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const OUTPUT_LIMIT: usize = 128 * 1024 * 1024;
const SIZE_QUERY_ARGUMENT_BYTES: usize = 512 * 1024;
const NIX: &str = match option_env!("SWIX_NIX") {
    Some(path) => path,
    None => "nix",
};
const NIX_STORE: &str = match option_env!("SWIX_NIX_STORE") {
    Some(path) => path,
    None => "nix-store",
};
const NIX_ENV: &str = match option_env!("SWIX_NIX_ENV") {
    Some(path) => path,
    None => "nix-env",
};

pub(super) fn scan(cancellation: &AtomicBool) -> CleanupGroup {
    let mut command = Command::new(NIX_STORE);
    command.args(["--gc", "--print-dead"]);
    let output = match run_command(
        &mut command,
        "Nix garbage scan",
        cancellation,
        SCAN_TIMEOUT,
        OUTPUT_LIMIT,
    ) {
        Ok(output) => output,
        Err(error) => return CleanupGroup::unavailable(CleanupKind::Nix, error),
    };
    let dead_paths = dead_store_paths(&output.stdout);
    let sizes = path_sizes(&dead_paths, cancellation).ok();

    let mut items = dead_paths
        .into_iter()
        .enumerate()
        .map(|(index, path)| CleanupItem {
            label: store_name(&path),
            detail: path,
            bytes: sizes.as_ref().and_then(|sizes| sizes.get(index).copied()),
            path: None,
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then_with(|| left.label.cmp(&right.label))
    });
    let reclaimable = items.iter().filter_map(|item| item.bytes).sum();

    let (generation_items, _) = old_generations();
    let generation_count = generation_items.len();
    items.extend(generation_items);
    let note = if generation_count == 0 {
        None
    } else {
        Some(format!(
            "{generation_count} old generation(s) are included; their shared closure size is resolved during collection"
        ))
    };
    CleanupGroup {
        kind: CleanupKind::Nix,
        available: true,
        reclaimable,
        items,
        note,
    }
}

pub(super) fn clean_user_profiles(cancellation: &AtomicBool) -> Result<(), String> {
    let (_, profiles) = old_generations();
    for profile in profiles {
        if cancellation.load(Ordering::Relaxed) {
            return Err(swix::command::CANCELLED.to_owned());
        }
        let mut command = Command::new(NIX_ENV);
        command
            .arg("--profile")
            .arg(&profile)
            .args(["--delete-generations", "old"]);
        run_command(
            &mut command,
            "old Nix profile cleanup",
            cancellation,
            CLEAN_TIMEOUT,
            1024 * 1024,
        )?;
    }
    Ok(())
}

pub(super) fn collect_store(cancellation: &AtomicBool) -> Result<(), String> {
    let mut command = Command::new(NIX);
    command.args(["store", "gc"]);
    run_command(
        &mut command,
        "Nix store cleanup",
        cancellation,
        CLEAN_TIMEOUT,
        1024 * 1024,
    )?;
    Ok(())
}

fn path_sizes(paths: &[String], cancellation: &AtomicBool) -> Result<Vec<u64>, String> {
    let mut sizes = Vec::with_capacity(paths.len());
    let mut start = 0;
    while start < paths.len() {
        let mut end = start;
        let mut argument_bytes = 0_usize;
        while let Some(path) = paths.get(end) {
            let required = path.len().saturating_add(1);
            if end > start && argument_bytes.saturating_add(required) > SIZE_QUERY_ARGUMENT_BYTES {
                break;
            }
            argument_bytes = argument_bytes.saturating_add(required);
            end += 1;
        }

        let batch = &paths[start..end];
        let mut command = Command::new(NIX_STORE);
        command.args(["--query", "--size"]).args(batch);
        let output = run_command(
            &mut command,
            "Nix path size scan",
            cancellation,
            SCAN_TIMEOUT,
            OUTPUT_LIMIT,
        )?;
        sizes.extend(parse_path_sizes(batch, &output.stdout)?);
        start = end;
    }
    Ok(sizes)
}

fn dead_store_paths(output: &[u8]) -> Vec<String> {
    let mut paths = String::from_utf8_lossy(output)
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("/nix/store/"))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    paths.sort_unstable();
    paths.dedup();
    paths
}

fn parse_path_sizes(paths: &[String], output: &[u8]) -> Result<Vec<u64>, String> {
    let output = std::str::from_utf8(output)
        .map_err(|error| format!("Nix path size scan returned invalid text: {error}"))?;
    let mut lines = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let mut sizes = Vec::with_capacity(paths.len());
    for path in paths {
        let line = lines
            .next()
            .ok_or("Nix path size scan returned too few sizes")?;
        let size = line
            .parse()
            .map_err(|error| format!("invalid size for {path}: {error}"))?;
        sizes.push(size);
    }
    if lines.next().is_some() {
        return Err("Nix path size scan returned too many sizes".to_owned());
    }
    Ok(sizes)
}

fn store_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.split_once('-').map(|(_, package)| package))
        .unwrap_or(path)
        .to_owned()
}

fn old_generations() -> (Vec<CleanupItem>, Vec<PathBuf>) {
    let mut roots = vec![PathBuf::from("/nix/var/nix/profiles")];
    if let Ok(user) = std::env::var("USER") {
        roots.push(PathBuf::from("/nix/var/nix/profiles/per-user").join(user));
    }
    if let Some(state) = xdg_state_home() {
        roots.push(state.join("nix/profiles"));
    }

    let mut items = Vec::new();
    let mut profiles = HashSet::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let generation = entry.path();
            let Some((profile_name, generation_number)) = generation_name(&generation) else {
                continue;
            };
            let profile = root.join(&profile_name);
            let Ok(current) = fs::canonicalize(&profile) else {
                continue;
            };
            let Ok(target) = fs::canonicalize(&generation) else {
                continue;
            };
            if current == target {
                continue;
            }
            items.push(CleanupItem {
                label: format!("{profile_name} generation {generation_number}"),
                detail: generation.display().to_string(),
                bytes: None,
                path: None,
            });
            if profile != Path::new("/nix/var/nix/profiles/system") {
                profiles.insert(profile);
            }
        }
    }
    items.sort_by(|left, right| left.detail.cmp(&right.detail));
    let mut profiles = profiles.into_iter().collect::<Vec<_>>();
    profiles.sort();
    (items, profiles)
}

fn generation_name(path: &Path) -> Option<(String, u64)> {
    let name = path.file_name()?.to_str()?.strip_suffix("-link")?;
    let (profile, generation) = name.rsplit_once('-')?;
    Some((profile.to_owned(), generation.parse().ok()?))
}

fn xdg_state_home() -> Option<PathBuf> {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_store_paths_from_gc_output() {
        let output = b"finding roots...\n/nix/store/abc-one\nnot a path\n/nix/store/def-two\n";
        assert_eq!(
            dead_store_paths(output),
            ["/nix/store/abc-one", "/nix/store/def-two"]
        );
    }

    #[test]
    fn reads_nix_store_sizes_in_path_order() {
        let paths = vec![
            "/nix/store/abc-one".to_owned(),
            "/nix/store/def-two".to_owned(),
        ];
        assert_eq!(parse_path_sizes(&paths, b"42\n7\n").unwrap(), [42, 7]);
        assert!(parse_path_sizes(&paths, b"42\n").is_err());
        assert!(parse_path_sizes(&paths, b"42\n7\n9\n").is_err());
    }

    #[test]
    fn identifies_profile_generations() {
        assert_eq!(
            generation_name(Path::new("/nix/var/nix/profiles/system-42-link")),
            Some(("system".to_owned(), 42))
        );
        assert_eq!(generation_name(Path::new("profile-link")), None);
    }
}
