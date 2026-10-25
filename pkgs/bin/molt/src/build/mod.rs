mod target;
pub(crate) mod ui;

use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::activation;
use crate::closure::query_diff;
use crate::config::Config;
use crate::nix::{NixBuildProgress, run, run_nix_command, strip_ansi};
use crate::report::config_diff;
use crate::report::{Report, ReportMetadata, remove_config_files, report_from_diff};

pub(crate) use target::{Target, active_profile, profile_target};

const BUILD_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);
const EVALUATION_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const LOG_TIMEOUT: Duration = Duration::from_secs(2 * 60);
const NIX: &str = "nix";

pub(crate) struct GcRoot {
    pub(crate) path: PathBuf,
}

impl Drop for GcRoot {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_file(&self.path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            eprintln!(
                "molt: failed to remove GC root {}: {error}",
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
    Finished(Box<Result<Report, String>>),
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
    let started = std::time::Instant::now();
    // Kept for the report; snapshots list each command's warnings so far, and nix repeats some
    // across evaluation and build.
    let warnings = RefCell::new(Vec::<String>::new());
    let nix_progress = |phase: BuildPhase, update: NixBuildProgress| {
        let mut known = warnings.borrow_mut();
        for warning in &update.warnings {
            if !known.contains(warning) {
                known.push(warning.clone());
            }
        }
        drop(known);
        progress(BuildUpdate::Nix(phase, Box::new(update)));
    };
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
        Command::new(NIX)
            .args(["eval", "--raw", "--log-format", "internal-json", "-v"])
            .arg(format!("{attr}.drvPath"))
            .current_dir(&config.flake_dir),
        "nix eval drvPath",
        cancellation,
        EVALUATION_TIMEOUT,
        |update| nix_progress(BuildPhase::Evaluate, update),
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
        Command::new(NIX)
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
        |update| nix_progress(BuildPhase::Build, update),
    )?;
    let output = PathBuf::from(
        String::from_utf8_lossy(&build.stdout)
            .split_whitespace()
            .next()
            .ok_or("nix build returned no output path")?,
    );
    let old = active_profile(target)?;
    let profile = profile_target(target)?;
    progress(BuildUpdate::Phase(
        BuildPhase::Compare,
        "Comparing the active and reviewed closures...",
    ));
    if cancellation.load(Ordering::Relaxed) {
        return Err(molt::command::CANCELLED.to_owned());
    }
    let diff =
        query_diff(&old, &output).map_err(|error| format!("closure diff failed: {error}"))?;
    if cancellation.load(Ordering::Relaxed) {
        return Err(molt::command::CANCELLED.to_owned());
    }
    let flake = flake.to_owned();
    let mut report = report_from_diff(
        ReportMetadata {
            target,
            flake,
            flake_dir: config.flake_dir.clone(),
            baseline: old,
            profile,
            output,
            gc_root,
        },
        diff,
    );
    progress(BuildUpdate::Phase(
        BuildPhase::Compare,
        "Comparing generated configuration files...",
    ));
    report.config =
        match config_diff::compare(target, &report.baseline, &report.output, cancellation) {
            Err(error) if error == molt::command::CANCELLED => return Err(error),
            result => result.map(Arc::new),
        };
    if let Ok(config) = &report.config {
        remove_config_files(&mut report.changes, config);
    }
    if target == Target::NixOs {
        progress(BuildUpdate::Phase(
            BuildPhase::Compare,
            "Inspecting service and activation impact...",
        ));
        report.operational_impact = activation::analyze_nixos(&report.output, cancellation);
    }
    report.built_in = Some(started.elapsed());
    report.warnings = warnings.into_inner();
    Ok(report)
}

/// The build log nix kept for `derivation`, without terminal colours.
pub(crate) fn derivation_log(
    derivation: &str,
    cancellation: &AtomicBool,
) -> Result<String, String> {
    let output = run(
        Command::new(NIX).arg("log").arg(derivation),
        "nix log",
        cancellation,
        LOG_TIMEOUT,
        256 * 1024 * 1024,
    )?;
    Ok(strip_ansi(&String::from_utf8_lossy(&output.stdout)))
}

fn create_gc_root(target: Target, generation: u64) -> Result<Arc<GcRoot>, String> {
    let runtime = env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).ok_or(
        "XDG_RUNTIME_DIR is unset; cannot protect the reviewed build from garbage collection",
    )?;
    let directory = runtime.join("molt");
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
