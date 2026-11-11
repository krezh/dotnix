//! Which of the features' activities an island shows, and when: running ones, an ended one a
//! little while or until its panel opens on any island, the most pressing first, and what is said
//! over a fullscreen window.

use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use crate::{
    activity::{Activity, Standing},
    panel::SharedString,
};

/// How long a routine result shows before it folds away.
pub const DONE_FOR: Duration = Duration::from_secs(4);
/// How long the ring over a fullscreen window widens to say what changed.
pub const ANNOUNCE_FOR: Duration = Duration::from_secs(3);

#[derive(Default)]
pub struct Activities {
    /// Most pressing first, with the feature each belongs to.
    pub shown: Vec<(&'static str, Activity)>,
    /// When each ended one was first had.
    ended: HashMap<SharedString, Instant>,
    /// Ended ones whose panel has opened since.
    seen: HashSet<SharedString>,
    /// What was said last, so the same thing is not said twice.
    told: Option<(SharedString, Standing)>,
    announce_until: Option<Instant>,
    /// Seen here and not yet told to the other islands.
    newly_seen: Vec<SharedString>,
    /// Which of `shown` the ring over a fullscreen window shows.
    ringed: Option<usize>,
}

impl Activities {
    /// Takes what every feature has under way; `open` is the feature whose panel shows, which has
    /// seen what it shows.
    pub fn update(&mut self, all: Vec<(&'static str, Activity)>, open: Option<&str>, now: Instant) {
        self.ended
            .retain(|key, _| all.iter().any(|(_, activity)| activity.key == *key));
        self.seen
            .retain(|key| all.iter().any(|(_, activity)| activity.key == *key));
        let mut shown: Vec<(&'static str, Activity)> = all
            .into_iter()
            .filter(|(owner, activity)| {
                if activity.standing == Standing::Running {
                    return true;
                }
                let since = *self.ended.entry(activity.key.clone()).or_insert(now);
                if open == Some(*owner) && self.seen.insert(activity.key.clone()) {
                    self.newly_seen.push(activity.key.clone());
                }
                !self.seen.contains(&activity.key)
                    && (activity.standing.waits() || now < since + DONE_FOR)
            })
            .collect();
        shown.sort_by_key(|(_, activity)| std::cmp::Reverse(activity.standing));
        let top = shown
            .first()
            .map(|(_, activity)| (activity.key.clone(), activity.standing));
        if let Some((key, standing)) = &top
            && self.told.as_ref() != top.as_ref()
            && *standing != Standing::Done
        {
            self.announce_until = Some(now + ANNOUNCE_FOR);
            self.told = Some((key.clone(), *standing));
        }
        if top.is_none() {
            self.told = None;
            self.announce_until = None;
        }
        self.shown = shown;
        self.ring(now);
    }

    /// What was seen on this island since last asked, for the others to stop showing too.
    pub fn take_newly_seen(&mut self) -> Vec<SharedString> {
        std::mem::take(&mut self.newly_seen)
    }

    /// Seen on another island; whether that hides anything here.
    pub fn see(&mut self, keys: &[SharedString]) -> bool {
        let mut hides = false;
        for key in keys {
            hides |= self.seen.insert(key.clone())
                && self.shown.iter().any(|(_, activity)| activity.key == *key);
        }
        hides
    }

    /// Whether the ring over a fullscreen window says what changed.
    pub fn announcing(&self, now: Instant) -> bool {
        self.announce_until.is_some_and(|until| now < until)
    }

    /// Over a fullscreen window the ring stays only for work under way; a result says what it is,
    /// then folds away, still waiting in the bubble once the window leaves fullscreen.
    pub fn ring(&mut self, now: Instant) {
        self.ringed = if self.announcing(now) && !self.shown.is_empty() {
            Some(0)
        } else {
            self.shown
                .iter()
                .position(|(_, activity)| activity.standing == Standing::Running)
        };
    }

    pub fn ringed(&self) -> Option<&Activity> {
        self.ringed
            .and_then(|at| self.shown.get(at))
            .map(|(_, activity)| activity)
    }

    pub fn running(&self) -> bool {
        self.shown
            .iter()
            .any(|(_, activity)| activity.standing == Standing::Running)
    }

    /// When what shows changes next without new state: a result folding away, or what was said
    /// going quiet.
    pub fn due(&self, now: Instant) -> Option<Instant> {
        let folds = self
            .shown
            .iter()
            .filter(|(_, activity)| activity.standing == Standing::Done)
            .filter_map(|(_, activity)| self.ended.get(&activity.key))
            .map(|since| *since + DONE_FOR);
        folds
            .chain(self.announce_until)
            .filter(|at| *at > now)
            .min()
    }
}

#[cfg(test)]
#[path = "activities_test.rs"]
mod tests;
