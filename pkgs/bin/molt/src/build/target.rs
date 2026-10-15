//! What a build targets: NixOS or Home Manager, and the profile each currently has active.

use std::env;
use std::fs;
use std::path::PathBuf;

use crate::config::Config;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
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

    pub(super) const fn evaluation_message(self) -> &'static str {
        match self {
            Self::HomeManager => "Evaluating the Home Manager configuration...",
            Self::NixOs => "Evaluating the NixOS configuration...",
        }
    }

    pub(super) const fn build_message(self) -> &'static str {
        match self {
            Self::HomeManager => "Building the Home Manager configuration...",
            Self::NixOs => "Building the NixOS configuration...",
        }
    }

    pub(crate) const fn slug(self) -> &'static str {
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

/// The profile an activation of `target` moves, as it points now. For NixOS this is the system
/// profile, which differs from the running system once a configuration is staged for the next
/// boot; Home Manager activates its profile directly.
pub(crate) fn profile_target(target: Target) -> Result<PathBuf, String> {
    match target {
        Target::NixOs => fs::canonicalize("/nix/var/nix/profiles/system")
            .map_err(|error| format!("could not resolve the NixOS system profile: {error}")),
        Target::HomeManager => active_profile(target),
    }
}
