use std::process::Command;

use super::{
    ACTIVATION_TIMEOUT, HelperPaths, ROLLBACK_TIMEOUT, acquire_activation_lock, cancellation,
    run_command,
};

pub(super) fn run(
    request_id: &str,
    nix: bool,
    journals: bool,
    keep_generations: usize,
    journal_max_age: &str,
    paths: &HelperPaths,
) -> Result<(), String> {
    if !nix && !journals {
        return Err("cleanup request did not select any system work".to_owned());
    }
    if keep_generations > 100 {
        return Err("cleanup generation retention must be between 0 and 100".to_owned());
    }
    if !valid_journal_max_age(journal_max_age) {
        return Err("cleanup journal retention is invalid".to_owned());
    }
    let cancellation =
        cancellation::OperationCancellation::start(request_id, &paths.cancellation_directory)?;
    let _lock = acquire_activation_lock(&paths.lock)?;
    let mut errors = Vec::new();

    if journals {
        let mut command = Command::new(&paths.journalctl);
        let vacuum = if journal_max_age == "0" {
            "--vacuum-size=1".to_owned()
        } else {
            format!("--vacuum-time={journal_max_age}")
        };
        command.args(["--rotate", &vacuum]);
        if let Err(error) = run_command(
            &mut command,
            "rotating and vacuuming journals",
            ROLLBACK_TIMEOUT,
            &cancellation.flag,
        ) {
            errors.push(error);
        }
    }

    if nix {
        let mut generations = Command::new(&paths.nix_env);
        let retention = generation_retention(keep_generations);
        generations
            .arg("--profile")
            .arg(&paths.profile)
            .args(["--delete-generations", &retention]);
        if let Err(error) = run_command(
            &mut generations,
            "deleting old NixOS generations",
            ACTIVATION_TIMEOUT,
            &cancellation.flag,
        ) {
            errors.push(error);
        }

        let mut garbage = Command::new(&paths.nix);
        garbage.args(["store", "gc"]);
        if let Err(error) = run_command(
            &mut garbage,
            "collecting the Nix store",
            ACTIVATION_TIMEOUT,
            &cancellation.flag,
        ) {
            errors.push(error);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// `nix-env --delete-generations` argument keeping the newest `keep_generations` up to and
/// including the current one, and every generation newer than it. Nix rejects `+0`, and the
/// current generation is always kept, so zero retention means `+1`.
fn generation_retention(keep_generations: usize) -> String {
    format!("+{}", keep_generations.max(1))
}

fn valid_journal_max_age(value: &str) -> bool {
    if value == "0" {
        return true;
    }
    let Some(unit_start) = value.find(|character: char| !character.is_ascii_digit()) else {
        return false;
    };
    let (amount, unit) = value.split_at(unit_start);
    !amount.is_empty()
        && amount.parse::<u64>().is_ok_and(|amount| amount > 0)
        && matches!(unit, "s" | "min" | "h" | "d" | "w" | "month" | "y")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use swix::test_support::write_executable_script;

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
                    cancellation_directory: root.join("cancel"),
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
        write_executable_script(
            path,
            &format!(
                "printf '{name} %s\\n' \"$*\" >> '{}'\nexit {}",
                log.display(),
                if succeeds { 0 } else { 7 },
            ),
        );
    }

    #[test]
    fn continues_nix_cleanup_when_journal_cleanup_fails() {
        let fixture = Fixture::new();
        write_recorder(&fixture.paths.journalctl, &fixture.log, "journalctl", false);

        let error = run("f1a-1", true, true, 3, "14d", &fixture.paths).unwrap_err();

        assert!(error.contains("rotating and vacuuming journals"));
        assert!(fixture.calls().contains("nix store gc"));
    }

    #[test]
    fn zero_retention_keeps_current_generation_and_removes_archived_journals() {
        let fixture = Fixture::new();

        run("f1a-2", true, true, 0, "0", &fixture.paths).unwrap();

        let calls = fixture.calls();
        assert!(calls.contains("journalctl --rotate --vacuum-size=1"));
        assert!(calls.contains("nix-env --profile"));
        assert!(calls.contains("--delete-generations +1"));
        assert!(!calls.contains("--delete-generations old"));
    }
}
