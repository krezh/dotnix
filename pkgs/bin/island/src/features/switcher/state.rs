use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Instant};

use crate::{
    apps::{self, App},
    capture::Shots,
    components::{
        gallery::{self, NAME_W},
        measure::Measure,
    },
    features,
    picture::Picture,
    presence::{Presence, Slot},
    spring::SpringConfig,
    ui::TITLE_SIZE,
    wm::{Snapshot, Window},
};

/// How cards come into the grid and leave it.
const CARDS: SpringConfig = SpringConfig::new(420.0, 41.0, 1.0);

/// What the worker found of the windows: the windows, most recently focused first, pictures of
/// them, their apps by the compositor's name for each, and the apps' icons already read, by file.
#[derive(Clone, Default)]
pub struct Previews {
    pub windows: Arc<Vec<Window>>,
    pub shots: Arc<Shots>,
    pub apps: Arc<HashMap<String, App>>,
    pub icons: Arc<HashMap<PathBuf, Picture>>,
}

impl PartialEq for Previews {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.windows, &other.windows)
            && Arc::ptr_eq(&self.shots, &other.shots)
            && Arc::ptr_eq(&self.apps, &other.apps)
            && Arc::ptr_eq(&self.icons, &other.icons)
    }
}

/// Each island's switcher: the windows, most recently used first, and which is picked.
pub struct Local {
    windows: Arc<Vec<Window>>,
    focused_monitor: Option<String>,
    apps: Arc<HashMap<String, App>>,
    /// Lines each app's name takes on a card.
    names: HashMap<String, usize>,
    /// Lines each title takes as the caption, by the grid's width.
    captions: HashMap<(String, u32), usize>,
    cards: Presence<Window>,
    /// Each card's place in the grid, kept for a card leaving it.
    places: HashMap<String, usize>,
    open: bool,
    shown: bool,
    /// Opened by a key held with Alt, so letting go of Alt picks; opened otherwise, it waits.
    pub(super) held: bool,
    /// Showing only the windows of the app focused last.
    same: bool,
    pub(super) selected: usize,
    pub(super) first_row: usize,
}

impl Default for Local {
    fn default() -> Self {
        Self {
            windows: Arc::default(),
            focused_monitor: None,
            apps: Arc::default(),
            names: HashMap::new(),
            captions: HashMap::new(),
            cards: Presence::new(CARDS),
            places: HashMap::new(),
            open: false,
            shown: false,
            held: false,
            same: false,
            selected: 0,
            first_row: 0,
        }
    }
}

impl Local {
    pub(super) fn sync(&mut self, previews: &Previews, snapshot: Option<&Snapshot>) {
        self.apps = previews.apps.clone();
        self.focused_monitor = snapshot
            .and_then(Snapshot::focused)
            .map(|monitor| monitor.name.clone());
        if Arc::ptr_eq(&self.windows, &previews.windows) {
            return;
        }
        let picked = self.picked().map(|window| window.address.clone());
        self.windows = previews.windows.clone();
        if !self.shown {
            return;
        }
        let kept =
            picked.and_then(|address| self.showing().position(|window| window.address == address));
        self.refresh(kept.unwrap_or(self.selected));
    }

    fn showing(&self) -> impl Iterator<Item = &Window> {
        let first = self.windows.first().map(|window| &window.class);
        self.windows
            .iter()
            .filter(move |window| !self.same || Some(&window.class) == first)
    }

    pub(super) fn shown_count(&self) -> usize {
        self.showing().count()
    }

    /// The cards to draw, back to front: those leaving first, under the ones staying.
    pub(super) fn cards(&self) -> Vec<(&Slot<Window>, usize, bool)> {
        let mut cards: Vec<(&Slot<Window>, usize, bool)> = self
            .cards
            .iter()
            .filter_map(|slot| {
                let place = *self.places.get(&slot.key)?;
                Some((slot, place, slot.presence.target() > 0.5))
            })
            .collect();
        cards.sort_by_key(|(_, _, staying)| *staying);
        cards
    }

    pub(super) fn picked(&self) -> Option<&Window> {
        self.showing().nth(self.selected)
    }

    pub(super) fn app(&self, class: &str) -> App {
        self.apps.get(class).cloned().unwrap_or_default()
    }

    /// What `window` is called: its app's name, or else its class.
    pub(super) fn name(&self, window: &Window) -> String {
        self.apps.get(&window.class).map_or_else(
            || window.class.clone(),
            |app| apps::name(app, &window.class),
        )
    }

    pub(super) fn name_lines(&self, name: &str) -> usize {
        self.names.get(name).copied().unwrap_or(1)
    }

    pub(super) fn caption_lines(&self, title: &str) -> usize {
        let width = gallery::width(self.shown_count()).to_bits();
        self.captions
            .get(&(title.to_owned(), width))
            .copied()
            .unwrap_or(1)
    }

