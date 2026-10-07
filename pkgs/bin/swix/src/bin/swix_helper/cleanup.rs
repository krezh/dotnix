use std::process::Command;

use super::{
    ACTIVATION_TIMEOUT, HelperPaths, ROLLBACK_TIMEOUT, acquire_activation_lock, run_command,
};

pub(super) fn run(nix: bool, journals: bool, paths: &HelperPaths) -> Result<(), String> {
    if !nix && !journals {
        return Err("cleanup request did not select any system work".to_owned());
    }
    let _lock = acquire_activation_lock(&paths.lock)?;
    let mut errors = Vec::new();

    if journals {
        let mut command = Command::new(&paths.journalctl);
        command.args(["--rotate", "--vacuum-size=1"]);
        if let Err(error) = run_command(
            &mut command,
            "rotating and vacuuming journals",
            ROLLBACK_TIMEOUT,
        ) {
            errors.push(error);
        }
    }

    if nix {
        let mut generations = Command::new(&paths.nix_env);
        generations
            .arg("--profile")
            .arg(&paths.profile)
            .args(["--delete-generations", "old"]);
        if let Err(error) = run_command(
            &mut generations,
            "deleting old NixOS generations",
            ACTIVATION_TIMEOUT,
        ) {
            errors.push(error);
        }

        let mut garbage = Command::new(&paths.nix);
        garbage.args(["store", "gc"]);
        if let Err(error) =
            run_command(&mut garbage, "collecting the Nix store", ACTIVATION_TIMEOUT)
        {
            errors.push(error);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
        log: PathBuf,
        paths: HelperPaths,
    }

    impl Fixture {
        fn new() -> Self {
            let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "swix-cleanup-helper-test-{}-{id}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            let log = root.join("calls");
            let nix_env = root.join("nix-env");
            let nix = root.join("nix");
            let journalctl = root.join("journalctl");
            write_recorder(&nix_env, &log, "nix-env", true);
            write_recorder(&nix, &log, "nix", true);
            write_recorder(&journalctl, &log, "journalctl", true);
            Self {
                paths: HelperPaths {
                    profile: root.join("system"),
                    store: root.join("store"),
                    lock: root.join("lock"),
                    nix_env,
                    nix,
                    journalctl,
                },
                root,
                log,
            }
        }

        fn calls(&self) -> String {
            fs::read_to_string(&self.log).unwrap_or_default()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn write_recorder(path: &Path, log: &Path, name: &str, succeeds: bool) {
        fs::write(
            path,
            format!(
                "#!/bin/sh\nprintf '{name} %s\\n' \"$*\" >> '{}'\nexit {}\n",
                log.display(),
                if succeeds { 0 } else { 7 },
            ),
        )
        .unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    fn continues_nix_cleanup_when_journal_cleanup_fails() {
        let fixture = Fixture::new();
        write_recorder(&fixture.paths.journalctl, &fixture.log, "journalctl", false);

        let error = run(true, true, &fixture.paths).unwrap_err();

        assert!(error.contains("rotating and vacuuming journals"));
        assert!(fixture.calls().contains("nix store gc"));
    }
}
