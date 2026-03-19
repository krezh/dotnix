use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::AppSettings;

const CONFIG_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedGames {
    pub version: String,
    pub games: HashMap<String, GameEntry>,
    pub settings: AppSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEntry {
    pub name: String,
    pub installed: bool,
    pub optiscaler_version: Option<String>,
    pub proxy_dll: Option<String>,
    pub install_path: String,
    #[serde(default)]
    pub exe_path: Option<String>,
    pub installed_date: Option<DateTime<Utc>>,
    pub last_verified: Option<DateTime<Utc>>,
}

impl Default for TrackedGames {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION.to_string(),
            games: HashMap::new(),
            settings: AppSettings::default(),
        }
    }
}

impl TrackedGames {
    /// Gets the configuration file path.
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("optiman");

        fs::create_dir_all(&config_dir)
            .context("Failed to create config directory")?;

        Ok(config_dir.join("tracked_games.json"))
    }

    /// Loads tracked games from disk.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            tracing::info!("No existing config found, creating default");
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)
            .context("Failed to read tracked games file")?;

        let tracked_games: TrackedGames = serde_json::from_str(&contents)
            .context("Failed to parse tracked games JSON")?;

        tracing::info!("Loaded {} tracked games", tracked_games.games.len());
        Ok(tracked_games)
    }

    /// Saves tracked games to disk.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        let contents = serde_json::to_string_pretty(self)
            .context("Failed to serialize tracked games")?;

        fs::write(&path, contents)
            .context("Failed to write tracked games file")?;

        tracing::debug!("Saved tracked games to {:?}", path);
        Ok(())
    }

    /// Adds or updates a game entry.
    pub fn upsert_game(&mut self, app_id: String, entry: GameEntry) {
        self.games.insert(app_id, entry);
    }

    /// Gets a game entry by app ID.
    pub fn get_game(&self, app_id: &str) -> Option<&GameEntry> {
        self.games.get(app_id)
    }

    /// Gets a mutable game entry by app ID.
    pub fn get_game_mut(&mut self, app_id: &str) -> Option<&mut GameEntry> {
        self.games.get_mut(app_id)
    }

    /// Removes a game entry.
    pub fn remove_game(&mut self, app_id: &str) -> Option<GameEntry> {
        self.games.remove(app_id)
    }
}