    pub(super) fn focused_monitor(&self) -> Option<&str> {
        self.focused_monitor.as_deref()
    }

    /// Lays the cards out again from the windows shown, picking `position`.
    fn refresh(&mut self, position: usize) {
        let items: Vec<(String, Window)> = self
            .showing()
            .map(|window| (window.address.clone(), window.clone()))
            .collect();
        self.cards.sync(&items, self.shown);
        for (place, (key, _)) in items.iter().enumerate() {
            self.places.insert(key.clone(), place);
        }
        let cards = &self.cards;
        self.places
            .retain(|key, _| cards.iter().any(|slot| &slot.key == key));
        self.select(position);
    }

    /// The identifiers of the windows on the rows in view.
    pub(super) fn in_view(&self) -> Vec<String> {
        let columns = gallery::columns(self.shown_count());
        self.showing()
            .skip(self.first_row * columns)
            .take(gallery::ROWS * columns)
            .map(|window| window.id.clone())
            .collect()
    }

    /// The last row the grid can scroll to, for `count` cards.
    fn last_row(count: usize) -> usize {
        count
            .div_ceil(gallery::columns(count))
            .max(1)
            .saturating_sub(gallery::ROWS)
    }

    pub(super) fn select(&mut self, position: usize) {
        let count = self.shown_count();
        self.selected = position.min(count.saturating_sub(1));
        let row = self.selected / gallery::columns(count);
        if row < self.first_row {
            self.first_row = row;
        } else if row >= self.first_row + gallery::ROWS {
            self.first_row = row + 1 - gallery::ROWS;
        }
        self.first_row = self.first_row.min(Self::last_row(count));
    }

    /// Goes round from the last to the first, and back.
    pub(super) fn step(&mut self, step: i32) {
        let count = self.shown_count() as i32;
        if count > 0 {
            self.select((self.selected as i32 + step).rem_euclid(count) as usize);
        }
    }

    /// Scrolls by whole rows, leaving the pick where it is.
    pub(super) fn scroll(&mut self, rows: i32) {
        let last = Self::last_row(self.shown_count());
        self.first_row = (self.first_row as i32 + rows).clamp(0, last as i32) as usize;
    }

    /// Only the windows of the app focused last, or all of them again, picking the one used
    /// before the focused window.
    pub(super) fn toggle_same(&mut self) {
        self.same = !self.same;
        self.first_row = 0;
        self.refresh(1);
    }

    /// Takes a window out at once, as it is being closed.
    pub(super) fn forget(&mut self, address: &str) {
        Arc::make_mut(&mut self.windows).retain(|window| window.address != address);
        self.refresh(self.selected);
    }
}

impl features::Local for Local {
    fn advance(&mut self, dt: f32, now: Instant) {
        self.cards.advance(dt, now);
    }

    fn is_settled(&self) -> bool {
        self.cards.is_settled()
    }

    fn panel_opened(&mut self) {
        self.open = false;
        self.shown = true;
        self.held = false;
        self.same = false;
        self.first_row = 0;
        self.refresh(1);
    }

    fn panel_closed(&mut self) {
        self.open = false;
        self.shown = false;
        self.same = false;
        self.selected = 0;
        self.first_row = 0;
    }

    /// `next` and `prev` step through the windows, `same` keeps to the focused app's; opening on
    /// one picks the window used before the focused one, or the last for `prev`.
    fn show(&mut self, view: &str) {
        let opening = !self.open;
        self.open = true;
        self.held = true;
        if opening {
            if view == "same" {
                self.toggle_same();
            } else if view == "prev" {
                self.select(self.shown_count().saturating_sub(1));
            }
            return;
        }
        match view {
            "prev" => self.step(-1),
            "same" => self.toggle_same(),
            _ => self.step(1),
        }
    }

    fn measure(&mut self, measure: &Measure<'_>) -> bool {
        let mut measured = false;
        for window in self.windows.iter() {
            let name = self.name(window);
            if !self.names.contains_key(&name) {
                let lines = measure.wrapped_lines(&name, TITLE_SIZE, NAME_W);
                self.names.insert(name, lines.max(1));
                measured = true;
            }
        }
        let width = gallery::width(self.shown_count());
        if let Some(title) = self.picked().map(|window| window.title.clone()) {
            let key = (title, width.to_bits());
            if !self.captions.contains_key(&key) {
                if self.captions.len() > 64 {
                    self.captions.clear();
                }
                let lines = measure.wrapped_lines(&key.0, TITLE_SIZE, width);
                self.captions.insert(key, lines.max(1));
                measured = true;
            }
        }
        measured
    }
}

#[cfg(test)]
#[path = "state_test.rs"]
mod tests;
