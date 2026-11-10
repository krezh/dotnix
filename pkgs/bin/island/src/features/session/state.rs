use std::{path::PathBuf, time::Instant};

use super::Action;
use crate::icons;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// The action waiting for its confirming click.
    pub armed: Option<Action>,
    pub leaving: Option<Leaving>,
}

/// The session ending: its apps closing, then the island switching off before the action.
#[derive(Clone, Debug, PartialEq)]
pub struct Leaving {
    pub action: Action,
    pub since: Instant,
    /// How many apps were open when it began.
    pub total: usize,
    /// Those still open.
    pub apps: Vec<App>,
    pub phase: Phase,
    /// Shows it all without closing anything or ending the session.
    pub rehearsal: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    /// Finding what is open.
    Finding,
    Closing,
    /// Something went wrong, so only cancelling is safe.
    Failed(String),
    /// Every app is gone; the island switches off, then the action runs.
    Off,
}

#[derive(Clone, Debug, PartialEq)]
pub struct App {
    pub key: String,
    pub name: String,
    pub title: String,
    pub icon: Option<PathBuf>,
    pub mood: Mood,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    Closing,
    /// Still open well after it was asked.
    Lingering,
    Killed,
}

impl Leaving {
    pub fn open(&self) -> usize {
        self.apps.len()
    }
}

impl App {
    pub fn new(key: String, class: &str, title: &str) -> Self {
        Self {
            key,
            name: name_of(class),
            title: title.to_owned(),
            icon: icon_of(class),
            mood: Mood::Closing,
        }
    }
}

/// A window class as people know the app: `dev.zed.Zed` is Zed, `com.mitchellh.ghostty` Ghostty.
pub fn name_of(class: &str) -> String {
    let last = class.rsplit('.').next().unwrap_or(class);
    let last = if last.is_empty() { class } else { last };
    let mut letters = last.chars();
    letters
        .next()
        .map(|first| first.to_uppercase().chain(letters).collect())
        .unwrap_or_default()
}

fn icon_of(class: &str) -> Option<PathBuf> {
    let lower = class.to_lowercase();
    let last = lower.rsplit('.').next().unwrap_or(&lower).to_owned();
    [class.to_owned(), lower, last]
        .iter()
        .filter(|name| !name.is_empty())
        .find_map(|name| icons::lookup(name))
}

#[cfg(test)]
#[path = "state_test.rs"]
mod tests;
