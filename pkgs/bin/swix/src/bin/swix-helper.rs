use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use swix::command::{self, OutputLimits};
use swix::protocol::OwnedActivationRequest;

const MAX_REQUEST_BYTES: usize = 4096;
const MAX_OUTPUT_BYTES: usize = 32 * 1024;
const ACTIVATION_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const ROLLBACK_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const PROFILE: &str = "/nix/var/nix/profiles/system";
const STORE: &str = "/nix/store";
const ACTIVATION_LOCK: &str = "/run/swix-helper.lock";

struct ActivationPaths {
    profile: PathBuf,
    store: PathBuf,
    lock: PathBuf,
    nix_env: PathBuf,
}

impl ActivationPaths {
    fn system() -> Self {
        Self {
            profile: PathBuf::from(PROFILE),
            store: PathBuf::from(STORE),
            lock: PathBuf::from(ACTIVATION_LOCK),
            nix_env: PathBuf::from(env!("SWIX_NIX_ENV")),
        }
    }
}

fn main() {
    let started = Instant::now();
    let result = handle_request();

    match &result {
        Ok(()) => eprintln!(
            "swix-helper: outcome=success elapsed_ms={}",
            started.elapsed().as_millis()
        ),
        Err(error) => eprintln!(
            "swix-helper: outcome=failure error={error} elapsed_ms={}",
            started.elapsed().as_millis()
        ),
    }

    let response = match &result {
        Ok(()) => "OK\n".to_owned(),
        Err(error) => format!("ERROR\n{error}\n"),
    };
    if let Err(error) = io::stdout().write_all(response.as_bytes()) {
        eprintln!("swix-helper: failed to write response: {error}");
        std::process::exit(1);
    }

    if result.is_err() {
        std::process::exit(1);
    }
}

fn handle_request() -> Result<(), String> {
    let mut input = Vec::with_capacity(MAX_REQUEST_BYTES + 1);
    io::stdin()
        .lock()
        .take((MAX_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|error| format!("failed to read switch request: {error}"))?;
    activate_request(&input, &ActivationPaths::system())
}

fn activate_request(input: &[u8], paths: &ActivationPaths) -> Result<(), String> {
    let request = parse_request(input)?;
    let requested = request.output;
    let closure = fs::canonicalize(&requested)
        .map_err(|error| format!("invalid system closure {}: {error}", requested.display()))?;
    validate_closure(&closure, &paths.store)?;
    let baseline = request.baseline;
    if !is_store_path(&baseline, &paths.store) {
        return Err(format!(
            "{} is not a top-level Nix store path",
            baseline.display()
        ));
    }
    eprintln!(
        "swix-helper: baseline={} closure={}",
        baseline.display(),
        closure.display()
    );

    let _lock = acquire_activation_lock(&paths.lock)?;
    let prior = fs::canonicalize(&paths.profile)
        .map_err(|error| format!("failed to resolve prior system profile target: {error}"))?;
    if prior != baseline {
        return Err(format!(
            "active profile changed from {} to {}; rebuild before switching",
            baseline.display(),
            prior.display()
        ));
    }
    eprintln!("swix-helper: prior_profile_target={}", prior.display());
    let deadline = Instant::now() + ACTIVATION_TIMEOUT;

    set_profile(
        &paths.nix_env,
        &paths.profile,
        &closure,
        "setting the requested system profile",
        deadline.saturating_duration_since(Instant::now()),
    )?;
    let mut activation = Command::new(closure.join("bin/switch-to-configuration"));
    activation.arg("switch");
    if let Err(activation_error) = run_command(
        &mut activation,
        "activation",
        deadline.saturating_duration_since(Instant::now()),
    ) {
        eprintln!("swix-helper: activation outcome=failure error={activation_error}");
        return match set_profile(
            &paths.nix_env,
            &paths.profile,
            &prior,
            "restoring the prior system profile",
            ROLLBACK_TIMEOUT,
        ) {
            Ok(()) => {
                eprintln!(
                    "swix-helper: rollback outcome=success target={}",
                    prior.display()
                );
                Err(format!(
                    "{activation_error}; restored prior system profile {}",
                    prior.display()
                ))
            }
            Err(rollback_error) => {
                eprintln!(
                    "swix-helper: rollback outcome=failure target={} error={rollback_error}",
                    prior.display()
                );
                Err(format!(
                    "{activation_error}; rollback to {} failed: {rollback_error}",
                    prior.display()
                ))
            }
        };
    }

    eprintln!("swix-helper: activation outcome=success");
    Ok(())
}

fn parse_request(input: &[u8]) -> Result<OwnedActivationRequest, String> {
    if input.len() > MAX_REQUEST_BYTES {
        return Err(format!(
            "switch request exceeds {MAX_REQUEST_BYTES}-byte limit"
        ));
    }
    serde_json::from_slice(input).map_err(|error| format!("invalid switch request: {error}"))
}

fn validate_closure(closure: &Path, store: &Path) -> Result<(), String> {
    if !is_store_path(closure, store) {
        return Err(format!(
            "{} is not a top-level Nix store path",
            closure.display()
        ));
    }

    let switch = closure.join("bin/switch-to-configuration");
    if !switch.is_file() {
        return Err(format!(
            "{} does not contain a regular bin/switch-to-configuration file",
            closure.display()
        ));
    }

    Ok(())
}

fn is_store_path(path: &Path, store: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(store) else {
        return false;
    };
    let mut components = relative.components();
    matches!(components.next(), Some(std::path::Component::Normal(_)))
        && components.next().is_none()
}

fn acquire_activation_lock(path: &Path) -> Result<File, String> {
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|error| format!("failed to open activation lock: {error}"))?;

    match lock.try_lock() {
        Ok(()) => {}
        Err(fs::TryLockError::WouldBlock) => {
            return Err("another activation is already running; retry later".to_owned());
        }
        Err(fs::TryLockError::Error(error)) => {
            return Err(format!("failed to acquire activation lock: {error}"));
        }
    }

    Ok(lock)
}

