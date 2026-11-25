//! What Home knows and how it changes: the figures and cards it is loaded with, and the repository
//! card's check and pull.

use std::sync::atomic::AtomicBool;

use crate::build::Target;
use crate::cleanup::CleanupEstimate;
use crate::config::{Config, load_config};
use crate::home::repository::{self, RepositoryStatus};
use crate::home::system::{self, SystemStatus};
use crate::inputs::{self, FlakeInput};
use crate::watchlist::db::{WatchlistSummary, open_db, summary};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Repository {
    /// Whether Pull stays on show from before the check, as it does in molt.
    Checking { pull: bool },
    Ready(RepositoryStatus),
    CheckFailed(String),
    /// Pulling; the card keeps showing what it showed before.
    Updating(Box<Repository>),
    UpdateFailed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Home {
    pub config: Result<Config, String>,
    pub system: SystemStatus,
    pub inputs: Option<Vec<FlakeInput>>,
    pub cleanup: Option<CleanupEstimate>,
    pub watchlist: WatchlistSummary,
    pub repository: Repository,
    /// Targets with a reviewed build still waiting to be switched to.
    pub reviewed: Vec<Target>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    Check,
    Pull,
    Checked(Result<RepositoryStatus, String>),
    Pulled(Result<RepositoryStatus, String>),
}

/// Work Home asks its front end to run, whose result comes back as a [`Msg`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Check,
    Pull,
}

impl Home {
    /// Reads everything Home shows, and asks for the repository check it opens with.
    pub fn load(reviewed: Vec<Target>) -> (Self, Option<Effect>) {
        let config = load_config();
        let inputs = config
            .as_ref()
            .ok()
            .and_then(|config| inputs::load(&config.flake_dir).ok())
            .map(|(_, inputs)| inputs);
        let watchlist = open_db()
            .ok()
            .flatten()
            .and_then(|conn| summary(&conn).ok())
            .unwrap_or_default();
        let effect = config.is_ok().then_some(Effect::Check);
        let home = Self {
            config,
            system: system::system_status(),
            inputs,
            cleanup: CleanupEstimate::load(),
            watchlist,
            repository: Repository::Checking { pull: false },
            reviewed,
        };
        (home, effect)
    }

    pub fn busy(&self) -> bool {
        matches!(
            self.repository,
            Repository::Checking { .. } | Repository::Updating(_)
        )
    }

    pub fn update(&mut self, msg: Msg) -> Option<Effect> {
        match msg {
            Msg::Check if !self.busy() => {
                let pull = self.can_pull();
                self.repository = Repository::Checking { pull };
                Some(Effect::Check)
            }
            Msg::Pull if self.can_pull() => {
                let before = std::mem::replace(
                    &mut self.repository,
                    Repository::Checking { pull: false },
                );
                self.repository = Repository::Updating(Box::new(before));
                Some(Effect::Pull)
            }
            Msg::Checked(result) if matches!(self.repository, Repository::Checking { .. }) => {
                self.repository = match result {
                    Ok(status) => Repository::Ready(status),
                    Err(error) => Repository::CheckFailed(error),
                };
                None
            }
            Msg::Pulled(result) if matches!(self.repository, Repository::Updating(_)) => {
                self.repository = match result {
                    Ok(status) => Repository::Ready(status),
                    Err(error) => Repository::UpdateFailed(error),
                };
                None
            }
            _ => None,
        }
    }

    pub fn can_pull(&self) -> bool {
        match &self.repository {
            Repository::Ready(status) => status.incoming > 0,
            Repository::UpdateFailed(_) => true,
            _ => false,
        }
    }
}

/// Runs an [`Effect`] to the end, on the caller's thread.
pub fn run(effect: Effect, home: &Home, cancellation: &AtomicBool) -> Msg {
    let Ok(config) = &home.config else {
        return match effect {
            Effect::Check => Msg::Checked(Err("Molt has no configuration".to_owned())),
            Effect::Pull => Msg::Pulled(Err("Molt has no configuration".to_owned())),
        };
    };
    match effect {
        Effect::Check => Msg::Checked(repository::check(&config.flake_dir, true, cancellation)),
        Effect::Pull => Msg::Pulled(repository::update(&config.flake_dir, cancellation)),
    }
}

#[cfg(test)]
#[path = "flow_test.rs"]
mod tests;
