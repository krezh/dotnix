use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use crate::config::Config;
use crate::nix::{NixBuildProgress, run, run_nix_command};
use crate::report::{Report, parse_report};
const BUILD_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);
const EVALUATION_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const DIFF_TIMEOUT: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    HomeManager,
    NixOs,
}

pub(crate) struct GcRoot {
    pub(crate) path: PathBuf,
}

impl Drop for GcRoot {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_file(&self.path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            eprintln!(
                "swix: failed to remove GC root {}: {error}",
                self.path.display()
            );
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BuildPhase {
    Evaluate,
    Build,
    Compare,
}

impl BuildPhase {
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Evaluate => 0,
            Self::Build => 1,
            Self::Compare => 2,
        }
    }
}
pub(crate) enum BuildEvent {
    Update(BuildUpdate),
    Finished(Result<Report, String>),
}

pub(crate) enum BuildUpdate {
    Phase(BuildPhase, &'static str),
    Nix(Box<NixBuildProgress>),
}

pub(crate) fn build_report(
    config: &Config,
    target: Target,
    generation: u64,
    cancellation: &AtomicBool,
    progress: impl Fn(BuildUpdate),
) -> Result<Report, String> {
    let attr = match target {
        Target::HomeManager => format!(
            ".#homeConfigurations.{}.activationPackage",
            config
                .home_flake
                .as_deref()
                .ok_or("Home Manager target is disabled")?
        ),
        Target::NixOs => format!(
            ".#nixosConfigurations.{}.config.system.build.toplevel",
            config.nixos_flake
        ),
    };
    progress(BuildUpdate::Phase(
        BuildPhase::Evaluate,
        match target {
            Target::HomeManager => "Evaluating the Home Manager configuration...",
            Target::NixOs => "Evaluating the NixOS configuration...",
        },
    ));
    let derivation = run_nix_command(
        Command::new("nix")
            .args(["eval", "--raw", "--log-format", "internal-json", "-v"])
            .arg(format!("{attr}.drvPath"))
            .current_dir(&config.flake_dir),
        "nix eval drvPath",
        cancellation,
        EVALUATION_TIMEOUT,
        |update| progress(BuildUpdate::Nix(Box::new(update))),
    )?;
    let derivation = String::from_utf8_lossy(&derivation.stdout);
    let derivation = derivation.trim();
    if !derivation.starts_with("/nix/store/") || !derivation.ends_with(".drv") {
        return Err(format!(
            "nix eval returned an invalid derivation path: {derivation}"
        ));
    }
    progress(BuildUpdate::Phase(
        BuildPhase::Build,
        match target {
            Target::HomeManager => "Building the Home Manager configuration...",
            Target::NixOs => "Building the NixOS configuration...",
        },
    ));
    let gc_root = create_gc_root(target, generation)?;
    let build = run_nix_command(
        Command::new("nix")
            .args([
                "build",
                "--print-out-paths",
                "--log-format",
                "internal-json",
                "-v",
                "--out-link",
            ])
            .arg(&gc_root.path)
            .arg(format!("{derivation}^out"))
            .current_dir(&config.flake_dir),
        "nix build",
        cancellation,
        BUILD_TIMEOUT,
        |update| progress(BuildUpdate::Nix(Box::new(update))),
    )?;
    let output = PathBuf::from(
        String::from_utf8_lossy(&build.stdout)
            .split_whitespace()
            .next()
            .ok_or("nix build returned no output path")?,
    );
    let old = match target {
        Target::NixOs => PathBuf::from("/run/current-system"),
        Target::HomeManager => home_manager_profile()?,
    };
    progress(BuildUpdate::Phase(
        BuildPhase::Compare,
        "Comparing the active and reviewed closures...",
    ));
    let diff = run(
        Command::new("dix")
            .arg("--output=json")
            .arg(old)
            .arg(&output),
        "dix",
        cancellation,
        DIFF_TIMEOUT,
        64 * 1024 * 1024,
    )?;
    let flake = match target {
        Target::HomeManager => config
            .home_flake
            .clone()
            .ok_or("Home Manager target is disabled")?,
        Target::NixOs => config.nixos_flake.clone(),
    };
    parse_report(
        target,
        flake,
        config.flake_dir.clone(),
        config.home_flake.is_some(),
        output,
        gc_root,
        &diff.stdout,
    )
}

fn create_gc_root(target: Target, generation: u64) -> Result<Arc<GcRoot>, String> {
    let runtime = env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).ok_or(
        "XDG_RUNTIME_DIR is unset; cannot protect the reviewed build from garbage collection",
    )?;
    let directory = runtime.join("swix");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("failed to create {}: {error}", directory.display()))?;
    let target = match target {
        Target::HomeManager => "home-manager",
        Target::NixOs => "nixos",
    };
    Ok(Arc::new(GcRoot {
        path: directory.join(format!(
            "reviewed-{target}-{}-{generation}",
            std::process::id()
        )),
    }))
}

fn home_manager_profile() -> Result<PathBuf, String> {
    let home = env::var_os("HOME").ok_or("HOME is unset")?;
    let state = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(home).join(".local/state"));
    fs::canonicalize(state.join("nix/profiles/home-manager"))
        .map_err(|error| format!("could not resolve the active Home Manager profile: {error}"))
}
