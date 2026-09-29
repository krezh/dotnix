use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use crate::config::Config;
use crate::nix::{NixBuildProgress, run, run_nix_command};
use crate::report::{Report, ReportMetadata, parse_report};
const BUILD_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);
const EVALUATION_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const DIFF_TIMEOUT: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    HomeManager,
    NixOs,
}

impl Target {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::HomeManager => "Home Manager",
            Self::NixOs => "NixOS",
        }
    }

    const fn evaluation_message(self) -> &'static str {
        match self {
            Self::HomeManager => "Evaluating the Home Manager configuration...",
            Self::NixOs => "Evaluating the NixOS configuration...",
        }
    }

    const fn build_message(self) -> &'static str {
        match self {
            Self::HomeManager => "Building the Home Manager configuration...",
            Self::NixOs => "Building the NixOS configuration...",
        }
    }

    pub(crate) const fn progress_message(self) -> &'static str {
        match self {
            Self::HomeManager => "Evaluating Home Manager and computing changes...",
            Self::NixOs => "Evaluating NixOS and computing changes...",
        }
    }

    const fn slug(self) -> &'static str {
        match self {
            Self::HomeManager => "home-manager",
            Self::NixOs => "nixos",
        }
    }

    pub(crate) fn flake(self, config: &Config) -> Result<&str, &'static str> {
        match self {
            Self::HomeManager => config
                .home_flake
                .as_deref()
                .ok_or("Home Manager target is disabled"),
            Self::NixOs => Ok(&config.nixos_flake),
        }
    }
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
    Nix(BuildPhase, Box<NixBuildProgress>),
}

pub(crate) fn build_report(
    config: &Config,
    target: Target,
    generation: u64,
    cancellation: &AtomicBool,
    progress: impl Fn(BuildUpdate),
) -> Result<Report, String> {
    let flake = target.flake(config)?;
    let attr = match target {
        Target::HomeManager => format!(".#homeConfigurations.{flake}.activationPackage"),
        Target::NixOs => format!(".#nixosConfigurations.{flake}.config.system.build.toplevel"),
    };
    progress(BuildUpdate::Phase(
        BuildPhase::Evaluate,
        target.evaluation_message(),
    ));
    let derivation = run_nix_command(
        Command::new("nix")
            .args(["eval", "--raw", "--log-format", "internal-json", "-v"])
            .arg(format!("{attr}.drvPath"))
            .current_dir(&config.flake_dir),
        "nix eval drvPath",
        cancellation,
        EVALUATION_TIMEOUT,
        |update| progress(BuildUpdate::Nix(BuildPhase::Evaluate, Box::new(update))),
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
        target.build_message(),
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
        |update| progress(BuildUpdate::Nix(BuildPhase::Build, Box::new(update))),
    )?;
    let output = PathBuf::from(
        String::from_utf8_lossy(&build.stdout)
            .split_whitespace()
            .next()
            .ok_or("nix build returned no output path")?,
    );
    let old = active_profile(target)?;
    progress(BuildUpdate::Phase(
        BuildPhase::Compare,
        "Comparing the active and reviewed closures...",
    ));
    let diff = run(
        Command::new("dix")
            .arg("--output=json")
            .arg(&old)
            .arg(&output),
        "dix",
        cancellation,
        DIFF_TIMEOUT,
        64 * 1024 * 1024,
    )?;
    let flake = flake.to_owned();
    parse_report(
        ReportMetadata {
            target,
            flake,
            flake_dir: config.flake_dir.clone(),
            baseline: old,
            output,
            gc_root,
        },
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
    let target = target.slug();
    Ok(Arc::new(GcRoot {
        path: directory.join(format!(
            "reviewed-{target}-{}-{generation}",
            std::process::id()
        )),
    }))
}

pub(crate) fn active_profile(target: Target) -> Result<PathBuf, String> {
    match target {
        Target::NixOs => fs::canonicalize("/run/current-system")
            .map_err(|error| format!("could not resolve the active NixOS profile: {error}")),
        Target::HomeManager => {
            let home = env::var_os("HOME").ok_or("HOME is unset")?;
            let state = env::var_os("XDG_STATE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(home).join(".local/state"));
            fs::canonicalize(state.join("nix/profiles/home-manager")).map_err(|error| {
                format!("could not resolve the active Home Manager profile: {error}")
            })
        }
    }
}
