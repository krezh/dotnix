use std::fs;
use std::path::Path;
use std::time::SystemTime;

const PROFILES: &str = "/nix/var/nix/profiles";
const CURRENT_SYSTEM: &str = "/run/current-system";

/// Facts about the running NixOS system, read from the profile links without invoking Nix.
#[derive(Debug, Default)]
pub(crate) struct SystemStatus {
    /// Profile generation whose closure is the running system, if it is still registered.
    pub(crate) generation: Option<u64>,
    /// Generation the system profile points to when it is not the running one: applied for the
    /// next boot.
    pub(crate) staged: Option<u64>,
    /// When `/run/current-system` was last pointed at a closure (switch or boot).
    pub(crate) activated: Option<SystemTime>,
    pub(crate) version: Option<String>,
}

pub(crate) fn system_status() -> SystemStatus {
    let current = fs::read_link(CURRENT_SYSTEM).ok();
    SystemStatus {
        generation: current
            .as_deref()
            .and_then(|current| running_generation(Path::new(PROFILES), current)),
        staged: current
            .as_deref()
            .and_then(|current| staged_generation(Path::new(PROFILES), current)),
        activated: fs::symlink_metadata(CURRENT_SYSTEM)
            .and_then(|metadata| metadata.modified())
            .ok(),
        version: fs::read_to_string(Path::new(CURRENT_SYSTEM).join("nixos-version"))
            .ok()
            .map(|version| version.trim().to_owned())
            .filter(|version| !version.is_empty()),
    }
}

fn running_generation(profiles: &Path, current: &Path) -> Option<u64> {
    fs::read_dir(profiles)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let generation = generation_number(entry.file_name().to_str()?)?;
            (fs::read_link(entry.path()).ok()? == current).then_some(generation)
        })
        .max()
}

fn staged_generation(profiles: &Path, current: &Path) -> Option<u64> {
    let link = fs::read_link(profiles.join("system")).ok()?;
    let generation = generation_number(link.file_name()?.to_str()?)?;
    (fs::read_link(profiles.join(&link)).ok()? != current).then_some(generation)
}

/// `42` from `system-42-link`.
fn generation_number(name: &str) -> Option<u64> {
    name.strip_prefix("system-")?
        .strip_suffix("-link")?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use super::*;

    #[test]
    fn running_generation_matches_the_active_closure_not_the_newest() {
        let root = std::env::temp_dir().join(format!("swix-system-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        symlink("/nix/store/aaa-nixos-system", root.join("system-41-link")).unwrap();
        symlink("/nix/store/bbb-nixos-system", root.join("system-42-link")).unwrap();
        symlink("system-42-link", root.join("system")).unwrap();
        symlink("/nix/store/aaa-nixos-system", root.join("home-manager-41-link")).unwrap();

        assert_eq!(
            running_generation(&root, Path::new("/nix/store/aaa-nixos-system")),
            Some(41)
        );
        assert_eq!(
            running_generation(&root, Path::new("/nix/store/ccc-nixos-system")),
            None
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn staged_generation_is_the_boot_profile_only_when_it_is_not_running() {
        let root = std::env::temp_dir().join(format!("swix-staged-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        symlink("/nix/store/aaa-nixos-system", root.join("system-41-link")).unwrap();
        symlink("/nix/store/bbb-nixos-system", root.join("system-42-link")).unwrap();
        symlink("system-42-link", root.join("system")).unwrap();

        assert_eq!(
            staged_generation(&root, Path::new("/nix/store/aaa-nixos-system")),
            Some(42)
        );
        assert_eq!(
            staged_generation(&root, Path::new("/nix/store/bbb-nixos-system")),
            None
        );
        fs::remove_dir_all(&root).unwrap();
    }
}