fn set_profile(
    nix_env: &Path,
    profile: &Path,
    target: &Path,
    label: &str,
    timeout: Duration,
) -> Result<(), String> {
    let mut command = Command::new(nix_env);
    command.arg("-p").arg(profile).arg("--set").arg(target);
    run_command(&mut command, label, timeout)
}

fn run_command(command: &mut Command, label: &str, timeout: Duration) -> Result<(), String> {
    let cancellation = AtomicBool::new(false);
    let output = command::run(
        command,
        label,
        &cancellation,
        timeout,
        OutputLimits {
            stdout: MAX_OUTPUT_BYTES,
            stderr: MAX_OUTPUT_BYTES,
        },
        |_| {},
        bounded_output,
    )?;
    log_command_output(label, &output);
    eprintln!("swix-helper: operation={label:?} outcome=success");
    Ok(())
}

fn log_command_output(label: &str, output: &Output) {
    if !output.stdout.is_empty() {
        eprintln!(
            "swix-helper: operation={label:?} stdout:\n{}",
            bounded_output(&output.stdout)
        );
    }
    if !output.stderr.is_empty() {
        eprintln!(
            "swix-helper: operation={label:?} stderr:\n{}",
            bounded_output(&output.stderr)
        );
    }
}

fn bounded_output(bytes: &[u8]) -> String {
    let limit = bytes.len().min(MAX_OUTPUT_BYTES);
    let mut output = String::from_utf8_lossy(&bytes[..limit])
        .trim_end()
        .to_owned();
    if bytes.len() > limit {
        output.push_str("\n[output truncated]");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::sync::atomic::{AtomicU64, Ordering};
    use swix::protocol::ActivationRequest;

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    struct ActivationFixture {
        root: PathBuf,
        paths: ActivationPaths,
        prior: PathBuf,
        requested: PathBuf,
    }

    impl ActivationFixture {
        fn new(activation_succeeds: bool, rollback_succeeds: bool) -> Self {
            let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
            let root =
                std::env::temp_dir().join(format!("swix-helper-test-{}-{id}", std::process::id()));
            if root.exists() {
                fs::remove_dir_all(&root).unwrap();
            }
            let store = root.join("store");
            let prior = store.join("prior-system");
            let requested = store.join("requested-system");
            fs::create_dir_all(prior.join("bin")).unwrap();
            fs::create_dir_all(requested.join("bin")).unwrap();
            write_script(
                &requested.join("bin/switch-to-configuration"),
                if activation_succeeds {
                    "exit 0"
                } else {
                    "printf 'activation fixture failed\\n' >&2\nexit 7"
                },
            );
            let profile = root.join("profiles/system");
            fs::create_dir_all(profile.parent().unwrap()).unwrap();
            symlink(&prior, &profile).unwrap();
            let nix_env = root.join("nix-env");
            let script = if rollback_succeeds {
                "ln -sfnT \"$4\" \"$2\"\nexit 0".to_owned()
            } else {
                let counter = root.join("nix-env-called");
                format!(
                    "if [ -e \"{}\" ]; then exit 9; fi\n: > \"{}\"\nln -sfnT \"$4\" \"$2\"\nexit 0",
                    counter.display(),
                    counter.display()
                )
            };
            write_script(&nix_env, &script);
            let paths = ActivationPaths {
                profile,
                store,
                lock: root.join("activation.lock"),
                nix_env,
            };
            Self {
                root,
                paths,
                prior,
                requested,
            }
        }

        fn activate(&self) -> Result<(), String> {
            let request = serde_json::to_vec(&ActivationRequest {
                baseline: &self.prior,
                output: &self.requested,
            })
            .unwrap();
            activate_request(&request, &self.paths)
        }
    }

    impl Drop for ActivationFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn write_script(path: &Path, body: &str) {
        fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    fn parses_activation_request() {
        let request = parse_request(
            br#"{"baseline":"/nix/store/old-system","output":"/nix/store/new-system"}"#,
        )
        .unwrap();
        assert_eq!(request.baseline, Path::new("/nix/store/old-system"));
        assert_eq!(request.output, Path::new("/nix/store/new-system"));
    }

    #[test]
    fn rejects_invalid_requests() {
        assert!(parse_request(b"").is_err());
        assert!(parse_request(br#"{"output":"/nix/store/new-system"}"#).is_err());
        assert!(parse_request(br#"{"baseline":1,"output":2}"#).is_err());
    }

    #[test]
    fn rejects_oversized_requests() {
        let input = vec![b'a'; MAX_REQUEST_BYTES + 1];
        assert!(parse_request(&input).is_err());
    }

    #[test]
    fn identifies_only_top_level_store_paths() {
        let store = Path::new("/nix/store");
        assert!(is_store_path(Path::new("/nix/store/hash-system"), store));
        assert!(!is_store_path(Path::new("/nix/store"), store));
        assert!(!is_store_path(
            Path::new("/nix/store/hash-system/bin"),
            store
        ));
        assert!(!is_store_path(Path::new("/tmp/hash-system"), store));
        assert!(!is_store_path(Path::new("nix/store/hash-system"), store));
    }
    #[test]
    fn truncates_command_output() {
        let output = bounded_output(&vec![b'a'; MAX_OUTPUT_BYTES + 1]);
        assert_eq!(
            output.len(),
            MAX_OUTPUT_BYTES + "\n[output truncated]".len()
        );
        assert!(output.ends_with("[output truncated]"));
    }

    #[test]
    fn activation_sets_the_requested_profile() {
        let fixture = ActivationFixture::new(true, true);
        fixture.activate().unwrap();
        assert_eq!(
            fs::canonicalize(&fixture.paths.profile).unwrap(),
            fixture.requested
        );
    }

    #[test]
    fn changed_profile_is_rejected_before_activation() {
        let fixture = ActivationFixture::new(true, true);
        let replacement = fixture.paths.store.join("replacement-system");
        fs::create_dir_all(&replacement).unwrap();
        fs::remove_file(&fixture.paths.profile).unwrap();
        symlink(&replacement, &fixture.paths.profile).unwrap();

        let error = fixture.activate().unwrap_err();
        assert!(error.contains("active profile changed"));
        assert_eq!(
            fs::canonicalize(&fixture.paths.profile).unwrap(),
            replacement
        );
    }

    #[test]
    fn failed_activation_restores_the_prior_profile() {
        let fixture = ActivationFixture::new(false, true);
        let error = fixture.activate().unwrap_err();
        assert!(error.contains("restored prior system profile"));
        assert_eq!(
            fs::canonicalize(&fixture.paths.profile).unwrap(),
            fixture.prior
        );
    }

    #[test]
    fn rollback_failure_is_reported_and_leaves_the_requested_profile() {
        let fixture = ActivationFixture::new(false, false);
        let error = fixture.activate().unwrap_err();
        assert!(error.contains("rollback to"));
        assert!(error.contains("failed"));
        assert_eq!(
            fs::canonicalize(&fixture.paths.profile).unwrap(),
            fixture.requested
        );
    }

    #[test]
    fn activation_lock_rejects_concurrent_requests() {
        let fixture = ActivationFixture::new(true, true);
        let _first = acquire_activation_lock(&fixture.paths.lock).unwrap();
        let error = acquire_activation_lock(&fixture.paths.lock).unwrap_err();
        assert!(error.contains("another activation is already running"));
    }

    #[test]
    fn helper_rejects_excessive_command_output() {
        let mut command = Command::new("sh");
        command.args(["-c", "yes x | head -c 32769"]);
        let error = run_command(&mut command, "noisy fixture", Duration::from_secs(2)).unwrap_err();
        assert!(error.contains("stdout exceeded"));
    }

    #[test]
    fn helper_times_out_commands_and_descendants() {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 2 & wait"]);
        let started = Instant::now();
        let error =
            run_command(&mut command, "slow fixture", Duration::from_millis(100)).unwrap_err();
        assert!(error.contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
